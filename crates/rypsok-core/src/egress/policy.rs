//! The agent-directed egress policy (RFC 011; REQ-SEC-006, REQ-SEC-009,
//! REQ-FETCH-007, REQ-FETCH-010): may this URL be requested, and may this
//! address be connected to.
//!
//! Built once from configuration. `judge_url` runs before any I/O;
//! `judge_address` runs at connect time inside the client's resolver
//! (`super::resolver`), so that the address judged is the address dialed
//! (REQ-SEC-007). No caller parameter reaches the policy.

use std::net::{IpAddr, Ipv4Addr};
use std::sync::LazyLock;

use ipnet::IpNet;
use url::Url;

use crate::model::DetailReason;

/// A destination host as the URL names it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Host {
    /// A name, lower-cased, to be resolved.
    Name(String),
    /// An address literal; every notation the URL parser accepts ends
    /// here in canonical form.
    Ip(IpAddr),
}

impl Host {
    /// The name, when the host is one.
    pub fn name(&self) -> Option<&str> {
        match self {
            Host::Name(name) => Some(name),
            Host::Ip(_) => None,
        }
    }
}

/// An operator rule of `[egress].allow` or `[egress].deny`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Rule {
    /// A network in CIDR notation, or a single address.
    Network(IpNet),
    /// A host name, exact or `*.suffix` for its subdomains.
    Host(HostPattern),
}

/// A host pattern: `name` matches that name; `*.name` matches any
/// subdomain of `name`, not `name` itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostPattern {
    subdomains: bool,
    name: String,
}

impl HostPattern {
    /// Whether the pattern matches a lower-cased name.
    pub fn matches(&self, name: &str) -> bool {
        if self.subdomains {
            name.len() > self.name.len()
                && name.ends_with(&self.name)
                && name.as_bytes()[name.len() - self.name.len() - 1] == b'.'
        } else {
            name == self.name
        }
    }
}

/// Why a rule text is not a rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidRule;

/// Parses one entry of `[egress].allow` or `[egress].deny`: a CIDR
/// (`10.0.0.0/8`, `fd00::/8`), a single address, or a host pattern
/// (`wiki.corp.example`, `*.corp.example`); never a scheme or a port.
pub fn parse_rule(text: &str) -> Result<Rule, InvalidRule> {
    let text = text.trim();
    if text.is_empty() {
        return Err(InvalidRule);
    }
    if text.contains('/') {
        return text
            .parse::<IpNet>()
            .map(Rule::Network)
            .map_err(|_| InvalidRule);
    }
    if let Ok(ip) = text.parse::<IpAddr>() {
        return Ok(Rule::Network(IpNet::from(ip)));
    }
    let (subdomains, name) = match text.strip_prefix("*.") {
        Some(rest) => (true, rest),
        None => (false, text),
    };
    let name = name.to_ascii_lowercase();
    let label_ok = |label: &str| {
        !label.is_empty()
            && label
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-')
            && !label.starts_with('-')
            && !label.ends_with('-')
    };
    // The last label of a name is never all digits: such a text is a
    // malformed address, not a host.
    let numeric_tail = name
        .rsplit('.')
        .next()
        .is_some_and(|label| label.bytes().all(|b| b.is_ascii_digit()));
    if name.contains(':') || !name.split('.').all(label_ok) || name.len() > 253 || numeric_tail {
        return Err(InvalidRule);
    }
    Ok(Rule::Host(HostPattern { subdomains, name }))
}

/// Why the policy refused. A closed vocabulary: never a URL, a name or an
/// address.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Denial {
    /// The scheme is not `https` (nor `http` when allowed).
    Scheme,
    /// The URL carries userinfo (REQ-FETCH-007).
    CredentialInUrl,
    /// The port is not one of LIM-F8.
    Port,
    /// The address is not globally routable, or an operator deny rule
    /// matched it.
    Address,
    /// The host name is one that never names a public site, or an operator
    /// deny rule matched it, or the URL has no host.
    HostName,
}

impl Denial {
    /// The name for logs.
    pub fn as_str(self) -> &'static str {
        match self {
            Denial::Scheme => "scheme",
            Denial::CredentialInUrl => "credential_in_url",
            Denial::Port => "port",
            Denial::Address => "address",
            Denial::HostName => "host_name",
        }
    }

    /// The reason of the `denied` error (external design §4).
    pub fn reason(self) -> DetailReason {
        match self {
            Denial::Scheme => DetailReason::Scheme,
            Denial::CredentialInUrl => DetailReason::CredentialInUrl,
            Denial::Port | Denial::Address | Denial::HostName => DetailReason::Egress,
        }
    }
}

/// A URL the policy admitted before I/O; its host still passes
/// `judge_address` at connect time unless it is an address literal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    /// The URL to request.
    pub url: Url,
    /// The host.
    pub host: Host,
    /// The port, explicit or the scheme's default.
    pub port: u16,
}

/// The policy of the agent-directed profile.
#[derive(Debug, Clone)]
pub struct EgressPolicy {
    allow: Vec<Rule>,
    deny: Vec<Rule>,
    allow_http: bool,
    ports: Vec<u16>,
}

/// Ranges that are never globally routable unicast (REQ-SEC-006). One
/// line per range with the document that defines it; tests iterate this
/// table.
pub const REFUSED_RANGES: &[(&str, &str)] = &[
    ("0.0.0.0/8", "this network (RFC 791)"),
    ("10.0.0.0/8", "private (RFC 1918)"),
    (
        "100.64.0.0/10",
        "carrier-grade NAT (RFC 6598); includes Alibaba metadata 100.100.100.200",
    ),
    ("127.0.0.0/8", "loopback (RFC 1122)"),
    (
        "169.254.0.0/16",
        "link-local (RFC 3927); includes cloud metadata 169.254.169.254",
    ),
    ("172.16.0.0/12", "private (RFC 1918)"),
    ("192.0.0.0/24", "IETF protocol assignments (RFC 6890)"),
    ("192.0.2.0/24", "documentation TEST-NET-1 (RFC 5737)"),
    ("192.88.99.0/24", "6to4 relay anycast (RFC 7526)"),
    ("192.168.0.0/16", "private (RFC 1918)"),
    ("198.18.0.0/15", "benchmarking (RFC 2544)"),
    ("198.51.100.0/24", "documentation TEST-NET-2 (RFC 5737)"),
    ("203.0.113.0/24", "documentation TEST-NET-3 (RFC 5737)"),
    ("224.0.0.0/4", "multicast (RFC 5771)"),
    ("240.0.0.0/4", "reserved and broadcast (RFC 1112)"),
    ("::/128", "unspecified (RFC 4291)"),
    ("::1/128", "loopback (RFC 4291)"),
    (
        "::ffff:0:0/96",
        "IPv4-mapped (RFC 4291); judged as IPv4 after canonicalization",
    ),
    ("64:ff9b::/96", "NAT64 well-known prefix (RFC 6052)"),
    ("64:ff9b:1::/48", "NAT64 local-use prefix (RFC 8215)"),
    ("100::/64", "discard-only (RFC 6666)"),
    (
        "2001::/23",
        "IETF protocol assignments (RFC 2928): Teredo, benchmarking, documentation 2001:db8::/32, ORCHID",
    ),
    ("2002::/16", "6to4 (RFC 3056)"),
    (
        "fc00::/7",
        "unique local (RFC 4193); includes AWS metadata fd00:ec2::254",
    ),
    ("fe80::/10", "link-local (RFC 4291)"),
    ("ff00::/8", "multicast (RFC 4291)"),
];

static REFUSED: LazyLock<Vec<IpNet>> = LazyLock::new(|| {
    REFUSED_RANGES
        .iter()
        .map(|(cidr, _)| cidr.parse().expect("the refused-range table parses"))
        .collect()
});

/// Name suffixes that resolve inside the host, never on the public
/// Internet (RFC 011).
const REFUSED_NAME_SUFFIXES: &[&str] = &[".localhost", ".local", ".internal", ".arpa"];

/// An IPv4-mapped IPv6 address is the IPv4 address it maps.
pub fn canonical(ip: IpAddr) -> IpAddr {
    match ip {
        IpAddr::V6(v6) => match v6.to_ipv4_mapped() {
            Some(v4) => IpAddr::V4(v4),
            None => ip,
        },
        IpAddr::V4(_) => ip,
    }
}

impl EgressPolicy {
    /// A policy from parsed rules.
    pub fn new(allow: Vec<Rule>, deny: Vec<Rule>, allow_http: bool, ports: Vec<u16>) -> Self {
        EgressPolicy {
            allow,
            deny,
            allow_http,
            ports,
        }
    }

    /// A policy from the configuration's texts. Every bad entry is
    /// reported with its key and index, as validation does.
    pub fn from_texts(
        allow: &[String],
        deny: &[String],
        allow_http: bool,
        ports: &[u16],
    ) -> Result<Self, Vec<String>> {
        let mut problems = Vec::new();
        let parse = |key: &str, texts: &[String], problems: &mut Vec<String>| -> Vec<Rule> {
            texts
                .iter()
                .enumerate()
                .filter_map(|(i, text)| match parse_rule(text) {
                    Ok(rule) => Some(rule),
                    Err(InvalidRule) => {
                        problems.push(format!(
                            "egress.{key}[{i}]: not a network in CIDR notation, an address or a host pattern"
                        ));
                        None
                    }
                })
                .collect()
        };
        let allow = parse("allow", allow, &mut problems);
        let deny = parse("deny", deny, &mut problems);
        if problems.is_empty() {
            Ok(EgressPolicy::new(allow, deny, allow_http, ports.to_vec()))
        } else {
            Err(problems)
        }
    }

    /// Whether plain HTTP is allowed (`[egress].allow_http`).
    pub fn allows_http(&self) -> bool {
        self.allow_http
    }

    /// Whether an operator allow rule matches the host name or address
    /// (strict mode admits such URLs without a prior search result).
    pub fn allowed_by_rule(&self, host: &Host) -> bool {
        self.allow.iter().any(|rule| rule_matches(rule, host, None))
    }

    /// Judges a URL before any I/O (REQ-SEC-009, REQ-FETCH-007). An
    /// address literal is judged here; a name is judged again at connect
    /// time, address by address.
    pub fn judge_url(&self, url: &Url) -> Result<Target, Denial> {
        match url.scheme() {
            "https" => {}
            "http" if self.allow_http => {}
            _ => return Err(Denial::Scheme),
        }
        if !url.username().is_empty() || url.password().is_some() {
            return Err(Denial::CredentialInUrl);
        }
        let port = url.port_or_known_default().ok_or(Denial::Port)?;
        if !self.ports.contains(&port) {
            return Err(Denial::Port);
        }
        let host = match url.host() {
            None => return Err(Denial::HostName),
            Some(url::Host::Ipv4(v4)) => Host::Ip(IpAddr::V4(v4)),
            Some(url::Host::Ipv6(v6)) => Host::Ip(IpAddr::V6(v6)),
            Some(url::Host::Domain(name)) => Host::Name(name.to_ascii_lowercase()),
        };
        match &host {
            Host::Ip(ip) => self.judge_address(&host, *ip)?,
            Host::Name(name) => self.judge_name(name)?,
        }
        Ok(Target {
            url: url.clone(),
            host,
            port,
        })
    }

    /// Judges a host name: deny rules, allow rules, then the built-in
    /// refusals of names that never name a public site.
    fn judge_name(&self, name: &str) -> Result<(), Denial> {
        let host = Host::Name(name.to_owned());
        if self.deny.iter().any(|r| rule_matches(r, &host, None)) {
            return Err(Denial::HostName);
        }
        if self.allow.iter().any(|r| rule_matches(r, &host, None)) {
            return Ok(());
        }
        let bare = name.strip_suffix('.').unwrap_or(name);
        if bare.is_empty()
            || bare == "localhost"
            || !bare.contains('.')
            || REFUSED_NAME_SUFFIXES.iter().any(|s| bare.ends_with(s))
        {
            return Err(Denial::HostName);
        }
        Ok(())
    }

    /// Judges one address that is about to be connected to for `host`
    /// (REQ-SEC-006): operator deny rules, operator allow rules, the
    /// built-in refused ranges, then admitted.
    pub fn judge_address(&self, host: &Host, ip: IpAddr) -> Result<(), Denial> {
        let ip = canonical(ip);
        if self.deny.iter().any(|r| rule_matches(r, host, Some(ip))) {
            return Err(match host {
                Host::Name(_)
                    if !self
                        .deny
                        .iter()
                        .any(|r| matches!(r, Rule::Network(n) if n.contains(&ip))) =>
                {
                    Denial::HostName
                }
                _ => Denial::Address,
            });
        }
        if self.allow.iter().any(|r| rule_matches(r, host, Some(ip))) {
            return Ok(());
        }
        if REFUSED.iter().any(|net| net.contains(&ip)) || ip == IpAddr::V4(Ipv4Addr::BROADCAST) {
            return Err(Denial::Address);
        }
        Ok(())
    }
}

/// Whether a rule matches a host (by name) or an address (by network).
fn rule_matches(rule: &Rule, host: &Host, ip: Option<IpAddr>) -> bool {
    match (rule, host, ip) {
        (Rule::Host(pattern), Host::Name(name), _) => pattern.matches(name),
        (Rule::Network(net), _, Some(ip)) => net.contains(&ip),
        (Rule::Network(net), Host::Ip(ip), None) => net.contains(&canonical(*ip)),
        _ => false,
    }
}

#[cfg(test)]
mod tests;
