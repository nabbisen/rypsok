//! Bounding a call by a deadline (RFC 021, D1).
//!
//! The one function that both tools use to give a call its maximum: LIM-S7
//! for `web_search` and LIM-F5 for `web_fetch`, each measured from the moment
//! the call begins. It has no clock of its own: the deadline is a point of
//! tokio's clock, as in the limiter, so tests with paused time are exact.

use std::future::Future;

use tokio::time::Instant;

/// Runs `call` until `deadline` and returns what it returns; after the
/// deadline it drops `call`, which cancels whatever the call had started, and
/// returns the error that `on_expiry` makes. `on_expiry` runs at most once,
/// and only on expiry: it is where the caller counts and logs the call that
/// the maximum ended.
///
/// A future can be left only where it awaits, so this bounds what the call
/// waits for; it does not interrupt code that never yields.
pub(crate) async fn within<T, E>(
    deadline: Instant,
    on_expiry: impl FnOnce() -> E,
    call: impl Future<Output = Result<T, E>>,
) -> Result<T, E> {
    match tokio::time::timeout_at(deadline, call).await {
        Ok(result) => result,
        Err(_) => Err(on_expiry()),
    }
}

#[cfg(test)]
mod tests;
