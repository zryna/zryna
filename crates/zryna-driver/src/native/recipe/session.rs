//! Linux compile-only session. Only the final owning function constructs actual success.
mod client;
mod config;
mod execute;
mod identity;
mod image;
mod inputs;
mod kernel;
mod resources;
mod toolroot;

use super::approval::{self, ApprovedNativeRecipe};
use super::archive;
use super::plan::{NativeRecipeError, denied};
use crate::native::native_c_v0::foreign_library::CapturedForeignLibrary;
use std::{
    fs::File,
    io::Read,
    time::{Duration, Instant},
};

pub use toolroot::{NativeToolRootApproval, native_tool_root_identity};

#[derive(Debug)]
pub(super) enum Failure {
    Enforcement(&'static str),
    Deadline(&'static str),
}
type Result<T> = std::result::Result<T, Failure>;
fn fail(detail: &'static str) -> Failure {
    Failure::Enforcement(detail)
}
fn expired(detail: &'static str) -> Failure {
    Failure::Deadline(detail)
}
impl Failure {
    fn detail(&self) -> &'static str {
        match self {
            Self::Enforcement(detail) | Self::Deadline(detail) => detail,
        }
    }
    fn into_error(self) -> NativeRecipeError {
        match self {
            Self::Enforcement(detail) => denied("TRUST-ENFORCEMENT", detail),
            Self::Deadline(detail) => denied("TRUST-TIMEOUT", detail),
        }
    }
}

/// Actual audited compile-only result. No deserializer, raw constructor or cache importer exists.
/// Native linking, release publication and loading still require their separate authorities.
#[derive(Debug)]
pub struct NativeRecipeSuccess {
    approved: ApprovedNativeRecipe,
    root: NativeToolRootApproval,
    object: CapturedForeignLibrary,
    owner: String,
    recipe_id: String,
    holder_id: String,
    elapsed: Duration,
}
impl NativeRecipeSuccess {
    /// The original retained source/lock/ABI issuer and independent operator approval.
    #[must_use]
    pub const fn approval(&self) -> &ApprovedNativeRecipe {
        &self.approved
    }
    /// Independently approved exact root, checked against all actual root archive members.
    #[must_use]
    pub const fn tool_root(&self) -> &NativeToolRootApproval {
        &self.root
    }
    /// Actual complete audited object; this immutable data alone is not an execution grant.
    #[must_use]
    pub const fn object(&self) -> &CapturedForeignLibrary {
        &self.object
    }
    /// Fresh owner identity whose two containers and three volumes were confirmed removed.
    #[must_use]
    pub fn owner(&self) -> &str {
        &self.owner
    }
    /// Actual recipe and trusted-holder IDs, both removed before this result was constructed.
    #[must_use]
    pub fn container_ids(&self) -> (&str, &str) {
        (&self.recipe_id, &self.holder_id)
    }
    /// Actual total execution and cleanup duration.
    #[must_use]
    pub const fn elapsed(&self) -> Duration {
        self.elapsed
    }
}
/// Returns a requested policy identity, without certifying actual host enforcement.
#[must_use]
pub fn native_recipe_policy_identity() -> crate::package_build::ExecutionPolicyIdentity {
    config::policy_identity()
}
/// Complete fixed output-relevant environment; no ambient host variables are forwarded.
#[must_use]
pub const fn native_recipe_environment() -> [&'static str; 8] {
    config::ENV
}

/// Executes one independently approved Linux x86-64 C compile in a closed local Docker root.
///
/// Requires the trusted local Docker daemon at `/var/run/docker.sock`. The 30-second job deadline
/// covers all host operations; a single additional 5-second deadline covers owned cleanup.
/// No source cache, release output, network acquisition, shell or user-selected host path is used.
/// # Errors
/// Fails closed on unsupported policy, mismatched inputs/tools/image, unavailable enforcement,
/// process failure, unknown teardown, output audit failure or unconfirmed resource cleanup.
pub fn execute_native_recipe(
    approved: ApprovedNativeRecipe,
    root: NativeToolRootApproval,
) -> std::result::Result<NativeRecipeSuccess, NativeRecipeError> {
    let started = Instant::now();
    let deadline = started + Duration::from_secs(30);
    let files = inputs::prepare(&approved, &root).map_err(Failure::into_error)?;
    let tar = archive::write_input(deadline, &files, 2 * 1024 * 1024)
        .map_err(|_| denied("TRUST-INPUT", "bounded input archive creation failed"))?;
    let owner = fresh_owner().map_err(Failure::into_error)?;
    let mut resources = resources::Resources {
        owner: owner.clone(),
        image: format!("sha256:{}", root.image_sha256),
        volumes: Vec::new(),
        containers: Vec::new(),
        client: client::Client { deadline, engine_uid: root.engine_uid },
    };
    let result = run(&mut resources, &approved, &root, &files, &tar);
    // No failure or unknown cleanup can mint success, even after a valid object was captured.
    let cleanup = resources.cleanup();
    if let Err(error) = cleanup {
        return Err(NativeRecipeError::Cleanup {
            primary: result.err().map(Box::new),
            owner,
            detail: error.detail(),
        });
    }
    let (object, recipe_id, holder_id) = result?;
    Ok(NativeRecipeSuccess {
        approved,
        root,
        object,
        owner,
        recipe_id,
        holder_id,
        elapsed: started.elapsed(),
    })
}
fn fresh_owner() -> Result<String> {
    let mut bytes = [0u8; 16];
    File::open("/dev/urandom")
        .and_then(|mut file| file.read_exact(&mut bytes))
        .map_err(|_| fail("fresh owned session identity unavailable"))?;
    Ok(toolroot::hex(&bytes))
}
fn run(
    resources: &mut resources::Resources,
    approved: &ApprovedNativeRecipe,
    root: &NativeToolRootApproval,
    files: &std::collections::BTreeMap<String, Vec<u8>>,
    tar: &[u8],
) -> std::result::Result<(CapturedForeignLibrary, String, String), NativeRecipeError> {
    let attempt = prepare_and_compile(resources, root, files, tar).map_err(Failure::into_error)?;
    let (recipe, holder) = attempt;
    resources.stop_recipe(recipe).map_err(Failure::into_error)?;
    resources.live(holder).map_err(Failure::into_error)?;
    let output = resources
        .archive(holder, "%2Foutput", 8 * 1024 * 1024 + 65_536)
        .map_err(Failure::into_error)?;
    resources.live(holder).map_err(Failure::into_error)?;
    let mut inventory = archive::extract(
        resources.client.deadline,
        &output,
        &inputs::expected_output(approved),
        8 * 1024 * 1024 + 65_536,
    )
    .map_err(|_| {
        denied("TRUST-OUTPUT", "complete output archive differs from independent inventory")
    })?;
    let bytes = inventory
        .remove("adapter.o")
        .ok_or_else(|| denied("TRUST-OUTPUT", "sole declared native object absent"))?;
    let object = approval::capture_output(approved, &bytes)?;
    if Instant::now() >= resources.client.deadline {
        return Err(denied("TRUST-TIMEOUT", "native output audit exceeded job deadline"));
    }
    Ok((
        object,
        resources.id(recipe).map_err(Failure::into_error)?.into(),
        resources.id(holder).map_err(Failure::into_error)?.into(),
    ))
}
fn prepare_and_compile(
    resources: &mut resources::Resources,
    root: &NativeToolRootApproval,
    files: &std::collections::BTreeMap<String, Vec<u8>>,
    tar: &[u8],
) -> Result<(usize, usize)> {
    image::preflight(resources.client, &resources.image)?;
    resources.volumes()?;
    let holder = resources.create(config::Role::Holder)?;
    let actual_root = resources.archive(holder, "%2F", 128 * 1024 * 1024)?;
    toolroot::verify(resources.client.deadline, &actual_root, root)?;
    drop(actual_root);
    resources.start(holder)?;
    resources.upload_input(holder, tar)?;
    let actual_input = resources.archive(holder, "%2Finput", 2 * 1024 * 1024)?;
    archive::extract(
        resources.client.deadline,
        &actual_input,
        &inputs::expected_input(files),
        2 * 1024 * 1024,
    )
    .map_err(|_| fail("actual complete prepared input inventory differs"))?;
    // Immutable preparation fence: no uploads or holder execs occur after this point.
    let recipe = resources.create(config::Role::Recipe)?;
    resources.start(recipe)?;
    execute::execute(resources,recipe,"inspect",b"{\"kernelMountFlags\":true,\"inputModes\":true,\"procMemoryReadOnly\":true,\"controlModes\":true,\"inheritedFdsClosed\":true}\n")?;
    execute::execute(
        resources,
        recipe,
        "check-boundaries",
        b"{\"declaredBoundaryDenials\":true,\"scratchWriteReadAllowed\":true}\n",
    )?;
    execute::execute(resources, recipe, "check-wx", b"{\"explicitMemoryDenials\":true}\n")?;
    execute::execute(resources, recipe, "compile", b"")?;
    // No new exec, start or restart is possible after returning this phase.
    Ok((recipe, holder))
}

#[cfg(test)]
mod tests;
