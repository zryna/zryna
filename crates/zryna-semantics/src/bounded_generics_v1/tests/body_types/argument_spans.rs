//! Argument diagnostics identify exact original tokens without changing phase selection.
use super::{IDENTITY, errors};

#[test]
fn explicit_borrow_arguments_point_to_the_access_keyword() {
    for access in ["Borrow", "BorrowMut"] {
        let source = format!(
            "{IDENTITY}\r\nfunction probe(loan:{access}<i32>):i32 {{identity<{access}<i32>>(loan); return 0;}}"
        );
        let errors = errors(&source);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].code(), "ZRYNA-M7001");
        let start = source.rfind(&format!("{access}<i32>")).expect("explicit original argument");
        let span = errors[0].primary_span().expect("authenticated access keyword");
        assert_eq!((span.start() as usize, span.end() as usize), (start, start + access.len()));
        assert_eq!(&source[span.start() as usize..span.end() as usize], access);
    }
}

#[test]
fn first_extra_argument_uses_utf8_byte_offsets_after_crlf() {
    let source = format!(
        "function banner():String {{return \"é😀\";}}\r\n{IDENTITY}\r\nfunction probe():i32 {{identity<i32, bool>(7); return 0;}}"
    );
    let errors = errors(&source);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].code(), "ZRYNA-M7001");
    let start = source.find("bool>(7)").expect("first excess original argument");
    let span = errors[0].primary_span().expect("authenticated bool argument");
    assert_eq!((span.start() as usize, span.end() as usize), (start, start + 4));
    assert_eq!(&source[span.start() as usize..span.end() as usize], "bool");
}

#[test]
fn omitted_argument_still_points_to_the_original_callee() {
    let source = format!("{IDENTITY} function probe():i32 {{identity(7); return 0;}}");
    let errors = errors(&source);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].code(), "ZRYNA-M7001");
    let start = source.find("identity(7)").expect("omitted original argument");
    let span = errors[0].primary_span().expect("original callee range");
    assert_eq!((span.start() as usize, span.end() as usize), (start, start + "identity".len()));
}
