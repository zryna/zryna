//! Original generated property body, invoked by the unchanged root test identity.
use super::*;

pub(super) fn run() {
    let vocabulary = [
        "export",
        "function",
        "(",
        ")",
        "{",
        "}",
        "interface",
        "extends",
        "ZrynaStruct",
        "return",
        "const",
        "let",
        "if",
        "else",
        "while",
        "i32",
        "x",
        "1",
        "true",
        "+",
        "-",
        "*",
        "=",
        "<",
        ">",
        ":",
        ";",
        "[",
        "]",
        ",",
        "\"x\"",
    ];
    let mut seed = 0x412_2026_u32;
    let mut retained = witness::open(|| {
        json!({"event":"header", "format":"zryna.m7.fuzz-witness.v2",
        "campaign":"parser-token-generated", "seed":"0x04122026", "vocabulary":vocabulary,
        "cases":2048, "versions":[2,3,4], "executionsPerVersion":2})
    });
    for index in 0..2048 {
        let mut text = "// π\r\n".to_owned();
        for _ in 0..4 + index % 47 {
            seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            text.push_str(vocabulary[seed as usize % vocabulary.len()]);
            text.push(' ');
        }
        witness::record(&mut retained, || {
            json!({"event":"input", "case":index,
            "path":"src/main.zry", "text":&text})
        });
        let sources =
            SourceMap::build(vec![SourceFileInput { path: "src/main.zry".to_owned(), text }])
                .expect("bounded mutated source");
        let lexed = lex(&sources).expect("bounded token grammar");
        for version in 2..=4 {
            let mut results = Vec::new();
            for _ in 0..2 {
                let mut trace =
                    parser_witness::parser(&mut retained, (index, version, results.len()));
                let result = match version {
                    2 => match parse_v2_recovering_candidate(&sources, &lexed) {
                        Ok(raw) => {
                            trace.raw(&raw);
                            let value = serde_json::to_value(&raw).expect("v2 JSON");
                            syntax_v2::verify_snapshot(raw, &sources)
                                .expect("generated v2 graph verifies");
                            json!({"result": value})
                        }
                        Err(error) => trace.rejection(rejection(error.diagnostic())),
                    },
                    3 => match parse_v3_straight_line_candidate(&sources, &lexed) {
                        Ok(raw) => {
                            trace.raw(&raw);
                            let value = serde_json::to_value(&raw).expect("v3 JSON");
                            syntax_v3::verify_snapshot(raw, &sources)
                                .expect("generated v3 graph verifies");
                            json!({"result": value})
                        }
                        Err(error) => trace.rejection(rejection(error.diagnostic())),
                    },
                    4 => match parse_v4_candidate(&sources, &lexed) {
                        Ok(raw) => {
                            trace.raw(&raw);
                            let value = serde_json::to_value(&raw).expect("v4 JSON");
                            syntax_v4::verify_snapshot(raw, &sources)
                                .expect("generated v4 graph verifies");
                            json!({"result": value})
                        }
                        Err(error) => trace.rejection(rejection(error.diagnostic())),
                    },
                    _ => unreachable!(),
                };
                results.push(result);
            }
            assert_eq!(results[0], results[1], "v{version} deterministic grammar case {index}");
        }
    }
    witness::record(&mut retained, || json!({"event":"complete", "cases":2048}));
}
