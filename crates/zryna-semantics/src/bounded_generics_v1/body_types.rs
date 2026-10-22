//! Original-source symbolic type checking for bounded templates and ordinary bodies.
//!
//! The result retains source use obligations. Move state, loans, closed instances, layouts,
//! executable capabilities and IR still require their own authorities.

use zryna_diagnostics::Diagnostic;
use zryna_source::{Span, UntrustedSpan};

use super::{DeclarationContext, DeclarationIdentity};
use model::{Tables, Ty};

mod arguments;
mod body_walk;
mod calls;
mod capabilities;
mod constraints;
mod constructions;
mod containers;
mod equality;
mod expressions;

mod matches;
mod model;
mod projections;
mod resources;
mod scalars;
mod signatures;
mod statements;
mod substitution;
#[cfg(test)]
#[path = "tests/body_types.rs"]
mod tests;
mod type_resolution;
mod value_names;
mod views;

pub use views::{TypeShape, TypeView};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum IntrinsicCall {
    Concat,
}
pub use resources::StorageReport;

/// A source rejection and an infrastructure failure have different meanings.
#[derive(Debug)]
pub enum BodyTypeFailure {
    /// Canonically ordered phase diagnostics, with no partial context.
    Diagnostics(Vec<Diagnostic>),
    /// A fallible allocation failed; this is not a source limit diagnostic.
    AllocationFailure,
    /// An invariant of the authenticated source-derived representation was violated.
    InternalFailure,
}

/// Checked symbolic source types, retaining the exact issuing declaration authority.
///
/// This is not a closed program, ownership result or backend input.
/// ```compile_fail
/// fn check(raw: &zryna_syntax::v5::VerifiedProjectSyntaxV5) {
///     zryna_semantics::bounded_generics_v1::body_types::check_body_types(raw);
/// }
/// ```
/// ```compile_fail
/// use zryna_semantics::bounded_generics_v1::body_types::BodyTypeContext;
/// fn invent() { let context = BodyTypeContext { declarations: (), tables: () }; }
/// ```
/// ```compile_fail
/// fn emit(context: &zryna_semantics::bounded_generics_v1::body_types::BodyTypeContext<'_, '_>) {
///     let _: &zryna_ir::data_ownership_v1::VerifiedProgram = context;
/// }
/// ```
#[derive(Debug)]
pub struct BodyTypeContext<'c, 's> {
    declarations: &'c DeclarationContext<'s>,
    tables: Tables,
    storage: StorageReport,
}

impl<'c, 's> BodyTypeContext<'c, 's> {
    /// Retains the body's resolved intrinsic classification, including import-alias precedence.
    pub(super) fn intrinsic_call(
        &self,
        owner: DeclarationIdentity,
        expression: u32,
    ) -> Option<IntrinsicCall> {
        self.function_records(owner)?.expressions.get(expression as usize)?.intrinsic_call
    }

    pub(super) fn source_owners(
        &self,
        module: u32,
    ) -> impl Iterator<Item = (DeclarationIdentity, usize)> {
        self.tables.sources[module as usize]
            .iter()
            .enumerate()
            .filter(|(_, record)| record.head.is_some())
            .map(|(index, record)| (record.owner, index))
    }

    fn function_records(&self, owner: DeclarationIdentity) -> Option<&model::FunctionRecords> {
        let offset = *self.tables.function_offsets.get(owner.module().index() as usize)?;
        self.tables
            .functions
            .get(offset.checked_add(owner.source_index() as usize)?)
            .filter(|records| records.owner == owner)
    }
    pub(super) fn source_type(
        &self,
        owner: DeclarationIdentity,
        occurrence: u32,
    ) -> Option<TypeView<'_, 'c, 's>> {
        let record =
            self.tables.sources.get(owner.module().index() as usize)?.get(occurrence as usize)?;
        if record.owner != owner || record.head.is_none() {
            return None;
        }
        Some(TypeView {
            context: self,
            function: owner,
            ty: Ty::source(owner.module().index(), occurrence),
        })
    }

    pub(super) fn source_argument(&self, owner: DeclarationIdentity, occurrence: u32) -> bool {
        let record = &self.tables.sources[owner.module().index() as usize][occurrence as usize];
        record.owner == owner && record.argument_occurrence
    }
    /// The exact retained original context, including its issuing source map and entry.
    #[must_use]
    pub const fn declarations(&self) -> &'c DeclarationContext<'s> {
        self.declarations
    }

    /// Actual table storage, not a layout or language admission certificate.
    #[must_use]
    pub const fn storage(&self) -> StorageReport {
        self.storage
    }

    /// Original expression use count; uses are retained even after a return or a consuming call.
    #[must_use]
    pub fn expression_count(&self, owner: DeclarationIdentity) -> Option<usize> {
        self.function_records(owner).map(|function| function.expressions.len())
    }

    /// Returns a read-only symbolic type for one original expression of the issuing owner.
    #[must_use]
    pub fn expression_type(
        &self,
        owner: DeclarationIdentity,
        expression: u32,
    ) -> Option<TypeView<'_, 'c, 's>> {
        let records = self.function_records(owner)?;
        let ty = records.expressions.get(expression as usize)?.ty?;
        Some(TypeView { context: self, function: owner, ty })
    }
}

pub(super) struct Checker<'c, 's> {
    context: &'c DeclarationContext<'s>,
    tables: Tables,
    capabilities: capabilities::Graph,
    names: constraints::Lane,
    opaque: constraints::Lane,
    arguments: constraints::Lane,
    constraints: constraints::Lane,
    equality: equality::Cache,
}

impl Checker<'_, '_> {
    fn span(&self, raw: UntrustedSpan) -> Span {
        self.context.sources().verify_span(raw).expect("retained authenticated source span")
    }

    fn expression(&self, owner: DeclarationIdentity, index: u32) -> Option<Ty> {
        self.tables.function(owner).expressions[index as usize].ty
    }
}

/// Checks all original bodies, including unused templates, without concrete specialization.
///
/// # Errors
/// Returns source diagnostics or an infrastructure failure, never a partial type context.
pub fn check_body_types<'c, 's>(
    context: &'c DeclarationContext<'s>,
) -> Result<BodyTypeContext<'c, 's>, BodyTypeFailure> {
    check_with_cache(context, 256)
}

fn check_with_cache<'c, 's>(
    context: &'c DeclarationContext<'s>,
    cache_capacity: usize,
) -> Result<BodyTypeContext<'c, 's>, BodyTypeFailure> {
    let mut checker = Checker {
        context,
        tables: resources::tables(context)?,
        capabilities: capabilities::Graph::empty(),
        names: constraints::Lane::new()?,
        opaque: constraints::Lane::new()?,
        arguments: constraints::Lane::new()?,
        constraints: constraints::Lane::new()?,
        equality: equality::Cache::new(cache_capacity)?,
    };
    type_resolution::resolve(&mut checker)?;
    checker.capabilities = capabilities::Graph::derive(context, &checker.tables)?;
    for module in context.modules() {
        for declaration in module.functions() {
            body_walk::check(&mut checker, declaration.identity())?;
        }
    }
    // No argument or exact-type candidate can consume the opaque lane's diagnostic prefix.
    for lane in [checker.names, checker.opaque, checker.arguments, checker.constraints] {
        if !lane.is_empty() {
            return Err(BodyTypeFailure::Diagnostics(lane.finish()));
        }
    }
    if checker
        .tables
        .functions
        .iter()
        .any(|function| function.expressions.iter().any(|record| record.ty.is_none()))
    {
        return Err(BodyTypeFailure::InternalFailure);
    }
    let mut storage = resources::report(&checker.tables)?;
    (storage.predicate_rows, storage.predicate_edges, storage.predicate_capacity_bytes) =
        checker.capabilities.storage();
    Ok(BodyTypeContext { declarations: context, tables: checker.tables, storage })
}
