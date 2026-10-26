//! Connect-time enforcement (RFC 011; REQ-SEC-007): the HTTP client's own
//! resolver judges every address a name resolves to, so that the address
//! judged is the address dialed. An address literal never reaches the
//! resolver; [`super::policy::EgressPolicy::judge_url`] judged it.

use std::future::Future;
use std::net::{IpAddr, SocketAddr};
use std::pin::Pin;
use std::sync::Arc;

use reqwest::dns::{Addrs, Name, Resolve, Resolving};

use super::policy::{Denial, EgressPolicy, Host};

/// A name lookup: the system resolver in production, a script in tests.
pub trait Lookup: Send + Sync + 'static {
    /// The addresses of a name, in the resolver's order.
    fn lookup(
        &self,
        name: String,
    ) -> Pin<Box<dyn Future<Output = std::io::Result<Vec<IpAddr>>> + Send>>;
}

/// The system resolver through Tokio's blocking pool.
#[derive(Debug, Default, Clone, Copy)]
pub struct SystemLookup;

impl Lookup for SystemLookup {
    fn lookup(
        &self,
        name: String,
    ) -> Pin<Box<dyn Future<Output = std::io::Result<Vec<IpAddr>>> + Send>> {
        Box::pin(async move {
            let addrs = tokio::net::lookup_host((name.as_str(), 0)).await?;
            Ok(addrs.map(|sa| sa.ip()).collect())
        })
    }
}

/// Why a name yielded no address to connect to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refused {
    /// Every address the name resolved to was refused by the policy.
    Policy(Denial),
    /// The name resolved to nothing.
    NoAddress,
}

impl std::fmt::Display for Refused {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Refused::Policy(denial) => write!(
                f,
                "egress policy refused every address ({})",
                denial.as_str()
            ),
            Refused::NoAddress => f.write_str("the name resolved to no address"),
        }
    }
}

impl std::error::Error for Refused {}

/// Keeps the addresses the policy admits for `name`, in order; an error
/// when none remains (REQ-SEC-006 applied to every address).
pub fn admitted(
    policy: &EgressPolicy,
    name: &str,
    ips: &[IpAddr],
) -> Result<Vec<SocketAddr>, Refused> {
    let host = Host::Name(name.to_ascii_lowercase());
    let mut last = None;
    let kept: Vec<SocketAddr> = ips
        .iter()
        .filter(|ip| match policy.judge_address(&host, **ip) {
            Ok(()) => true,
            Err(denial) => {
                last = Some(denial);
                tracing::debug!(
                    event = "address_refused",
                    denial = denial.as_str(),
                    "an address the name resolved to was refused"
                );
                false
            }
        })
        .map(|ip| SocketAddr::new(*ip, 0))
        .collect();
    if kept.is_empty() {
        Err(last.map_or(Refused::NoAddress, Refused::Policy))
    } else {
        Ok(kept)
    }
}

/// The resolver the agent client uses: lookup, then judgment of every
/// address. Nothing is cached, so a changed DNS answer is judged again.
pub struct JudgingResolver<L: Lookup = SystemLookup> {
    policy: Arc<EgressPolicy>,
    lookup: L,
}

impl<L: Lookup> std::fmt::Debug for JudgingResolver<L> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JudgingResolver").finish_non_exhaustive()
    }
}

impl JudgingResolver<SystemLookup> {
    /// A resolver over the system lookup.
    pub fn new(policy: Arc<EgressPolicy>) -> Self {
        JudgingResolver {
            policy,
            lookup: SystemLookup,
        }
    }
}

impl<L: Lookup> JudgingResolver<L> {
    /// A resolver over any lookup (tests script the answers).
    pub fn with_lookup(policy: Arc<EgressPolicy>, lookup: L) -> Self {
        JudgingResolver { policy, lookup }
    }
}

impl<L: Lookup> Resolve for JudgingResolver<L> {
    fn resolve(&self, name: Name) -> Resolving {
        let policy = Arc::clone(&self.policy);
        let name = name.as_str().to_owned();
        let lookup = self.lookup.lookup(name.clone());
        Box::pin(async move {
            let ips = lookup.await?;
            let addrs = admitted(&policy, &name, &ips)?;
            Ok(Box::new(addrs.into_iter()) as Addrs)
        })
    }
}

#[cfg(test)]
mod tests;
