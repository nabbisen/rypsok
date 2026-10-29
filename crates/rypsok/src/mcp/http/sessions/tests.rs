//! Tests of the session bound (REQ-SEC-024, RFC 019 D5): the wrapper over
//! the SDK's real in-memory manager, and over a manager that never
//! finishes creating a session. Every test names the requirement it
//! verifies.

use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;

use super::*;

type Local = BoundedSessions<LocalSessionManager>;

fn bounded(max: usize) -> Arc<Local> {
    Arc::new(BoundedSessions::new(LocalSessionManager::default(), max))
}

/// REQ-SEC-024: the SDK closes one session from two places (its `DELETE`
/// handler and the worker's exit) and the two can run at once. However
/// often one id is closed, the count equals the live sessions: after each
/// round exactly one more session can be created and the next is refused.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn req_sec_024_a_session_closed_twice_is_counted_once() {
    let sessions = bounded(3);
    let mut ids = Vec::new();
    for _ in 0..3 {
        ids.push(sessions.create_session().await.unwrap().0);
    }
    assert!(matches!(
        sessions.create_session().await,
        Err(BoundedError::LimitReached)
    ));
    for round in 0..200 {
        let victim = ids.remove(round % ids.len());
        let (a, b) = (Arc::clone(&sessions), Arc::clone(&sessions));
        let (id_a, id_b) = (victim.clone(), victim.clone());
        let first = tokio::spawn(async move { a.close_session(&id_a).await });
        let second = tokio::spawn(async move { b.close_session(&id_b).await });
        first.await.unwrap().unwrap();
        second.await.unwrap().unwrap();
        assert_eq!(
            sessions.live(),
            2,
            "round {round}: closed twice, counted once"
        );
        ids.push(
            sessions
                .create_session()
                .await
                .unwrap_or_else(|_| panic!("round {round}: one slot is free"))
                .0,
        );
        assert_eq!(sessions.live(), 3, "round {round}");
        assert!(
            matches!(
                sessions.create_session().await,
                Err(BoundedError::LimitReached)
            ),
            "round {round}: and only one"
        );
    }
    // An id that never existed changes nothing.
    sessions
        .close_session(&SessionId::from("no-such-session"))
        .await
        .unwrap();
    assert_eq!(sessions.live(), 3);
}

/// REQ-SEC-024: the refusal's text is a fixed sentence, because the SDK
/// puts the error's text in the body of its 500 when the gate's cell is
/// not set; and the cell is set exactly when the wrapper refuses inside
/// the gate's scope. If an SDK change moved `create_session` out of the
/// request's task, the cell would stay unset and this pins what a client
/// would then read.
#[tokio::test]
async fn req_sec_024_the_fallback_is_a_fixed_500() {
    let sessions = bounded(0);
    let error = sessions.create_session().await.map(|_| ()).unwrap_err();
    assert_eq!(error.to_string(), "session limit reached");
    assert!(std::error::Error::source(&error).is_none());

    // Outside the scope: no cell, the refusal still works.
    assert!(matches!(error, BoundedError::LimitReached));
    // Inside the scope: the cell says it was a refusal.
    let (result, refused) = scope_refusals(sessions.create_session()).await;
    assert!(result.is_err() && refused);
    // A creation that succeeds leaves the cell unset.
    let roomy = bounded(1);
    let (result, refused) = scope_refusals(roomy.create_session()).await;
    assert!(result.is_ok() && !refused);
    // A task spawned from inside the scope does not see it.
    let sessions = bounded(0);
    let (inner, refused) = scope_refusals(async move {
        tokio::spawn(async move { sessions.create_session().await.is_err() })
            .await
            .unwrap()
    })
    .await;
    assert!(inner && !refused, "a spawned task has no cell");
}

/// A manager whose `create_session` never finishes.
struct Stalled;

struct NoStream;

impl Stream for NoStream {
    type Item = ServerSseMessage;
    fn poll_next(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Poll::Ready(None)
    }
}

impl SessionManager for Stalled {
    type Error = std::io::Error;
    type Transport = <LocalSessionManager as SessionManager>::Transport;

    async fn create_session(&self) -> Result<(SessionId, Self::Transport), Self::Error> {
        std::future::pending().await
    }
    async fn initialize_session(
        &self,
        _: &SessionId,
        _: ClientJsonRpcMessage,
    ) -> Result<ServerJsonRpcMessage, Self::Error> {
        Err(std::io::Error::other("unused"))
    }
    async fn has_session(&self, _: &SessionId) -> Result<bool, Self::Error> {
        Ok(false)
    }
    async fn close_session(&self, _: &SessionId) -> Result<(), Self::Error> {
        Ok(())
    }
    async fn create_stream(
        &self,
        _: &SessionId,
        _: ClientJsonRpcMessage,
    ) -> Result<impl Stream<Item = ServerSseMessage> + Send + Sync + 'static, Self::Error> {
        Ok(NoStream)
    }
    async fn accept_message(
        &self,
        _: &SessionId,
        _: ClientJsonRpcMessage,
    ) -> Result<(), Self::Error> {
        Ok(())
    }
    async fn create_standalone_stream(
        &self,
        _: &SessionId,
    ) -> Result<impl Stream<Item = ServerSseMessage> + Send + Sync + 'static, Self::Error> {
        Ok(NoStream)
    }
    async fn resume(
        &self,
        _: &SessionId,
        _: String,
    ) -> Result<impl Stream<Item = ServerSseMessage> + Send + Sync + 'static, Self::Error> {
        Ok(NoStream)
    }
}

/// REQ-SEC-024: a creation that is cancelled (the request timed out, the
/// client went away) gives its slot back; a bound that leaked a slot per
/// cancelled request would turn a slow client into a permanent refusal.
#[tokio::test]
async fn req_sec_024_a_cancelled_creation_gives_the_slot_back() {
    let sessions = BoundedSessions::new(Stalled, 2);
    for round in 0..10 {
        let waited =
            tokio::time::timeout(Duration::from_millis(5), sessions.create_session()).await;
        assert!(waited.is_err(), "round {round}: the creation stalls");
        assert_eq!(sessions.live(), 0, "round {round}: the slot is back");
    }
    // Two creations in flight fill the bound; a third is refused at once;
    // dropping them frees both.
    let mut first = Box::pin(sessions.create_session());
    let mut second = Box::pin(sessions.create_session());
    assert!(poll_once(first.as_mut()).is_pending());
    assert!(poll_once(second.as_mut()).is_pending());
    assert_eq!(sessions.live(), 2);
    assert!(matches!(
        sessions.create_session().await,
        Err(BoundedError::LimitReached)
    ));
    drop(first);
    assert_eq!(sessions.live(), 1, "dropping one frees one");
    drop(second);
    assert_eq!(sessions.live(), 0);
}

/// Polls a future once with a waker that does nothing.
fn poll_once<F: std::future::Future>(future: Pin<&mut F>) -> Poll<F::Output> {
    let waker = std::task::Waker::noop();
    future.poll(&mut Context::from_waker(waker))
}

/// REQ-SEC-024: a restore never creates a session outside the count.
#[tokio::test]
async fn req_sec_024_restore_never_creates_a_session() {
    let sessions = bounded(1);
    let outcome = sessions
        .restore_session(SessionId::from("restored"))
        .await
        .unwrap();
    assert!(matches!(outcome, RestoreOutcome::NotSupported));
    assert_eq!(sessions.live(), 0);
    assert!(
        !sessions
            .has_session(&SessionId::from("restored"))
            .await
            .unwrap()
    );
}
