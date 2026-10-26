use super::{
    Body, Closed, DeferredCall, Failure, Resolver, Target, UntrustedSpan, Value, charge, exhausted,
    lookup, originals, raw, reject, reserve,
};
use zryna_syntax::{v4::RawIdentifierSyntax, v5::RawTypeArgumentList};
enum Prepared {
    Value(Value),
    Loan(u32),
}
impl Body<'_, '_> {
    pub(super) fn call(
        &mut self,
        callee: &RawIdentifierSyntax,
        type_arguments: Option<&RawTypeArgumentList>,
        arguments: &[u32],
        span: UntrustedSpan,
        depth: usize,
    ) -> Result<Value, Failure> {
        for b in &self.state.locals {
            charge(self.spent, b.name.len().min(callee.text.len()) + 1)?;
            if b.name == callee.text {
                return Err(reject("call is shadowed by lexical value"));
            }
        }
        if callee.text == "concat" {
            return self.concat(arguments, type_arguments.is_some(), callee.span, span, depth);
        }
        let Target::Function(module, function) =
            self.originals
                .resolve_v4(self.resolver.module, &callee.text, &mut |n| charge(self.spent, n))?
        else {
            return Err(reject("source call target is not a function"));
        };
        charge(self.spent, 1)?;
        let original = &self.originals.units[module as usize].functions[function as usize];
        let arity = original.type_parameters.as_ref().map_or(0, |p| p.parameters.len());
        let supplied = type_arguments.map_or(&[][..], |l| l.arguments.as_slice());
        if arity != supplied.len() || arguments.len() != original.parameters.len() {
            return Err(reject("source call arity differs"));
        }
        charge(self.spent, arity * 2 + arguments.len())?;
        let mut types = reserve(arity)?;
        for at in supplied {
            let Closed::Stored(key) = originals::resolve(&self.resolver, *at, self.spent)? else {
                return Err(reject("generic argument is not stored"));
            };
            let id = lookup(&self.program.type_keys, &key, self.spent)?;
            self.used[id as usize] = true;
            types.push(key);
        }
        let mut borrowed = reserve(arity)?;
        borrowed.extend(types.iter().map(Vec::as_slice));
        let resolver = Resolver {
            originals: self.originals,
            module,
            parameters: original.type_parameters.as_ref(),
            arguments: borrowed,
        };
        let mark = self.output.loans.len();
        let mut prepared = reserve(arguments.len())?;
        for (at, parameter) in arguments.iter().zip(&original.parameters) {
            let expected = originals::resolve(&resolver, parameter.type_syntax, self.spent)?;
            if let Closed::Borrow(key, exclusive) = expected {
                let referent = self.loan_referent(&key)?;
                let id = self.loan_argument(*at, referent, exclusive, span, depth + 1)?;
                prepared.push(Prepared::Loan(id));
            } else {
                let value = self.expression(*at, depth + 1)?;
                if value.ty != self.closed_type(expected)? {
                    return Err(reject("call operand differs from original substitution"));
                }
                prepared.push(Prepared::Value(value));
            }
        }
        let result = originals::resolve(&resolver, original.result_type, self.spent)?;
        let result = self.closed_type(result)?;
        let target = self.call_target(module, function, &types)?;
        charge(self.spent, prepared.len())?;
        let mut operands = reserve(prepared.len())?;
        for value in prepared {
            match value {
                Prepared::Value(value) => {
                    self.consume(value, span)?;
                    operands.push(raw::CallArgument::Value(value.id));
                }
                Prepared::Loan(id) => {
                    self.require_live_loan(id, span)?;
                    operands.push(raw::CallArgument::Loan(id));
                }
            }
        }
        let actions = self.actions()?;
        charge(self.spent, 1)?;
        self.calls.try_reserve(1).map_err(|_| Failure::AllocationFailure)?;
        self.calls.push(DeferredCall {
            block: self.block,
            position: self.output.blocks[self.block].instructions.len(),
            span,
            actions,
            target,
        });
        let operation = if arity == 0 {
            raw::Operation::SourceCall { module, function, arguments: operands, failure: None }
        } else {
            raw::Operation::ClosedGenericCall {
                instance: u32::try_from(target).map_err(|_| Failure::InternalFailure)?,
                arguments: operands,
                failure: None,
            }
        };
        let value = self.emit(result, operation, span)?;
        self.end_created_loans(mark, span)?;
        Ok(value)
    }
    fn call_target(
        &mut self,
        module: u32,
        function: u32,
        types: &[Vec<u8>],
    ) -> Result<usize, Failure> {
        let arity = types.len();
        let length = if arity == 0 {
            9
        } else {
            types
                .iter()
                .try_fold(13usize, |n, k| n.checked_add(4)?.checked_add(k.len()))
                .ok_or(Failure::InternalFailure)?
        };
        if length > 4096 {
            return Err(exhausted("closed call key exceeds inherited cap"));
        }
        charge(self.spent, length)?;
        let mut key = reserve(length)?;
        key.push(if arity == 0 { 0x41 } else { 0x40 });
        key.extend_from_slice(&module.to_le_bytes());
        key.extend_from_slice(&function.to_le_bytes());
        if arity != 0 {
            key.extend_from_slice(
                &u32::try_from(arity).map_err(|_| Failure::InternalFailure)?.to_le_bytes(),
            );
            for t in types {
                key.extend_from_slice(
                    &u32::try_from(t.len()).map_err(|_| Failure::InternalFailure)?.to_le_bytes(),
                );
                key.extend_from_slice(t);
            }
        }
        let mut lo = 0;
        let mut hi = self.program.functions.len();
        let target = loop {
            if lo == hi {
                return Err(reject("complete demanded function instance is absent"));
            }
            let mid = lo + (hi - lo) / 2;
            let candidate = &self.program.functions[mid].key;
            charge(self.spent, candidate.len().min(key.len()) + 1)?;
            match candidate.cmp(&key) {
                std::cmp::Ordering::Less => lo = mid + 1,
                std::cmp::Ordering::Greater => hi = mid,
                std::cmp::Ordering::Equal => break mid,
            }
        };
        Ok(target)
    }
}
