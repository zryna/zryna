//! Private seals of this actual originating observation, never cross-process product authority.
use super::*;
use std::os::unix::process::ExitStatusExt as _;
pub(super) struct Observation {
    artifacts: Vec<(String, Vec<u8>)>,
    records: serde_json::Value,
}
fn records(records: &[process::Record]) -> serde_json::Value {
    serde_json::Value::Array(records.iter().map(|r| serde_json::json!({
        "metadata":export::record(r),"raw_status":r.output.as_ref().map(|o|o.status.into_raw()),
        "stdout":r.output.as_ref().map(|o|o.stdout.as_slice()),
        "stderr":r.output.as_ref().map(|o|o.stderr.as_slice())
    })).collect())
}
impl Observation {
    pub(super) fn capture(session: &Session<'_>) -> Result<Self, Diagnostic> {
        if session.records.iter().map(|r| r.name).collect::<Vec<_>>()
            != ["compile-client", "compile-runtime", "link", "inspect-shim"]
        {
            return Err(rejected());
        }
        for record in &session.records {
            let output = record.output.as_ref().ok_or_else(rejected)?;
            if record.diagnostic.is_some() || !output.status.success() || !output.stderr.is_empty()
            {
                return Err(rejected());
            }
        }
        Ok(Self { artifacts: session.artifacts.clone(), records: records(&session.records) })
    }
    pub(super) fn product(&self, name: &str) -> Result<&[u8], Diagnostic> {
        let mut found = self.artifacts.iter().filter(|(n, _)| n == name);
        let bytes = &found.next().ok_or_else(rejected)?.1;
        if found.next().is_some() {
            return Err(rejected());
        }
        Ok(bytes)
    }
    pub(super) fn check(
        &self,
        session: &Session<'_>,
        stages: &[native::NativeStage],
    ) -> Result<(), Diagnostic> {
        if session.artifacts.get(..self.artifacts.len()) != Some(self.artifacts.as_slice())
            || records(session.records.get(..4).ok_or_else(rejected)?) != self.records
        {
            return Err(rejected());
        }
        for (index, path, name, limit) in [
            (0, "program.o", "program.o", zryna_backend_native::MAX_NATIVE_OBJECT_BYTES),
            (0, "runtime.o", "foreign.o", zryna_backend_native::MAX_NATIVE_OBJECT_BYTES),
            (1, "invocation.c", "client.c", native::MAX_NATIVE_HARNESS_BYTES),
            (2, "invocation.c", "private-runtime.c", native::MAX_NATIVE_HARNESS_BYTES),
            (1, "runtime.o", "compile-client.o", zryna_backend_native::MAX_NATIVE_OBJECT_BYTES),
            (2, "runtime.o", "compile-runtime.o", zryna_backend_native::MAX_NATIVE_OBJECT_BYTES),
            (0, "invocation.elf", "final.elf", native::MAX_NATIVE_EXECUTABLE_BYTES),
        ] {
            let stage = stages.get(index).ok_or_else(rejected)?;
            stage.revalidate()?;
            if material::read(&stage.directory.join(path), limit)? != self.product(name)? {
                return Err(rejected());
            }
        }
        Ok(())
    }
}
