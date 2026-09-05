use super::generic_call_fixture::Case;
use super::named_import_calls::{Base, imported_fixture};
use super::*;

#[test]
fn named_import_resolution_negatives_are_exact_ordered_replayable_and_recoverable() {
    let cases = [
        (
            "choose",
            "select",
            "./bad.zry",
            vec![
                (
                    "ZRYNA-M3016",
                    "module 'src/bad.zry' is absent from the authenticated source closure",
                    "compile the complete driver-authenticated module closure",
                    true,
                ),
                (
                    "ZRYNA-M3016",
                    "authenticated source closure contains module 'src/lib.zry' unreachable from the selected entry",
                    "pass one complete source-map-bound verified snapshot",
                    false,
                ),
            ],
        ),
        (
            "missing",
            "select",
            "./lib.zry",
            vec![(
                "ZRYNA-M3016",
                "module 'src/lib.zry' does not export function 'missing'",
                "import one explicitly exported function using its exact declared spelling",
                true,
            )],
        ),
        (
            "Choose",
            "select",
            "./lib.zry",
            vec![(
                "ZRYNA-M3016",
                "imported function 'Choose' has the wrong portable ASCII case",
                "import one explicitly exported function using its exact declared spelling",
                true,
            )],
        ),
        (
            "choose",
            "caller",
            "./lib.zry",
            vec![(
                "ZRYNA-M3002",
                "callable name 'caller' collides under portable ASCII case folding",
                "use an exact unique import alias that does not match another import or function",
                true,
            )],
        ),
    ];
    for (imported, local, path, expected_rows) in cases {
        let (sources, raw) = imported_fixture(Base::Mixed(Case::Direct), imported, local, path);
        let main =
            raw.files.iter().find(|file| file.path == "src/main.zry").expect("fixture element");
        let expected_span = if path == "./bad.zry" {
            main.imports[0].specifier.token_span
        } else if local == "caller" {
            main.imports[0].bindings[0].local.span
        } else {
            main.imports[0].bindings[0].imported.span
        };
        let syntax = verify_snapshot(raw, &sources).expect("authenticated negative import");
        let entry = sources
            .file_id(&NormalizedSourcePath::new("src/main.zry").expect("fixture path"))
            .expect("fixture path");
        let input =
            SemanticInput::try_new(&syntax, &sources, entry).expect("source-bound semantic input");
        let expected = lower(input).expect_err("invalid import");
        assert_eq!(expected.len(), expected_rows.len());
        for (diagnostic, (code, message, guidance, located)) in expected.iter().zip(expected_rows) {
            assert_eq!(diagnostic.code, code);
            assert_eq!(diagnostic.message, message);
            assert_eq!(diagnostic.guidance, guidance);
            assert_eq!(diagnostic.primary_span(), located.then(|| span(&sources, expected_span)));
        }
        assert_eq!(lower(input).expect_err("deterministic replay"), expected);

        let (valid_sources, valid_raw) =
            imported_fixture(Base::Mixed(Case::Direct), "choose", "select", "./lib.zry");
        let valid_syntax = verify_snapshot(valid_raw, &valid_sources).expect("fixture element");
        let valid_entry = valid_sources
            .file_id(&NormalizedSourcePath::new("src/main.zry").expect("fixture path"))
            .expect("fixture path");
        lower(
            SemanticInput::try_new(&valid_syntax, &valid_sources, valid_entry)
                .expect("source-bound semantic input"),
        )
        .expect("pristine recovery");
    }
}
