use super::generic_call_fixture::{Case, fixture, single_string_fixture};
use super::generic_vec_fixture::Element;
use super::*;
use zryna_syntax::v4::{
    RawExpressionKind, RawImportBindingSyntax, RawImportSyntax, RawModuleSpecifierSyntax,
};

#[derive(Clone, Copy)]
enum Base {
    Mixed(Case),
    OwnedToCopy,
}

fn rewrite_spans(value: &mut serde_json::Value, file: u32, cutoff: u32, shift: u32) {
    match value {
        serde_json::Value::Object(object)
            if object.contains_key("file")
                && object.contains_key("start")
                && object.contains_key("end") =>
        {
            *object.get_mut("file").expect("file") = file.into();
            for key in ["start", "end"] {
                let current = object[key].as_u64().expect("offset") as u32;
                if current >= cutoff {
                    *object.get_mut(key).expect("offset") = (current + shift).into();
                }
            }
        }
        serde_json::Value::Object(object) => {
            for child in object.values_mut() {
                rewrite_spans(child, file, cutoff, shift);
            }
        }
        serde_json::Value::Array(values) => {
            for child in values {
                rewrite_spans(child, file, cutoff, shift);
            }
        }
        _ => {}
    }
}

fn span(file: u32, start: usize, end: usize) -> zryna_source::UntrustedSpan {
    zryna_source::UntrustedSpan {
        file,
        start: start.try_into().expect("offset"),
        end: end.try_into().expect("offset"),
    }
}

fn imported_fixture(
    base: Base,
    imported_name: &str,
    local_name: &str,
    import_path: &str,
) -> (SourceMap, RawProjectSyntaxSnapshot) {
    let (source, raw) = match base {
        Base::Mixed(case) => fixture(&Element::String, case),
        Base::OwnedToCopy => single_string_fixture(),
    };
    let prefix = format!("import {{ {imported_name} as {local_name} }} from '{import_path}';\n");
    let mut main = shift_snapshot(raw.clone(), 0, prefix.len() as u32).files.remove(0);
    let call = main.functions[0]
        .body
        .expressions
        .iter_mut()
        .find_map(|expression| match &mut expression.kind {
            RawExpressionKind::Call { callee, .. } => Some(callee),
            _ => None,
        })
        .expect("call");
    call.text = local_name.into();
    let call_start = call.span.start as usize;
    let call_end = call.span.end as usize;
    let local = prefix.find(local_name).expect("local");
    let imported = prefix.find(imported_name).expect("imported");
    let from = prefix.find("from").expect("from");
    let token = prefix.find(&format!("'{import_path}'")).expect("specifier");
    main.imports = vec![RawImportSyntax {
        span: span(0, 0, prefix.len() - 1),
        import_span: span(0, 0, 6),
        bindings: vec![RawImportBindingSyntax {
            span: span(0, imported, local + local_name.len()),
            imported: RawIdentifierSyntax {
                text: imported_name.into(),
                span: span(0, imported, imported + imported_name.len()),
            },
            local: RawIdentifierSyntax {
                text: local_name.into(),
                span: span(0, local, local + local_name.len()),
            },
            as_span: Some(span(0, local - 3, local - 1)),
        }],
        from_span: span(0, from, from + 4),
        specifier: RawModuleSpecifierSyntax {
            text: import_path.into(),
            token_span: span(0, token, token + import_path.len() + 2),
            value_span: span(0, token + 1, token + import_path.len() + 1),
        },
        semicolon_span: span(0, prefix.len() - 2, prefix.len() - 1),
    }];

    let mut library = raw.files[0].clone();
    library.id = 0;
    library.path = "src/lib.zry".into();
    let insertion = library.functions[1].span.start as usize;
    let mut library_value = serde_json::to_value(library).expect("library");
    rewrite_spans(&mut library_value, 0, insertion as u32, 7);
    let mut library: RawSourceUnit = serde_json::from_value(library_value).expect("library");
    library.functions[1].span.start = insertion as u32;
    library.functions[1].export_span = Some(span(0, insertion, insertion + 6));

    main.id = 1;
    let mut main_value = serde_json::to_value(main).expect("main");
    rewrite_spans(&mut main_value, 1, u32::MAX, 0);
    let main = serde_json::from_value(main_value).expect("main");

    let mut library_source = source.clone();
    library_source.insert_str(insertion, "export ");
    let mut main_source = format!("{prefix}{source}");
    main_source.replace_range(call_start..call_end, local_name);
    let sources = SourceMap::build(vec![
        SourceFileInput { path: "src/main.zry".into(), text: main_source },
        SourceFileInput { path: "src/lib.zry".into(), text: library_source },
    ])
    .expect("sources");
    (
        sources,
        RawProjectSyntaxSnapshot {
            schema_version: PROTOCOL_VERSION,
            files: vec![library, main],
            diagnostics: vec![],
        },
    )
}

#[test]
fn named_import_alias_retains_canonical_cross_module_identity_and_owned_cleanup() {
    let (sources, raw) =
        imported_fixture(Base::Mixed(Case::Direct), "choose", "select", "./lib.zry");
    let syntax = verify_snapshot(raw, &sources).expect("authenticated imported call");
    let entry =
        sources.file_id(&NormalizedSourcePath::new("src/main.zry").expect("path")).expect("entry");
    let program = lower(SemanticInput::try_new(&syntax, &sources, entry).expect("input"))
        .unwrap_or_else(|errors| panic!("{errors:?}"));
    let modules = program.modules().collect::<Vec<_>>();
    assert_eq!(modules.len(), 2);
    let caller = modules[1].functions().next().expect("caller");
    let call = caller
        .blocks()
        .next()
        .expect("block")
        .instructions()
        .find(|instruction| instruction.callee().is_some())
        .expect("call");
    let callee = call.callee().expect("callee");
    assert_eq!((callee.module(), callee.declaration()), (0, 1));
    assert_eq!(call.call_arguments().count(), 3);
    assert_eq!(call.derived_drop_actions().count(), 1);
    assert_eq!(modules[0].functions().count(), 2);
}

#[test]
fn named_import_calls_cover_nested_owned_results_and_owned_input_copy_results() {
    let cases = [
        (Base::Mixed(Case::Nested), "choose", "select", 3_usize),
        (Base::OwnedToCopy, "consume", "process", 1_usize),
    ];
    for (base, imported, local, arguments) in cases {
        let (sources, raw) = imported_fixture(base, imported, local, "./lib.zry");
        let syntax = verify_snapshot(raw, &sources).expect("authenticated imported call");
        let entry = sources
            .file_id(&NormalizedSourcePath::new("src/main.zry").expect("path"))
            .expect("entry");
        let program = lower(SemanticInput::try_new(&syntax, &sources, entry).expect("input"))
            .unwrap_or_else(|errors| panic!("{errors:?}"));
        let caller =
            program.modules().nth(1).expect("entry module").functions().next().expect("caller");
        let call = caller
            .blocks()
            .next()
            .expect("block")
            .instructions()
            .find(|instruction| instruction.callee().is_some())
            .expect("call");
        assert_eq!(
            (call.callee().expect("callee").module(), call.callee().expect("callee").declaration()),
            (0, 1)
        );
        assert_eq!(call.call_arguments().count(), arguments);
    }
}

#[test]
fn named_import_resolution_rejects_absent_unexported_wrong_case_and_colliding_names_exactly() {
    let cases = [
        (
            "choose",
            "select",
            "./bad.zry",
            "module 'src/bad.zry' is absent from the authenticated source closure",
        ),
        ("caller", "select", "./lib.zry", "module 'src/lib.zry' does not export function 'caller'"),
        (
            "Choose",
            "select",
            "./lib.zry",
            "imported function 'Choose' has the wrong portable ASCII case",
        ),
        (
            "choose",
            "caller",
            "./lib.zry",
            "callable name 'caller' collides under portable ASCII case folding",
        ),
    ];
    for (imported, local, path, message) in cases {
        let (sources, raw) = imported_fixture(Base::Mixed(Case::Direct), imported, local, path);
        let syntax = verify_snapshot(raw, &sources).expect("authenticated negative import");
        let entry = sources
            .file_id(&NormalizedSourcePath::new("src/main.zry").expect("path"))
            .expect("entry");
        let errors = lower(SemanticInput::try_new(&syntax, &sources, entry).expect("input"))
            .expect_err("invalid import");
        assert_eq!(errors.len(), if path == "./bad.zry" { 2 } else { 1 }, "{errors:?}");
        assert_eq!(errors[0].code, if local == "caller" { "ZRYNA-M3002" } else { "ZRYNA-M3016" });
        assert_eq!(errors[0].message, message);
        assert!(errors[0].primary_span().is_some());
        if path == "./bad.zry" {
            assert_eq!(
                errors[1].message,
                "authenticated source closure contains module 'src/lib.zry' unreachable from the selected entry"
            );
        }
    }
}
