//! Direct symbolic original checking, without executable claims or fabricated type IDs.
use super::{
    Closed, Expr, Failure, Frame, Originals, Resolver, SourceMap, Target, UntrustedSpan, add,
    charge, clone_type, copy_key, describe, description, diagnostic, equal, exhausted, key, push,
    reject, reserve, resolution, resolve, small, symbolic_fact, v5,
};
use zryna_syntax::{v4::RawStatementKind as Statement, v5::RawFunctionSyntax};
mod calls;
mod constructions;
mod containers;
mod expressions;
mod growth;
mod indexed;
mod loans;
mod loops;
mod matching;
mod partial;
mod places;
mod reads;
mod statements;
mod strings;

struct Local<'s> {
    name: &'s str,
    ty: Closed,
    available: bool,
    moved_at: Option<UntrustedSpan>,
    holes: Vec<partial::Hole>,
    mutable: bool,
    loan: Option<loans::Loan>,
}
#[derive(Clone, Copy, Eq, PartialEq)]
enum Use {
    Move,
    Observe,
    Loan,
}
struct Checker<'a, 's, 'w> {
    resolver: Resolver<'a, 's>,
    owner: Frame,
    original: &'s RawFunctionSyntax,
    sources: &'a SourceMap,
    result: Closed,
    locals: Vec<Local<'s>>,
    retained_reads: Vec<places::Place>,
    retained_accesses: Vec<(places::Place, bool)>,
    spent: &'w mut usize,
}
pub(super) fn all(
    originals: &Originals<'_>,
    sources: &SourceMap,
    spent: &mut usize,
) -> Result<(), Failure> {
    charge(spent, originals.units.len())?;
    for unit in originals.units {
        charge(spent, add(unit.data_declarations.len(), unit.functions.len())?)?;
        for (index, data) in unit.data_declarations.iter().enumerate() {
            let args = opaque(data.type_parameters.as_ref(), spent)?;
            let owner = Frame { kind: 1, module: unit.id, index: small(index)? };
            let desc = description::identity(originals, unit.id, small(index)?, &args, spent)?;
            symbolic_fact(originals, owner, &desc.ty, spent)?;
        }
        for (index, original) in unit.functions.iter().enumerate() {
            function(originals, sources, unit.id, small(index)?, original, spent)?;
        }
    }
    Ok(())
}
fn opaque(
    parameters: Option<&v5::RawTypeParameterList>,
    spent: &mut usize,
) -> Result<Vec<Vec<u8>>, Failure> {
    let count = parameters.map_or(0, |list| list.parameters.len());
    if count > 2 {
        return Err(reject("original generic arity exceeds two"));
    }
    charge(spent, count)?;
    let mut args = reserve(count)?;
    for slot in 0..count {
        args.push(copy_key(
            &[0x30 + u8::try_from(slot).map_err(|_| Failure::InternalFailure)?],
            spent,
        )?);
    }
    Ok(args)
}
fn function<'s>(
    originals: &Originals<'s>,
    sources: &SourceMap,
    module: u32,
    index: u32,
    original: &'s RawFunctionSyntax,
    spent: &mut usize,
) -> Result<(), Failure> {
    let args = opaque(original.type_parameters.as_ref(), spent)?;
    charge(spent, args.len())?;
    let mut arguments = reserve(args.len())?;
    arguments.extend(args.iter().map(Vec::as_slice));
    let resolver =
        Resolver { originals, module, parameters: original.type_parameters.as_ref(), arguments };
    let result = resolve(&resolver, original.result_type, spent)?;
    let mut checker = Checker {
        resolver,
        owner: Frame { kind: 0, module, index },
        original,
        sources,
        result,
        locals: Vec::new(),
        retained_reads: Vec::new(),
        retained_accesses: Vec::new(),
        spent,
    };
    for parameter in &original.parameters {
        let ty = resolve(&checker.resolver, parameter.type_syntax, checker.spent)?;
        let referent = match &ty {
            Closed::Borrow(key, _) => Closed::Stored(copy_key(key, checker.spent)?),
            _ => clone_type(&ty, checker.spent)?,
        };
        symbolic_fact(originals, checker.owner, &referent, checker.spent)?;
        let formal = matches!(ty, Closed::Borrow(..));
        checker.bind(&parameter.name.text, ty)?;
        if formal {
            let root = checker.locals.len() - 1;
            checker.locals[root].loan =
                Some(loans::Loan { root, path: Vec::new(), parent: None, indexed: false });
        }
    }
    let returned = checker.block(original.body.root_block, 1)?;
    if !returned && checker.result != Closed::Unit {
        return Err(checker.error(
            original.span,
            "ZRYNA-M7006",
            "original function does not return its declared result",
        ));
    }
    Ok(())
}
impl<'s> Checker<'_, 's, '_> {
    fn error(&self, span: UntrustedSpan, code: &'static str, message: &str) -> Failure {
        diagnostic(self.sources, span, code, message)
    }
    fn bind(&mut self, name: &'s str, ty: Closed) -> Result<(), Failure> {
        self.bind_mutable(name, ty, false)
    }
    fn bind_mutable(&mut self, name: &'s str, ty: Closed, mutable: bool) -> Result<(), Failure> {
        for local in &self.locals {
            charge(self.spent, add(local.name.len().min(name.len()), 1)?)?;
            if local.name.eq_ignore_ascii_case(name) {
                return Err(reject("original lexical binding shadowing is unfinished"));
            }
        }
        charge(self.spent, size_of::<Vec<partial::Hole>>())?;
        push(
            &mut self.locals,
            Local {
                name,
                ty,
                available: true,
                moved_at: None,
                holes: Vec::new(),
                mutable,
                loan: None,
            },
            self.spent,
        )
    }
    fn local(&mut self, name: &str, span: UntrustedSpan, usage: Use) -> Result<Closed, Failure> {
        let index = self.binding(name, span)?;
        self.read_local(index, span, usage)
    }
    fn expect(
        &mut self,
        actual: &Closed,
        expected: &Closed,
        span: UntrustedSpan,
    ) -> Result<(), Failure> {
        if !equal(actual, expected, self.spent)? {
            return Err(self.error(
                span,
                "ZRYNA-M7006",
                "original operand differs from its declared symbolic type",
            ));
        }
        Ok(())
    }
}

fn check_depth(depth: usize) -> Result<(), Failure> {
    if u32::try_from(depth).is_ok_and(|depth| depth <= zryna_syntax::v4::MAX_NESTING_DEPTH) {
        Ok(())
    } else {
        Err(exhausted("original source exceeds inherited syntax nesting depth"))
    }
}
