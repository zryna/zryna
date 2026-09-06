use std::collections::BTreeSet;

use super::{Errors, RawFunctionBodySyntax, RawStatementKind};
use zryna_source::NormalizedSourcePath;

pub(super) fn verify(
    raw: &RawFunctionBodySyntax,
    parameters: &BTreeSet<String>,
    path: &NormalizedSourcePath,
    errors: &mut Errors,
) {
    let mut names = vec![BTreeSet::new(); raw.blocks.len()];
    if let Some(root) = names.first_mut() {
        root.clone_from(parameters);
    }
    for statement in &raw.statements {
        let RawStatementKind::WeakUpgrade { binding, success_block, .. } = &statement.kind else {
            continue;
        };
        let Some(scope) = usize::try_from(*success_block).ok().and_then(|id| names.get_mut(id))
        else {
            continue;
        };
        if !scope.insert(binding.text.clone()) {
            errors.node(path, "duplicate weak-upgrade binding name");
        }
    }
    for (block_id, block) in raw.blocks.iter().enumerate() {
        let Some(scope) = names.get_mut(block_id) else {
            continue;
        };
        for statement_id in &block.statements {
            let Some(statement) =
                usize::try_from(*statement_id).ok().and_then(|id| raw.statements.get(id))
            else {
                continue;
            };
            let RawStatementKind::LocalDeclaration { name, .. } = &statement.kind else {
                continue;
            };
            if !scope.insert(name.text.clone()) {
                errors.node(path, "duplicate function-local binding name");
            }
        }
    }
}
