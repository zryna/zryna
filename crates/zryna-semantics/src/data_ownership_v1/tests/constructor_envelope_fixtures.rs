use super::*;

#[derive(Clone, Copy, Debug)]
pub(in crate::data_ownership_v1) enum Fixture {
    Pair,
    Nested,
    Array,
    Enum,
    EmptyEnum,
    WholeClone,
    Projection,
    PartialTransfer,
}

pub(in crate::data_ownership_v1) fn snapshot(kind: Fixture) -> (String, RawProjectSyntaxSnapshot) {
    let (source, response) = match kind {
        Fixture::Pair => (OWNED_PAIR_SOURCE, OWNED_PAIR_RESPONSE),
        Fixture::Nested => (NESTED_OWNED_SOURCE, NESTED_OWNED_RESPONSE),
        Fixture::Array => (OWNED_ARRAY_SOURCE, OWNED_ARRAY_RESPONSE),
        Fixture::Enum => (OWNED_ENUM_STRING_SOURCE, OWNED_ENUM_STRING_RESPONSE),
        Fixture::EmptyEnum => (OWNED_ENUM_NONE_SOURCE, OWNED_ENUM_NONE_RESPONSE),
        Fixture::WholeClone => {
            return clone_final_return_snapshot(OWNED_PAIR_SOURCE, OWNED_PAIR_RESPONSE);
        }
        Fixture::Projection => return owned_pair_projected_return_snapshot("first"),
        Fixture::PartialTransfer => return owned_pair_partial_local_transfer_snapshot(),
    };
    (source.to_owned(), response_snapshot(response))
}

pub(in crate::data_ownership_v1) fn sources(text: &str) -> SourceMap {
    sources_for(text)
}

pub(in crate::data_ownership_v1) fn input<'a>(
    syntax: &'a zryna_syntax::v4::ProjectSyntaxSnapshot,
    sources: &'a SourceMap,
) -> SemanticInput<'a> {
    pair_input(syntax, sources)
}
