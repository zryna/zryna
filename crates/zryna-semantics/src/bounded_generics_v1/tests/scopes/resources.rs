//! Original import diagnostic terminal and inherited checked resource counters.

use crate::bounded_generics_v1::resources::{Metric, add};

use super::*;

fn missing_imports(count: usize) -> Project {
    fixtures::project(&["main.zry"], |_, unit| {
        for index in 0..count {
            unit.import(
                &format!("./missing{index:04}.zry"),
                &[("Unknown", &format!("unresolved{index}"))],
            );
        }
        unit.function("score", &[], true);
    })
}

#[test]
fn diagnostic_terminal_returns_no_context() {
    let exact = missing_imports(255);
    let exact_errors = context(&exact, "main.zry").expect_err("255 original ordinary errors");
    assert_eq!(exact_errors.len(), 255);
    assert!(exact_errors.iter().all(|error| error.code() == "ZRYNA-M3016"));
    let extra = missing_imports(260);
    let errors = context(&extra, "main.zry").expect_err("no partial context at terminal budget");
    assert_eq!(errors.len(), 256);
    assert!(errors[..255].iter().all(|error| error.code() == "ZRYNA-M3016"));
    assert_eq!(errors[255].code(), "ZRYNA-M7201");
    assert_eq!(
        source_text(&extra, errors[255].primary_span().expect("first extra import")),
        "\"./missing0255.zry\""
    );
    let shadows = fixtures::project(&["main.zry", "values.zry"], |path, unit| {
        if path == "main.zry" {
            for index in 0..260 {
                unit.import(
                    &format!("./missing{index:04}.zry"),
                    &[("Unknown", &format!("unresolved{index}"))],
                );
            }
            unit.import("./values.zry", &[("Box", "T")]);
            for index in 0..260 {
                unit.function(&format!("owner{index:04}"), &["T"], true);
            }
        } else {
            unit.data("Box", &["X"], false, true);
        }
    });
    let errors =
        context(&shadows, "main.zry").expect_err("D slots precede every stored M candidate");
    assert_eq!(errors.len(), 256);
    assert!(errors[..255].iter().all(|error| error.code() == "ZRYNA-D7001"));
    assert_eq!(errors[255].code(), "ZRYNA-M7201");
    let at = errors[255].primary_span().expect("first extra owning parameter");
    assert_eq!(source_text(&shadows, at), "T");
    let source = shadows.sources.source(at.file()).expect("owning source");
    assert_eq!(
        at.start() as usize,
        source.text().find("owner0255<T").expect("canonical first extra declaration")
            + "owner0255<".len()
    );
}

#[test]
fn module_binding_checked_counters_exact_and_extra() {
    // Synthetic counter witnesses test pre-allocation limits independently of source admissibility.
    for (metric, frozen_limit) in [
        (Metric::Modules, 4096),
        (Metric::SourceBytes, 8 * 1024 * 1024),
        (Metric::ImportsPerModule, 4096),
        (Metric::Imports, 65_536),
        (Metric::NamesPerImport, 256),
        (Metric::ImportedNames, 65_536),
        (Metric::DataPerModule, 4096),
        (Metric::Data, 16_384),
        (Metric::FunctionsPerModule, 4096),
        (Metric::Functions, 16_384),
        (Metric::TypesPerModule, 65_536),
        (Metric::Types, 262_144),
    ] {
        assert_eq!(metric.limit(), frozen_limit);
        assert_eq!(
            add(frozen_limit - 1, 1, metric).expect("exact inherited counter"),
            frozen_limit
        );
        let extra = add(frozen_limit, 1, metric)
            .expect_err("first extra cannot allocate a context")
            .diagnostic();
        assert_eq!(extra.code(), "ZRYNA-M7201");
        assert!(extra.message().contains(&(frozen_limit + 1).to_string()));
        assert!(extra.primary_span().is_none());
        let overflow =
            add(usize::MAX, 1, metric).expect_err("checked addition cannot wrap").diagnostic();
        assert_eq!(overflow.code(), "ZRYNA-M7201");
        assert!(overflow.message().contains("checked-add overflow"));
    }
}
