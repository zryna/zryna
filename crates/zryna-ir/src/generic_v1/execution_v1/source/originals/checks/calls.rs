//! Exact loan argument roles are prepared in source order and rechecked together.
use super::{
    Checker, Closed, Expr, Failure, Resolver, Target, UntrustedSpan, Use, add, charge, push,
    reserve, resolution, resolve, v5,
};
impl Checker<'_, '_, '_> {
    pub(super) fn call(
        &mut self,
        callee: &zryna_syntax::v4::RawIdentifierSyntax,
        type_arguments: Option<&v5::RawTypeArgumentList>,
        args: &[u32],
        span: UntrustedSpan,
        depth: usize,
    ) -> Result<Closed, Failure> {
        for local in &self.locals {
            charge(self.spent, add(local.name.len().min(callee.text.len()), 1)?)?;
            if local.name == callee.text {
                return Err(self.error(
                    span,
                    "ZRYNA-M7006",
                    "original call is shadowed by a lexical value",
                ));
            }
        }
        if callee.text == "concat" {
            return self.concat(args, type_arguments.is_some(), callee.span, depth);
        }
        resolution::name_credit(&self.resolver, &callee.text, self.spent)?;
        let Target::Function(module, index) =
            self.resolver.originals.resolve(self.resolver.module, &callee.text)?
        else {
            return Err(super::reject("original call names data"));
        };
        let original = &self.resolver.originals.units[module as usize].functions[index as usize];
        let arity = original.type_parameters.as_ref().map_or(0, |p| p.parameters.len());
        if arity != type_arguments.map_or(0, |p| p.arguments.len())
            || (arity == 0 && type_arguments.is_some())
            || original.parameters.len() != args.len()
        {
            return Err(self.error(
                callee.span,
                "ZRYNA-M7006",
                "original call requires exact type/value arity",
            ));
        }
        let types = self.arguments(type_arguments)?;
        charge(self.spent, types.len())?;
        let mut arguments = reserve(types.len())?;
        arguments.extend(types.iter().map(Vec::as_slice));
        let resolver = Resolver {
            originals: self.resolver.originals,
            module,
            parameters: original.type_parameters.as_ref(),
            arguments,
        };
        let mut loans = Vec::new();
        for (at, parameter) in args.iter().zip(&original.parameters) {
            let expected = resolve(&resolver, parameter.type_syntax, self.spent)?;
            if let Closed::Borrow(_, exclusive) = &expected {
                super::check_depth(depth + 1)?;
                charge(self.spent, 1)?;
                let operand = &self.original.body.expressions[*at as usize];
                let Expr::Reference { name } = &operand.kind else {
                    return Err(self.error(
                        operand.span,
                        "ZRYNA-M7006",
                        "loan argument requires an exact loan binding",
                    ));
                };
                let index = self.binding(&name.text, name.span)?;
                let actual = self.read_local(index, name.span, Use::Loan)?;
                self.expect(&actual, &expected, name.span)?;
                let place = self.local_place(index)?;
                push(&mut loans, (place, *exclusive), self.spent)?;
            } else {
                let actual = self.expression(*at, Use::Move, depth + 1)?;
                self.expect(&actual, &expected, self.original.body.expressions[*at as usize].span)?;
            }
        }
        for (index, (loan, exclusive)) in loans.iter().enumerate() {
            self.access(loan, *exclusive, false, callee.span)?;
            for (other, other_exclusive) in &loans[..index] {
                charge(self.spent, 1)?;
                if (*exclusive || *other_exclusive) && self.overlaps(loan, other)? {
                    return Err(self.error(
                        callee.span,
                        "ZRYNA-M7007",
                        "call operands alias an exclusive referent",
                    ));
                }
            }
        }
        let result = resolve(&resolver, original.result_type, self.spent)?;
        if matches!(result, Closed::Borrow(..)) {
            return Err(self.error(
                callee.span,
                "ZRYNA-M7007",
                "a loan cannot escape a function result",
            ));
        }
        Ok(result)
    }
}
