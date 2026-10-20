//! One module per driver, each behind a Cargo feature: `wikipedia` under
//! `engine-wikipedia`, `brave` under `engine-brave` (RFC 006).
//!
//! Every driver declares the configuration keys it accepts; the
//! configuration refuses any other key in its table (RFC 004).

/// The configuration keys a driver type declares, or `None` for an unknown
/// type. Drivers compiled out by feature are unknown.
pub fn declared_keys(kind: &str) -> Option<&'static [&'static str]> {
    match kind {
        #[cfg(feature = "engine-wikipedia")]
        "wikipedia" => Some(&[]),
        #[cfg(feature = "engine-brave")]
        "brave" => Some(&["api_key_env", "api_key_file", "endpoint"]),
        _ => None,
    }
}

#[cfg(test)]
mod tests;
