use super::generic_call_fixture::Case;
use super::named_import_calls::{Base, imported_fixture, rewrite_spans, span};
use super::*;
use zryna_syntax::v4::RawExpressionKind;

pub(super) fn multi_hop_fixture() -> (SourceMap, RawProjectSyntaxSnapshot) {
    let (sources, raw) =
        imported_fixture(Base::Mixed(Case::Direct), "choose", "select", "./lib.zry");
    let library_path = NormalizedSourcePath::new("src/lib.zry").expect("fixture path");
    let entry_path = NormalizedSourcePath::new("src/main.zry").expect("fixture path");
    let library_text = sources
        .source(sources.file_id(&library_path).expect("source-map-bound fixture text"))
        .expect("source-map-bound fixture text")
        .text()
        .to_owned();
    let original = sources
        .source(sources.file_id(&entry_path).expect("source-map-bound fixture text"))
        .expect("source-map-bound fixture text")
        .text()
        .to_owned();
    let library =
        raw.files.iter().find(|file| file.path == "src/lib.zry").expect("fixture element").clone();
    let original_main =
        raw.files.iter().find(|file| file.path == "src/main.zry").expect("fixture element").clone();

    let mut mid = original_main.clone();
    mid.id = 2;
    mid.path = "src/mid.zry".into();
    let insertion = mid.functions[0].span.start;
    let mut value = serde_json::to_value(mid).expect("serializable fixture syntax");
    rewrite_spans(&mut value, 2, u32::MAX, 0);
    rewrite_spans(&mut value, 2, insertion, 7);
    let mut mid: RawSourceUnit = serde_json::from_value(value).expect("rebound fixture syntax");
    mid.functions[0].span.start = insertion;
    mid.functions[0].export_span = Some(span(2, insertion as usize, insertion as usize + 6));
    let mut mid_text = original.clone();
    mid_text.insert_str(insertion as usize, "export ");
    let unused = mid.functions[0].parameters.pop().expect("unused keep parameter");
    mid_text.replace_range(
        unused.span.start as usize - 2..unused.span.end as usize,
        &" ".repeat((unused.span.end - unused.span.start + 2) as usize),
    );
    let removed_type = unused.type_syntax;
    mid.type_syntax.remove(removed_type as usize);
    let function = &mut mid.functions[0];
    for parameter in &mut function.parameters {
        if parameter.type_syntax > removed_type {
            parameter.type_syntax -= 1;
        }
    }
    if function.result_type > removed_type {
        function.result_type -= 1;
    }

    let mut entry = original_main;
    entry.imports[0].bindings[0].imported.text = "caller".into();
    entry.imports[0].bindings[0].local.text = "bridge".into();
    entry.imports[0].specifier.text = "./mid.zry".into();
    let call = entry.functions[0]
        .body
        .expressions
        .iter_mut()
        .find_map(|expression| match &mut expression.kind {
            RawExpressionKind::Call { callee, .. } => Some(callee),
            _ => None,
        })
        .expect("fixture element");
    call.text = "bridge".into();
    let entry_text = original
        .replacen("choose as select", "caller as bridge", 1)
        .replacen("./lib.zry", "./mid.zry", 1)
        .replacen("select(", "bridge(", 1);

    let mid_path = NormalizedSourcePath::new("src/mid.zry").expect("fixture path");
    let sources = SourceMap::build(vec![
        SourceFileInput { path: library_path.as_str().into(), text: library_text },
        SourceFileInput { path: entry_path.as_str().into(), text: entry_text },
        SourceFileInput { path: mid_path.as_str().into(), text: mid_text },
    ])
    .expect("fixture element");
    (
        sources,
        RawProjectSyntaxSnapshot {
            schema_version: PROTOCOL_VERSION,
            files: vec![library, entry, mid],
            diagnostics: vec![],
        },
    )
}

#[test]
fn named_import_calls_follow_a_canonical_acyclic_multi_hop_chain() {
    let (sources, raw) = multi_hop_fixture();
    let syntax = verify_snapshot(raw, &sources).expect("authenticated multi-hop closure");
    let entry_path = NormalizedSourcePath::new("src/main.zry").expect("fixture path");
    let entry_id = sources.file_id(&entry_path).expect("authenticated file identity");
    let program = lower(
        SemanticInput::try_new(&syntax, &sources, entry_id).expect("source-bound semantic input"),
    )
    .unwrap_or_else(|errors| panic!("{errors:?}"));
    let modules = program.modules().collect::<Vec<_>>();
    let entry_call = modules[1]
        .functions()
        .next()
        .expect("fixture element")
        .blocks()
        .next()
        .expect("fixture element")
        .instructions()
        .find_map(FaultVerifiedInstruction::callee)
        .expect("fixture element");
    let mid_call = modules[2]
        .functions()
        .next()
        .expect("fixture element")
        .blocks()
        .next()
        .expect("fixture element")
        .instructions()
        .find_map(FaultVerifiedInstruction::callee)
        .expect("fixture element");
    assert_eq!((entry_call.module(), entry_call.declaration()), (2, 0));
    assert_eq!((mid_call.module(), mid_call.declaration()), (0, 0));
}
