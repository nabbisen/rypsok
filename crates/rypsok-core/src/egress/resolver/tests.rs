//! Tests of connect-time enforcement (REQ-SEC-007, REQ-TEST-005). Every
//! test names the requirement it verifies.

use std::net::{IpAddr, Ipv4Addr};
use std::str::FromStr;
use std::sync::Mutex;

use super::*;
use crate::egress::policy::parse_rule;

/// A lookup whose answers are scripted call by call; the last answer
/// repeats.
struct Scripted(Mutex<Vec<Vec<IpAddr>>>);

impl Lookup for Scripted {
    fn lookup(
        &self,
        _name: String,
    ) -> Pin<Box<dyn Future<Output = std::io::Result<Vec<IpAddr>>> + Send>> {
        let mut answers = self.0.lock().unwrap();
        let answer = if answers.len() > 1 {
            answers.remove(0)
        } else {
            answers.first().cloned().unwrap_or_default()
        };
        Box::pin(async move { Ok(answer) })
    }
}

fn ip(s: &str) -> IpAddr {
    s.parse().unwrap()
}

fn policy() -> Arc<EgressPolicy> {
    Arc::new(EgressPolicy::new(Vec::new(), Vec::new(), false, vec![443]))
}

async fn resolve(resolver: &impl Resolve, name: &str) -> Result<Vec<SocketAddr>, String> {
    resolver
        .resolve(Name::from_str(name).unwrap())
        .await
        .map(|addrs| addrs.collect())
        .map_err(|e| e.to_string())
}

/// REQ-SEC-007 and REQ-TEST-005: a DNS answer that changes between two
/// resolutions (rebinding) is judged again; the refused answer yields no
/// address, so no connection can reach it.
#[tokio::test]
async fn req_sec_007_rebinding_between_resolutions_is_judged_again() {
    let resolver = JudgingResolver::with_lookup(
        policy(),
        Scripted(Mutex::new(vec![
            vec![ip("93.184.216.34")],
            vec![ip("127.0.0.1")],
            vec![ip("169.254.169.254")],
        ])),
    );
    let first = resolve(&resolver, "rebind.example").await.unwrap();
    assert_eq!(first, vec![SocketAddr::new(ip("93.184.216.34"), 0)]);
    let second = resolve(&resolver, "rebind.example").await.unwrap_err();
    assert!(
        second.contains("egress policy refused every address (address)"),
        "{second}"
    );
    let third = resolve(&resolver, "rebind.example").await.unwrap_err();
    assert!(third.contains("refused"), "{third}");
}

/// REQ-SEC-006 and REQ-TEST-005: a mixed answer keeps only the admitted
/// addresses, in the resolver's order; a name that resolves to nothing is
/// an error of its own.
#[tokio::test]
async fn req_test_005_mixed_answers_keep_only_admitted_addresses() {
    let resolver = JudgingResolver::with_lookup(
        policy(),
        Scripted(Mutex::new(vec![
            vec![
                ip("10.0.0.5"),
                ip("2606:4700::1111"),
                ip("::ffff:127.0.0.1"),
                ip("93.184.216.34"),
            ],
            vec![],
        ])),
    );
    let kept = resolve(&resolver, "mixed.example").await.unwrap();
    assert_eq!(
        kept,
        vec![
            SocketAddr::new(ip("2606:4700::1111"), 0),
            SocketAddr::new(ip("93.184.216.34"), 0)
        ]
    );
    assert_eq!(
        resolve(&resolver, "empty.example").await.unwrap_err(),
        "the name resolved to no address"
    );
}

/// REQ-SEC-006: an operator allow rule admits what the built-in table
/// refuses, at connect time as well; a deny rule by name refuses every
/// address of that name.
#[tokio::test]
async fn req_sec_006_rules_apply_at_connect_time() {
    let policy = Arc::new(EgressPolicy::new(
        vec![parse_rule("127.0.0.1/32").unwrap()],
        vec![parse_rule("blocked.example").unwrap()],
        false,
        vec![443],
    ));
    let resolver = JudgingResolver::with_lookup(
        Arc::clone(&policy),
        Scripted(Mutex::new(vec![vec![
            IpAddr::V4(Ipv4Addr::LOCALHOST),
            ip("127.0.0.2"),
        ]])),
    );
    let kept = resolve(&resolver, "mirror.example").await.unwrap();
    assert_eq!(
        kept,
        vec![SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0)]
    );
    let refused = admitted(&policy, "blocked.example", &[ip("93.184.216.34")]).unwrap_err();
    assert_eq!(refused, Refused::Policy(Denial::HostName));
}

/// REQ-SEC-006 and REQ-SEC-007 (RFC 019 D1): the resolver's name meets the
/// operator's rules in its canonical form, and a malformed name yields no
/// address.
#[test]
fn req_sec_006_resolver_names_are_canonical() {
    let denying = EgressPolicy::new(
        Vec::new(),
        vec![parse_rule("blocked.invalid").unwrap()],
        false,
        vec![443],
    );
    let public = [ip("93.184.216.34")];
    for name in ["blocked.invalid", "blocked.invalid.", "BLOCKED.Invalid."] {
        assert_eq!(
            admitted(&denying, name, &public),
            Err(Refused::Policy(Denial::HostName)),
            "{name}"
        );
    }
    for name in ["blocked.invalid..", "a..example", ".example"] {
        assert_eq!(
            admitted(&policy(), name, &public),
            Err(Refused::Policy(Denial::HostName)),
            "{name} is malformed"
        );
    }
    assert!(admitted(&policy(), "fine.example.", &public).is_ok());
}
