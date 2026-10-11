use super::*;

pub(in super::super) fn observe_link(
    fixture: &Fixture,
    requirements: &HandleLinkRequirements,
    library: &CapturedForeignLibrary,
    foreign: &linked_output::CompiledObject,
    client: &str,
) -> Result<linked_output::Observation, linked_output::Failure> {
    linked_output::check_foreign(requirements, library, foreign)?;
    let inputs = linked_output::capture_from_environment(requirements, library)?;
    observe_retained(fixture, requirements, library, foreign, client, inputs.as_ref())
}

fn observe_retained(
    fixture: &Fixture,
    requirements: &HandleLinkRequirements,
    library: &CapturedForeignLibrary,
    foreign: &linked_output::CompiledObject,
    client: &str,
    inputs: Option<&linked_output::RetainedLinkInputs>,
) -> Result<linked_output::Observation, linked_output::Failure> {
    linked_output::check_foreign(requirements, library, foreign)?;
    if let Some(inputs) = inputs {
        inputs.check_tools(foreign.tools())?;
    }
    let runtime = runtime::compile_observed(fixture, requirements, foreign.tools())?;
    let private_header =
        requirements.object().program().source().runtime_abi().native_linux_x86_64_header();
    if Some(&sha(private_header)) != requirements.private_runtime_header_sha256() {
        return Err(linked_output::rejected().into());
    }
    let source = format!(
        "{}\n{}\n#include <assert.h>\n#include <stdlib.h>\n#include <string.h>\n{}\n{client}",
        std::str::from_utf8(private_header).map_err(|_| linked_output::rejected())?,
        requirements.object().header(),
        include_str!("../oracle.c")
    );
    let root = fixture.observation_root()?;
    if let Some(inputs) = inputs {
        return linked_output::observe_with_inventory(
            &root,
            requirements,
            library,
            foreign,
            source.as_bytes(),
            Some(&runtime),
            &linked_output::InventoryRequest { inputs, wrap: true },
        );
    }
    linked_output::observe(
        &root,
        requirements,
        library,
        foreign,
        source.as_bytes(),
        Some(&runtime),
        true,
    )
}

pub(super) fn check_disaffiliated_control(
    fixture: &Fixture,
    requirements: &HandleLinkRequirements,
    library: &CapturedForeignLibrary,
    foreign: &linked_output::CompiledObject,
) {
    if let Some(inputs) = linked_output::capture_from_environment(requirements, library)
        .expect("configured independent capture")
    {
        let control = inputs
            .capture_disaffiliated_control(requirements, library, foreign.tools())
            .expect("real pinned hostile-role capture");
        let before = linked_output::invocation_attempts();
        let failure =
            observe_retained(fixture, requirements, library, foreign, "not C", Some(&control))
                .expect_err(
                    "missing originating Tool affiliation must reject before runtime compiler",
                );
        assert_eq!(
            linked_output::invocation_attempts(),
            before,
            "no new runtime/client/link attempt"
        );
        assert!(failure.invocations.is_empty());
        fixture.empty();
        eprintln!("m5-private-byte-tool-affiliation-rejected-before-new-process {before}");
    }
}
