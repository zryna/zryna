//! Host admission is explicit and complete before any selected target can execute.
use super::{CommandFailure, NodeRuntimeCapability, TargetSelection, failure};
use crate::{
    CommandFailureKind,
    native::{LinuxX8664LinkToolchain, NativeProcessLimits, discover_linux_native_toolchain},
};
pub(crate) struct GenericRunHosts<'a> {
    targets: TargetSelection,
    node: Option<&'a NodeRuntimeCapability>,
    pub(super) tools: Option<LinuxX8664LinkToolchain>,
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    pub(super) limits: NativeProcessLimits,
}
impl<'a> GenericRunHosts<'a> {
    pub(crate) fn prepare(
        targets: TargetSelection,
        node: Option<&'a NodeRuntimeCapability>,
    ) -> Result<Self, CommandFailure> {
        selection(targets)?;
        if targets.javascript() || targets.webassembly() {
            node.ok_or_else(|| {
                failure::boundary(
                    "selected JavaScript/Wasm execution requires an admitted Node host",
                )
            })?
            .revalidate()
            .map_err(failure::preparation)?;
        }
        let limits = NativeProcessLimits::default();
        let tools =
            if targets.native() {
                Some(discover_linux_native_toolchain(limits).map_err(|error| {
                    failure::native(vec![error], CommandFailureKind::Preparation)
                })?)
            } else {
                None
            };
        Ok(Self {
            targets,
            node,
            tools,
            #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
            limits,
        })
    }
    pub(super) fn validate(&self, targets: TargetSelection) -> Result<(), CommandFailure> {
        selection(targets)?;
        if self.targets != targets || self.tools.is_some() != targets.native() {
            return Err(failure::boundary("run targets differ from their captured hosts"));
        }
        if targets.javascript() || targets.webassembly() {
            self.node()?.revalidate().map_err(failure::preparation)?;
        }
        Ok(())
    }
    pub(super) fn node(&self) -> Result<&'a NodeRuntimeCapability, CommandFailure> {
        self.node.ok_or_else(|| failure::boundary("missing admitted Node runtime"))
    }
}
fn selection(targets: TargetSelection) -> Result<(), CommandFailure> {
    if targets.component() {
        return Err(failure::boundary("bounded generics does not admit component execution"));
    }
    Ok(())
}
