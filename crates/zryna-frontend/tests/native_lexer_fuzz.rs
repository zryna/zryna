//! Deterministic generated-boundary evidence for the native lexer.

use serde_json::json;
use zryna_frontend::native_lexer::lex;
use zryna_source::{SourceFileInput, SourceMap};
#[path = "m7_fuzz_witness/lexer.rs"]
mod lexer_witness;
#[path = "m7_fuzz_witness/support.rs"]
mod witness;

fn sources(text: &str) -> SourceMap {
    SourceMap::build(vec![SourceFileInput {
        path: "src/main.zry".to_owned(),
        text: text.to_owned(),
    }])
    .expect("bounded lexer fixture")
}

#[test]
fn deterministic_fuzzed_boundaries_are_total_lossless_and_source_bound() {
    let fragments = [
        "name",
        "123",
        "===",
        "!==",
        "=>",
        "<=",
        ">=",
        "{",
        "}",
        "(",
        ")",
        "[",
        "]",
        ":",
        ";",
        ",",
        ".",
        "+",
        "-",
        "*",
        " ",
        "\r\n",
        "\u{2028}",
        "// note\n",
        "/* block */",
        "'text'",
        "\"text\"",
        "'bad\\'quote'",
        "?",
        "é",
        "/* open",
    ];
    let mut state = 0x4d59_5df4_d0f3_3173_u64;
    let mut retained = witness::open(|| {
        json!({"event":"header", "format":"zryna.m7.fuzz-witness.v2",
        "campaign":"lexer-generated", "seed":"0x4d595df4d0f33173", "fragments":fragments,
        "cases":256, "fragmentsPerCase":32,"executionsPerCase":2})
    });
    for case in 0..256 {
        let mut text = String::new();
        for _ in 0..32 {
            state = state.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
            let fragment_count = u64::try_from(fragments.len()).expect("bounded fragment corpus");
            let index = usize::try_from(state % fragment_count).expect("bounded fragment index");
            text.push_str(fragments[index]);
        }
        witness::record(&mut retained, || {
            json!({"event":"input", "case":case,
            "path":"src/main.zry", "text":&text})
        });
        let map = sources(&text);
        let first_result = lex(&map);
        witness::record(&mut retained, || {
            json!({"event":"output","case":case,
            "execution":0,"outcome":lexer_witness::lexer_output(&first_result)})
        });
        let first = first_result.unwrap_or_else(|error| panic!("fuzz case {case}: {error}"));
        let second_result = lex(&map);
        witness::record(&mut retained, || {
            json!({"event":"output","case":case,
            "execution":1,"outcome":lexer_witness::lexer_output(&second_result)})
        });
        let second = second_result.expect("deterministic fuzz replay");
        assert_eq!(first, second);
        let file = &first.files()[0];
        let source = map.source(file.id()).expect("lexed file belongs to source map");
        let reconstructed = file
            .lexemes()
            .iter()
            .map(|lexeme| {
                let span = lexeme.span();
                map.resolve(span).expect("lexeme span is source authenticated");
                &source.text()[span.start() as usize..span.end() as usize]
            })
            .collect::<String>();
        assert_eq!(reconstructed, source.text());
    }
    witness::record(&mut retained, || json!({"event":"complete", "cases":256}));
}
