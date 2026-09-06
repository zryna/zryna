use super::*;

fn call_statement(builder: &mut Builder) {
    let start = builder.source.len();
    let keyword_span = builder.text("const");
    builder.text(" ");
    let name = builder.name("observed");
    builder.text(": ");
    let type_syntax = builder.ty(&Ty::Named("i32"));
    builder.text(" ");
    let equals_span = builder.text("=");
    builder.text(" ");
    let call_start = builder.source.len();
    let callee = builder.name("relay");
    let open_paren_span = builder.text("(");
    let argument = builder.reference("loan");
    let close_paren_span = builder.text(")");
    let expression = builder.expression(
        call_start,
        RawExpressionKind::Call {
            callee,
            open_paren_span,
            arguments: vec![argument],
            close_paren_span,
        },
    );
    let semicolon_span = builder.text(";");
    builder.statements.push(RawStatementSyntax {
        span: at(start, builder.source.len()),
        kind: RawStatementKind::LocalDeclaration {
            keyword_span,
            mutable: false,
            name,
            type_syntax,
            equals_span,
            initializer: expression,
            semicolon_span,
        },
    });
    builder.text(" ");
}

fn callee(builder: &mut Builder, ty: &Ty, exclusive: bool) -> RawFunctionSyntax {
    let start = builder.source.len();
    let function_span = builder.text("function");
    builder.text(" ");
    let name = builder.name("relay");
    builder.text("(");
    let parameter_start = builder.source.len();
    let parameter_name = builder.name("loan");
    builder.text(": ");
    let type_start = builder.source.len();
    let keyword_span = builder.text(if exclusive { "BorrowMut" } else { "Borrow" });
    let less_than_span = builder.text("<");
    let argument = builder.ty(ty);
    let greater_than_span = builder.text(">");
    let type_syntax = u32::try_from(builder.types.len()).expect("borrow type");
    builder.types.push(RawTypeSyntax {
        span: at(type_start, builder.source.len()),
        kind: if exclusive {
            RawTypeSyntaxKind::BorrowMut {
                keyword_span,
                less_than_span,
                argument,
                greater_than_span,
            }
        } else {
            RawTypeSyntaxKind::Borrow { keyword_span, less_than_span, argument, greater_than_span }
        },
    });
    let parameters = vec![RawParameterSyntax {
        span: at(parameter_start, builder.source.len()),
        name: parameter_name,
        type_syntax,
    }];
    builder.text("): ");
    let result_type = builder.ty(&Ty::Named("i32"));
    builder.text(" ");
    let open_brace_span = builder.text("{");
    builder.text(" const seen: ");
    let local_start = open_brace_span.end as usize + 1;
    let local_name =
        RawIdentifierSyntax { text: "seen".into(), span: at(local_start + 6, local_start + 10) };
    let local_type = builder.ty(ty);
    builder.text(" ");
    let equals_span = builder.text("=");
    builder.text(" ");
    let initializer = builder.clone_value(|builder| builder.reference("loan"));
    let semicolon_span = builder.text(";");
    builder.statements.push(RawStatementSyntax {
        span: at(local_start, builder.source.len()),
        kind: RawStatementKind::LocalDeclaration {
            keyword_span: at(local_start, local_start + 5),
            mutable: false,
            name: local_name,
            type_syntax: local_type,
            equals_span,
            initializer,
            semicolon_span,
        },
    });
    builder.text(" return ");
    let return_start = builder.source.len() - 7;
    let literal_start = builder.source.len();
    builder.text("0");
    let value =
        builder.expression(literal_start, RawExpressionKind::I32Literal { spelling: "0".into() });
    let semicolon_span = builder.text(";");
    builder.statements.push(RawStatementSyntax {
        span: at(return_start, builder.source.len()),
        kind: RawStatementKind::Return {
            keyword_span: at(return_start, return_start + 6),
            value,
            semicolon_span,
        },
    });
    builder.text(" ");
    let close_brace_span = builder.text("}");
    let body_span = at(open_brace_span.start as usize, close_brace_span.end as usize);
    RawFunctionSyntax {
        span: at(start, builder.source.len()),
        export_span: None,
        function_span,
        name,
        parameters,
        result_type,
        body: RawFunctionBodySyntax {
            span: body_span,
            root_block: 0,
            blocks: vec![RawBlockSyntax {
                span: body_span,
                open_brace_span,
                statements: vec![0, 1],
                close_brace_span,
            }],
            statements: std::mem::take(&mut builder.statements),
            expressions: std::mem::take(&mut builder.expressions),
        },
    }
}

fn fixture(place: BorrowedPlace, exclusive: bool) -> (String, RawProjectSyntaxSnapshot) {
    let (mut builder, declarations, _) =
        initial(if matches!(place, BorrowedPlace::StructRoot | BorrowedPlace::StructField) {
            &Element::Struct
        } else {
            &Element::String
        });
    let root = root_type(place);
    builder.text("\nfunction ");
    let function_start = builder.source.len() - 9;
    let function_span = at(function_start, function_start + 8);
    let name = builder.name("observe");
    builder.text("(");
    let parameters = vec![builder.parameter("incoming", &root)];
    builder.text("): ");
    let result_type = builder.ty(&root);
    builder.text(" ");
    let open_brace_span = builder.text("{");
    builder.text(" ");
    builder.local("items", &root, true, "incoming");
    let nested = u32::try_from(builder.statements.len()).expect("nested");
    let inner_open = builder.text("{");
    builder
        .statements
        .push(RawStatementSyntax { span: inner_open, kind: RawStatementKind::Block { block: 1 } });
    builder.text(" ");
    alias(&mut builder, "loan", place, exclusive, None);
    call_statement(&mut builder);
    let inner_close = builder.text("}");
    builder.statements[nested as usize].span.end = inner_close.end;
    builder.text(" return ");
    let return_start = builder.source.len() - 7;
    let value = builder.reference("items");
    let semicolon_span = builder.text(";");
    builder.statements.push(RawStatementSyntax {
        span: at(return_start, builder.source.len()),
        kind: RawStatementKind::Return {
            keyword_span: at(return_start, return_start + 6),
            value,
            semicolon_span,
        },
    });
    builder.text(" ");
    let close_brace_span = builder.text("}");
    let body_span = at(open_brace_span.start as usize, close_brace_span.end as usize);
    let caller_statements = std::mem::take(&mut builder.statements);
    let caller_expressions = std::mem::take(&mut builder.expressions);
    let caller = RawFunctionSyntax {
        span: at(function_start, builder.source.len()),
        export_span: None,
        function_span,
        name,
        parameters,
        result_type,
        body: RawFunctionBodySyntax {
            span: body_span,
            root_block: 0,
            blocks: vec![
                RawBlockSyntax {
                    span: body_span,
                    open_brace_span,
                    statements: vec![0, nested, nested + 3],
                    close_brace_span,
                },
                RawBlockSyntax {
                    span: at(inner_open.start as usize, inner_close.end as usize),
                    open_brace_span: inner_open,
                    statements: vec![nested + 1, nested + 2],
                    close_brace_span: inner_close,
                },
            ],
            statements: caller_statements,
            expressions: caller_expressions,
        },
    };
    builder.text(" ");
    let relay_function = callee(&mut builder, &referent_type(place), exclusive);
    (
        builder.source,
        RawProjectSyntaxSnapshot {
            schema_version: PROTOCOL_VERSION,
            files: vec![RawSourceUnit {
                id: 0,
                path: "src/main.zry".into(),
                imports: Vec::new(),
                type_syntax: builder.types,
                data_declarations: declarations,
                functions: vec![caller, relay_function],
            }],
            diagnostics: Vec::new(),
        },
    )
}

#[test]
fn static_owned_borrows_pass_shared_and_exclusive_authority_to_direct_calls() {
    for place in [
        BorrowedPlace::StructRoot,
        BorrowedPlace::StructField,
        BorrowedPlace::ArrayRoot,
        BorrowedPlace::ArrayElement,
    ] {
        for exclusive in [false, true] {
            let (source, raw) = fixture(place, exclusive);
            let sources = sources_for(&source);
            let syntax = verify_snapshot(raw, &sources).expect("authenticated projected call");
            let program = lower(pair_input(&syntax, &sources))
                .unwrap_or_else(|errors| panic!("{errors:?}\n{source}"));
            let caller =
                program.modules().next().expect("module").functions().next().expect("caller");
            let instructions =
                caller.blocks().next().expect("block").instructions().collect::<Vec<_>>();
            let begin = instructions
                .iter()
                .position(|i| i.kind() == VerifiedInstructionKind::BeginBorrow)
                .expect("begin");
            let call = instructions
                .iter()
                .position(|i| i.kind() == VerifiedInstructionKind::DirectCall)
                .expect("call");
            let end = instructions
                .iter()
                .position(|i| i.kind() == VerifiedInstructionKind::EndBorrow)
                .expect("end");
            assert!(begin < call && call < end);
            assert_eq!(instructions[call].failure_ended_borrows().count(), 1);
            assert_eq!(
                instructions[call]
                    .cleanup()
                    .and_then(|id| caller.cleanup_plans().find(|plan| plan.id() == id))
                    .expect("CallTrap")
                    .site()
                    .role(),
                VerifiedCleanupRole::CallTrap
            );
            assert!(
                instructions[end + 1..]
                    .iter()
                    .any(|i| i.kind() == VerifiedInstructionKind::MoveFromPlace)
            );
        }
    }
}
