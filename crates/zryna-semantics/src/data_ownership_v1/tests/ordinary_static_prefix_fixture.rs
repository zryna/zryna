use super::*;

fn element(f: &mut Builder, sibling: i32, name: &str) -> u32 {
    let root = f.reference("items");
    let vector = index(f, root, Some(sibling), "unused");
    index(f, vector, None, name)
}

pub(crate) fn fixture(same: bool) -> (String, RawProjectSyntaxSnapshot) {
    let (mut f, declarations, _) = initial(&Element::String);
    let container = Ty::Array(Box::new(Ty::Vec(Box::new(Ty::String))), 2);
    let mut functions =
        vec![function(&mut f, "observe", &[("incoming", container.clone())], &container, |f| {
            f.local("items", &container, true, "incoming");
            let start = f.source.len();
            let target = element(f, 0, "firstIndex");
            f.text(" ");
            let equals_span = f.text("=");
            f.text(" ");
            let value = f.clone_value(|f| element(f, i32::from(!same), "secondIndex"));
            let semicolon_span = f.text(";");
            f.statements.push(RawStatementSyntax {
                span: at(start, f.source.len()),
                kind: RawStatementKind::Assignment { target, equals_span, value, semicolon_span },
            });
            f.text(" return ");
            f.reference("items")
        })];
    for name in ["firstIndex", "secondIndex"] {
        functions.push(function(&mut f, name, &[], &Ty::Named("i32"), |f| {
            f.text("return ");
            let start = f.source.len();
            f.text("0");
            f.expression(start, RawExpressionKind::I32Literal { spelling: "0".into() })
        }));
    }
    let raw = RawProjectSyntaxSnapshot {
        schema_version: PROTOCOL_VERSION,
        files: vec![RawSourceUnit {
            id: 0,
            path: "src/main.zry".into(),
            imports: Vec::new(),
            type_syntax: f.types,
            data_declarations: declarations,
            functions,
        }],
        diagnostics: Vec::new(),
    };
    (f.source, raw)
}
