//! Secrets by reference (REQ-CFG-004) and the type that holds them
//! redacted (REQ-SEC-020).

use std::fmt;
use std::path::PathBuf;

/// A value that must never appear in logs or debug output. `Debug` prints
/// `[redacted]`; there is no `Display`.
#[derive(Clone, PartialEq, Eq)]
pub struct Secret<T>(T);

impl<T> Secret<T> {
    /// Wraps a value.
    pub fn new(value: T) -> Self {
        Secret(value)
    }

    /// Reads the value. The name makes every use visible in review.
    pub fn expose_secret(&self) -> &T {
        &self.0
    }
}

impl<T> fmt::Debug for Secret<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("[redacted]")
    }
}

/// Where a credential is read from, once, at startup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SecretSource {
    /// The named environment variable.
    Env(String),
    /// The file at this path.
    File(PathBuf),
}

/// Why a secret could not be read. The message names the source, never a
/// value.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SecretError {
    /// The environment variable is unset or empty. The name is shown only
    /// when it looks like a variable name: the commonest mistake is to paste
    /// the secret into the key that should name its variable, and the
    /// message goes to the log (RFC 020, L1 of the 020l review).
    #[error("{}", env_unset_text(.0))]
    EnvUnset(String),
    /// The file cannot be read or is empty.
    #[error("secret file {0} cannot be read or is empty")]
    FileUnreadable(PathBuf),
}

/// Whether a text can be shown as the name of an environment variable:
/// upper-case letters, digits and underscores, not starting with a digit,
/// at most 64 characters.
fn looks_like_a_variable_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && !name.starts_with(|c: char| c.is_ascii_digit())
        && name
            .chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
}

fn env_unset_text(name: &str) -> String {
    if looks_like_a_variable_name(name) {
        format!("environment variable {name} is unset or empty")
    } else {
        "the value is not the name of an environment variable (it is not shown); this key names a variable that holds the secret, such as BRAVE_API_KEY, it does not hold the secret".to_owned()
    }
}

impl SecretSource {
    /// Reads the secret. A file secret is trimmed of surrounding whitespace.
    /// On Unix, a world-readable secret file is reported through `warning`
    /// so that the caller can log it; the secret is still used.
    pub fn resolve(&self) -> Result<(Secret<String>, Option<String>), SecretError> {
        match self {
            SecretSource::Env(name) => match std::env::var(name) {
                Ok(value) if !value.trim().is_empty() => {
                    Ok((Secret(value.trim().to_owned()), None))
                }
                _ => Err(SecretError::EnvUnset(name.clone())),
            },
            SecretSource::File(path) => {
                let text = std::fs::read_to_string(path)
                    .map_err(|_| SecretError::FileUnreadable(path.clone()))?;
                let value = text.trim();
                if value.is_empty() {
                    return Err(SecretError::FileUnreadable(path.clone()));
                }
                let warning = world_readable(path)
                    .then(|| format!("secret file {} is readable by everyone", path.display()));
                Ok((Secret(value.to_owned()), warning))
            }
        }
    }
}

#[cfg(unix)]
fn world_readable(path: &std::path::Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(path)
        .map(|m| m.permissions().mode() & 0o004 != 0)
        .unwrap_or(false)
}

#[cfg(not(unix))]
fn world_readable(_path: &std::path::Path) -> bool {
    false
}
