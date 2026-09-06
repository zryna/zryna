use super::*;
use crate::data_ownership_v1::function_catalog::{
    FunctionBorrowParameter, FunctionCatalog, FunctionParameterOrder, FunctionResolution,
    FunctionSignature,
};
use crate::data_ownership_v1::{SemanticInput, Ty, lower, raw};
use zryna_layout::{StorageTarget, TypeCategory, raw as graph};
use zryna_source::{NormalizedSourcePath, SourceFileInput, SourceMap};
use zryna_syntax::v4::{
    PROTOCOL_VERSION, RawExpressionKind, RawIdentifierSyntax, RawImportBindingSyntax,
    RawImportSyntax, RawModuleSpecifierSyntax, RawProjectSyntaxSnapshot, RawSourceUnit,
    RawStatementKind, RawTypeSyntaxKind, verify_snapshot,
};

fn authorities() -> (SourceMap, zryna_layout::VerifiedLayouts, zryna_source::Span) {
    let sources = SourceMap::build(vec![
        SourceFileInput { path: "src/main.zry".into(), text: "Parcel".into() },
        SourceFileInput { path: "src/lib.zry".into(), text: "Parcel\nChoice".into() },
    ])
    .expect("source map");
    let main = sources
        .file_id(&NormalizedSourcePath::new("src/main.zry").expect("main path"))
        .expect("main");
    let library = sources
        .file_id(&NormalizedSourcePath::new("src/lib.zry").expect("library path"))
        .expect("library");
    let main_span = sources.span(main, 0, 6).expect("main declaration");
    let foreign_span = sources.span(library, 0, 6).expect("foreign declaration");
    let enum_span = sources.span(library, 7, 13).expect("enum declaration");
    let types = vec![
        graph::TypeNode { id: graph::NodeId(0), span: None, kind: graph::TypeKind::Bool },
        graph::TypeNode { id: graph::NodeId(1), span: None, kind: graph::TypeKind::I32 },
        graph::TypeNode { id: graph::NodeId(2), span: None, kind: graph::TypeKind::String },
        graph::TypeNode {
            id: graph::NodeId(3),
            span: Some(main_span),
            kind: graph::TypeKind::Struct {
                module: graph::ModuleId(1),
                declaration: 0,
                fields: vec![graph::Field { ordinal: 0, ty: graph::NodeId(2) }],
            },
        },
        graph::TypeNode {
            id: graph::NodeId(4),
            span: Some(foreign_span),
            kind: graph::TypeKind::Struct {
                module: graph::ModuleId(0),
                declaration: 0,
                fields: vec![graph::Field { ordinal: 0, ty: graph::NodeId(2) }],
            },
        },
        graph::TypeNode {
            id: graph::NodeId(5),
            span: Some(enum_span),
            kind: graph::TypeKind::Enum {
                module: graph::ModuleId(0),
                declaration: 1,
                variants: vec![
                    graph::Variant { ordinal: 0, payload: None },
                    graph::Variant { ordinal: 1, payload: Some(graph::NodeId(4)) },
                ],
            },
        },
        graph::TypeNode {
            id: graph::NodeId(6),
            span: None,
            kind: graph::TypeKind::FixedArray { element: graph::NodeId(4), length: 2 },
        },
        graph::TypeNode {
            id: graph::NodeId(7),
            span: None,
            kind: graph::TypeKind::Vec { element: graph::NodeId(4) },
        },
        graph::TypeNode {
            id: graph::NodeId(8),
            span: None,
            kind: graph::TypeKind::Shared { payload: graph::NodeId(5) },
        },
        graph::TypeNode {
            id: graph::NodeId(9),
            span: None,
            kind: graph::TypeKind::Weak { payload: graph::NodeId(5) },
        },
    ];
    let raw = graph::Graph {
        modules: vec![
            graph::Module { id: graph::ModuleId(0), source_file: library, data_declarations: 2 },
            graph::Module { id: graph::ModuleId(1), source_file: main, data_declarations: 1 },
        ],
        types,
        program_roots: (3..10).map(graph::NodeId).collect(),
    };
    let layouts = zryna_layout::verify(&raw, &sources, StorageTarget::Linear32V1)
        .expect("sealed ownership graph");
    (sources, layouts, foreign_span)
}

fn ty(record: zryna_layout::VerifiedType<'_>) -> Ty {
    Ty {
        layout: record.id(),
        ir: raw::TypeId(record.id().index()),
        category: record.category(),
        drop_kind: record.drop_kind(),
        runtime_kind: record.runtime_kind(),
        cloneable: true,
    }
}

fn category(layouts: &zryna_layout::VerifiedLayouts, category: TypeCategory) -> Ty {
    ty(layouts.types().find(|record| record.category() == category).expect("fixture category"))
}

fn nominal(layouts: &zryna_layout::VerifiedLayouts, module: u32, declaration: u32) -> Ty {
    ty(layouts
        .types()
        .find(|record| record.nominal_identity() == Some((module, declaration)))
        .expect("fixture nominal"))
}

fn signature(id: raw::FunctionId, parameters: Vec<Ty>, result: Ty) -> FunctionSignature {
    FunctionSignature {
        id,
        name: "consume".into(),
        parameter_order: (0..parameters.len())
            .map(|index| FunctionParameterOrder::Value(u32::try_from(index).expect("parameter")))
            .collect(),
        parameters,
        borrow_parameters: Vec::new(),
        result,
        private: true,
    }
}

fn relocate(unit: RawSourceUnit, file: u32, delta: i64) -> RawSourceUnit {
    fn visit(value: &mut serde_json::Value, file: u32, delta: i64) {
        match value {
            serde_json::Value::Object(object)
                if object.contains_key("file")
                    && object.contains_key("start")
                    && object.contains_key("end") =>
            {
                *object.get_mut("file").expect("span file") = file.into();
                for key in ["start", "end"] {
                    let current = i64::try_from(object[key].as_u64().expect("span offset"))
                        .expect("bounded span offset");
                    *object.get_mut(key).expect("span offset") =
                        u64::try_from(current + delta).expect("relocated offset").into();
                }
            }
            serde_json::Value::Object(object) => {
                for child in object.values_mut() {
                    visit(child, file, delta);
                }
            }
            serde_json::Value::Array(values) => {
                for child in values {
                    visit(child, file, delta);
                }
            }
            _ => {}
        }
    }
    let mut value = serde_json::to_value(unit).expect("source unit");
    visit(&mut value, file, delta);
    serde_json::from_value(value).expect("relocated source unit")
}

fn retain_function(mut file: RawSourceUnit, index: usize) -> RawSourceUnit {
    let mut function = file.functions[index].clone();
    let mut remap = vec![u32::MAX; file.type_syntax.len()];
    let mut types = Vec::new();
    for (old, ty) in file.type_syntax.iter().enumerate() {
        if ty.span.start >= function.span.start && ty.span.end <= function.span.end {
            remap[old] = u32::try_from(types.len()).expect("type index");
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
    file.imports.clear();
    file.data_declarations.clear();
    file.type_syntax = types;
    file.functions = vec![function];
    file
}

#[allow(clippy::too_many_lines)]
fn imported_vec_source() -> (SourceMap, RawProjectSyntaxSnapshot) {
    use crate::data_ownership_v1::tests::generic_call_fixture::{Case, fixture};
    use crate::data_ownership_v1::tests::generic_vec_fixture::Element;

    let (source, raw) = fixture(&Element::Vec, Case::Direct);
    let original = &raw.files[0];
    let caller_function = &original.functions[0];
    let target_function = &original.functions[1];
    let prefix = "import { choose as select } from './lib.zry';\n";
    let mut caller_source =
        source[caller_function.span.start as usize..caller_function.span.end as usize].to_owned();
    let call = caller_function
        .body
        .expressions
        .iter()
        .find_map(|expression| match &expression.kind {
            RawExpressionKind::Call { callee, .. } if callee.text == "choose" => Some(callee),
            _ => None,
        })
        .expect("caller target");
    let call_start =
        usize::try_from(call.span.start - caller_function.span.start).expect("call offset");
    caller_source.replace_range(call_start..call_start + "choose".len(), "select");
    let main_source = format!("{prefix}{caller_source}");
    let library_source = format!(
        "export {}",
        &source[target_function.span.start as usize..target_function.span.end as usize]
    );
    let sources = SourceMap::build(vec![
        SourceFileInput { path: "src/main.zry".into(), text: main_source },
        SourceFileInput { path: "src/lib.zry".into(), text: library_source },
    ])
    .expect("imported Vec sources");
    let library_id = sources
        .file_id(&NormalizedSourcePath::new("src/lib.zry").expect("path"))
        .expect("library")
        .index();
    let main_id = sources
        .file_id(&NormalizedSourcePath::new("src/main.zry").expect("path"))
        .expect("main")
        .index();
    let mut library = relocate(
        retain_function(original.clone(), 1),
        library_id,
        7_i64 - i64::from(target_function.span.start),
    );
    library.id = library_id;
    library.path = "src/lib.zry".into();
    library.functions[0].span.start = 0;
    library.functions[0].export_span =
        Some(zryna_source::UntrustedSpan { file: library_id, start: 0, end: 6 });

    let mut main = relocate(
        retain_function(original.clone(), 0),
        main_id,
        i64::try_from(prefix.len()).expect("prefix") - i64::from(caller_function.span.start),
    );
    main.id = main_id;
    main.path = "src/main.zry".into();
    let call = main.functions[0]
        .body
        .expressions
        .iter_mut()
        .find_map(|expression| match &mut expression.kind {
            RawExpressionKind::Call { callee, .. } if callee.text == "choose" => Some(callee),
            _ => None,
        })
        .expect("relocated caller target");
    call.text = "select".into();
    let imported = prefix.find("choose").expect("imported name");
    let local = prefix.find("select").expect("local name");
    let from = prefix.find("from").expect("from keyword");
    let specifier = prefix.find("'./lib.zry'").expect("specifier");
    let span = |start: usize, end: usize| zryna_source::UntrustedSpan {
        file: main_id,
        start: u32::try_from(start).expect("span"),
        end: u32::try_from(end).expect("span"),
    };
    main.imports = vec![RawImportSyntax {
        span: span(0, prefix.len() - 1),
        import_span: span(0, 6),
        bindings: vec![RawImportBindingSyntax {
            span: span(imported, local + "select".len()),
            imported: RawIdentifierSyntax {
                text: "choose".into(),
                span: span(imported, imported + "choose".len()),
            },
            local: RawIdentifierSyntax {
                text: "select".into(),
                span: span(local, local + "select".len()),
            },
            as_span: Some(span(local - 3, local - 1)),
        }],
        from_span: span(from, from + 4),
        specifier: RawModuleSpecifierSyntax {
            text: "./lib.zry".into(),
            token_span: span(specifier, specifier + "'./lib.zry'".len()),
            value_span: span(specifier + 1, specifier + "'./lib.zry".len()),
        },
        semicolon_span: span(prefix.len() - 2, prefix.len() - 1),
    }];
    (
        sources,
        RawProjectSyntaxSnapshot {
            schema_version: PROTOCOL_VERSION,
            files: vec![library, main],
            diagnostics: Vec::new(),
        },
    )
}

#[test]
fn imported_signature_accepts_the_complete_sealed_by_value_graph() {
    let (sources, layouts, at) = authorities();
    let parameters = [
        TypeCategory::Struct,
        TypeCategory::Enum,
        TypeCategory::FixedArray,
        TypeCategory::Vec,
        TypeCategory::Shared,
        TypeCategory::Weak,
    ]
    .map(|kind| category(&layouts, kind))
    .to_vec();
    let target = signature(
        raw::FunctionId { module: raw::ModuleId(0), declaration: 0 },
        parameters,
        category(&layouts, TypeCategory::Vec),
    );
    let mut errors = Errors::new(&sources);
    assert!(supported_signature(&target, &layouts, at, &mut errors));
    assert!(errors.finish().is_empty());
}

#[test]
fn imported_vec_signature_lowers_through_the_canonical_foreign_function() {
    let (sources, raw) = imported_vec_source();
    let syntax = verify_snapshot(raw, &sources).expect("authenticated imported Vec source");
    let entry =
        sources.file_id(&NormalizedSourcePath::new("src/main.zry").expect("path")).expect("entry");
    let program = lower(SemanticInput::try_new(&syntax, &sources, entry).expect("semantic input"))
        .unwrap_or_else(|errors| panic!("{errors:?}"));
    let caller = program.modules().nth(1).expect("main module").functions().next().expect("caller");
    let call = caller
        .blocks()
        .next()
        .expect("caller block")
        .instructions()
        .find(|instruction| instruction.callee().is_some())
        .expect("imported call");
    let target = call.callee().expect("callee identity");
    assert_eq!((target.module(), target.declaration()), (0, 0));
    assert_eq!(call.call_arguments().count(), 3);
}

#[test]
fn imported_producer_and_consumer_retain_one_foreign_nominal_identity() {
    let (_, layouts, _) = authorities();
    let local = nominal(&layouts, 1, 0);
    let foreign = nominal(&layouts, 0, 0);
    assert_ne!(local, foreign, "same-shaped nominals retain distinct sealed identities");
    let local_signature =
        signature(raw::FunctionId { module: raw::ModuleId(1), declaration: 0 }, vec![local], local);
    let foreign_signature = signature(
        raw::FunctionId { module: raw::ModuleId(0), declaration: 1 },
        vec![foreign],
        category(&layouts, TypeCategory::I32),
    );
    let mut producer = signature(
        raw::FunctionId { module: raw::ModuleId(0), declaration: 0 },
        Vec::new(),
        foreign,
    );
    producer.name = "produce".into();
    let mut catalog = FunctionCatalog {
        modules: vec![
            vec![Some(producer.clone()), Some(foreign_signature.clone())],
            vec![Some(local_signature)],
        ],
    };
    catalog.bind_import(1, "foreignProduce".into(), &producer);
    catalog.bind_import(1, "foreignConsume".into(), &foreign_signature);
    let FunctionResolution::Exact(imported_producer) = catalog.resolve(1, "foreignProduce") else {
        panic!("imported producer alias")
    };
    let FunctionResolution::Exact(imported) = catalog.resolve(1, "foreignConsume") else {
        panic!("imported alias")
    };
    assert_eq!(imported.id, foreign_signature.id);
    assert_eq!(imported.parameters, [foreign]);
    assert_eq!(imported_producer.id, producer.id);
    assert_eq!(imported_producer.result, imported.parameters[0]);
    assert_ne!(imported.parameters, [local], "local same-shaped nominal cannot substitute");
}

#[test]
fn borrowed_import_rejects_exactly_and_recovers_deterministically() {
    let (sources, layouts, at) = authorities();
    let foreign = nominal(&layouts, 0, 0);
    let mut target = signature(
        raw::FunctionId { module: raw::ModuleId(0), declaration: 0 },
        Vec::new(),
        foreign,
    );
    target.borrow_parameters.push(FunctionBorrowParameter {
        referent: foreign,
        access: raw::BorrowAccess::Shared,
        span: at,
    });
    target.parameter_order.push(FunctionParameterOrder::Borrow(0));
    let reject = || {
        let mut errors = Errors::new(&sources);
        assert!(!supported_signature(&target, &layouts, at, &mut errors));
        errors.finish()
    };
    let expected = reject();
    assert_eq!(reject(), expected);
    assert_eq!(expected.len(), 1);
    assert_eq!(expected[0].code, "ZRYNA-M3016");
    assert_eq!(
        expected[0].message,
        "named-import call signature is outside the sealed by-value ownership graph"
    );

    let valid = signature(
        raw::FunctionId { module: raw::ModuleId(0), declaration: 0 },
        vec![foreign],
        foreign,
    );
    let mut errors = Errors::new(&sources);
    assert!(supported_signature(&valid, &layouts, at, &mut errors));
    assert!(errors.finish().is_empty());
}
