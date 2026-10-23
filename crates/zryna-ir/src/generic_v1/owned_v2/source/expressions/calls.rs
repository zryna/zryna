//! Exact original call substitutions use the admission-specific materializer.
use super::super::{Builder, Value};
use crate::generic_v1::{
    Failure, raw, reject, reserve,
    source::Target,
    source_types::{Closed, Resolver},
};
impl Builder<'_, '_> {
    pub(super) fn call(
        &mut self,
        callee: &zryna_syntax::v4::RawIdentifierSyntax,
        type_arguments: Option<&zryna_syntax::v5::RawTypeArgumentList>,
        arguments: &[u32],
        span: zryna_source::UntrustedSpan,
        depth: usize,
    ) -> Result<Value, Failure> {
        if self.locals.iter().any(|local| local.name == callee.text) {
            return Err(reject(
                "source call names a lexical value rather than its shadowed original function",
            ));
        }
        let Target::Function(module, function) = (if self.type_facts.admission()
            == super::super::Admission::StructV4
        {
            self.resolver
                .originals
                .resolve_v4(self.resolver.module, &callee.text, &mut |n| self.budget.nominal(n))?
        } else {
            self.resolver.originals.resolve(self.resolver.module, &callee.text)?
        }) else {
            return Err(reject("source call target is not an original function"));
        };
        let original = &self.resolver.originals.units[module as usize].functions[function as usize];
        let arity = original.type_parameters.as_ref().map_or(0, |list| list.parameters.len());
        if arity != type_arguments.map_or(0, |list| list.arguments.len())
            || (arity == 0 && type_arguments.is_some())
        {
            return Err(reject("source call requires the exact explicit generic arity"));
        }
        let mut types = reserve(arity)?;
        for occurrence in type_arguments.map_or(&[][..], |list| &list.arguments) {
            let Closed::Stored(key) = self.resolve(*occurrence)? else {
                return Err(reject("generic call argument is not a stored value type"));
            };
            types.push(key);
        }
        let mut keys = reserve(arity)?;
        keys.extend(types.iter().map(Vec::as_slice));
        let resolver = Resolver {
            originals: self.resolver.originals,
            module,
            parameters: original.type_parameters.as_ref(),
            arguments: keys,
        };
        if arguments.len() != original.parameters.len() {
            return Err(reject("source call has wrong value arity"));
        }
        let mut operands = reserve(arguments.len())?;
        for (argument, parameter) in arguments.iter().zip(&original.parameters) {
            let value = self.expression(*argument, depth + 1)?;
            if value.ty
                != self.type_facts.resolve(
                    &resolver,
                    super::super::facts::Frame { kind: 0, module, index: function },
                    parameter.type_syntax,
                    true,
                    &mut self.budget,
                )?
            {
                return Err(reject("source call operand is not exact original substitution"));
            }
            operands.push(value.id);
        }
        let result = self.type_facts.resolve(
            &resolver,
            super::super::facts::Frame { kind: 0, module, index: function },
            original.result_type,
            true,
            &mut self.budget,
        )?;
        let operation = self.call_operation(module, function, &types, operands)?;
        self.emit(result, span, operation)
    }
    fn call_operation(
        &mut self,
        module: u32,
        function: u32,
        types: &[Vec<u8>],
        operands: Vec<u32>,
    ) -> Result<raw::Operation, Failure> {
        Ok(if types.is_empty() {
            raw::Operation::SourceCall { module, function, arguments: operands }
        } else {
            let instance = if self.symbolic {
                0
            } else {
                if self.type_facts.admission() == super::super::Admission::StructV4 {
                    let bytes = types
                        .iter()
                        .try_fold(13usize, |n, key| n.checked_add(4)?.checked_add(key.len()))
                        .ok_or(Failure::InternalFailure)?;
                    let probes = usize::BITS - self.program.functions.len().leading_zeros() + 1;
                    self.budget.nominal(
                        bytes.checked_mul(probes as usize + 1).ok_or(Failure::InternalFailure)?,
                    )?;
                }
                let key = function_key(module, function, types)?;
                let index = self
                    .program
                    .functions
                    .binary_search_by(|candidate| candidate.key.cmp(&key))
                    .map_err(|_| reject("complete demanded generic instance is absent"))?;
                u32::try_from(index).map_err(|_| Failure::InternalFailure)?
            };
            raw::Operation::ClosedGenericCall { instance, arguments: operands }
        })
    }
}

pub(super) fn function_key(
    module: u32,
    function: u32,
    arguments: &[Vec<u8>],
) -> Result<Vec<u8>, Failure> {
    let count = arguments
        .iter()
        .try_fold(13usize, |length, key| length.checked_add(4)?.checked_add(key.len()))
        .ok_or(Failure::InternalFailure)?;
    if count > 4096 {
        return Err(crate::generic_v1::budget("source call key exceeds 4096 bytes"));
    }
    let mut key = reserve(count)?;
    key.push(0x40);
    for lane in
        [module, function, u32::try_from(arguments.len()).map_err(|_| Failure::InternalFailure)?]
    {
        key.extend_from_slice(&lane.to_le_bytes());
    }
    for argument in arguments {
        key.extend_from_slice(
            &u32::try_from(argument.len()).map_err(|_| Failure::InternalFailure)?.to_le_bytes(),
        );
        key.extend_from_slice(argument);
    }
    Ok(key)
}
