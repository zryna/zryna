//! Written input and stable metadata controls over genuine retained material.
use super::{Context, NativeProcessLimits, expected, fs, rejected, replace, with_inputs};
use std::os::unix::fs::PermissionsExt as _;
#[test]
fn written_inputs_bind_expected_bytes_and_reject_content_and_same_byte_replacement() {
    for name in ["program.o", "runtime.c", "invocation.c"] {
        with_inputs(|stage, deps, bytes| {
            let path = stage.capability_file_path(name).expect("input");
            let mut wrong = expected(bytes, name).to_vec();
            wrong[0] ^= 1;
            fs::write(&path, &wrong).expect("wrong pre-capture bytes");
            assert_eq!(
                Context::capture(stage, deps, NativeProcessLimits::default(), bytes)
                    .err()
                    .expect("do not adopt changed expected bytes")
                    .code,
                "ZRYNA-C4166"
            );
            fs::write(&path, expected(bytes, name)).expect("restore immutable source bytes");
            let mut context = Context::capture(stage, deps, NativeProcessLimits::default(), bytes)
                .expect("fresh exact snapshot");
            fs::write(&path, &wrong).expect("post-capture content mutation");
            rejected(&context.compile_runtime().expect_err("changed input rejects before GNU"));
            fs::write(&path, expected(bytes, name)).expect("new pristine source");
            let mut context = Context::capture(stage, deps, NativeProcessLimits::default(), bytes)
                .expect("fresh input snapshot");
            replace(stage, name, expected(bytes, name));
            rejected(
                &context
                    .compile_runtime()
                    .expect_err("same-byte inode replacement rejects before GNU"),
            );
            Context::capture(stage, deps, NativeProcessLimits::default(), bytes)
                .expect("pristine fresh retry")
                .validate_inputs()
                .expect("new exact snapshot valid");
        });
    }
}
#[test]
fn restored_contents_and_mtime_cannot_revive_a_captured_input() {
    for name in ["program.o", "runtime.c", "invocation.c"] {
        with_inputs(|stage, deps, bytes| {
            let path = stage.capability_file_path(name).expect("input");
            let original = fs::metadata(&path).expect("original metadata");
            let mut context = Context::capture(stage, deps, NativeProcessLimits::default(), bytes)
                .expect("exact source snapshot");
            let mut wrong = expected(bytes, name).to_vec();
            wrong[0] ^= 1;
            fs::write(&path, &wrong).expect("mutate in place");
            fs::write(&path, expected(bytes, name)).expect("restore source bytes");
            fs::OpenOptions::new()
                .write(true)
                .open(&path)
                .expect("owned file")
                .set_times(
                    fs::FileTimes::new().set_modified(original.modified().expect("original mtime")),
                )
                .expect("restore exact mtime");
            assert_eq!(
                fs::metadata(&path).expect("after metadata").modified().expect("mtime"),
                original.modified().expect("original mtime")
            );
            rejected(
                &context
                    .compile_runtime()
                    .expect_err("ctime change is retained despite byte/mtime restoration"),
            );
        });
    }
}
#[test]
fn unix_mode_drift_and_link_before_compilation_are_rejected() {
    with_inputs(|stage, deps, bytes| {
        let context = Context::capture(stage, deps, NativeProcessLimits::default(), bytes)
            .expect("exact inputs");
        rejected(&context.link().expect_err("no unretained runtime object"));
        let path = stage.capability_file_path("program.o").expect("input");
        fs::set_permissions(path, fs::Permissions::from_mode(0o400)).expect("owned mode mutation");
        assert_eq!(context.validate_inputs().expect_err("mode change").code, "ZRYNA-C4166");
    });
}
