//! A bound on the sessions of the handshake-based revisions over HTTP
//! (RFC 019 D5; REQ-SEC-024, LIM-H6, THR-27).
//!
//! The SDK creates one session per legacy `initialize` and sets no limit.
//! [`BoundedSessions`] wraps a session manager, **owns the count** and
//! refuses to create a session beyond the bound. The count is
//! `live + pending` behind one lock that is never held across an `.await`:
//! `live` is the set of session ids that exist, `pending` the creations in
//! flight. A set, not a counter, because the SDK closes one session from
//! two places (its `DELETE` handler and the session worker's exit) and the
//! two can run at once; closing an id that is already gone changes nothing,
//! so the count cannot drift or underflow.
//!
//! The SDK answers a failed `create_session` with a 500 whose body carries
//! the error's text. A refusal is therefore also signalled through a
//! task-local cell that the gate scopes around the SDK call
//! ([`scope_refusals`]); the gate then discards the SDK's response and
//! answers 503 itself. The error's text is a fixed sentence, because it is
//! what a client would read if the cell were ever unset.

use std::cell::Cell;
use std::collections::HashSet;
use std::fmt;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use futures_core::Stream;
use rmcp::model::{ClientJsonRpcMessage, ServerJsonRpcMessage};
use rmcp::transport::streamable_http_server::session::{
    EventStore, RestoreOutcome, ServerSseMessage, SessionId, SessionManager,
};

/// The `Retry-After` hint, in seconds, of a refused handshake: a fixed,
/// documented value (RFC 019 D5). It is a hint; sessions expire after 300 s
/// without traffic.
pub const SESSION_RETRY_AFTER_S: u64 = 30;

tokio::task_local! {
    /// Set by [`BoundedSessions`] when it refuses a session, read by the gate.
    static REFUSED_SESSION: Cell<bool>;
}

/// Runs `future` with a cell that [`BoundedSessions`] sets when it refuses
/// a session inside it, and returns the output with whether it did. The
/// SDK awaits `create_session` inline in the request's task, so the cell
/// is visible there; a call made from another task leaves it unset.
pub async fn scope_refusals<F: std::future::Future>(future: F) -> (F::Output, bool) {
    REFUSED_SESSION
        .scope(Cell::new(false), async {
            let output = future.await;
            (output, REFUSED_SESSION.with(Cell::get))
        })
        .await
}

/// Why a session operation failed.
#[derive(Debug)]
pub enum BoundedError<E> {
    /// The bound was reached; no session was created.
    LimitReached,
    /// The wrapped manager failed; its error, unchanged.
    Delegate(E),
}

impl<E: fmt::Display> fmt::Display for BoundedError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            // The SDK would put this text in a 500 body: fixed, and
            // revealing nothing.
            BoundedError::LimitReached => f.write_str("session limit reached"),
            BoundedError::Delegate(error) => error.fmt(f),
        }
    }
}

impl<E: std::error::Error + 'static> std::error::Error for BoundedError<E> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            BoundedError::LimitReached => None,
            BoundedError::Delegate(error) => error.source(),
        }
    }
}

#[derive(Debug, Default)]
struct Count {
    live: HashSet<SessionId>,
    pending: usize,
}

/// A session manager that holds at most `max` live sessions.
#[derive(Debug)]
pub struct BoundedSessions<M> {
    inner: M,
    max: usize,
    count: Mutex<Count>,
}

impl<M> BoundedSessions<M> {
    /// Wraps `inner` with a bound of `max` sessions.
    pub fn new(inner: M, max: usize) -> Self {
        BoundedSessions {
            inner,
            max,
            count: Mutex::new(Count::default()),
        }
    }

    /// The number of sessions counted: live ones plus creations in flight.
    /// Read-only, for tests.
    #[cfg(test)]
    pub fn live(&self) -> usize {
        let count = self.lock();
        count.live.len() + count.pending
    }

    /// The lock, recovered if a thread panicked while holding it: both
    /// fields stay consistent after a panic in these few lines.
    fn lock(&self) -> MutexGuard<'_, Count> {
        self.count.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// A creation in flight. Dropping it before the creation finished (the
/// request was cancelled or timed out) gives the slot back.
struct Reservation<'a, M> {
    sessions: &'a BoundedSessions<M>,
    armed: bool,
}

impl<M> Drop for Reservation<'_, M> {
    fn drop(&mut self) {
        if self.armed {
            let mut count = self.sessions.lock();
            count.pending = count.pending.saturating_sub(1);
        }
    }
}

impl<M: SessionManager> SessionManager for BoundedSessions<M> {
    type Error = BoundedError<M::Error>;
    type Transport = M::Transport;

    async fn create_session(&self) -> Result<(SessionId, Self::Transport), Self::Error> {
        {
            let mut count = self.lock();
            if count.live.len() + count.pending >= self.max {
                drop(count);
                let _ = REFUSED_SESSION.try_with(|cell| cell.set(true));
                return Err(BoundedError::LimitReached);
            }
            count.pending += 1;
        }
        let mut reservation = Reservation {
            sessions: self,
            armed: true,
        };
        let created = self.inner.create_session().await;
        // Nothing awaits from here: the reservation becomes a live session
        // (or nothing) in one step.
        let mut count = self.lock();
        count.pending = count.pending.saturating_sub(1);
        reservation.armed = false;
        match created {
            Ok((id, transport)) => {
                count.live.insert(id.clone());
                Ok((id, transport))
            }
            Err(error) => Err(BoundedError::Delegate(error)),
        }
    }

    async fn initialize_session(
        &self,
        id: &SessionId,
        message: ClientJsonRpcMessage,
    ) -> Result<ServerJsonRpcMessage, Self::Error> {
        self.inner
            .initialize_session(id, message)
            .await
            .map_err(BoundedError::Delegate)
    }

    async fn has_session(&self, id: &SessionId) -> Result<bool, Self::Error> {
        self.inner
            .has_session(id)
            .await
            .map_err(BoundedError::Delegate)
    }

    async fn close_session(&self, id: &SessionId) -> Result<(), Self::Error> {
        // Gone from the count first; the wrapped manager's close is
        // idempotent, and a second close of the same id changes nothing.
        self.lock().live.remove(id);
        self.inner
            .close_session(id)
            .await
            .map_err(BoundedError::Delegate)
    }

    async fn create_stream(
        &self,
        id: &SessionId,
        message: ClientJsonRpcMessage,
    ) -> Result<impl Stream<Item = ServerSseMessage> + Send + Sync + 'static, Self::Error> {
        self.inner
            .create_stream(id, message)
            .await
            .map_err(BoundedError::Delegate)
    }

    async fn accept_message(
        &self,
        id: &SessionId,
        message: ClientJsonRpcMessage,
    ) -> Result<(), Self::Error> {
        self.inner
            .accept_message(id, message)
            .await
            .map_err(BoundedError::Delegate)
    }

    async fn create_standalone_stream(
        &self,
        id: &SessionId,
    ) -> Result<impl Stream<Item = ServerSseMessage> + Send + Sync + 'static, Self::Error> {
        self.inner
            .create_standalone_stream(id)
            .await
            .map_err(BoundedError::Delegate)
    }

    async fn resume(
        &self,
        id: &SessionId,
        last_event_id: String,
    ) -> Result<impl Stream<Item = ServerSseMessage> + Send + Sync + 'static, Self::Error> {
        self.inner
            .resume(id, last_event_id)
            .await
            .map_err(BoundedError::Delegate)
    }

    /// Never restores. The SDK calls this only when a session store is
    /// configured, and rypsok configures none; a restore would also create
    /// a session outside the count, so it is refused here whatever the
    /// wrapped manager supports.
    async fn restore_session(
        &self,
        _id: SessionId,
    ) -> Result<RestoreOutcome<Self::Transport>, Self::Error> {
        Ok(RestoreOutcome::NotSupported)
    }

    fn event_store(&self) -> Option<Arc<dyn EventStore>> {
        self.inner.event_store()
    }
}

#[cfg(test)]
mod tests;
