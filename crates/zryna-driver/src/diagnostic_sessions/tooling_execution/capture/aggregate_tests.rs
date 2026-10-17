use super::validate_sizes;

#[test]
fn nine_file_aggregate_exact_limit_and_first_extra() {
    let maximum = 16 * 1_024 * 1_024;
    let sizes = [maximum - 8, 1, 1, 1, 1, 1, 1, 1, 1];
    validate_sizes(sizes).expect("exact aggregate limit");
    let mut extra = sizes;
    extra[8] += 1;
    let error = validate_sizes(extra).expect_err("first aggregate byte beyond the limit");
    assert_eq!(error.code(), "ZRYNA-D3001");
    assert!(error.message().contains("exceeds its byte limit"));
}

#[test]
fn nine_file_aggregate_arithmetic_overflow_rejects() {
    let error = validate_sizes([usize::MAX, 1, 0, 0, 0, 0, 0, 0, 0])
        .expect_err("checked aggregate arithmetic");
    assert_eq!(error.code(), "ZRYNA-D3001");
    assert!(error.message().contains("overflowed"));
}

#[test]
fn aggregate_is_not_individual_material_authentication() {
    validate_sizes([0; 9]).expect("only arithmetic is exercised here");
    assert!(validate_sizes([16 * 1_024 * 1_024; 9]).is_err());
}

#[test]
fn modular_28_file_aggregate_exact_limit_and_first_extra() {
    let maximum = 16 * 1_024 * 1_024;
    let mut sizes = [1; 28];
    sizes[0] = maximum - 27;
    validate_sizes(sizes).expect("all 28 exact-limit members count");
    sizes[27] += 1;
    assert!(
        validate_sizes(sizes)
            .expect_err("last module exceeds aggregate")
            .message()
            .contains("exceeds its byte limit")
    );
    sizes[27] = usize::MAX;
    assert!(
        validate_sizes(sizes)
            .expect_err("module arithmetic cannot wrap")
            .message()
            .contains("overflowed")
    );
}

#[test]
fn stage_budget_validation_includes_modular_payloads() {
    use super::{CapturedFile, CapturedToolingClosure, CapturedV4};
    fn empty() -> CapturedFile {
        CapturedFile { bytes: Vec::new(), sha256: [0; 32] }
    }
    let mut modules = (0..19).map(|_| empty()).collect::<Vec<_>>();
    modules[18].bytes = vec![0; 16 * 1_024 * 1_024];
    let mut captured = CapturedToolingClosure {
        worker: empty(),
        worker_v3: empty(),
        limits_v3: empty(),
        worker_v4: empty(),
        limits_v4: empty(),
        wrapper_manifest: empty(),
        wrapper: empty(),
        typescript_manifest: empty(),
        typescript: empty(),
        v4: CapturedV4::Modular(modules),
    };
    captured.validate_size().expect("exact modular aggregate arithmetic");
    captured.worker.bytes.push(0);
    assert!(
        captured
            .validate_size()
            .expect_err("module and base byte share one budget")
            .message()
            .contains("exceeds its byte limit")
    );
    assert!(
        super::super::stage::ToolingStage::create(&captured)
            .expect_err("oversized modular closure rejected before staging")
            .message()
            .contains("exceeds its byte limit")
    );
}
