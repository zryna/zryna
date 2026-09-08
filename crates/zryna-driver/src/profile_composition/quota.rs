use std::collections::{BTreeMap, BTreeSet};

use zryna_diagnostics::Diagnostic;

use super::{
    INVALID, RESOURCE, error,
    model::{Capability, Instance, Reservation},
};

const MAX_ENDPOINT_BYTES: usize = 259;

fn fail(message: &str) -> Vec<Diagnostic> {
    vec![error(RESOURCE, message)]
}

fn token(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 1024
        && value.bytes().all(|byte| byte.is_ascii_alphanumeric() || b"-_.:".contains(&byte))
}

// Internal endpoints arrive already normalized. Reject aliases instead of guessing host semantics.
fn endpoint(value: &str) -> bool {
    let Some((host, port)) = value.rsplit_once(':') else {
        return false;
    };
    !host.is_empty()
        && host.len() <= 253
        && !host.starts_with('.')
        && !host.ends_with('.')
        && !host.contains("..")
        && host
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || b".-".contains(&byte))
        && port.parse::<u16>().is_ok_and(|number| number > 0 && number.to_string() == port)
}

pub(super) fn validate_reservation(value: &Reservation) -> Result<(), Vec<Diagnostic>> {
    if value.environment.len() > 128 || value.preopens.len() > 16 || value.endpoints.len() > 128 {
        return Err(fail("reservation entry structural limit exceeded"));
    }
    let mut bytes = 0_usize;
    for (key, text) in &value.environment {
        if key.is_empty() || key.contains(['\0', '=']) || text.contains('\0') {
            return Err(vec![error(INVALID, "malformed environment entry")]);
        }
        bytes = bytes
            .checked_add(key.len())
            .and_then(|n| n.checked_add(text.len()))
            .filter(|n| *n <= 65_536)
            .ok_or_else(|| fail("environment bytes limit 65536 exceeded at 65537"))?;
    }
    if value.endpoints.iter().any(|id| id.len() > MAX_ENDPOINT_BYTES) {
        return Err(fail("endpoint bytes limit 259 exceeded at 260"));
    }
    if value.preopens.iter().any(|id| !token(id)) || value.endpoints.iter().any(|id| !endpoint(id))
    {
        return Err(vec![error(INVALID, "noncanonical preopen authority or endpoint identity")]);
    }
    Ok(())
}

pub(super) fn aggregate<'a>(
    nodes: impl Iterator<Item = &'a Instance>,
) -> Result<[u64; 10], Vec<Diagnostic>> {
    let mut total = [0_u64; 10];
    let mut environment = BTreeMap::new();
    let mut preopens = BTreeSet::new();
    let mut endpoints = BTreeSet::new();
    for node in nodes {
        let value = &node.reservation;
        for (key, text) in &value.environment {
            if environment.insert(key, text).is_some_and(|previous| previous != text) {
                return Err(fail("conflicting environment values in shared instance closure"));
            }
        }
        preopens.extend(&value.preopens);
        endpoints.extend(&value.endpoints);
        for (index, amount) in [
            (0, value.subscriptions),
            (1, value.timers),
            (5, value.descriptors),
            (7, value.operations),
            (9, value.random_total),
        ] {
            total[index] = total[index]
                .checked_add(amount)
                .ok_or_else(|| fail("reservation arithmetic overflow"))?;
        }
        total[8] = total[8].max(value.random_per_call);
        let used = [
            value.subscriptions > 0 || value.timers > 0,
            !value.environment.is_empty(),
            !value.preopens.is_empty() || value.descriptors > 0,
            !value.endpoints.is_empty() || value.operations > 0,
            value.random_per_call > 0 || value.random_total > 0,
        ];
        for (capability, used) in [
            Capability::Clock,
            Capability::Environment,
            Capability::Filesystem,
            Capability::Network,
            Capability::Randomness,
        ]
        .into_iter()
        .zip(used)
        {
            if used && !node.requirements.iter().any(|request| request.capability == capability) {
                return Err(fail("reservation belongs to an undeclared direct capability"));
            }
        }
        if value.random_per_call > value.random_total {
            return Err(fail("per-call randomness exceeds the declared cumulative reservation"));
        }
    }
    total[2] = environment.len() as u64;
    total[3] = environment
        .iter()
        .try_fold(0_u64, |bytes, (key, text)| {
            bytes.checked_add(key.len() as u64)?.checked_add(text.len() as u64)
        })
        .ok_or_else(|| fail("environment byte arithmetic overflow"))?;
    total[4] = preopens.len() as u64;
    total[6] = endpoints.len() as u64;
    Ok(total)
}
