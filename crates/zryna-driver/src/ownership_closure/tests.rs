use std::collections::BTreeMap;

use zryna_source::{NormalizedSourcePath, UntrustedSpan};

use super::{DiscoveredSource, Import, ImportBinding, support::imports_match};

fn path(value: &str) -> NormalizedSourcePath {
    NormalizedSourcePath::new(value.to_owned()).expect("test path must be normalized")
}

const fn span(file: u32, start: u32, end: u32) -> UntrustedSpan {
    UntrustedSpan { file, start, end }
}

fn import(file: u32) -> Import {
    Import {
        span: span(file, 0, 52),
        specifier: "./dependency.zry".to_owned(),
        specifier_span: span(file, 31, 49),
        bindings: vec![ImportBinding {
            imported: "value".to_owned(),
            local: "local_value".to_owned(),
            imported_span: span(file, 9, 14),
            local_span: span(file, 18, 29),
        }],
    }
}

fn final_imports(file: u32) -> BTreeMap<NormalizedSourcePath, Vec<Import>> {
    BTreeMap::from([(path("main.zry"), vec![import(file)])])
}

fn discovered(file: u32) -> BTreeMap<NormalizedSourcePath, DiscoveredSource> {
    BTreeMap::from([(
        path("main.zry"),
        DiscoveredSource {
            text: "source".to_owned(),
            sha256: [0; 32],
            imports: vec![import(file)],
        },
    )])
}

#[test]
fn final_import_matching_ignores_only_map_local_file_ids() {
    assert!(imports_match(&final_imports(41), &discovered(7)));
}

#[test]
fn final_import_matching_rejects_every_malformed_identity_or_offset() {
    let discovered = discovered(7);
    let mut malformed = Vec::new();

    malformed.push(BTreeMap::new());
    let mut extra_path = final_imports(41);
    extra_path.insert(path("other.zry"), Vec::new());
    malformed.push(extra_path);

    let mut missing_import = final_imports(41);
    missing_import.get_mut(&path("main.zry")).expect("fixture").clear();
    malformed.push(missing_import);

    for mutate in [
        |item: &mut Import| item.span.start += 1,
        |item: &mut Import| item.specifier.push('x'),
        |item: &mut Import| item.specifier_span.end -= 1,
        |item: &mut Import| item.bindings[0].imported.push('x'),
        |item: &mut Import| item.bindings[0].local.push('x'),
        |item: &mut Import| item.bindings[0].imported_span.start += 1,
        |item: &mut Import| item.bindings[0].local_span.end -= 1,
    ] {
        let mut imports = final_imports(41);
        mutate(&mut imports.get_mut(&path("main.zry")).expect("fixture")[0]);
        malformed.push(imports);
    }

    let mut missing_binding = final_imports(41);
    missing_binding.get_mut(&path("main.zry")).expect("fixture")[0].bindings.clear();
    malformed.push(missing_binding);

    for imports in malformed {
        assert!(!imports_match(&imports, &discovered));
    }
}
