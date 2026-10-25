//! Compiled output and fresh destination guards; existing bounds stay unchanged.
use super::{Context, NativeProcessLimits, fs, rejected, replace, with_inputs};
#[test]
fn actual_compiled_runtime_pin_rejects_content_replacement_and_restoration_before_link() {
    for mutation in ["content", "replace", "restore"] {
        with_inputs(|stage, deps, bytes| {
            let mut context = Context::capture(stage, deps, NativeProcessLimits::default(), bytes)
                .expect("exact inputs");
            let runtime = context
                .compile_runtime()
                .expect("actual GNU runtime and retained output before audit");
            context.validate_inputs().expect("pristine four pinned files");
            let path = stage.capability_file_path("runtime.o").expect("compiled output path");
            let before = fs::metadata(&path).expect("compiled metadata");
            if mutation == "replace" {
                replace(stage, "runtime.o", &runtime);
            } else {
                let mut wrong = runtime.clone();
                wrong[0] ^= 1;
                fs::write(&path, &wrong).expect("compiled output mutation");
                if mutation == "restore" {
                    fs::write(&path, &runtime).expect("restore exact compiled bytes");
                    fs::OpenOptions::new()
                        .write(true)
                        .open(&path)
                        .expect("owned runtime")
                        .set_times(
                            fs::FileTimes::new()
                                .set_modified(before.modified().expect("compiled mtime")),
                        )
                        .expect("restore compiled mtime");
                }
            }
            rejected(&context.link().expect_err("changed compiled object rejects before linker"));
        });
    }
    with_inputs(|stage, deps, bytes| {
        let mut context = Context::capture(stage, deps, NativeProcessLimits::default(), bytes)
            .expect("fresh pristine retry");
        context.compile_runtime().expect("fresh actual GNU output");
        context.validate_inputs().expect("fresh four-file snapshot");
    });
}
#[test]
fn occupied_or_symlinked_generated_paths_reject_before_the_tool() {
    for name in ["runtime.o", "invocation.elf"] {
        for symlink in [false, true] {
            with_inputs(|stage, deps, bytes| {
                let mut context =
                    Context::capture(stage, deps, NativeProcessLimits::default(), bytes)
                        .expect("exact inputs");
                if name == "invocation.elf" {
                    context.compile_runtime().expect("actual pinned runtime");
                }
                let path = stage.capability_file_path(name).expect("generated path");
                if symlink {
                    std::os::unix::fs::symlink("program.o", &path).expect("hostile output symlink");
                } else {
                    fs::write(&path, b"occupied immutable witness").expect("occupied output");
                }
                let errors = if name == "runtime.o" {
                    context.compile_runtime().expect_err("runtime output must be absent")
                } else {
                    context.link().expect_err("image output must be absent")
                };
                rejected(&errors);
                if symlink {
                    assert_eq!(
                        fs::read_link(&path).expect("link unchanged"),
                        std::path::PathBuf::from("program.o")
                    );
                } else {
                    assert_eq!(
                        fs::read(&path).expect("occupied bytes retained"),
                        b"occupied immutable witness"
                    );
                }
                context.validate_inputs().expect("no source input changed by rejected output");
            });
        }
    }
}
