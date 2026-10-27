//! The Streamable HTTP transport (RFC 013; REQ-MCP-003, REQ-SEC-018,
//! REQ-SEC-019): a loopback listener by default, the SDK's tower service
//! behind a gate that rypsok owns. The gate accepts one path, bounds the
//! request body, authenticates the caller with a static bearer token and
//! attaches the caller's identity to the request; the SDK validates `Host`
//! and `Origin`, serves the 2026-07-28 revision statelessly and keeps the
//! handshake-based revisions in memory until the operator disables them.
//! Nothing of HTTP reaches `rypsok-core`.

use std::convert::Infallible;
use std::future::Future;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::Duration;

use bytes::Bytes;
use http::{HeaderValue, Request, Response, StatusCode, header};
use http_body_util::combinators::BoxBody;
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper_util::rt::{TokioExecutor, TokioIo, TokioTimer};
use hyper_util::server::conn::auto;
use hyper_util::server::graceful::GracefulShutdown;
use hyper_util::service::TowerToHyperService;
use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;
use rmcp::transport::streamable_http_server::session::never::NeverSessionManager;
use rmcp::transport::{StreamableHttpServerConfig, StreamableHttpService};
use rypsok_core::config::{Config, ConfigError, Secret};
use rypsok_core::limits::defaults::SHUTDOWN_MAX_MS;
use tokio::net::TcpListener;
use tokio::sync::Semaphore;
use tokio_util::sync::CancellationToken;
use tower_service::Service;

use super::Adapter;
use super::rate::CallerId;

/// The one path the listener serves.
pub const PATH: &str = "/mcp";

/// The shortest token accepted, in bytes (RFC 013).
pub const TOKEN_MIN_BYTES: usize = 32;

/// How long an HTTP/2 keep-alive ping may go unanswered before the
/// connection is closed (LIM-H3 on HTTP/2).
const H2_PING_TIMEOUT: Duration = Duration::from_secs(10);

/// One authenticated caller: an identifier and its token by reference.
#[derive(Debug, Clone)]
pub struct Caller {
    /// The identifier the limits and the logs use.
    pub id: CallerId,
    /// The token; never logged.
    pub token: Secret<String>,
}

/// What the listener needs (RFC 013).
#[derive(Debug, Clone)]
pub struct HttpSettings {
    /// `[service.http].bind`.
    pub bind: SocketAddr,
    /// `Host` values accepted; loopback names and the bind host when the
    /// operator listed none.
    pub allowed_hosts: Vec<String>,
    /// `Origin` values accepted; every present `Origin` is refused when
    /// the operator listed none.
    pub allowed_origins: Vec<String>,
    /// Serve the handshake-based revisions with sessions in memory.
    pub legacy_revisions: bool,
    /// The callers; empty on loopback means every request is `local`.
    pub callers: Vec<Caller>,
    /// LIM-H1.
    pub max_connections: u32,
    /// LIM-H2.
    pub max_request_bytes: u64,
    /// LIM-H3.
    pub idle_timeout: Duration,
}

impl HttpSettings {
    /// Reads the settings of a validated configuration and resolves the
    /// callers' tokens; a token that cannot be read or is too short is a
    /// configuration error naming the key.
    pub fn from_config(config: &Config) -> Result<Self, ConfigError> {
        let http = &config.service.http;
        let bind: SocketAddr = http.bind.parse().map_err(|_| {
            ConfigError::new(vec![
                "service.http.bind: must be an address and a port".to_owned(),
            ])
        })?;
        let mut problems = Vec::new();
        let mut callers = Vec::new();
        for (i, caller) in http.auth.callers.iter().enumerate() {
            let key = format!("service.http.auth.callers[{i}]");
            let Some(source) = caller.secret_source() else {
                problems.push(format!("{key}: token_env or token_file is required"));
                continue;
            };
            match source.resolve() {
                Ok((token, warning)) => {
                    if let Some(warning) = warning {
                        tracing::warn!(event = "secret_file_permissions", caller = %caller.id, "{warning}");
                    }
                    if token.expose_secret().len() < TOKEN_MIN_BYTES {
                        problems.push(format!(
                            "{key}: the token must be at least {TOKEN_MIN_BYTES} bytes"
                        ));
                        continue;
                    }
                    callers.push(Caller {
                        id: CallerId::new(&caller.id),
                        token,
                    });
                }
                Err(error) => problems.push(format!("{key}: {error}")),
            }
        }
        if !problems.is_empty() {
            return Err(ConfigError::new(problems));
        }
        let allowed_hosts = if http.allowed_hosts.is_empty() {
            let mut hosts = vec![
                "localhost".to_owned(),
                "127.0.0.1".to_owned(),
                "::1".to_owned(),
            ];
            // An unspecified address is never a `Host` a client sends;
            // validation requires `allowed_hosts` for such a bind.
            let bind_host = bind.ip().to_string();
            if !bind.ip().is_unspecified() && !hosts.contains(&bind_host) {
                hosts.push(bind_host);
            }
            hosts
        } else {
            http.allowed_hosts.clone()
        };
        Ok(HttpSettings {
            bind,
            allowed_hosts,
            allowed_origins: http.allowed_origins.clone(),
            legacy_revisions: http.legacy_revisions,
            callers,
            max_connections: http.max_connections,
            max_request_bytes: http.max_request_bytes,
            idle_timeout: Duration::from_secs(u64::from(http.idle_timeout_s)),
        })
    }
}

/// Binds the listener; the ephemeral port of tests is read back from it.
pub async fn bind(settings: &HttpSettings) -> std::io::Result<TcpListener> {
    TcpListener::bind(settings.bind).await
}

/// Serves until the shutdown token is cancelled.
pub async fn serve(
    adapter: Adapter,
    settings: HttpSettings,
    shutdown: CancellationToken,
) -> std::io::Result<()> {
    let listener = bind(&settings).await?;
    run(adapter, listener, settings, shutdown).await
}

/// Serves on a bound listener until the shutdown token is cancelled, then
/// drains the open connections within LIM-O1. The handshake-based
/// revisions get sessions in memory when `legacy_revisions` is on; the
/// 2026-07-28 revision is stateless either way.
pub async fn run(
    adapter: Adapter,
    listener: TcpListener,
    settings: HttpSettings,
    shutdown: CancellationToken,
) -> std::io::Result<()> {
    let local = listener.local_addr()?;
    tracing::info!(
        event = "listening",
        address = %local,
        callers = settings.callers.len(),
        legacy_revisions = settings.legacy_revisions,
        "listening"
    );
    let mut config = StreamableHttpServerConfig::default()
        .with_allowed_hosts(settings.allowed_hosts.clone())
        .with_legacy_session_mode(settings.legacy_revisions)
        .with_json_response(true)
        .with_max_request_body_bytes(
            usize::try_from(settings.max_request_bytes).unwrap_or(usize::MAX),
        );
    config = if settings.allowed_origins.is_empty() {
        config.enforce_origin_validation()
    } else {
        config.with_allowed_origins(settings.allowed_origins.clone())
    };
    config.cancellation_token = shutdown.child_token();
    // With legacy revisions off, the adapter advertises the 2026-07-28
    // revision alone: `server/discover` lists one revision and the SDK
    // answers `initialize` with `UNSUPPORTED_PROTOCOL_VERSION` (RFC 013
    // amendment of 2026-10-27). Over stdio the adapter is unchanged.
    let adapter = if settings.legacy_revisions {
        adapter
    } else {
        adapter.without_legacy_revisions()
    };
    let inner = if settings.legacy_revisions {
        let factory_adapter = adapter;
        Inner::Local(StreamableHttpService::new(
            move || Ok(factory_adapter.clone()),
            Arc::new(LocalSessionManager::default()),
            config,
        ))
    } else {
        let factory_adapter = adapter;
        Inner::Never(StreamableHttpService::new(
            move || Ok(factory_adapter.clone()),
            Arc::new(NeverSessionManager::default()),
            config,
        ))
    };
    let gate = Gate {
        inner,
        auth: Arc::new(Auth {
            callers: settings.callers,
        }),
        max_request_bytes: settings.max_request_bytes,
    };
    let permits = Arc::new(Semaphore::new(
        usize::try_from(settings.max_connections)
            .unwrap_or(usize::MAX)
            .max(1),
    ));
    let graceful = GracefulShutdown::new();
    let mut builder = auto::Builder::new(TokioExecutor::new());
    // LIM-H3: on HTTP/1 the time a connection may sit between requests;
    // on HTTP/2 a keep-alive ping after the same idle time, and the
    // connection is closed when the ping goes unanswered, so that a dead
    // peer frees its LIM-H1 permit.
    builder
        .http1()
        .timer(TokioTimer::new())
        .header_read_timeout(settings.idle_timeout);
    builder
        .http2()
        .timer(TokioTimer::new())
        .keep_alive_interval(settings.idle_timeout)
        .keep_alive_timeout(H2_PING_TIMEOUT);
    loop {
        let (stream, remote) = tokio::select! {
            _ = shutdown.cancelled() => break,
            accepted = listener.accept() => match accepted {
                Ok(accepted) => accepted,
                Err(error) => {
                    tracing::warn!(event = "accept_failed", error = %error, "accept failed");
                    continue;
                }
            },
        };
        let Ok(permit) = Arc::clone(&permits).try_acquire_owned() else {
            tracing::warn!(
                event = "http_refused",
                reason = "too_many_connections",
                family = address_family(&remote),
                "connection refused: LIM-H1"
            );
            drop(stream);
            continue;
        };
        let connection = builder
            .serve_connection_with_upgrades(
                TokioIo::new(stream),
                TowerToHyperService::new(gate.clone()),
            )
            .into_owned();
        let connection = graceful.watch(connection);
        tokio::spawn(async move {
            let _permit = permit;
            if let Err(error) = connection.await {
                tracing::debug!(event = "connection_ended", error = %error, "connection ended with an error");
            }
        });
    }
    drop(listener);
    let _ = tokio::time::timeout(
        Duration::from_millis(u64::from(SHUTDOWN_MAX_MS)),
        graceful.shutdown(),
    )
    .await;
    tracing::info!(event = "listener_closed", "listener closed");
    Ok(())
}

fn address_family(remote: &SocketAddr) -> &'static str {
    if remote.is_ipv4() { "ipv4" } else { "ipv6" }
}

type GateResponse = Response<BoxBody<Bytes, Infallible>>;
type GateFuture = Pin<Box<dyn Future<Output = Result<GateResponse, Infallible>> + Send>>;

#[derive(Clone)]
enum Inner {
    Local(StreamableHttpService<Adapter, LocalSessionManager>),
    Never(StreamableHttpService<Adapter, NeverSessionManager>),
}

/// The layer in front of the SDK: path, body bound, authentication,
/// caller identity.
#[derive(Clone)]
struct Gate {
    inner: Inner,
    auth: Arc<Auth>,
    max_request_bytes: u64,
}

impl Service<Request<Incoming>> for Gate {
    type Response = GateResponse;
    type Error = Infallible;
    type Future = GateFuture;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Infallible>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Request<Incoming>) -> GateFuture {
        let (mut parts, body) = req.into_parts();
        if parts.uri.path() != PATH {
            return ready(empty(StatusCode::NOT_FOUND));
        }
        let declared = parts
            .headers
            .get(header::CONTENT_LENGTH)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<u64>().ok());
        if declared.is_some_and(|len| len > self.max_request_bytes) {
            tracing::warn!(
                event = "http_refused",
                reason = "body_too_large",
                "request refused: LIM-H2"
            );
            return ready(empty(StatusCode::PAYLOAD_TOO_LARGE));
        }
        let caller = match self.auth.check(&parts.headers) {
            Ok(caller) => caller,
            Err(response) => return ready(*response),
        };
        parts.extensions.insert(caller);
        let req = Request::from_parts(parts, body);
        let inner = self.inner.clone();
        Box::pin(async move {
            match inner {
                Inner::Local(mut service) => service.call(req).await,
                Inner::Never(mut service) => service.call(req).await,
            }
        })
    }
}

fn ready(response: GateResponse) -> GateFuture {
    Box::pin(std::future::ready(Ok(response)))
}

fn empty(status: StatusCode) -> GateResponse {
    Response::builder()
        .status(status)
        .body(Full::new(Bytes::new()).boxed())
        .expect("a status and an empty body build")
}

/// Static bearer tokens (D-27).
struct Auth {
    callers: Vec<Caller>,
}

impl Auth {
    /// The caller of a request: `local` when no caller is configured;
    /// otherwise the one whose token matches in constant time, or a 401.
    fn check(&self, headers: &http::HeaderMap) -> Result<CallerId, Box<GateResponse>> {
        if self.callers.is_empty() {
            return Ok(CallerId::local());
        }
        let presented = headers
            .get(header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| {
                v.strip_prefix("Bearer ")
                    .or_else(|| v.strip_prefix("bearer "))
            })
            .map(str::trim)
            .unwrap_or("");
        let mut found = None;
        for caller in &self.callers {
            // Every caller is compared, whatever the outcome, so the time
            // does not depend on which token matched.
            if constant_time_eq(
                caller.token.expose_secret().as_bytes(),
                presented.as_bytes(),
            ) {
                found = found.or(Some(caller.id.clone()));
            }
        }
        match found {
            Some(id) => Ok(id),
            None => {
                tracing::warn!(
                    event = "http_refused",
                    reason = "unauthorized",
                    "request refused"
                );
                let mut response = empty(StatusCode::UNAUTHORIZED);
                response.headers_mut().insert(
                    header::WWW_AUTHENTICATE,
                    HeaderValue::from_static("Bearer realm=\"rypsok\""),
                );
                Err(Box::new(response))
            }
        }
    }
}

/// Equality in time that depends on the longer length only, never on the
/// position of the first difference.
pub fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    let mut difference = (a.len() ^ b.len()) as u8;
    for i in 0..a.len().max(b.len()) {
        let x = a.get(i).copied().unwrap_or(0);
        let y = b.get(i).copied().unwrap_or(0);
        difference |= x ^ y;
    }
    difference == 0
}

#[cfg(test)]
mod tests;
