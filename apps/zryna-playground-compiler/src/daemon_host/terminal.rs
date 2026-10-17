//! A closed cleanup acknowledgement; unknown host exits never receive this assertion.

use serde::Serialize;
use std::io::Write;
use zryna_driver::PlaygroundDaemonError;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct Failure {
    format: &'static str,
    version: u32,
    reason: String,
    cleanup_confirmed: bool,
    owner: Option<String>,
    primary: Option<String>,
    cleanup: Option<String>,
}

impl Failure {
    pub(super) fn clean(reason: &str) -> Self {
        Self {
            format: "zryna.playground-host-error.v1",
            version: 1,
            reason: reason.to_owned(),
            cleanup_confirmed: true,
            owner: None,
            primary: None,
            cleanup: None,
        }
    }

    pub(super) fn driver(error: &PlaygroundDaemonError) -> Self {
        let mut failure = Self::clean(error.reason());
        // Every driver operation returns only after cleanup; uncertain cleanup retains an owner.
        // A teardown primary with already-cleaned resources is conservatively unconfirmed too.
        failure.cleanup_confirmed = error.reason() != "PLAYGROUND-HOST-TEARDOWN"
            && error.owner().is_none()
            && error.cleanup().is_none();
        failure.owner = error.owner().map(str::to_owned);
        failure.primary = error.primary().map(|primary| primary.reason().to_owned());
        failure.cleanup = error.cleanup().map(|cleanup| cleanup.reason().to_owned());
        failure
    }

    pub(super) fn write(&self) {
        let mut output = std::io::stderr().lock();
        if serde_json::to_writer(&mut output, self).is_ok() {
            let _ = output.write_all(b"\n");
        }
    }
}

impl From<String> for Failure {
    fn from(reason: String) -> Self {
        Self::clean(&reason)
    }
}

impl From<&str> for Failure {
    fn from(reason: &str) -> Self {
        Self::clean(reason)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn clean_pre_host_failure_has_one_closed_non_secret_record() {
        let value = serde_json::to_value(Failure::clean("PLAYGROUND-REQUEST-SHAPE"))
            .expect("terminal record");
        assert_eq!(value.as_object().expect("closed").len(), 7);
        assert_eq!(value["cleanupConfirmed"], true);
        assert!(value["owner"].is_null());
        assert!(value["primary"].is_null());
        assert!(value["cleanup"].is_null());
    }
}
