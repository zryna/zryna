//! Original symbolic ownership checking requires no closed instance or layout authority.
use super::{Admission, Failure, graph as raw, source};
use crate::generic_v1::{reject, reserve, source::Originals};
use zryna_source::SourceMap;
use zryna_syntax::v5::VerifiedProjectSyntaxV5;

/// Independently checks every original body's opaque affinity and loans before discovery.
/// This grants neither executable nor layout authority.
/// # Errors
/// Rejects foreign syntax, unsupported originals, invalid moves/loans and inherited budgets.
pub fn check_original_bodies(
    syntax: &VerifiedProjectSyntaxV5,
    sources: &SourceMap,
) -> Result<(), Failure> {
    check_originals_for(syntax, sources, Admission::LegacyOwned)
}

/// Checks every original in the distinct private Struct-v4 source domain.
/// # Errors
/// Rejects foreign syntax, unsupported originals, invalid ownership and inherited budgets.
pub fn check_original_bodies_v4(
    syntax: &VerifiedProjectSyntaxV5,
    sources: &SourceMap,
) -> Result<(), Failure> {
    check_originals_for(syntax, sources, Admission::StructV4)
}

fn check_originals_for(
    syntax: &VerifiedProjectSyntaxV5,
    sources: &SourceMap,
    admission: Admission,
) -> Result<(), Failure> {
    if !syntax.is_bound_to(sources) {
        return Err(reject("original ownership syntax belongs to a foreign source map"));
    }
    let mut modules = reserve(syntax.files().len())?;
    let mut declarations = reserve(0)?;
    for unit in syntax.files() {
        modules.push(raw::Module {
            id: unit.id,
            functions: u32::try_from(unit.functions.len()).map_err(|_| Failure::InternalFailure)?,
        });
        for (index, function) in unit.functions.iter().enumerate() {
            declarations.try_reserve(1).map_err(|_| Failure::AllocationFailure)?;
            declarations.push(raw::Declaration {
                module: unit.id,
                function: u32::try_from(index).map_err(|_| Failure::InternalFailure)?,
                parameters: u32::try_from(
                    function.type_parameters.as_ref().map_or(0, |list| list.parameters.len()),
                )
                .map_err(|_| Failure::InternalFailure)?,
                span: function.span,
            });
        }
    }
    let program = raw::Program {
        modules,
        declarations,
        type_keys: Vec::new(),
        universe: [0; 32],
        linear32: [0; 32],
        linux_x86_64: [0; 32],
        functions: Vec::new(),
    };
    let originals = Originals::check(&program, syntax)?;
    match admission {
        Admission::LegacyOwned => source::check_originals(&program, &originals, sources),
        Admission::StructV4 => {
            source::check_originals_for(&program, &originals, sources, admission)
        }
    }
}
