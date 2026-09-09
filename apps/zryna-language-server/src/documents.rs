use std::collections::BTreeMap;

use zryna_source::{SourceError, SourceFileInput, SourceMap};

#[derive(Clone, Debug)]
pub(crate) struct Document {
    pub(crate) path: String,
    pub(crate) text: String,
    pub(crate) version: i64,
}

pub(crate) fn source_map(documents: &BTreeMap<String, Document>) -> Result<SourceMap, SourceError> {
    SourceMap::build(
        documents
            .values()
            .map(|document| SourceFileInput {
                path: document.path.clone(),
                text: document.text.clone(),
            })
            .collect(),
    )
}
