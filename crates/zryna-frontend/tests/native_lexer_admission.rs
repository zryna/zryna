//! Raw-byte admission evidence for the native lexer boundary.

use zryna_frontend::native_lexer::{
    MAX_SOURCE_BYTES_PER_PROJECT, NativeSourceBytes, admit_and_lex,
};
use zryna_source::{MAX_SOURCE_FILE_BYTES, MAX_SOURCE_FILES};

fn raw(path: &str, bytes: impl Into<Vec<u8>>) -> NativeSourceBytes {
    NativeSourceBytes { path: path.to_owned(), bytes: bytes.into() }
}

#[test]
fn raw_byte_admission_preserves_utf8_and_returns_its_source_authority() {
    let admitted = admit_and_lex(vec![raw(
        "src/main.zry",
        "// 😀\nexport function identity(value: i32): i32 { return value; }".as_bytes(),
    )])
    .expect("valid raw UTF-8 source");
    assert!(admitted.project().is_bound_to(admitted.sources()));
    let file = &admitted.project().files()[0];
    let source = admitted.sources().source(file.id()).expect("admitted source authority");
    let reconstructed = file
        .lexemes()
        .iter()
        .map(|lexeme| {
            let span = lexeme.span();
            admitted.sources().resolve(span).expect("authenticated lexical span");
            &source.text()[span.start() as usize..span.end() as usize]
        })
        .collect::<String>();
    assert_eq!(reconstructed, source.text());
}

#[test]
fn raw_byte_admission_rejects_exact_invalid_and_truncated_utf8_ranges() {
    let cases = [(vec![b'a', 0xff, b'b'], (1, 2)), (vec![b'a', 0xf0, 0x9f, 0x98], (1, 4))];
    for (bytes, expected) in cases {
        let first = admit_and_lex(vec![raw("src/main.zry", bytes.clone())])
            .expect_err("malformed UTF-8 must fail before source authority");
        let second =
            admit_and_lex(vec![raw("src/main.zry", bytes)]).expect_err("malformed UTF-8 replay");
        assert_eq!(first, second);
        assert_eq!(first.diagnostic().code(), "ZRYNA-F1501");
        assert_eq!(first.diagnostic().path(), Some("src/main.zry"));
        assert!(first.diagnostic().primary_span().is_none());
        let location = first.raw_byte_span().expect("pre-authority byte range");
        assert_eq!(location.file(), 0);
        assert_eq!(location.path().as_str(), "src/main.zry");
        assert_eq!((location.start(), location.end()), expected);
    }
}

#[test]
fn raw_byte_admission_checks_paths_and_sizes_before_decoding() {
    let collision = admit_and_lex(vec![raw("src/A.zry", b"a".to_vec()), raw("src/a.zry", [0xff])])
        .expect_err("portable path collision precedes decoding");
    assert_eq!(collision.diagnostic().code(), "ZRYNA-S1004");
    assert!(collision.raw_byte_span().is_none());

    let invalid_path = admit_and_lex(vec![raw("../escape.zry", [0xff])])
        .expect_err("portable path validation precedes decoding");
    assert_eq!(invalid_path.diagnostic().code(), "ZRYNA-S1001");
    assert!(invalid_path.raw_byte_span().is_none());

    let oversized_malformed =
        admit_and_lex(vec![raw("src/main.zry", vec![0xff; MAX_SOURCE_FILE_BYTES + 1])])
            .expect_err("size limit precedes malformed UTF-8");
    assert_eq!(oversized_malformed.diagnostic().code(), "ZRYNA-F1502");
    let location = oversized_malformed.raw_byte_span().expect("first extra raw byte");
    let file_limit = u32::try_from(MAX_SOURCE_FILE_BYTES).expect("source byte limit fits u32");
    assert_eq!((location.start(), location.end()), (file_limit, file_limit + 1));
}

#[test]
#[ignore = "proportional production-limit raw byte admission proof"]
fn raw_byte_limits_accept_exact_and_reject_complete_first_extra_inputs() {
    assert_eq!(MAX_SOURCE_BYTES_PER_PROJECT, 4 * MAX_SOURCE_FILE_BYTES);
    let exact_count = (0..MAX_SOURCE_FILES)
        .map(|index| raw(&format!("src/count-{index:04}.zry"), Vec::new()))
        .collect();
    assert_eq!(
        admit_and_lex(exact_count).expect("exact raw file-count boundary").project().files().len(),
        MAX_SOURCE_FILES
    );
    let extra_count = (0..=MAX_SOURCE_FILES)
        .map(|index| raw(&format!("src/count-{index:04}.zry"), [0xff]))
        .collect();
    let count_error =
        admit_and_lex(extra_count).expect_err("first extra file rejects before decoding");
    assert_eq!(count_error.diagnostic().code(), "ZRYNA-S1002");
    assert!(count_error.raw_byte_span().is_none());

    let exact_file = format!("/*{}*/", "x".repeat(MAX_SOURCE_FILE_BYTES - 4)).into_bytes();
    let admitted = admit_and_lex(vec![raw("src/exact.zry", exact_file.clone())])
        .expect("exact raw file-byte boundary");
    assert!(admitted.project().is_bound_to(admitted.sources()));

    let mut extra_file = exact_file;
    extra_file.push(b'x');
    let file_error = admit_and_lex(vec![raw("src/extra.zry", extra_file)])
        .expect_err("first extra raw file byte");
    assert_eq!(file_error.diagnostic().code(), "ZRYNA-F1502");
    let file_range = file_error.raw_byte_span().expect("raw file range");
    let file_limit = u32::try_from(MAX_SOURCE_FILE_BYTES).expect("source byte limit fits u32");
    assert_eq!((file_range.start(), file_range.end()), (file_limit, file_limit + 1));

    let exact_project = (0..4)
        .map(|index| {
            raw(
                &format!("src/f{index}.zry"),
                format!("/*{}*/", "x".repeat(MAX_SOURCE_FILE_BYTES - 4)).into_bytes(),
            )
        })
        .collect();
    let admitted = admit_and_lex(exact_project).expect("exact raw project-byte boundary");
    assert_eq!(admitted.project().files().len(), 4);

    let mut extra_project = (0..4)
        .map(|index| {
            raw(
                &format!("src/f{index}.zry"),
                format!("/*{}*/", "x".repeat(MAX_SOURCE_FILE_BYTES - 4)).into_bytes(),
            )
        })
        .collect::<Vec<_>>();
    extra_project.push(raw("src/z.zry", b"x".to_vec()));
    let project_error = admit_and_lex(extra_project).expect_err("first extra raw project byte");
    assert_eq!(project_error.diagnostic().code(), "ZRYNA-F1502");
    let project_range = project_error.raw_byte_span().expect("raw project range");
    assert_eq!(project_range.file(), 4);
    assert_eq!(project_range.path().as_str(), "src/z.zry");
    assert_eq!((project_range.start(), project_range.end()), (0, 1));
}
