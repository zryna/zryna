use std::{fs, path::PathBuf};

use zryna_ir::{Expr, ExprId, ExprKind, Function, Program, Type, VerifiedProgram};
use zryna_source::{SourceFileInput, SourceMap};

use crate::WitSource;

pub(super) fn sources() -> Vec<WitSource> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let logical_root = "spec/wit/capability-profiles-v1/worlds.wit";
    let mut sources = vec![WitSource::new(
        logical_root,
        fs::read(root.join("../..").join(logical_root)).expect("accepted WIT root"),
    )];
    for package in ["cli", "clocks", "filesystem", "http", "io", "random", "sockets"] {
        let directory = root.join("tests/wit-world-audit-v1/dependencies").join(package);
        let mut files = fs::read_dir(directory)
            .expect("pinned package directory")
            .map(|entry| entry.expect("pinned package entry").path())
            .filter(|path| path.extension().is_some_and(|extension| extension == "wit"))
            .collect::<Vec<_>>();
        files.sort();
        for file in files {
            let name = file.file_name().expect("WIT name").to_string_lossy();
            sources.push(WitSource::new(
                format!("wasi/{package}/{name}"),
                fs::read(&file).expect("WIT source"),
            ));
        }
    }
    assert_eq!(sources.len(), 34);
    sources
}

/// Backend verifier fixture; actual source compilation is covered separately at the driver boundary.
pub(super) fn program() -> VerifiedProgram {
    program_with_arity(2)
}

pub(super) fn program_with_arity(arity: usize) -> VerifiedProgram {
    let sources =
        SourceMap::build(vec![SourceFileInput { path: "add.zry".into(), text: "add".into() }])
            .expect("source map");
    let file = sources.verify_file_id(0).expect("source identity");
    let span = sources.span(file, 0, 3).expect("source span");
    zryna_ir::verify(
        Program {
            functions: vec![Function {
                name: "add".into(),
                parameters: vec![Type::I32; arity],
                return_type: Type::I32,
                expressions: vec![
                    Expr { ty: Type::I32, span, kind: ExprKind::Parameter(0) },
                    Expr { ty: Type::I32, span, kind: ExprKind::Parameter(1) },
                    Expr {
                        ty: Type::I32,
                        span,
                        kind: ExprKind::I32Add { lhs: ExprId(0), rhs: ExprId(1) },
                    },
                ],
                body: ExprId(2),
            }],
        },
        &sources,
    )
    .expect("verified scalar add fixture")
}
