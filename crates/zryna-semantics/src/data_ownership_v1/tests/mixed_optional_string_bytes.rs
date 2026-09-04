use super::super::constructor_resources::tests::{root_value, run_statement, with_snapshot};
use super::*;
use crate::data_ownership_v1::owned_string_read::{self, StringBytes};
use crate::data_ownership_v1::span;
use crate::data_ownership_v1::tests::mixed_string_read_scopes::{ReadCase, read_fixture};
use std::panic::{AssertUnwindSafe, catch_unwind};
use zryna_diagnostics::Diagnostic;

#[test]
fn mixed_optional_string_bytes_preserve_known_zero_unknown_and_exact_overflow_diagnostic() {
    let (source, snapshot) = read_fixture(ReadCase::LocalConcat);
    for (left, right, expected) in [
        (Some(0), Some(0), Some(StringBytes::Known(0))),
        (Some(1), Some(2), Some(StringBytes::Known(3))),
        (
            Some(zryna_ownership_runtime_abi::MAX_STRING_BYTES - 1),
            Some(1),
            Some(StringBytes::Known(zryna_ownership_runtime_abi::MAX_STRING_BYTES)),
        ),
        (None, Some(0), Some(StringBytes::Unknown)),
        (Some(0), None, Some(StringBytes::Unknown)),
        (None, None, Some(StringBytes::Unknown)),
        (None, Some(u64::MAX), Some(StringBytes::Unknown)),
        (Some(u64::MAX), Some(1), None),
        (Some(zryna_ownership_runtime_abi::MAX_STRING_BYTES), Some(1), None),
    ] {
        let mut diagnostic = None;
        let errors = with_snapshot(&source, snapshot.clone(), |lowerer, _| {
            let at = span(lowerer.input.sources(), lowerer.function.span);
            assert_eq!(
                owned_string_read::concat_optional_bytes(left, right, at, lowerer.errors),
                expected
            );
            if expected.is_none() {
                diagnostic = Some(Diagnostic::error_at(
                    "ZRYNA-M3012",
                    at,
                    "String concatenation exceeds the sealed runtime byte limit",
                    "reduce the statically known concatenated String size",
                ));
            }
        });
        assert_eq!(errors, diagnostic.into_iter().collect::<Vec<_>>());
    }
}

fn tag_case(unknown: bool, mutate: bool) {
    let (source, snapshot) = read_fixture(ReadCase::LocalConcat);
    let errors = with_snapshot(&source, snapshot, |lowerer, ty| {
        assert!(run_statement(lowerer, 0, ty));
        // Isolated witness test: unknown is seeded by removing a known compiler fact.
        // Authentic projected-Unknown source evidence lives in separate source controls.
        if unknown {
            assert_eq!(lowerer.preparation_facts.string_bytes.remove(&raw::PlaceId(1)), Some(1));
        }
        let name = lowerer
            .function
            .body
            .expressions
            .iter()
            .find_map(|expression| {
                if let zryna_syntax::v4::RawExpressionKind::Reference { name } = &expression.kind
                    && name.text == "text"
                {
                    Some(name.clone())
                } else {
                    None
                }
            })
            .expect("source local read");
        let at = span(lowerer.input.sources(), name.span);
        let legacy = owned_string_read::local(
            &name,
            &lowerer.bindings,
            &lowerer.owners,
            &lowerer.preparation_facts.string_bytes,
            at,
            lowerer.errors,
        );
        assert_eq!(
            legacy,
            if unknown { None } else { Some((raw::PlaceId(1), 1)) },
            "legacy Vec helper remains known-only"
        );
        assert!(lowerer.errors.is_empty());
        let id = root_value(lowerer, 1);
        let mut prepared = PreparedValue::prepare(lowerer, id, ty).expect("typed available read");
        let read = prepared
            .plan
            .steps
            .iter_mut()
            .find_map(|step| match &mut step.operation {
                Operation::StringRead(read) if read.place == raw::PlaceId(1) => Some(read),
                _ => None,
            })
            .expect("local read witness");
        assert_eq!(read.bytes, if unknown { StringBytes::Unknown } else { StringBytes::Known(1) });
        if mutate {
            read.bytes = if unknown { StringBytes::Known(0) } else { StringBytes::Unknown };
            let failure =
                catch_unwind(AssertUnwindSafe(|| prepared.consume())).expect_err("tag mismatch");
            let text = failure
                .downcast_ref::<String>()
                .map(String::as_str)
                .or_else(|| failure.downcast_ref::<&str>().copied())
                .expect("invariant text");
            assert!(text.contains("String read actual byte fact"), "{text}");
        } else {
            let concat = prepared
                .plan
                .steps
                .iter()
                .find_map(|step| match step.operation {
                    Operation::Leaf(Leaf::StringConcat { bytes, .. }) => Some(bytes),
                    _ => None,
                })
                .expect("concat witness");
            assert_eq!(concat, if unknown { StringBytes::Unknown } else { StringBytes::Known(2) });
            prepared.consume();
        }
    });
    assert!(errors.is_empty());
}

#[test]
fn mixed_optional_string_bytes_bind_known_unknown_tags_without_zero_substitution() {
    for unknown in [false, true] {
        tag_case(unknown, false);
        tag_case(unknown, true);
    }
}

#[test]
fn mixed_optional_string_clone_binds_authentic_root_and_projection_shape() {
    for mutation in 0..3 {
        let (source, snapshot) = read_fixture(ReadCase::LiteralClone);
        let errors = with_snapshot(&source, snapshot, |lowerer, ty| {
            assert!(run_statement(lowerer, 0, ty));
            let id = root_value(lowerer, 1);
            let mut prepared =
                PreparedValue::prepare(lowerer, id, ty).expect("literal clone scope");
            let source = prepared
                .plan
                .steps
                .iter_mut()
                .find_map(|step| match &mut step.operation {
                    Operation::Leaf(Leaf::StringClone { source, .. }) => Some(source),
                    _ => None,
                })
                .expect("clone descriptor");
            assert_eq!(source.place, source.root);
            assert!(source.is_root);
            if mutation == 0 {
                prepared.consume();
                return;
            }
            if mutation == 1 {
                assert_ne!(source.root, raw::PlaceId(1));
                source.root = raw::PlaceId(1);
            } else {
                source.is_root = false;
            }
            let failure = catch_unwind(AssertUnwindSafe(|| prepared.consume()))
                .expect_err("descriptor mismatch");
            let text = failure
                .downcast_ref::<String>()
                .map(String::as_str)
                .or_else(|| failure.downcast_ref::<&str>().copied())
                .expect("invariant text");
            assert!(text.contains("String clone authentic root and shape"), "{text}");
        });
        assert!(errors.is_empty());
    }
}
