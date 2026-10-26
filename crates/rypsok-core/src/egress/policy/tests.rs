//! Tests of the agent-directed policy (REQ-TEST-005). Every test names the
//! requirement it verifies. The policy under test is the production
//! policy; nothing is relaxed for tests.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use proptest::prelude::*;
use url::Url;

use super::*;

fn policy() -> EgressPolicy {
    EgressPolicy::new(Vec::new(), Vec::new(), false, vec![80, 443])
}

fn http_policy() -> EgressPolicy {
    EgressPolicy::new(Vec::new(), Vec::new(), true, vec![80, 443])
}

fn name(host: &str) -> Host {
    Host::Name(host.to_owned())
}

/// REQ-SEC-006 and REQ-TEST-005: every refused range, in both families,
/// refuses an address inside it; well-known public addresses are admitted.
#[test]
fn req_sec_006_every_refused_range_refuses_and_public_addresses_pass() {
    let p = policy();
    for (cidr, basis) in REFUSED_RANGES {
        let net: IpNet = cidr.parse().unwrap();
        let inside = net.hosts().next().unwrap_or(net.addr());
        assert_eq!(
            p.judge_address(&name("example.org"), inside),
            Err(Denial::Address),
            "{cidr} ({basis}) must refuse {inside}"
        );
        assert_eq!(
            p.judge_address(&name("example.org"), net.addr()),
            Err(Denial::Address),
            "{cidr} ({basis}) must refuse its network address"
        );
    }
    for public in [
        "93.184.216.34",
        "8.8.8.8",
        "2606:4700::1111",
        "2a00:1450:4001:80b::200e",
    ] {
        let ip: IpAddr = public.parse().unwrap();
        assert_eq!(
            p.judge_address(&name("example.org"), ip),
            Ok(()),
            "{public}"
        );
    }
    assert_eq!(
        p.judge_address(&name("x.example"), IpAddr::V4(Ipv4Addr::BROADCAST)),
        Err(Denial::Address)
    );
    assert_eq!(REFUSED_RANGES.len(), 26, "the table is the one of RFC 011");
}

/// REQ-SEC-006 and REQ-TEST-005: an IPv4-mapped IPv6 address is judged as
/// the IPv4 address it maps, in both directions.
#[test]
fn req_sec_006_ipv4_mapped_addresses_are_canonicalized() {
    let p = policy();
    let mapped_loopback: IpAddr = "::ffff:127.0.0.1".parse().unwrap();
    assert_eq!(canonical(mapped_loopback), IpAddr::V4(Ipv4Addr::LOCALHOST));
    assert_eq!(
        p.judge_address(&name("a.example"), mapped_loopback),
        Err(Denial::Address)
    );
    let mapped_public: IpAddr = "::ffff:93.184.216.34".parse().unwrap();
    assert_eq!(p.judge_address(&name("a.example"), mapped_public), Ok(()));
    let mapped_private = IpAddr::V6(Ipv6Addr::new(0, 0, 0, 0, 0, 0xffff, 0x0a00, 0x0001));
    assert_eq!(
        p.judge_address(&name("a.example"), mapped_private),
        Err(Denial::Address)
    );
}

/// REQ-TEST-005: every address-literal notation the URL parser accepts
/// (decimal, octal, hexadecimal, short forms, bracketed IPv6, mapped) is
/// canonical before judgment and refused when it names loopback or a
/// metadata address.
#[test]
fn req_test_005_address_literal_notations_are_judged_canonically() {
    let p = http_policy();
    for literal in [
        "http://2130706433/",
        "http://0177.0.0.1/",
        "http://0x7f000001/",
        "http://0x7f.0.0.1/",
        "http://127.1/",
        "http://127.0.1/",
        "http://[::1]/",
        "http://[::ffff:127.0.0.1]/",
        "http://[::ffff:7f00:1]/",
        "http://0/",
        "http://169.254.169.254/latest/meta-data/",
        "http://[fd00:ec2::254]/",
        "http://100.100.100.200/",
        "http://[64:ff9b::7f00:1]/",
    ] {
        let url = Url::parse(literal).unwrap();
        assert_eq!(p.judge_url(&url), Err(Denial::Address), "{literal}");
    }
    let public = Url::parse("http://93.184.216.34/").unwrap();
    let target = p.judge_url(&public).unwrap();
    assert_eq!(target.host, Host::Ip("93.184.216.34".parse().unwrap()));
    assert_eq!(target.port, 80);
}

/// REQ-FETCH-007 and REQ-SEC-009: credentials in the URL, a scheme other
/// than https (or http when allowed), and a port outside LIM-F8 are
/// refused before any I/O, each with its own denial.
#[test]
fn req_sec_009_scheme_port_and_credentials_are_judged_before_io() {
    let p = policy();
    let judge = |u: &str| p.judge_url(&Url::parse(u).unwrap());
    assert_eq!(
        judge("https://user:pw@example.org/"),
        Err(Denial::CredentialInUrl)
    );
    assert_eq!(
        judge("https://user@example.org/"),
        Err(Denial::CredentialInUrl)
    );
    assert_eq!(judge("http://example.org/"), Err(Denial::Scheme));
    assert_eq!(judge("ftp://example.org/"), Err(Denial::Scheme));
    assert_eq!(judge("file:///etc/passwd"), Err(Denial::Scheme));
    assert_eq!(judge("https://example.org:8443/"), Err(Denial::Port));
    assert_eq!(judge("https://example.org:443/x").unwrap().port, 443);
    assert_eq!(
        http_policy()
            .judge_url(&Url::parse("http://example.org/").unwrap())
            .unwrap()
            .port,
        80
    );
    let gopher = EgressPolicy::new(Vec::new(), Vec::new(), true, vec![80, 443, 8080]);
    assert_eq!(
        gopher
            .judge_url(&Url::parse("http://example.org:8080/").unwrap())
            .unwrap()
            .port,
        8080
    );
    assert_eq!(Denial::Port.reason(), DetailReason::Egress);
    assert_eq!(Denial::Scheme.reason(), DetailReason::Scheme);
    assert_eq!(
        Denial::CredentialInUrl.reason(),
        DetailReason::CredentialInUrl
    );
}

/// REQ-SEC-006: names that never name a public site are refused by name,
/// before resolution: `localhost`, single labels, `.local`, `.internal`,
/// `.localhost`, `.arpa`.
#[test]
fn req_sec_006_names_that_never_name_a_public_site_are_refused() {
    let p = policy();
    for host in [
        "localhost",
        "LOCALHOST.",
        "intranet",
        "printer.local",
        "db.internal",
        "app.localhost",
        "1.0.0.127.in-addr.arpa",
        "metadata.google.internal",
    ] {
        let url = Url::parse(&format!("https://{host}/")).unwrap();
        assert_eq!(p.judge_url(&url), Err(Denial::HostName), "{host}");
    }
    let ok = p
        .judge_url(&Url::parse("https://Example.ORG/path").unwrap())
        .unwrap();
    assert_eq!(ok.host, Host::Name("example.org".into()));
}

/// REQ-SEC-006: operator rules apply in the order deny, allow, built-in:
/// a deny rule refuses even an allowed network; an allow rule admits a
/// built-in refused range (a private mirror) and a name by pattern.
#[test]
fn req_sec_006_deny_wins_over_allow_wins_over_builtin() {
    let p = EgressPolicy::new(
        vec![
            parse_rule("10.0.0.0/8").unwrap(),
            parse_rule("*.corp.example").unwrap(),
            parse_rule("203.0.113.7").unwrap(),
        ],
        vec![
            parse_rule("10.1.0.0/16").unwrap(),
            parse_rule("evil.corp.example").unwrap(),
        ],
        false,
        vec![443],
    );
    let ip = |s: &str| -> IpAddr { s.parse().unwrap() };
    assert_eq!(p.judge_address(&name("a.example"), ip("10.2.0.1")), Ok(()));
    assert_eq!(
        p.judge_address(&name("a.example"), ip("10.1.0.1")),
        Err(Denial::Address)
    );
    assert_eq!(
        p.judge_address(&name("a.example"), ip("192.168.0.1")),
        Err(Denial::Address)
    );
    assert_eq!(
        p.judge_address(&name("a.example"), ip("203.0.113.7")),
        Ok(())
    );
    assert_eq!(
        p.judge_address(&name("a.example"), ip("203.0.113.8")),
        Err(Denial::Address)
    );
    // A host pattern admits the name and every address it resolves to.
    assert_eq!(
        p.judge_url(&Url::parse("https://wiki.corp.example/").unwrap())
            .map(|t| t.host),
        Ok(Host::Name("wiki.corp.example".into()))
    );
    assert_eq!(
        p.judge_address(&name("wiki.corp.example"), ip("172.16.5.5")),
        Ok(())
    );
    assert!(p.allowed_by_rule(&name("wiki.corp.example")));
    assert!(
        !p.allowed_by_rule(&name("corp.example")),
        "`*.` excludes the name itself"
    );
    assert_eq!(
        p.judge_url(&Url::parse("https://evil.corp.example/").unwrap()),
        Err(Denial::HostName)
    );
    assert_eq!(
        p.judge_address(&name("evil.corp.example"), ip("93.184.216.34")),
        Err(Denial::HostName)
    );
    // A literal admitted by rule.
    let literal = p
        .judge_url(&Url::parse("https://10.2.0.9/").unwrap())
        .unwrap();
    assert_eq!(literal.host, Host::Ip(ip("10.2.0.9")));
}

/// REQ-CFG-005: rule texts are validated with the key and the index in
/// the message; valid forms are a CIDR, an address and a host pattern.
#[test]
fn req_cfg_005_rule_texts_are_validated_by_key_and_index() {
    let problems = EgressPolicy::from_texts(
        &[
            "10.0.0.0/8".into(),
            "not a rule!".into(),
            "*.ok.example".into(),
        ],
        &["999.1.1.1".into()],
        false,
        &[443],
    )
    .unwrap_err();
    assert_eq!(
        problems,
        vec![
            "egress.allow[1]: not a network in CIDR notation, an address or a host pattern",
            "egress.deny[0]: not a network in CIDR notation, an address or a host pattern",
        ]
    );
    assert!(parse_rule("fd00::/8").is_ok());
    assert!(parse_rule("2001:db8::1").is_ok());
    assert!(parse_rule("Wiki.Corp.Example").is_ok());
    assert!(parse_rule("*.corp.example").is_ok());
    for bad in [
        "",
        "https://x.example",
        "x.example:443",
        "-bad.example",
        "*.",
        "a..b",
    ] {
        assert_eq!(parse_rule(bad), Err(InvalidRule), "{bad:?}");
    }
}

proptest! {
    /// REQ-SEC-006: every IPv4 address the policy admits is one the
    /// standard library classifies as neither private, loopback,
    /// link-local, multicast, broadcast, unspecified nor documentation.
    #[test]
    fn req_sec_006_admitted_ipv4_is_global_by_std(bits in any::<u32>()) {
        let ip = Ipv4Addr::from(bits);
        if policy().judge_address(&name("a.example"), IpAddr::V4(ip)).is_ok() {
            prop_assert!(!ip.is_private());
            prop_assert!(!ip.is_loopback());
            prop_assert!(!ip.is_link_local());
            prop_assert!(!ip.is_multicast());
            prop_assert!(!ip.is_broadcast());
            prop_assert!(!ip.is_unspecified());
            prop_assert!(!ip.is_documentation());
        }
    }

    /// REQ-SEC-006: every IPv6 address the policy admits is neither
    /// loopback, unspecified, multicast, unique-local nor link-local, and
    /// not an IPv4-mapped address of a refused IPv4 range.
    #[test]
    fn req_sec_006_admitted_ipv6_is_global_by_std(bits in any::<u128>()) {
        let ip = Ipv6Addr::from(bits);
        if policy().judge_address(&name("a.example"), IpAddr::V6(ip)).is_ok() {
            prop_assert!(!ip.is_loopback());
            prop_assert!(!ip.is_unspecified());
            prop_assert!(!ip.is_multicast());
            prop_assert!(!ip.is_unique_local());
            prop_assert!(!ip.is_unicast_link_local());
            if let Some(v4) = ip.to_ipv4_mapped() {
                prop_assert!(!v4.is_private() && !v4.is_loopback() && !v4.is_link_local());
            }
        }
    }
}
