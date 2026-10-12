//! Hostile copies of actual producer observations, never new compiler authority.
use super::*;
use std::os::unix::process::ExitStatusExt as _;

pub(super) fn provenance(
    measured: &material::OrdinaryRustMeasurement,
    requirements: &HandleLinkRequirements,
    library: &CapturedRustProfileLibrary,
    tools: &material::OrdinaryTools,
    parent: &Path,
) {
    let mutations: &[fn(&mut material::OrdinaryRustMeasurement)] = &[
        |m| m.records[1].program = PathBuf::from("/hostile/compiler"),
        |m| m.records[1].arguments.push("--hostile".into()),
        |m| m.records[1].name = "hostile-record",
        |m| m.records[1].directory = PathBuf::from("/hostile/directory"),
        |m| m.records[1].elapsed_ms += 1,
        |m| {
            m.records[1].output.as_mut().expect("actual compiler output").status =
                std::process::ExitStatus::from_raw(1 << 8);
        },
        |m| m.records[0].output.as_mut().expect("actual version output").stdout[0] ^= 1,
        |m| m.records[1].output.as_mut().expect("actual compiler output").stderr.push(b'!'),
        |m| m.records[1].diagnostic = Some(rejected()),
        |m| m.records.swap(0, 1),
        |m| m.evidence = PathBuf::from("/hostile/receipt"),
        |m| m.receipt[0] ^= 1,
        |m| m.rust_observations["selected_sysroot"] = "/hostile/sysroot".into(),
        |m| m.rust_observations["compiler"] = "/hostile/compiler".into(),
        |m| m.rust_observations["pins"][1]["sha256"] = "hostile-driver".into(),
    ];
    let before = process::attempts();
    for mutate in mutations {
        let mut hostile = measured.clone();
        mutate(&mut hostile);
        let request = ImportRequest { requirements, library, measurement: &hostile };
        assert!(hostile.capture(requirements).is_err());
        assert!(observe_import(&request, tools, parent, Mode::Scalar).is_err());
        assert_eq!(
            process::attempts(),
            before,
            "mutated Rust producer evidence before new C effects"
        );
    }
    assert!(measured.capture(requirements).is_ok(), "pristine genuine producer remains usable");
}

pub(super) fn wrong_modes(
    measured: &material::OrdinaryRustMeasurement,
    scalar: &HandleLinkRequirements,
    bytes: &HandleLinkRequirements,
    tools: &material::OrdinaryTools,
    parent: &Path,
) {
    let before = process::attempts();
    for (requirements, mode) in
        [(scalar, Mode::Bytes { missing_increment: false }), (bytes, Mode::Scalar)]
    {
        let library = measured.capture(requirements).expect("genuine same-selected capture");
        let request = ImportRequest { requirements, library: &library, measurement: measured };
        assert!(request.check().is_ok(), "origin binding alone cannot establish mode");
        assert!(observe_import(&request, tools, parent, mode).is_err());
        assert_eq!(process::attempts(), before, "wrong mode refused before any new C attempt");
    }
    for (requirements, mode, marker) in [
        (scalar, Mode::Scalar, b"ordinary-rust-scalar:9calls\n".as_slice()),
        (
            bytes,
            Mode::Bytes { missing_increment: false },
            b"ordinary-rust-bytes:4entered4prechecks\n".as_slice(),
        ),
    ] {
        let library = measured.capture(requirements).expect("pristine selected-mode capture");
        let request = ImportRequest { requirements, library: &library, measurement: measured };
        let (_, session) =
            observe_import(&request, tools, parent, mode).expect("pristine mode recovery");
        tests::successful(&session, marker);
    }
}

pub(super) fn artifact_bounds() {
    let cap = 96 * 1024 * 1024;
    assert!(export::receipt_fits(cap - 1, 63, 1));
    assert!(!export::receipt_fits(cap, 63, 1));
    assert!(!export::receipt_fits(0, 64, 1));
    assert!(!export::receipt_fits(usize::MAX, 0, 1));
}
