use super::{
    Operation, PlaygroundDaemon, Result, archive,
    client::Client,
    image, materials, reject,
    resources::Resources,
    state::{Phase, State},
};
use serde::Deserialize;
use std::{
    fs::File,
    io::Read,
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};

/// Actual fixed compiler/browser bytes, released only after complete owned cleanup.
pub struct PlaygroundDaemonResponse {
    stdout: Vec<u8>,
    owner: String,
    container_id: String,
    elapsed: Duration,
}

impl PlaygroundDaemonResponse {
    /// Complete bounded output of the authenticated fixed child.
    #[must_use]
    pub fn stdout(&self) -> &[u8] {
        &self.stdout
    }
    /// Consumes the complete bounded response bytes after confirmed cleanup.
    #[must_use]
    pub fn into_stdout(self) -> Vec<u8> {
        self.stdout
    }
    /// Fresh owner whose containers and both private volumes were removed.
    #[must_use]
    pub fn owner(&self) -> &str {
        &self.owner
    }
    /// Private runtime namespace identity, removed before response issuance.
    #[must_use]
    pub fn container_id(&self) -> &str {
        &self.container_id
    }
    /// Actual execution and cleanup duration.
    #[must_use]
    pub const fn elapsed(&self) -> Duration {
        self.elapsed
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SourceRequest {
    version: u32,
    revision: u64,
    source: String,
}

pub(super) fn source_request(bytes: &[u8]) -> Result<()> {
    if bytes.is_empty() || bytes.len() > 32_768 {
        return Err(reject("PLAYGROUND-REQUEST-LIMIT", "finite one-source request required"));
    }
    let value: SourceRequest = serde_json::from_slice(bytes)
        .map_err(|_| reject("PLAYGROUND-REQUEST-SHAPE", "closed source carrier required"))?;
    if value.version != 1
        || value.revision == 0
        || value.revision > (1u64 << 53) - 1
        || value.source.len() > 4096
    {
        return Err(reject("PLAYGROUND-REQUEST-LIMIT", "source revision or byte budget exceeded"));
    }
    Ok(())
}

impl PlaygroundDaemon {
    /// Executes one closed source request through the fixed isolated compiler composition.
    ///
    /// # Errors
    /// Rejects invalid requests, unavailable host enforcement, cancellation or unproved cleanup.
    pub fn compile(
        &mut self,
        request: &[u8],
        cancel: &AtomicBool,
    ) -> Result<PlaygroundDaemonResponse> {
        source_request(request)?;
        self.run(Operation::Compile, request, cancel)
    }

    /// Executes a bounded inert evaluation carrier through the fixed contained browser.
    ///
    /// The trusted caller and fixed evaluator must validate the closed frame/authority/argument
    /// carrier. This method admits no executable, source path, generic argv or host capability.
    /// # Errors
    /// Rejects over-budget carriers, unavailable host enforcement, cancellation or failed cleanup.
    pub fn evaluate(
        &mut self,
        request: &[u8],
        cancel: &AtomicBool,
    ) -> Result<PlaygroundDaemonResponse> {
        if request.is_empty() || request.len() > 2 * 1024 * 1024 {
            return Err(reject("PLAYGROUND-REQUEST-LIMIT", "finite evaluation frame required"));
        }
        self.run(Operation::Evaluate, request, cancel)
    }

    fn run(
        &mut self,
        operation: Operation,
        request: &[u8],
        cancel: &AtomicBool,
    ) -> Result<PlaygroundDaemonResponse> {
        if self.poisoned {
            return Err(reject(
                "PLAYGROUND-HOST-TEARDOWN",
                "earlier cleanup failure stops admission",
            ));
        }
        if cancel.load(Ordering::Acquire) {
            return Err(reject("PLAYGROUND-CANCELLED", "cancelled before host admission"));
        }
        let started = Instant::now();
        let deadline = started + Duration::from_secs(operation.seconds());
        let owner = fresh_owner()?;
        let mut state = State { owner: owner.clone(), phase: Phase::Preparing };
        let mut resources = Resources {
            owner: owner.clone(),
            approval: &self.approval,
            operation,
            volumes: Vec::new(),
            containers: Vec::new(),
            client: Client {
                deadline,
                engine_uid: self.approval.record.engine_uid,
                cancel: Some(cancel),
            },
        };
        let result = attempt(&mut resources, &mut state, request);
        let cleanup = resources.cleanup();
        if let Err(cleanup) = cleanup {
            self.poisoned = true;
            return Err(state.cleanup_failure(result.err(), cleanup));
        }
        let (stdout, container_id) = result?;
        state.advance(Phase::Collected, Phase::Removed)?;
        state.success(cancel)?;
        Ok(PlaygroundDaemonResponse { stdout, owner, container_id, elapsed: started.elapsed() })
    }
}

fn attempt(
    resources: &mut Resources<'_>,
    state: &mut State,
    request: &[u8],
) -> Result<(Vec<u8>, String)> {
    // Read-only image admission precedes every owned resource creation.
    image::preflight(
        resources.client,
        &format!("sha256:{}", resources.approval.record.compiler_image_sha256),
    )?;
    if resources.operation == Operation::Evaluate {
        image::preflight(
            resources.client,
            &format!("sha256:{}", resources.approval.record.evaluator_image_sha256),
        )?;
    }
    materials::inspect(
        resources.client,
        &resources.approval.record,
        resources.operation == Operation::Evaluate,
    )?;
    let input = archive::input(request, resources.client.deadline)?;
    resources.volumes()?;
    let runtime = resources.create(false)?;
    resources.start(runtime)?;
    state.advance(Phase::Preparing, Phase::Member)?;
    resources.audit(runtime)?;
    state.advance(Phase::Member, Phase::Audited)?;
    let staging = resources.create(true)?;
    resources.start(staging)?;
    resources.upload(staging, &input)?;
    archive::verify_input(&resources.archive(staging, false)?, request, resources.client.deadline)?;
    resources.stop(staging)?;
    resources.remove(staging)?;
    state.advance(Phase::Audited, Phase::InputSealed)?;
    state.advance(Phase::InputSealed, Phase::Dispatched)?;
    let exit = resources.execute(runtime)?;
    resources.quiescent(runtime)?;
    // Local-driver tmpfs contents disappear when their last active mount is removed.
    // Pin output with the fixed read-only collector before stopping its producer.
    // No output is read until the producer is quiescent and its namespace has stopped.
    let reader = resources.create(true)?;
    resources.start(reader)?;
    resources.stop(runtime)?;
    state.advance(Phase::Dispatched, Phase::Quiescent)?;
    let container_id = resources.id(runtime)?.to_owned();
    let (stdout, stderr) = archive::output(
        &resources.archive(reader, true)?,
        resources.operation,
        resources.client.deadline,
    )?;
    resources.stop(reader)?;
    resources.remove(reader)?;
    state.advance(Phase::Quiescent, Phase::Collected)?;
    if exit != 0 || !stderr.is_empty() {
        let mut error =
            reject("PLAYGROUND-HOST-CHILD", "actual fixed compiler or evaluator refused");
        error.exit_code = Some(exit);
        error.stderr = stderr;
        return Err(error);
    }
    Ok((stdout, container_id))
}

fn fresh_owner() -> Result<String> {
    let mut bytes = [0u8; 16];
    File::open("/dev/urandom").and_then(|mut file| file.read_exact(&mut bytes)).map_err(|_| {
        reject("PLAYGROUND-HOST-IDENTITY", "fresh owned attempt identity unavailable")
    })?;
    let mut owner = String::with_capacity(32);
    for byte in bytes {
        owner.push(char::from(b"0123456789abcdef"[usize::from(byte >> 4)]));
        owner.push(char::from(b"0123456789abcdef"[usize::from(byte & 15)]));
    }
    Ok(owner)
}
