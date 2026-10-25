use crate::generic_closure::test_source::MemoryRoot;
use sha2::{Digest, Sha256};
use std::path::Path;
pub(super) fn read(path: &str) -> Vec<u8> {
    std::fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests").join(path))
        .expect("unchanged existing source material")
}
fn literal(path: &str, expected: &str) -> String {
    let bytes = read(path);
    assert_eq!(format!("{:x}", Sha256::digest(&bytes)), expected);
    String::from_utf8(bytes).expect("original UTF8")
}
pub(super) fn source(case: &str) -> (MemoryRoot, &'static str, u32) {
    match case {
        "scalar" => (
            MemoryRoot::new([(
                "main.zry".into(),
                literal(
                    "m7-generic-full-profile-first-vertical/literals/scalar-i32-clone.zry",
                    "81336e343ddb166fbeea0fb3fa3d882b4ed93766fb50b540504a3840ca999c81",
                ),
            )]),
            "m7-generic-full-profile-first-vertical/scalar-i32-clone-typescript.json",
            0,
        ),
        "entry-one" => (
            MemoryRoot::new([
                (
                    "a.zry".into(),
                    literal(
                        "m7-generic-execution-entry-one/literals/a.zry",
                        "b0b189e867c2bbdac4fd226e525636bdb3a37496213d4ce913593f0f69c3615b",
                    ),
                ),
                (
                    "main.zry".into(),
                    literal(
                        "m7-generic-execution-entry-one/literals/main.zry",
                        "cef91934e0bb7437b845b0a83ea950da5111f56e49ec657d694ae432c45c9b55",
                    ),
                ),
            ]),
            "m7-generic-execution-entry-one/imported-generic-entry-one-typescript.json",
            1,
        ),
        "mixed" => (
            MemoryRoot::new([(
                "main.zry".into(),
                literal(
                    "m7-generic-execution-recursive-owner/mixed/mixed-catalogue.zry",
                    "5d28cef948a289b7e808b5efe8a94c4c4d7f73ef5d47896195e1e2fc6077f8b6",
                ),
            )]),
            "m7-generic-execution-recursive-owner/mixed/mixed.typescript.json",
            0,
        ),
        "unused" => (
            MemoryRoot::new([(
                "main.zry".into(),
                literal(
                    "m7-generic-full-profile-first-vertical/literals/opaque-clone-unused.zry",
                    "be745749a61e30106c0b3b1949be693e71849970d65f305dd5265aa2d0888352",
                ),
            )]),
            "m7-generic-full-profile-first-vertical/opaque-clone-unused-typescript.json",
            0,
        ),
        _ => panic!("closed original source case"),
    }
}

pub(super) fn packet(path: &str) -> Vec<u8> {
    let expected = match path {
        "m7-generic-full-profile-first-vertical/scalar-i32-clone-typescript.json" => {
            "11df1a2ef0cedcf73d4a52a008ac5a1bddaa8e1d34ad961aa4b08051f49a0c99"
        }
        "m7-generic-execution-entry-one/imported-generic-entry-one-typescript.json" => {
            "04b142061eff00e44f7dd5993a57027ef36b97c2cac64d06e9cf0c5fb95504ba"
        }
        "m7-generic-execution-recursive-owner/mixed/mixed.typescript.json" => {
            "918cd89ca415c91acbf589cb6a52aacbd113facdb2e24febd48a4ffe2fbea9b5"
        }
        _ => panic!("fixed actual packet"),
    };
    let bytes = read(path);
    assert_eq!(format!("{:x}", Sha256::digest(&bytes)), expected);
    bytes
}
