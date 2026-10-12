//! Fixed client-side call instrumentation; no foreign-library or execution authority.

use super::super::*;

#[derive(Clone, Copy)]
pub(in super::super) enum Counter {
    Live,
    MissingIncrement,
}

impl Counter {
    pub(super) const fn evidence_name(self) -> &'static str {
        match self {
            Self::Live => "sum-bytes-live",
            Self::MissingIncrement => "sum-bytes-missing-increment-control",
        }
    }
}

const CLIENT: &str = r"
static volatile size_t sum_calls;
int32_t sum_bytes(const uint8_t *, size_t, int32_t *);
int32_t __real_sum_bytes(const uint8_t *, size_t, int32_t *);
int32_t __wrap_sum_bytes(const uint8_t *bytes, size_t length, int32_t *out) {
  ++sum_calls;
  return __real_sum_bytes(bytes, length, out);
}
";

pub(super) fn client_source(counter: Option<Counter>) -> String {
    match counter {
        None => String::new(),
        Some(Counter::Live) => CLIENT.to_owned(),
        Some(Counter::MissingIncrement) => {
            assert_eq!(CLIENT.matches("++sum_calls;").count(), 1);
            CLIENT.replace("++sum_calls;", "(void)sum_calls;")
        }
    }
}

pub(in super::super) fn observe(
    fixture: &Fixture,
    requirements: &HandleLinkRequirements,
    captured: &CapturedForeignLibrary,
    library: &str,
    client: &str,
    sanitized: bool,
    counter: Counter,
) -> crate::native::BoundedProcessOutput {
    super::execute(
        fixture,
        requirements,
        captured,
        library,
        client,
        super::Execution {
            sanitized,
            timeout: crate::native::MAX_NATIVE_RUN_TIMEOUT,
            counter: Some(counter),
        },
    )
    .expect("actual separately linked byte execution with fixed sum counter")
}
