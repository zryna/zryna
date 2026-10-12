//! Post-link data audits retain the original raw ELF on every comparator failure.
use super::*;

pub(super) struct Inputs<'a> {
    pub(super) requirements: &'a HandleLinkRequirements,
    pub(super) library: &'a CapturedForeignLibrary,
    pub(super) foreign: &'a CompiledObject,
    pub(super) client: &'a CompiledObject,
    pub(super) runtime: Option<&'a CompiledObject>,
    pub(super) inventory: Option<&'a RetainedLinkInputs>,
}

pub(super) fn audit(
    stages: &[NativeStage],
    link: Invocation,
    context: &Inputs<'_>,
) -> Result<Observation, Failure> {
    let requirements = context.requirements;
    let library = context.library;
    let foreign = context.foreign;
    let client = context.client;
    let runtime = context.runtime;
    let inputs = context.inventory;
    let stage = stages.first().ok_or_else(rejected)?;
    let mut retained_final = None;
    let mut comparator_stage = "link-process";
    (|| -> Result<Observation, Failure> {
        if !link.status.success() || !link.stderr.is_empty() {
            return Err(rejected().into());
        }
        comparator_stage = "linker-trace-decoding";
        let trace = process::trace(&link.stdout)?;
        let executable = stage.capability_file_path("invocation.elf")?;
        comparator_stage = "bounded-final-elf-read";
        retained_final = Some(staging::bytes(&executable, native::MAX_NATIVE_EXECUTABLE_BYTES)?);
        let final_elf = retained_final.as_ref().ok_or_else(rejected)?;
        comparator_stage = "limited-native-executable-audit";
        let (_, audited) = native::audit_staged_executable(&executable, "zryna_c_v0_i_dispatch")?;
        if final_elf.as_slice() != audited.as_ref() {
            return Err(rejected().into());
        }
        comparator_stage = "legacy-elf-requirements";
        let dependencies = elf::read(final_elf)?;
        comparator_stage = "retained-inventory-comparison";
        let inventory = if let Some(inputs) = inputs {
            let closure = inputs.audit_observed(
                foreign.tools(),
                &trace,
                stages,
                runtime.is_some(),
                final_elf,
            )?;
            Some((inputs.clone(), closure))
        } else {
            None
        };
        // There is intentionally no executable mode, Run phase, loader lookup or permission seal.
        Ok(Observation {
            requirements: requirements.clone(),
            library: library.clone(),
            foreign: foreign.clone(),
            client: client.clone(),
            runtime: runtime.cloned(),
            link,
            final_elf: retained_final.take().ok_or_else(rejected)?,
            dependencies,
            trace,
            inventory,
        })
    })()
    .map_err(|mut failure| {
        failure.final_elf = retained_final.take();
        failure.comparator_stage = failure.comparator_stage.or(Some(comparator_stage));
        failure
    })
}
