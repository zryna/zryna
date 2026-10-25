//! Explicit typed V5 helpers; no compilation-profile registration.
use super::{CompilerProvider, ProviderSession, preparation_failure};
use crate::{CommandFailure, native_provider::ComparisonFrontendV5};
use zryna_frontend::VerifiedFrontendProviderV5;
mod bootstrap;

impl ProviderSession<'_> {
    pub(crate) fn frontend_v5(
        &self,
    ) -> Result<Box<dyn VerifiedFrontendProviderV5>, CommandFailure> {
        self.revalidate()?;
        let native = || self.native()?.frontend_v5().map_err(preparation_failure);
        let bootstrap = || {
            bootstrap::configured(
                &self.compiler_root.join("adapters/typescript-6"),
                self.bootstrap_node()?,
            )
        };
        Ok(match self.provider {
            CompilerProvider::Native => Box::new(native()?),
            CompilerProvider::Bootstrap => Box::new(bootstrap()?),
            CompilerProvider::Compare => {
                Box::new(ComparisonFrontendV5::new(native()?, bootstrap()?))
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn private_native_session_v5_is_explicit_and_does_not_register_a_profile() {
        let image = std::env::current_exe().expect("current image");
        let bytes = std::fs::metadata(image).expect("image metadata").len();
        let captured = crate::NativeProviderCapability::capture_current();
        if bytes > 512 * 1024 * 1024 {
            assert_eq!(captured.expect_err("unchanged executable ceiling").code(), "ZRYNA-F1101");
            return;
        }
        let compiler = std::env::temp_dir();
        let session = ProviderSession {
            compiler_root: &compiler,
            provider: CompilerProvider::Native,
            native: Some(captured.expect("bounded image")),
            node: None,
        };
        let frontend = session.frontend_v5().expect("private typed helper");
        assert_eq!(frontend.analysis_work_units(), 1);
        assert_eq!(frontend.minimum_analysis_timeout(), zryna_frontend::MIN_WORKER_TIMEOUT);
        assert!(session.node().is_none());
    }
}
