//! Production GNU invocation under the existing captured tools and finite process bounds.

mod inputs;
use super::dependencies::Dependencies;
use crate::native::{
    BoundedProcessOutput, NativeProcessLimits, NativeStage, ProcessPhase, run_bounded_process,
};
pub(super) use inputs::Expected as ExpectedInputs;
use std::ffi::OsString;
use zryna_diagnostics::Diagnostic;

pub(super) struct Context<'s> {
    pub(super) stage: &'s NativeStage,
    pub(super) dependencies: &'s Dependencies,
    pub(super) limits: NativeProcessLimits,
    inputs: inputs::Inputs,
}

impl<'s> Context<'s> {
    fn fresh_output(&self, name: &str) -> Result<std::path::PathBuf, Vec<Diagnostic>> {
        self.stage.revalidate().map_err(|error| vec![error])?;
        let path = self.stage.capability_file_path(name).map_err(|error| vec![error])?;
        match std::fs::symlink_metadata(&path) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(path),
            _ => Err(vec![super::staging_error()]),
        }
    }
    pub(super) fn capture(
        stage: &'s NativeStage,
        dependencies: &'s Dependencies,
        limits: NativeProcessLimits,
        expected: ExpectedInputs<'_>,
    ) -> Result<Self, Diagnostic> {
        let inputs = inputs::Inputs::capture(stage, expected)?;
        Ok(Self { stage, dependencies, limits, inputs })
    }
    pub(super) fn validate_inputs(&self) -> Result<(), Diagnostic> {
        self.inputs.validate(self.stage)
    }
    pub(super) fn invoke(
        &self,
        args: &[OsString],
    ) -> Result<BoundedProcessOutput, Vec<Diagnostic>> {
        self.validate_inputs().map_err(|error| vec![error])?;
        self.dependencies.revalidate().map_err(|error| vec![error])?;
        let directory = self.stage.capability_directory_path();
        let output = run_bounded_process(
            &self.dependencies.tools().driver,
            args,
            &directory,
            self.limits.link_timeout(),
            self.limits.tool_output_bytes(),
            self.limits.tool_output_bytes(),
            ProcessPhase::Link,
            Some(&directory),
        );
        let validation = self.dependencies.revalidate().and_then(|()| self.validate_inputs());
        let mut errors = Vec::new();
        if let Err(error) = validation {
            errors.push(error);
        }
        let output = match output {
            Ok(output) => output,
            Err(error) => {
                errors.insert(0, error);
                return Err(errors);
            }
        };
        if !errors.is_empty() {
            return Err(errors);
        }
        if !output.status.success() || !output.stdout.is_empty() || !output.stderr.is_empty() {
            return Err(vec![rejected(args, &output)]);
        }
        Ok(output)
    }

    pub(super) fn compile_runtime(&mut self) -> Result<Vec<u8>, Vec<Diagnostic>> {
        self.inputs.require_uncompiled().map_err(|error| vec![error])?;
        let source = self.stage.capability_file_path("runtime.c").map_err(|error| vec![error])?;
        let object = self.fresh_output("runtime.o")?;
        let mut args = [
            "-std=c11",
            "-O0",
            "-g0",
            "-pedantic",
            "-Wall",
            "-Wextra",
            "-Werror",
            "-fno-common",
            "-fno-ident",
            "-fno-pie",
            "-fno-pic",
            "-fcf-protection=none",
            "-fno-stack-protector",
            "-fno-unwind-tables",
            "-fno-asynchronous-unwind-tables",
            "-c",
        ]
        .map(OsString::from)
        .to_vec();
        args.extend([source.into_os_string(), "-o".into(), object.clone().into_os_string()]);
        self.invoke(&args)?;
        let (identity, bytes) =
            super::stable::read(&object, zryna_backend_native::MAX_NATIVE_OBJECT_BYTES)
                .map_err(|error| vec![error])?;
        self.inputs.compiled(self.stage, &identity, &bytes).map_err(|error| vec![error])?;
        Ok(bytes)
    }

    pub(super) fn link(&self) -> Result<Vec<u8>, Vec<Diagnostic>> {
        self.inputs.require_compiled().map_err(|error| vec![error])?;
        let path = |name| self.stage.capability_file_path(name).map_err(|error| vec![error]);
        let executable = self.fresh_output("invocation.elf")?;
        let mut args = [
            "-std=c11",
            "-O0",
            "-g0",
            "-fno-ident",
            "-fno-pie",
            "-no-pie",
            "-fno-stack-protector",
            "-fcf-protection=none",
            "-Wl,--build-id=none",
            "-Wl,--fatal-warnings",
            "-Wl,--no-undefined",
            "-Wl,-z,noexecstack,-z,relro,-z,now",
            "-Wl,--dynamic-linker=/lib64/ld-linux-x86-64.so.2",
            "-o",
        ]
        .map(OsString::from)
        .to_vec();
        args.extend([
            executable.clone().into_os_string(),
            path("invocation.c")?.into_os_string(),
            path("program.o")?.into_os_string(),
            path("runtime.o")?.into_os_string(),
        ]);
        self.invoke(&args)?;
        super::stable::read(&executable, crate::native::MAX_NATIVE_EXECUTABLE_BYTES)
            .map(|(_, bytes)| bytes)
            .map_err(|error| vec![error])
    }
}

fn rejected(args: &[OsString], output: &BoundedProcessOutput) -> Diagnostic {
    Diagnostic::error(
        "ZRYNA-C4167",
        None,
        "the captured GNU toolchain rejected generic runtime compilation or invocation linking",
        format!(
            "Retain the exact typed compiler inputs and documented GNU toolchain. Actual argv: {args:?}; status: {}; stdout: {}; stderr: {}",
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ),
    )
}
