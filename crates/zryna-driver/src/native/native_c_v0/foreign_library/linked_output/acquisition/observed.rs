use super::*;

impl RetainedLinkInputs {
    pub(crate) fn audit_observed(
        &self,
        tools: &super::super::LinuxX8664LinkToolchain,
        trace: &[String],
        stages: &[super::super::NativeStage],
        runtime: bool,
        final_elf: &[u8],
    ) -> Result<closure::ClosureInventory, super::super::Failure> {
        self.check_tools(tools)
            .map_err(|error| comparison(error, "postlink-selected-original-bytes"))?;
        let mut objects = vec![
            ("client", stages[1].capability_file_path("runtime.o")?.to_string_lossy().into_owned()),
            (
                "program",
                stages[0].capability_file_path("program.o")?.to_string_lossy().into_owned(),
            ),
            (
                "foreign",
                stages[0].capability_file_path("runtime.o")?.to_string_lossy().into_owned(),
            ),
        ];
        if runtime {
            objects.push((
                "runtime",
                stages[2].capability_file_path("runtime.o")?.to_string_lossy().into_owned(),
            ));
        }
        self.check_order(trace, &objects)
            .map_err(|error| comparison(error, "postlink-exact-ordered-inputs"))?;
        closure::audit(final_elf, &self.providers(), self.provider_expectation())
            .map_err(|error| comparison(error, "postlink-declared-provider-closure"))
    }
}

fn comparison(error: Diagnostic, stage: &'static str) -> super::super::Failure {
    let mut failure: super::super::Failure = error.into();
    failure.comparator_stage = Some(stage);
    failure
}
