//! Separately links observed hostile Rust bodies; final function/real-call identity stays exact.
use super::*;
use object::{Object as _, ObjectSymbol as _};

pub(super) fn run<'a>(
    request: &Request<'_>,
    tools: &'a material::OrdinaryTools,
    parent: &Path,
) -> Result<(PathBuf, Session<'a>), Diagnostic> {
    request.check()?;
    tools.check()?;
    let client = clients::build(request)?;
    let workspace = Workspace::new(parent)?;
    let stages = workspace.stages(3)?;
    let mut session = Session::new(tools);
    session.artifacts.extend([
        ("program.o".into(), request.requirements.object().bytes().to_vec()),
        ("program-header.h".into(), request.requirements.object().header().as_bytes().to_vec()),
        ("foreign.o".into(), request.hostile.object().to_vec()),
        ("foreign.rs".into(), request.hostile.source().to_vec()),
        ("client.c".into(), client.as_bytes().to_vec()),
        ("library-header.h".into(), capture::HEADER.to_vec()),
        ("library-policy.json".into(), capture::POLICY.to_vec()),
        (
            "hostile-prerequisite.json".into(),
            serde_json::to_vec(&request.hostile.report()).map_err(|_| rejected())?,
        ),
    ]);
    let result = (|| {
        stages[0].write_input(&stages[0].object, request.requirements.object().bytes())?;
        stages[0].write_input(&stages[0].directory.join("runtime.o"), request.hostile.object())?;
        process::compile(&mut session, &stages[1], "compile-client", client.as_bytes())?;
        let runtime = request.requirements.private_runtime_source().ok_or_else(rejected)?;
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
        process::compile(&mut session, &stages[2], "compile-runtime", runtime)?;
        let mode = Mode::Bytes { missing_increment: false };
        process::link(&mut session, &stages, mode)?;
        verify_targets(&mut session, &stages[0], request.hostile)?;
        request.check()?;
        process::run(&mut session, &stages[0], mode)
    })();
    for (index, name) in [(1, "client-partial.o"), (2, "runtime-partial.o")] {
        let complete = if index == 1 { "compile-client.o" } else { "compile-runtime.o" };
        if !session.artifacts.iter().any(|(n, _)| n == complete)
            && let Ok(bytes) = material::read(
                &stages[index].directory.join("runtime.o"),
                zryna_backend_native::MAX_NATIVE_OBJECT_BYTES,
            )
        {
            session.artifacts.push((name.into(), bytes));
        }
    }
    if !session.artifacts.iter().any(|(n, _)| n == "final.elf")
        && let Ok(bytes) = material::read(
            &stages[0].directory.join("invocation.elf"),
            native::MAX_NATIVE_EXECUTABLE_BYTES,
        )
    {
        session.artifacts.push(("final.elf".into(), bytes));
    }
    cleanup(&mut session, &stages, &[]);
    let path = complete(&mut session, result, &workspace, request.hostile.kind().name());
    Ok((path, session))
}
fn verify_targets(
    session: &mut Session<'_>,
    stage: &native::NativeStage,
    hostile: &HostileObservation,
) -> Result<(), Diagnostic> {
    hostile.check()?;
    session.phase = "hostile-rust-function-byte-comparison";
    let final_bytes =
        &session.artifacts.iter().find(|(n, _)| n == "final.elf").ok_or_else(rejected)?.1;
    targets::compare_functions(hostile.object(), final_bytes)?;
    let file = object::File::parse(final_bytes.as_slice()).map_err(|_| rejected())?;
    let mut symbols = file.symbols().filter(|s| s.name().ok() == Some("sum_bytes"));
    let symbol = symbols.next().ok_or_else(rejected)?;
    if symbols.next().is_some() {
        return Err(rejected());
    }
    let address = symbol.address();
    let args = vec![
        "-d".into(),
        "--no-show-raw-insn".into(),
        "--disassemble=__wrap_sum_bytes".into(),
        stage.capability_file_path("invocation.elf")?.into_os_string(),
    ];
    let objdump = session.tools.objdump.clone();
    process::invoke(session, stage, "inspect-shim", &objdump, args, false)?;
    process::successful(session)?;
    let bytes =
        &session.records.last().and_then(|r| r.output.as_ref()).ok_or_else(rejected)?.stdout;
    session.phase = "hostile-rust-real-call-target-comparison";
    targets::compare_target(bytes, address, "sum_bytes")?;
    session.targets_checked = true;
    Ok(())
}
