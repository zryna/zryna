//! One original source frame, retaining immutable admission and separate budget credit.
use super::{
    Admission, Budget, Builder, Closed, Extension, Failure, Originals, RawFunctionSyntax, Resolver,
    normalize, raw, reject, reserve, type_facts,
};
use std::collections::BTreeMap;

pub(super) struct Frame<'a, 'b> {
    pub module: u32,
    pub original: &'b RawFunctionSyntax,
    pub arguments: Vec<&'a [u8]>,
    pub symbolic: bool,
    pub admission: Admission,
}

pub(super) fn build<'a, 'b>(
    program: &'a raw::Program,
    originals: &'a Originals<'b>,
    sources: &'a zryna_source::SourceMap,
    frame: Frame<'a, 'b>,
    budget: &mut Budget,
) -> Result<(Vec<raw::Block>, Vec<Extension>), Failure> {
    let Frame { module, original, arguments, symbolic, admission } = frame;
    let resolver =
        Resolver { originals, module, parameters: original.type_parameters.as_ref(), arguments };
    let result = resolver.resolve_symbolic(original.result_type)?;
    let mut builder = Builder {
        program,
        sources,
        resolver,
        original,
        symbolic,
        blocks: reserve(1)?,
        block: 0,
        next: 0,
        locals: reserve(original.parameters.len())?,
        scope_start: 0,
        result,
        extensions: Vec::new(),
        affine: Vec::new(),
        alive: Vec::new(),
        loans: BTreeMap::new(),
        loan_parents: BTreeMap::new(),
        budget: *budget,
        type_facts: type_facts::SourceTypeFacts::new(admission),
    };
    builder.new_block(original.body.span)?;
    for parameter in &original.parameters {
        let ty = builder.resolve(parameter.type_syntax)?;
        if matches!(ty, Closed::Unit) {
            return Err(reject("Copy executable parameters cannot carry unit or loans"));
        }
        let value = builder.value(ty)?;
        let definition = builder.definition(&value)?;
        builder.blocks[0].parameters.push(definition);
        builder.bind(&parameter.name.text, value)?;
    }
    builder.source_block(original.body.root_block, 0)?;
    normalize::run(&mut builder.blocks, &mut builder.extensions)?;
    *budget = builder.budget;
    Ok((builder.blocks, builder.extensions))
}
