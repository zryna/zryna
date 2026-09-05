use super::generic_call_fixture::{Case, fixture, single_string_fixture};
use super::generic_vec_fixture::Element;
use super::*;
use zryna_syntax::v4::{
    RawExpressionKind, RawImportBindingSyntax, RawImportSyntax, RawModuleSpecifierSyntax,
    RawStatementKind, RawTypeSyntaxKind,
};

#[derive(Clone, Copy)]
pub(super) enum Base {
    Mixed(Case),
    OwnedToCopy,
    ZeroArgument,
    BoolResult,
    WrongArity,
}

fn zero_argument_fixture() -> (String, RawProjectSyntaxSnapshot) {
    let (mut source, mut raw) = private_string_call_fixture();
    let caller = &mut raw.files[0].functions[0];
    let outer = caller.body.expressions.remove(2);
    let RawExpressionKind::Call { callee, open_paren_span, close_paren_span, .. } = outer.kind
    else {
        panic!("identity call")
    };
    source.replace_range(callee.span.start as usize..open_paren_span.end as usize, "         ");
    source.replace_range(close_paren_span.start as usize..close_paren_span.end as usize, " ");
    let RawStatementKind::LocalDeclaration { initializer, .. } =
        &mut caller.body.statements[1].kind
    else {
        panic!("value local")
    };
    *initializer = 1;
    let RawStatementKind::Return { value, .. } = &mut caller.body.statements[2].kind else {
        panic!("return")
    };
    *value -= 1;
    let RawExpressionKind::Clone { value, .. } = &mut caller.body.expressions[3].kind else {
        panic!("return clone")
    };
    *value -= 1;
    (source, raw)
}

fn bool_result_fixture() -> (String, RawProjectSyntaxSnapshot) {
    let (mut source, mut raw) = single_string_fixture();
    let file = &mut raw.files[0];
    let mut result_types =
        file.functions.iter().map(|function| function.result_type).collect::<Vec<_>>();
    result_types.sort_by_key(|id| std::cmp::Reverse(file.type_syntax[*id as usize].span.start));
    for id in result_types {
        let at = file.type_syntax[id as usize].span;
        source.replace_range(at.start as usize..at.end as usize, "bool");
        let mut value = serde_json::to_value(&*file).expect("file");
        rewrite_spans(&mut value, 0, at.end, 1);
        *file = serde_json::from_value(value).expect("file");
        let RawTypeSyntaxKind::Named { name } = &mut file.type_syntax[id as usize].kind else {
            panic!("i32 result")
        };
        name.text = "bool".into();
    }
    let literal = file.functions[1].body.expressions[0].span;
    source.replace_range(literal.start as usize..literal.end as usize, "false");
    let mut value = serde_json::to_value(&*file).expect("file");
    rewrite_spans(&mut value, 0, literal.end, 4);
    *file = serde_json::from_value(value).expect("file");
    file.functions[1].body.expressions[0].kind = RawExpressionKind::BoolLiteral { value: false };
    (source, raw)
}

fn wrong_arity_fixture() -> (String, RawProjectSyntaxSnapshot) {
    let (mut source, mut raw) = fixture(&Element::String, Case::Direct);
    let caller = &mut raw.files[0].functions[0];
    let RawStatementKind::Return { value: call_id, .. } = caller.body.statements[0].kind else {
        panic!("return")
    };
    let RawExpressionKind::Call { arguments, .. } =
        &mut caller.body.expressions[call_id as usize].kind
    else {
        panic!("call")
    };
    let removed = arguments.pop().expect("third argument");
    let at = caller.body.expressions[removed as usize].span;
    source.replace_range(at.start as usize - 2..at.end as usize, "       ");
    caller.body.expressions.remove(removed as usize);
    let RawStatementKind::Return { value, .. } = &mut caller.body.statements[0].kind else {
        panic!("return")
    };
    *value -= 1;
    (source, raw)
}

pub(super) fn rewrite_spans(value: &mut serde_json::Value, file: u32, cutoff: u32, shift: u32) {
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

pub(super) fn span(file: u32, start: usize, end: usize) -> zryna_source::UntrustedSpan {
    zryna_source::UntrustedSpan {
        file,
        start: start.try_into().expect("offset"),
        end: end.try_into().expect("offset"),
    }
}

fn retain_function(file: &mut RawSourceUnit, index: usize) {
    let mut function = file.functions[index].clone();
    let mut remap = vec![u32::MAX; file.type_syntax.len()];
    let mut types = Vec::new();
    for (old, ty) in file.type_syntax.iter().enumerate() {
        if ty.span.start >= function.span.start && ty.span.end <= function.span.end {
            remap[old] = types.len().try_into().expect("type index");
            types.push(ty.clone());
        }
    }
    let mapped = |id: &mut u32| *id = remap[*id as usize];
    for ty in &mut types {
        match &mut ty.kind {
            RawTypeSyntaxKind::Vec { argument, .. }
            | RawTypeSyntaxKind::Shared { argument, .. }
            | RawTypeSyntaxKind::Weak { argument, .. }
            | RawTypeSyntaxKind::Borrow { argument, .. }
            | RawTypeSyntaxKind::BorrowMut { argument, .. } => mapped(argument),
            RawTypeSyntaxKind::FixedArray { element, .. } => mapped(element),
            RawTypeSyntaxKind::Missing
            | RawTypeSyntaxKind::Named { .. }
            | RawTypeSyntaxKind::String { .. } => {}
        }
    }
    for parameter in &mut function.parameters {
        mapped(&mut parameter.type_syntax);
    }
    mapped(&mut function.result_type);
    for statement in &mut function.body.statements {
        if let RawStatementKind::LocalDeclaration { type_syntax, .. } = &mut statement.kind {
            mapped(type_syntax);
        }
    }
    for expression in &mut function.body.expressions {
        match &mut expression.kind {
            RawExpressionKind::FixedArrayConstruction { type_syntax, .. }
            | RawExpressionKind::VecConstruction { type_syntax, .. } => mapped(type_syntax),
            _ => {}
        }
    }
    file.type_syntax = types;
    file.functions = vec![function];
}

pub(super) fn imported_fixture(
    base: Base,
    imported_name: &str,
    local_name: &str,
    import_path: &str,
) -> (SourceMap, RawProjectSyntaxSnapshot) {
    let (sources, raw, _, _) = imported_fixture_paths(
        base,
        imported_name,
        local_name,
        import_path,
        "src/main.zry",
        "src/lib.zry",
    );
    (sources, raw)
}

pub(in crate::data_ownership_v1) fn imported_zero_argument_fixture()
-> (SourceMap, RawProjectSyntaxSnapshot) {
    imported_fixture(Base::ZeroArgument, "producer", "factoryx", "./lib.zry")
}

fn imported_fixture_paths(
    base: Base,
    imported_name: &str,
    local_name: &str,
    import_path: &str,
    main_path: &str,
    library_path: &str,
) -> (SourceMap, RawProjectSyntaxSnapshot, usize, usize) {
    let (main_id, library_id) = if main_path < library_path { (0_u32, 1_u32) } else { (1, 0) };
    let (source, raw, target, call_name) = match base {
        Base::Mixed(case) => {
            let (source, raw) = fixture(&Element::String, case);
            (source, raw, 1, "choose")
        }
        Base::OwnedToCopy => {
            let (source, raw) = single_string_fixture();
            (source, raw, 1, "consume")
        }
        Base::ZeroArgument => {
            let (source, raw) = zero_argument_fixture();
            (source, raw, 2, "producer")
        }
        Base::BoolResult => {
            let (source, raw) = bool_result_fixture();
            (source, raw, 1, "consume")
        }
        Base::WrongArity => {
            let (source, raw) = wrong_arity_fixture();
            (source, raw, 1, "choose")
        }
    };
    let prefix = format!("import {{ {imported_name} as {local_name} }} from '{import_path}';\n");
    let mut main = shift_snapshot(raw.clone(), 0, prefix.len() as u32).files.remove(0);
    let call = main.functions[0]
        .body
        .expressions
        .iter_mut()
        .find_map(|expression| match &mut expression.kind {
            RawExpressionKind::Call { callee, .. } if callee.text == call_name => Some(callee),
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
    retain_function(&mut main, 0);

    let mut library = raw.files[0].clone();
    library.id = library_id;
    library.path = library_path.into();
    let insertion = library.functions[target].span.start as usize;
    retain_function(&mut library, target);
    let mut library_value = serde_json::to_value(library).expect("library");
    rewrite_spans(&mut library_value, library_id, insertion as u32, 7);
    let mut library: RawSourceUnit = serde_json::from_value(library_value).expect("library");
    library.functions[0].span.start = insertion as u32;
    library.functions[0].export_span = Some(span(library_id, insertion, insertion + 6));

    main.id = main_id;
    main.path = main_path.into();
    let mut main_value = serde_json::to_value(main).expect("main");
    rewrite_spans(&mut main_value, main_id, u32::MAX, 0);
    let main = serde_json::from_value(main_value).expect("main");

    let mut library_source = source.clone();
    library_source.insert_str(insertion, "export ");
    let mut main_source = format!("{prefix}{source}");
    main_source.replace_range(call_start..call_end, local_name);
    let sources = SourceMap::build(vec![
        SourceFileInput { path: main_path.into(), text: main_source },
        SourceFileInput { path: library_path.into(), text: library_source },
    ])
    .expect("sources");
    let files = if library_id == 0 { vec![library, main] } else { vec![main, library] };
    (
        sources,
        RawProjectSyntaxSnapshot { schema_version: PROTOCOL_VERSION, files, diagnostics: vec![] },
        main_id as usize,
        library_id as usize,
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
    assert_eq!((callee.module(), callee.declaration()), (0, 0));
    let arguments = call
        .call_arguments()
        .map(|argument| match argument {
            VerifiedCallArgument::Value(value) => value.index(),
            VerifiedCallArgument::Borrow(_) => panic!("by-value call"),
        })
        .collect::<Vec<_>>();
    assert_eq!(arguments, [4, 5, 6], "left/count/right preparation order");
    assert_eq!(
        call.derived_drop_actions().map(|action| action.root().index()).collect::<Vec<_>>(),
        [3],
        "CallTrap retains only the untransferred caller survivor"
    );
    let call_cleanup =
        caller.cleanup_plans().find(|plan| Some(plan.id()) == call.cleanup()).unwrap();
    assert_eq!(call_cleanup.site().role(), VerifiedCleanupRole::CallTrap);
    let callee_function = modules[0].functions().next().unwrap();
    assert_eq!(
        callee_function
            .blocks()
            .next()
            .unwrap()
            .terminator()
            .derived_drop_actions()
            .map(|action| action.root().index())
            .collect::<Vec<_>>(),
        [2],
        "callee transfers its returned left input and cleans the right input exactly once"
    );
    assert_eq!(
        caller
            .blocks()
            .next()
            .unwrap()
            .terminator()
            .derived_drop_actions()
            .map(|action| action.root().index())
            .collect::<Vec<_>>(),
        [3],
        "returned owned result is transferred while the caller survivor is cleaned"
    );
    assert_eq!(modules[0].functions().count(), 1);
}

#[test]
fn named_import_calls_cover_nested_owned_results_and_owned_input_copy_results() {
    let cases = [
        (Base::Mixed(Case::Nested), "choose", "select", 3_usize),
        (Base::OwnedToCopy, "consume", "process", 1_usize),
        (Base::ZeroArgument, "producer", "factoryx", 0_usize),
        (Base::BoolResult, "consume", "process", 1_usize),
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
            (0, 0)
        );
        assert_eq!(call.call_arguments().count(), arguments);
    }
}

#[test]
fn named_import_resolution_is_independent_of_authenticated_path_order() {
    let paths = [
        ("src/main.zry", "src/lib.zry", "./lib.zry"),
        ("src/a-main.zry", "src/z-lib.zry", "./z-lib.zry"),
    ];
    for (main_path, library_path, import_path) in paths {
        let (sources, raw, main, library) = imported_fixture_paths(
            Base::Mixed(Case::Direct),
            "choose",
            "select",
            import_path,
            main_path,
            library_path,
        );
        let syntax = verify_snapshot(raw, &sources).expect("authenticated ordered closure");
        let entry =
            sources.file_id(&NormalizedSourcePath::new(main_path).expect("path")).expect("entry");
        let program = lower(SemanticInput::try_new(&syntax, &sources, entry).expect("input"))
            .unwrap_or_else(|errors| panic!("{errors:?}"));
        let caller = program.modules().nth(main).expect("entry module").functions().next().unwrap();
        let callee = caller
            .blocks()
            .next()
            .unwrap()
            .instructions()
            .find_map(|instruction| instruction.callee())
            .expect("imported call");
        assert_eq!((callee.module() as usize, callee.declaration()), (library, 0));
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
