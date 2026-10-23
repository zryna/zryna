//! Source function setup and type queries; called in canonical replay order.
use super::{
    Body, Closed, Domain, Failure, Originals, Resolver, SourceMap, State, TypeWorld, UntrustedSpan,
    charge, copy_key, diagnostic, keys, lookup, originals, raw, reject, reserve,
};
impl<'p, 's> Body<'p, 's> {
    pub(super) fn new(
        program: &'p raw::Program,
        originals: &'p Originals<'s>,
        sources: &'p SourceMap,
        world: &'p TypeWorld,
        fi: usize,
        used: &'p mut [bool],
        spent: &'p mut usize,
    ) -> Result<Self, Failure> {
        let key = &program.functions[fi].key;
        charge(
            spent,
            key.len()
                .checked_mul(4)
                .and_then(|n| n.checked_add(67))
                .ok_or(Failure::InternalFailure)?,
        )?;
        let decoded = keys::decode(
            key,
            if key.first() == Some(&0x40) { Domain::FunctionInstance } else { Domain::SourceRoot },
        )?;
        let (module, index) = decoded.declaration().ok_or(Failure::InternalFailure)?;
        let original = originals
            .units
            .get(module as usize)
            .and_then(|u| u.functions.get(index as usize))
            .ok_or_else(|| reject("closed function refers to absent original"))?;
        let count = original.type_parameters.as_ref().map_or(0, |p| p.parameters.len());
        if count != decoded.arguments().len() {
            return Err(reject("closed function argument arity differs"));
        }
        charge(spent, count)?;
        let mut arguments = reserve(count)?;
        for key in decoded.arguments() {
            let id = lookup(&program.type_keys, key, spent)?;
            used[id as usize] = true;
            arguments.push(key);
        }
        let resolver = Resolver {
            originals,
            module,
            parameters: original.type_parameters.as_ref(),
            arguments,
        };
        let output = raw::Function {
            key: copy_key(key, spent)?,
            original_span: original.span,
            executable_export: None,
            parameters: Vec::new(),
            result_type: raw::ValueType::Unit,
            places: Vec::new(),
            loans: Vec::new(),
            blocks: Vec::new(),
            cleanup_plans: Vec::new(),
        };
        let mut body = Self {
            program,
            originals,
            sources,
            world,
            resolver,
            original,
            function: fi,
            output,
            block: 0,
            next: 0,
            next_local: 0,
            owners: Vec::new(),
            loan_scopes: Vec::new(),
            loan_places: Vec::new(),
            regions: Vec::new(),
            state: State {
                locals: Vec::new(),
                live: Vec::new(),
                order: Vec::new(),
                loans: Vec::new(),
                loan_order: Vec::new(),
            },
            used,
            calls: Vec::new(),
            spent,
        };
        body.new_block()?;
        body.output.result_type = body.resolve(original.result_type)?;
        if key.first() == Some(&0x41)
            && module == program.entry_module
            && original.export_span.is_some()
        {
            charge(body.spent, original.name.text.len())?;
            let mut name = String::new();
            name.try_reserve(original.name.text.len()).map_err(|_| Failure::AllocationFailure)?;
            name.push_str(&original.name.text);
            body.output.executable_export = Some(name);
        }
        body.initialize_parameters()?;
        Ok(body)
    }
    fn initialize_parameters(&mut self) -> Result<(), Failure> {
        for (ordinal, p) in self.original.parameters.iter().enumerate() {
            let closed = originals::resolve(&self.resolver, p.type_syntax, self.spent)?;
            if let Closed::Borrow(key, exclusive) = closed {
                let ty = self.loan_referent(&key)?;
                let access = if exclusive { raw::Access::Exclusive } else { raw::Access::Shared };
                let id = self.add_loan(
                    ty,
                    access,
                    raw::LoanOrigin::Formal(
                        u32::try_from(ordinal).map_err(|_| Failure::InternalFailure)?,
                    ),
                    self.original.span,
                    p.span,
                    false,
                )?;
                charge(self.spent, 1)?;
                self.output.parameters.try_reserve(1).map_err(|_| Failure::AllocationFailure)?;
                self.output.parameters.push(raw::Parameter::Loan(raw::LoanDefinition {
                    id,
                    referent: ty,
                    access,
                }));
                self.bind_loan(&p.name.text, id, ty, exclusive)?;
                continue;
            }
            let ty = self.closed_type(closed)?;
            let raw::ValueType::Stored(ty) = ty else {
                return Err(self.error(
                    p.span,
                    "ZRYNA-M3008",
                    "private parameter must have a value or exact loan role",
                ));
            };
            let value = self.definition(raw::ValueType::Stored(ty), false, p.span)?;
            charge(self.spent, 1)?;
            self.output.parameters.try_reserve(1).map_err(|_| Failure::AllocationFailure)?;
            self.output.parameters.push(raw::Parameter::Value(value.clone()));
            let place = self.place(
                ty,
                raw::PlaceKind::Parameter(
                    u32::try_from(ordinal).map_err(|_| Failure::InternalFailure)?,
                ),
                p.span,
            )?;
            self.state.live[place as usize] = true;
            if !self.copy(ty)? {
                self.owners[value.id as usize] = Some(place);
                charge(self.spent, 1)?;
                self.state.order.try_reserve(1).map_err(|_| Failure::AllocationFailure)?;
                self.state.order.push(place);
            }
            self.bind(&p.name.text, place, ty)?;
        }
        Ok(())
    }
    pub(super) fn error(&self, span: UntrustedSpan, code: &'static str, message: &str) -> Failure {
        diagnostic(self.sources, span, code, message)
    }
    pub(super) fn copy(&mut self, ty: u32) -> Result<bool, Failure> {
        charge(self.spent, 1)?;
        self.world.fact(ty).map(|f| f.copy).ok_or(Failure::InternalFailure)
    }
    pub(super) fn resolve(&mut self, at: u32) -> Result<raw::ValueType, Failure> {
        let ty = originals::resolve(&self.resolver, at, self.spent)?;
        self.closed_type(ty)
    }
    pub(super) fn closed_type(&mut self, ty: Closed) -> Result<raw::ValueType, Failure> {
        match ty {
            Closed::Stored(key) => {
                let id = lookup(&self.program.type_keys, &key, self.spent)?;
                self.used[id as usize] = true;
                Ok(raw::ValueType::Stored(id))
            }
            Closed::Unit => Ok(raw::ValueType::Unit),
            Closed::Borrow(..) => {
                Err(reject("borrow replay remains an open full-profile implementation gate"))
            }
        }
    }
}
