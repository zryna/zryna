use super::named_import_calls::{Base, imported_fixture, rewrite_spans, span};
use super::named_import_graphs::multi_hop_fixture;
use super::*;
use crate::data_ownership_v1::function_catalog::{FunctionResolution, build_function_catalog};
use crate::data_ownership_v1::import_resolution;
use crate::data_ownership_v1::layout_graph::build_graph;
use crate::data_ownership_v1::type_model::map_node_types;
use generic_call_fixture::Case;
use zryna_syntax::v4::{
    RawExpressionKind, RawExpressionSyntax, RawIdentifierSyntax, RawImportBindingSyntax,
    RawImportSyntax, RawModuleSpecifierSyntax,
};

#[test]
fn named_import_alias_binding_does_not_create_a_declaration_identity() {
    // A declaration-only lookup is the invariant: an alias may be called in its importing
    // module, but cannot become that module's exported declaration by catalog insertion.
    let (sources, raw) =
        imported_fixture(Base::Mixed(Case::Direct), "choose", "select", "./lib.zry");
    let syntax = verify_snapshot(raw, &sources).expect("authenticated import");
    let entry =
        sources.file_id(&NormalizedSourcePath::new("src/main.zry").expect("path")).expect("entry");
    let input = SemanticInput::try_new(&syntax, &sources, entry).expect("input");
    let mut errors = Errors::new(&sources);
    semantic_preflight(input, &mut errors);
    let (graph, declarations) = build_graph(input, &mut errors);
    let layouts = zryna_layout::verify(&graph, &sources, zryna_layout::StorageTarget::Linear32V1)
        .expect("layouts");
    let node_types = map_node_types(&graph, &layouts, &mut errors);
    let mut catalog =
        build_function_catalog(input, &declarations, &graph, &node_types, &mut errors);
    import_resolution::resolve_imports(input, &mut catalog, &mut errors);
    assert!(errors.finish().is_empty());
    let main = usize::try_from(entry.index()).expect("module");
    assert!(matches!(catalog.resolve(main, "select"), FunctionResolution::Exact(_)));
    assert_eq!(
        catalog.modules[main]
            .iter()
            .flatten()
            .filter(|signature| signature.id.module.0 as usize != main)
            .count(),
        1
    );
    assert_eq!(input.syntax().files()[main].functions().len(), 1);
}

#[test]
fn named_import_alias_cannot_be_reexported_as_a_target_declaration() {
    let (sources, mut raw) = multi_hop_fixture();
    for file in &mut raw.files {
        let (path, id) = match file.path.as_str() {
            "src/mid.zry" => ("src/aaa.zry", 0),
            "src/lib.zry" => ("src/lib.zry", 1),
            "src/main.zry" => ("src/zzz.zry", 2),
            path => panic!("unexpected path {path}"),
        };
        let mut value = serde_json::to_value(&*file).expect("serializable fixture syntax");
        rewrite_spans(&mut value, id, u32::MAX, 0);
        *file = serde_json::from_value(value).expect("rebound fixture syntax");
        file.id = id;
        file.path = path.into();
    }
    raw.files.sort_by_key(|file| file.id);
    let main =
        raw.files.iter_mut().find(|file| file.path == "src/zzz.zry").expect("fixture element");
    main.imports[0].bindings[0].imported.text = "select".into();
    main.imports[0].specifier.text = "./aaa.zry".into();
    let expected_span = main.imports[0].bindings[0].imported.span;
    let inputs = [
        ("src/aaa.zry", "src/mid.zry"),
        ("src/lib.zry", "src/lib.zry"),
        ("src/zzz.zry", "src/main.zry"),
    ]
    .into_iter()
    .map(|(path, old_path)| {
        let normalized = NormalizedSourcePath::new(old_path).expect("fixture path");
        let source = sources
            .source(sources.file_id(&normalized).expect("source-map-bound fixture text"))
            .expect("source-map-bound fixture text");
        SourceFileInput {
            path: path.into(),
            text: if path == "src/zzz.zry" {
                source.text().replacen("caller as bridge", "select as bridge", 1).replacen(
                    "./mid.zry",
                    "./aaa.zry",
                    1,
                )
            } else {
                source.text().to_owned()
            },
        }
    })
    .collect();
    let sources = SourceMap::build(inputs).expect("fixture element");
    let syntax = verify_snapshot(raw, &sources).expect("authenticated re-export attempt");
    let entry = sources
        .file_id(&NormalizedSourcePath::new("src/zzz.zry").expect("fixture path"))
        .expect("fixture path");
    let input =
        SemanticInput::try_new(&syntax, &sources, entry).expect("source-bound semantic input");
    let expected = lower(input).expect_err("aliases are not declarations");
    assert_eq!(expected.len(), 1);
    assert_eq!(expected[0].code, "ZRYNA-M3016");
    assert_eq!(expected[0].message, "module 'src/aaa.zry' does not export function 'select'");
    assert_eq!(
        expected[0].primary_span(),
        Some(crate::data_ownership_v1::span(&sources, expected_span))
    );
    assert_eq!(lower(input).expect_err("replay"), expected);
}

#[test]
fn named_import_rejects_an_existing_nonexported_declaration_exactly() {
    let (sources, mut raw) =
        imported_fixture(Base::Mixed(Case::Direct), "choose", "select", "./lib.zry");
    let library =
        raw.files.iter_mut().find(|file| file.path == "src/lib.zry").expect("fixture element");
    library.functions[0].export_span = None;
    let binding_span =
        raw.files.iter().find(|file| file.path == "src/main.zry").expect("fixture element").imports
            [0]
        .bindings[0]
            .imported
            .span;
    let inputs = ["src/lib.zry", "src/main.zry"]
        .into_iter()
        .map(|path| {
            let normalized = NormalizedSourcePath::new(path).expect("fixture path");
            let source = sources
                .source(sources.file_id(&normalized).expect("source-map-bound fixture text"))
                .expect("source-map-bound fixture text");
            SourceFileInput {
                path: path.into(),
                text: if path == "src/lib.zry" {
                    source.text().replacen("export ", "       ", 1)
                } else {
                    source.text().to_owned()
                },
            }
        })
        .collect();
    let sources = SourceMap::build(inputs).expect("fixture element");
    let syntax = verify_snapshot(raw, &sources).expect("authenticated private declaration");
    let entry = sources
        .file_id(&NormalizedSourcePath::new("src/main.zry").expect("fixture path"))
        .expect("fixture path");
    let input =
        SemanticInput::try_new(&syntax, &sources, entry).expect("source-bound semantic input");
    let expected = lower(input).expect_err("private target");
    assert_eq!(expected.len(), 1);
    assert_eq!(expected[0].code, "ZRYNA-M3016");
    assert_eq!(expected[0].message, "module 'src/lib.zry' does not export function 'choose'");
    assert_eq!(
        expected[0].primary_span(),
        Some(crate::data_ownership_v1::span(&sources, binding_span))
    );
    assert_eq!(lower(input).expect_err("replay"), expected);
}

#[test]
fn named_import_calls_preserve_exact_type_and_affine_rejections_with_recovery() {
    for (case, code) in [(Case::WrongType, "ZRYNA-M3016"), (Case::RepeatedOwner, "ZRYNA-M3014")] {
        let (sources, raw) = imported_fixture(Base::Mixed(case), "choose", "select", "./lib.zry");
        let caller =
            raw.files.iter().find(|file| file.path == "src/main.zry").expect("fixture element");
        let arguments = caller.functions[0]
            .body
            .expressions
            .iter()
            .find_map(|expression| match &expression.kind {
                RawExpressionKind::Call { arguments, .. } => Some(arguments),
                _ => None,
            })
            .expect("fixture element");
        let argument = if matches!(case, Case::WrongType) { arguments[0] } else { arguments[2] };
        let expected_span = caller.functions[0].body.expressions[argument as usize].span;
        let syntax = verify_snapshot(raw, &sources).expect("authenticated invalid imported call");
        let entry = sources
            .file_id(&NormalizedSourcePath::new("src/main.zry").expect("fixture path"))
            .expect("fixture path");
        let input =
            SemanticInput::try_new(&syntax, &sources, entry).expect("source-bound semantic input");
        let expected = lower(input).expect_err("invalid imported call");
        assert_eq!(expected.len(), 1);
        assert_eq!(expected[0].code, code);
        assert_eq!(
            expected[0].message,
            if matches!(case, Case::WrongType) {
                "aggregate operand has the wrong exact type"
            } else {
                "aggregate value 'left' is moved or only partially available"
            }
        );
        assert_eq!(
            expected[0].primary_span(),
            Some(crate::data_ownership_v1::span(&sources, expected_span))
        );
        assert_eq!(lower(input).expect_err("deterministic replay"), expected);
    }
}

#[test]
fn named_import_malformed_path_is_rejected_by_syntax_exactly() {
    let (sources, raw) =
        imported_fixture(Base::Mixed(Case::Direct), "choose", "select", "../Lib.ts");
    let expected = verify_snapshot(raw, &sources).expect_err("nonportable import path");
    assert_eq!(expected.len(), 1);
    assert_eq!(expected[0].code, "ZRYNA-Y4002");
    assert_eq!(
        expected[0].message,
        "module specifier is not canonical explicit-relative .zry syntax"
    );
    assert_eq!(expected[0].guidance, "return source-faithful canonical protocol-v4 syntax");
    let encoded = serde_json::to_value(&expected[0]).expect("serializable fixture syntax");
    assert_eq!(encoded["primary"]["kind"], "workspace-path");
    assert_eq!(encoded["primary"]["path"], "src/main.zry");
    assert_eq!(
        verify_snapshot(
            imported_fixture(Base::Mixed(Case::Direct), "choose", "select", "../Lib.ts").1,
            &sources
        )
        .expect_err("replay"),
        expected
    );
}

#[test]
fn named_import_visibility_does_not_enable_owned_entry_abi() {
    let (sources, mut raw) =
        imported_fixture(Base::Mixed(Case::Direct), "choose", "select", "./lib.zry");
    let main =
        raw.files.iter_mut().find(|file| file.path == "src/main.zry").expect("fixture element");
    let insertion = main.functions[0].span.start;
    let main_id = main.id;
    let mut value = serde_json::to_value(&*main).expect("serializable fixture syntax");
    rewrite_spans(&mut value, main_id, insertion, 7);
    *main = serde_json::from_value(value).expect("rebound fixture syntax");
    main.functions[0].span.start = insertion;
    main.functions[0].export_span = Some(span(main_id, insertion as usize, insertion as usize + 6));
    let expected_span = main.functions[0].span;
    let main_path = NormalizedSourcePath::new("src/main.zry").expect("fixture path");
    let library_path = NormalizedSourcePath::new("src/lib.zry").expect("fixture path");
    let mut main_text = sources
        .source(sources.file_id(&main_path).expect("source-map-bound fixture text"))
        .expect("source-map-bound fixture text")
        .text()
        .to_owned();
    main_text.insert_str(insertion as usize, "export ");
    let library_text = sources
        .source(sources.file_id(&library_path).expect("source-map-bound fixture text"))
        .expect("source-map-bound fixture text")
        .text()
        .to_owned();
    let sources = SourceMap::build(vec![
        SourceFileInput { path: main_path.as_str().into(), text: main_text },
        SourceFileInput { path: library_path.as_str().into(), text: library_text },
    ])
    .expect("fixture element");
    let syntax = verify_snapshot(raw, &sources).expect("authenticated public entry");
    let entry = sources.file_id(&main_path).expect("authenticated file identity");
    let errors = lower(
        SemanticInput::try_new(&syntax, &sources, entry).expect("source-bound semantic input"),
    )
    .expect_err("owned entry ABI remains excluded");
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].code, "ZRYNA-M3010");
    assert_eq!(
        errors[0].primary_span(),
        Some(crate::data_ownership_v1::span(&sources, expected_span))
    );
}

#[test]
fn named_import_call_rejects_wrong_arity_before_argument_preparation() {
    let (sources, raw) = imported_fixture(Base::WrongArity, "choose", "select", "./lib.zry");
    let caller =
        raw.files.iter().find(|file| file.path == "src/main.zry").expect("fixture element");
    let call_span = caller.functions[0]
        .body
        .expressions
        .iter()
        .find_map(|expression| {
            matches!(expression.kind, RawExpressionKind::Call { .. }).then_some(expression.span)
        })
        .expect("fixture element");
    let syntax = verify_snapshot(raw, &sources).expect("authenticated wrong arity");
    let entry = sources
        .file_id(&NormalizedSourcePath::new("src/main.zry").expect("fixture path"))
        .expect("fixture path");
    let input =
        SemanticInput::try_new(&syntax, &sources, entry).expect("source-bound semantic input");
    let expected = lower(input).expect_err("wrong arity");
    assert_eq!(expected.len(), 1);
    assert_eq!(expected[0].code, "ZRYNA-M3016");
    assert_eq!(
        expected[0].message,
        "call to 'select' has 2 arguments but its signature requires 3"
    );
    assert_eq!(expected[0].guidance, "pass every exact declared value argument in source order");
    assert_eq!(
        expected[0].primary_span(),
        Some(crate::data_ownership_v1::span(&sources, call_span))
    );
    assert_eq!(lower(input).expect_err("replay"), expected);
}

#[test]
fn named_import_graph_rejects_a_cycle_at_the_closing_edge_exactly() {
    let (sources, mut raw) =
        imported_fixture(Base::Mixed(Case::Direct), "choose", "select", "./lib.zry");
    let library_path = NormalizedSourcePath::new("src/lib.zry").expect("fixture path");
    let main_path = NormalizedSourcePath::new("src/main.zry").expect("fixture path");
    let library_id = sources.file_id(&library_path).expect("authenticated file identity");
    let main_id = sources.file_id(&main_path).expect("authenticated file identity");
    let old_library =
        sources.source(library_id).expect("source-map-bound fixture text").text().to_owned();
    let main_text =
        sources.source(main_id).expect("source-map-bound fixture text").text().to_owned();
    let prefix = "import { choose as cyclex } from './lib.zry';\n";
    let file =
        raw.files.iter_mut().find(|file| file.path == "src/lib.zry").expect("fixture element");
    let mut value = serde_json::to_value(&*file).expect("serializable fixture syntax");
    rewrite_spans(
        &mut value,
        file.id,
        0,
        u32::try_from(prefix.len()).expect("bounded import prefix"),
    );
    *file = serde_json::from_value(value).expect("rebound fixture syntax");
    let imported = prefix.find("choose").expect("fixture element");
    let local = prefix.find("cyclex").expect("fixture element");
    let from = prefix.find("from").expect("fixture element");
    let token = prefix.find("'./lib.zry'").expect("fixture element");
    let expected_cycle_span = span(file.id, token, token + 11);
    file.imports.push(RawImportSyntax {
        span: span(file.id, 0, prefix.len() - 1),
        import_span: span(file.id, 0, 6),
        bindings: vec![RawImportBindingSyntax {
            span: span(file.id, imported, local + 6),
            imported: RawIdentifierSyntax {
                text: "choose".into(),
                span: span(file.id, imported, imported + 6),
            },
            local: RawIdentifierSyntax {
                text: "cyclex".into(),
                span: span(file.id, local, local + 6),
            },
            as_span: Some(span(file.id, local - 3, local - 1)),
        }],
        from_span: span(file.id, from, from + 4),
        specifier: RawModuleSpecifierSyntax {
            text: "./lib.zry".into(),
            token_span: span(file.id, token, token + 11),
            value_span: span(file.id, token + 1, token + 10),
        },
        semicolon_span: span(file.id, prefix.len() - 2, prefix.len() - 1),
    });
    let sources = SourceMap::build(vec![
        SourceFileInput {
            path: library_path.as_str().into(),
            text: format!("{prefix}{old_library}"),
        },
        SourceFileInput { path: main_path.as_str().into(), text: main_text },
    ])
    .expect("fixture element");
    let syntax = verify_snapshot(raw, &sources).expect("authenticated cyclic closure");
    let entry = sources.file_id(&main_path).expect("authenticated file identity");
    let input =
        SemanticInput::try_new(&syntax, &sources, entry).expect("source-bound semantic input");
    let expected = lower(input).expect_err("cycle");
    assert_eq!(expected.len(), 1);
    assert_eq!(expected[0].code, "ZRYNA-M3016");
    assert_eq!(expected[0].message, "the resolved module import graph contains a cycle");
    assert_eq!(expected[0].guidance, "remove the cyclic relative import chain");
    assert_eq!(
        expected[0].primary_span(),
        Some(crate::data_ownership_v1::span(&sources, expected_cycle_span))
    );
    assert_eq!(lower(input).expect_err("replay"), expected);
    let (valid_sources, valid_raw) =
        imported_fixture(Base::Mixed(Case::Direct), "choose", "select", "./lib.zry");
    let valid_syntax = verify_snapshot(valid_raw, &valid_sources).expect("valid recovery syntax");
    let valid_entry = valid_sources
        .file_id(&NormalizedSourcePath::new("src/main.zry").expect("fixture path"))
        .expect("fixture path");
    lower(
        SemanticInput::try_new(&valid_syntax, &valid_sources, valid_entry)
            .expect("source-bound semantic input"),
    )
    .expect("valid recovery after cyclic closure");
}

#[test]
fn named_import_target_still_obeys_the_direct_call_cycle_verifier() {
    let (sources, mut raw) =
        imported_fixture(Base::Mixed(Case::Direct), "choose", "select", "./lib.zry");
    let library_path = NormalizedSourcePath::new("src/lib.zry").expect("fixture path");
    let main_path = NormalizedSourcePath::new("src/main.zry").expect("fixture path");
    let library_id = sources.file_id(&library_path).expect("authenticated file identity");
    let main_id = sources.file_id(&main_path).expect("authenticated file identity");
    let mut library_text =
        sources.source(library_id).expect("source-map-bound fixture text").text().to_owned();
    let main_text =
        sources.source(main_id).expect("source-map-bound fixture text").text().to_owned();
    let file =
        raw.files.iter_mut().find(|file| file.path == "src/lib.zry").expect("fixture element");
    let function = &mut file.functions[0];
    let old = function.body.expressions[0].span;
    let replacement = "choose(left, count, right)";
    library_text.replace_range(old.start as usize..old.end as usize, replacement);
    let delta =
        u32::try_from(replacement.len()).expect("bounded replacement") - (old.end - old.start);
    let mut value = serde_json::to_value(&*file).expect("serializable fixture syntax");
    rewrite_spans(&mut value, file.id, old.end, delta);
    *file = serde_json::from_value(value).expect("rebound fixture syntax");
    let function = &mut file.functions[0];
    let id = file.id;
    let left = old.start as usize + replacement.find("left").expect("fixture element");
    function.body.expressions[0].span = span(id, left, left + 4);
    let left_span = function.body.expressions[0].span;
    let RawExpressionKind::Reference { name } = &mut function.body.expressions[0].kind else {
        panic!("left")
    };
    name.span = left_span;
    let start = old.start as usize;
    let count = start + replacement.find("count").expect("fixture element");
    let right = start + replacement.find("right").expect("fixture element");
    function.body.expressions.push(RawExpressionSyntax {
        span: span(id, count, count + 5),
        kind: RawExpressionKind::Reference {
            name: RawIdentifierSyntax { text: "count".into(), span: span(id, count, count + 5) },
        },
    });
    function.body.expressions.push(RawExpressionSyntax {
        span: span(id, right, right + 5),
        kind: RawExpressionKind::Reference {
            name: RawIdentifierSyntax { text: "right".into(), span: span(id, right, right + 5) },
        },
    });
    let open = start + 6;
    function.body.expressions.push(RawExpressionSyntax {
        span: span(id, start, start + replacement.len()),
        kind: RawExpressionKind::Call {
            callee: RawIdentifierSyntax { text: "choose".into(), span: span(id, start, start + 6) },
            open_paren_span: span(id, open, open + 1),
            arguments: vec![0, 1, 2],
            close_paren_span: span(id, start + replacement.len() - 1, start + replacement.len()),
        },
    });
    let RawStatementKind::Return { value, .. } = &mut function.body.statements[0].kind else {
        panic!("return")
    };
    *value = 3;
    let sources = SourceMap::build(vec![
        SourceFileInput { path: library_path.as_str().into(), text: library_text },
        SourceFileInput { path: main_path.as_str().into(), text: main_text },
    ])
    .expect("fixture element");
    let syntax = verify_snapshot(raw, &sources).expect("authenticated recursive callee");
    let entry = sources.file_id(&main_path).expect("authenticated file identity");
    let errors = lower(
        SemanticInput::try_new(&syntax, &sources, entry).expect("source-bound semantic input"),
    )
    .expect_err("call cycle");
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].code, "ZRYNA-I3009");
    assert_eq!(errors[0].message, "direct call graph contains a cycle");
    assert_eq!(errors[0].guidance, "use one acyclic exact-signature direct call graph");
}
