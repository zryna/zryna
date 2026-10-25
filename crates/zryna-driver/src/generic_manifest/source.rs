//! Owned receipt projection; compilation brands remain private and are never serialized.
use super::model::{
    GenericEdgeRecord, GenericExportRecord, GenericModuleRecord, GenericSourceRecord,
};
use crate::source_scope::GenericSourceReceipt;
use std::fmt::Write as _;

pub(crate) fn hex(bytes: &[u8; 32]) -> String {
    let mut value = String::with_capacity(64);
    for byte in bytes {
        write!(&mut value, "{byte:02x}").expect("fixed digest String");
    }
    value
}

pub(crate) fn receipt(receipt: &GenericSourceReceipt) -> GenericSourceRecord {
    GenericSourceRecord {
        entrypoint: receipt.entrypoint().as_str().to_owned(),
        entry_module: receipt.entry_module(),
        modules: receipt
            .modules()
            .iter()
            .map(|module| GenericModuleRecord {
                id: module.id(),
                path: module.path().to_owned(),
                bytes: module.bytes(),
                sha256: hex(module.sha256()),
            })
            .collect(),
        edges: receipt
            .edges()
            .iter()
            .map(|edge| GenericEdgeRecord {
                importer: edge.importer().to_owned(),
                target: edge.target().to_owned(),
                specifier: edge.specifier().to_owned(),
                imported: edge.imported().to_owned(),
                local: edge.local().to_owned(),
            })
            .collect(),
        graph_sha256: hex(receipt.graph_sha256()),
        execution_sha256: hex(receipt.execution_sha256()),
        type_universe_sha256: hex(receipt.type_universe_sha256()),
        linear32_sha256: hex(receipt.linear32_sha256()),
        linux_x86_64_sha256: hex(receipt.linux_x86_64_sha256()),
        runtime_abi_sha256: hex(receipt.runtime_abi_sha256()),
    }
}

pub(crate) fn exports(receipt: &GenericSourceReceipt) -> Vec<GenericExportRecord> {
    let mut exports = receipt
        .exports()
        .iter()
        .map(|export| GenericExportRecord {
            name: export.name().to_owned(),
            parameters: export.parameters().to_vec(),
            result: export.result(),
        })
        .collect::<Vec<_>>();
    exports.sort_by(|left, right| left.name.cmp(&right.name));
    exports
}
