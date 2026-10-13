//! Held immutable inputs and bounded private C sanitizer processes; no production seal.
use super::*;
pub(super) fn observe<'a>(
    request: &ImportRequest<'a>,
    tools: &'a material::OrdinaryTools,
    parent: &Path,
    case: Case,
) -> Result<(PathBuf, Session<'a>), Diagnostic> {
    request.check()?;
    tools.check()?;
    let client = source::client(request, case)?;
    let runtime = request.requirements.private_runtime_source().ok_or_else(rejected)?;
    let workspace = Workspace::new(parent)?;
    let stages = workspace.stages(3)?;
    let mut session = Session::new(tools);
    session.measurement = Some(request.measurement);
    capture_import_artifacts(&mut session, request, &client);
    session.artifacts.push(("private-runtime.c".into(), runtime.to_vec()));
    session.artifacts.push((
        "private-runtime-header.h".into(),
        request
            .requirements
            .object()
            .program()
            .source()
            .runtime_abi()
            .native_linux_x86_64_header()
            .to_vec(),
    ));
    session.artifacts.push(("sanitizer-scope.json".into(), serde_json::to_vec(&serde_json::json!({
        "case":case.name(), "instrumented":"C client and private runtime only",
        "independent_hook_material_bindings_available":policy::reviewed(case).is_some(),
        "Rust_instrumented":false,"Zryna_instrumented":false,"production_seal":false,
        "loader_selection_proven":false,"complete_input_use":false,"private_recipe_qualified":false
    })).map_err(|_| rejected())?));
    let result = (|| {
        if let Some(bindings) = policy::reviewed(case) {
            bindings.materials(case)?;
        }
        stages[0].write_input(&stages[0].object, request.requirements.object().bytes())?;
        stages[0]
            .write_input(&stages[0].directory.join("runtime.o"), &request.measurement.object)?;
        build::compile(&mut session, &stages[1], "compile-client", client.as_bytes())?;
        if let Some(bindings) = policy::reviewed(case) {
            bindings.materials(case)?;
        }
        build::compile(&mut session, &stages[2], "compile-runtime", runtime)?;
        if let Some(bindings) = policy::reviewed(case) {
            bindings.materials(case)?;
        }
        build::link(&mut session, &stages, Mode::Bytes { missing_increment: false })?;
        let observation = targets::verify(&mut session, &stages, case)?;
        request.check()?;
        run(&mut session, &stages, case, &observation)?;
        request.check()
    })();
    retain_partial(&mut session, &stages);
    cleanup(&mut session, &stages, &[]);
    let path = complete(&mut session, result, &workspace, case.name());
    Ok((path, session))
}
fn retain_partial(session: &mut Session<'_>, stages: &[native::NativeStage]) {
    for (index, file, name, completed, limit) in [
        (
            1,
            "runtime.o",
            "client-partial.o",
            "compile-client.o",
            zryna_backend_native::MAX_NATIVE_OBJECT_BYTES,
        ),
        (
            2,
            "runtime.o",
            "runtime-partial.o",
            "compile-runtime.o",
            zryna_backend_native::MAX_NATIVE_OBJECT_BYTES,
        ),
        (0, "invocation.elf", "final.elf", "final.elf", native::MAX_NATIVE_EXECUTABLE_BYTES),
    ] {
        if !session.artifacts.iter().any(|(n, _)| n == completed)
            && let Ok(bytes) = material::read(&stages[index].directory.join(file), limit)
        {
            session.artifacts.push((name.into(), bytes));
        }
    }
}
fn run(
    session: &mut Session<'_>,
    stages: &[native::NativeStage],
    case: Case,
    observation: &observation::Observation,
) -> Result<(), Diagnostic> {
    if !session.targets_checked {
        return Err(rejected());
    }
    let stage = stages.first().ok_or_else(rejected)?;
    let data = session
        .records
        .iter()
        .find(|r| r.name == "inspect-shim")
        .and_then(|r| r.output.as_ref())
        .ok_or_else(rejected)?
        .stdout
        .clone();
    let bindings = policy::reviewed(case).ok_or_else(rejected)?;
    observation.check(session, stages)?;
    bindings.materials(case)?;
    targets::compare(observation.product("final.elf")?, &data, bindings)?;
    let executable = stage.capability_file_path("invocation.elf")?;
    native::prepare_executable_mode(&executable)?;
    observation.check(session, stages)?;
    process::invoke(session, stage, "run", &executable, Vec::new(), true)?;
    observation.check(session, stages)?;
    bindings.materials(case)
}
