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
}

/// The special-purpose registries of the IANA as test data (RFC 020 D-2): a
/// committed file, not derived from `REFUSED_RANGES`.
const SPECIAL_PURPOSE: &str = include_str!("special_purpose.csv");

/// Blocks that the registry calls globally reachable and the table refuses
/// all the same, because they sit inside a larger refused block (stricter
/// than the registry, on purpose): `192.0.0.0/24` and `2001::/23` are
/// refused whole, and `64:ff9b::/96` (NAT64) is refused whole because the
/// address it carries is another host's.
const STRICTER_THAN_THE_REGISTRY: &[&str] = &[
    "192.0.0.9/32",
    "192.0.0.10/32",
    "64:ff9b::/96",
    "2001:1::1/128",
    "2001:1::2/128",
    "2001:1::3/128",
    "2001:3::/32",
    "2001:4:112::/48",
    "2001:20::/28",
    "2001:30::/28",
];

/// The block judged by the IPv4 address it maps, not refused as a block.
const JUDGED_AS_IPV4: &str = "::ffff:0:0/96";

/// (block, globally_reachable) of every row of the data file.
fn special_purpose() -> Vec<(IpNet, String)> {
    SPECIAL_PURPOSE
        .lines()
        .filter(|line| !line.starts_with('#') && !line.trim().is_empty())
        .map(|line| {
            let fields: Vec<&str> = line.split(',').collect();
            assert_eq!(
                fields.len(),
                4,
                "block,name,globally_reachable,date: {line}"
            );
            let block: IpNet = fields[0].parse().unwrap_or_else(|_| panic!("{line}"));
            (block, fields[2].to_owned())
        })
        .collect()
}

/// The first, an inner and the last address of a block.
fn probes(block: &IpNet) -> [IpAddr; 3] {
    let (first, last) = (block.network(), block.broadcast());
    let inner = match (first, last) {
        (IpAddr::V4(a), IpAddr::V4(b)) => IpAddr::V4(Ipv4Addr::from(
            u32::from(a) + (u32::from(b) - u32::from(a)) / 2,
        )),
        (IpAddr::V6(a), IpAddr::V6(b)) => IpAddr::V6(Ipv6Addr::from(
            u128::from(a) + (u128::from(b) - u128::from(a)) / 2,
        )),
        _ => unreachable!("a network has one family"),
    };
    [first, inner, last]
}

/// REQ-SEC-006 (RFC 020 D-2, NET-4): every block of the IANA special-purpose
/// registries whose "Globally Reachable" is not true, and the two deprecated
/// forms, is refused at its first, an inner and its last address; a block the
/// registry calls globally reachable is admitted, except the ones named as
/// stricter. No packet is sent: the judgment is a lookup in a table.
#[test]
fn req_sec_006_every_special_purpose_block_is_refused() {
    let p = policy();
    let rows = special_purpose();
    assert!(
        rows.len() >= 50,
        "{} rows: the data file is whole",
        rows.len()
    );
    for needed in [
        "2001:db8::/32",
        "3fff::/20",
        "5f00::/16",
        "100:0:0:1::/64",
        "::/96",
        "fec0::/10",
    ] {
        let needed: IpNet = needed.parse().unwrap();
        assert!(
            rows.iter().any(|(b, _)| *b == needed),
            "{needed} is in the data"
        );
    }
    let mut failures: Vec<String> = Vec::new();
    for (block, reachable) in &rows {
        let stricter = STRICTER_THAN_THE_REGISTRY.contains(&block.to_string().as_str());
        for address in probes(block) {
            let judged = p.judge_address(&name("example.org"), address);
            let expected = if block.to_string() == JUDGED_AS_IPV4 {
                // Judged as the IPv4 address it maps (the first and the last
                // map to 0.0.0.0 and 255.255.255.255, which are refused).
                p.judge_address(&name("example.org"), canonical(address))
            } else if reachable == "true" && !stricter {
                Ok(())
            } else {
                Err(Denial::Address)
            };
            if judged != expected {
                failures.push(format!(
                    "{address} in {block} ({reachable}): judged {judged:?}, expected {expected:?}"
                ));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{} wrong:\n{}",
        failures.len(),
        failures.join("\n")
    );
    for entry in STRICTER_THAN_THE_REGISTRY {
        let entry: IpNet = entry.parse().unwrap();
        assert!(
            rows.iter().any(|(b, r)| *b == entry && r == "true"),
            "{entry} is a registry block marked globally reachable"
        );
    }
    let mapped = |text: &str| p.judge_address(&name("example.org"), text.parse().unwrap());
    assert_eq!(mapped("::ffff:10.0.0.1"), Err(Denial::Address));
    assert_eq!(mapped("::ffff:93.184.216.34"), Ok(()));
}

/// REQ-SEC-006 (RFC 020 D-2): the addresses the audit named.
#[test]
fn req_sec_006_the_addresses_of_the_audit_are_refused() {
    let p = policy();
    for literal in [
        "::7f00:1",
        "fec0::1",
        "2001:db8::1",
        "3fff::1",
        "5f00::1",
        "100:0:0:1::1",
    ] {
        let ip: IpAddr = literal.parse().unwrap();
        assert_eq!(
            p.judge_address(&name("example.org"), ip),
            Err(Denial::Address),
            "{literal}"
        );
        let url = Url::parse(&format!("http://[{literal}]/")).unwrap();
        assert_eq!(
            http_policy().judge_url(&url),
            Err(Denial::Address),
            "{literal}"
        );
    }
    for public in [
        "2001:4860:4860::8888",
        "2606:4700::1111",
        "2620:4f:8000::1",
        "192.31.196.1",
        "192.52.193.1",
        "192.175.48.1",
    ] {
        let ip: IpAddr = public.parse().unwrap();
        assert_eq!(
            p.judge_address(&name("example.org"), ip),
            Ok(()),
            "{public}"
        );
    }
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

/// REQ-SEC-006 (RFC 019 D1): an operator rule matches the canonical form
/// of a name; a root dot or capitals do not get past it, and a malformed
/// name is refused as malformed, never normalized into a match.
#[test]
fn req_sec_006_rules_match_the_canonical_name() {
    let p = EgressPolicy::new(
        Vec::new(),
        vec![
            parse_rule("blocked.invalid").unwrap(),
            parse_rule("*.ads.invalid").unwrap(),
        ],
        false,
        vec![443],
    );
    for host in [
        "blocked.invalid",
        "blocked.invalid.",
        "BLOCKED.INVALID.",
        "x.ads.invalid",
        "x.ads.invalid.",
        "X.Ads.Invalid.",
    ] {
        let url = Url::parse(&format!("https://{host}/")).unwrap();
        assert_eq!(p.judge_url(&url), Err(Denial::HostName), "{host}");
    }
    // The resolver's name and a directly built `Host` are canonicalized
    // by `judge_address` as well.
    let public: IpAddr = "93.184.216.34".parse().unwrap();
    for host in ["blocked.invalid", "blocked.invalid.", "BLOCKED.invalid."] {
        assert_eq!(
            p.judge_address(&Host::Name(host.to_owned()), public),
            Err(Denial::HostName),
            "{host}"
        );
    }
    // Malformed names are refused whatever the rules say.
    for host in [
        "blocked.invalid..",
        "blocked.invalid...",
        "a..b",
        ".a.example",
    ] {
        let url = Url::parse(&format!("https://{host}/")).unwrap();
        assert_eq!(p.judge_url(&url), Err(Denial::HostName), "{host}");
        assert_eq!(
            p.judge_address(&Host::Name(host.to_owned()), public),
            Err(Denial::HostName),
            "{host}"
        );
    }
    // `HostName` is also the answer to a matching rule; only an empty deny
    // list proves that a malformed name is refused as malformed.
    let open = EgressPolicy::new(
        vec![parse_rule("blocked.invalid").unwrap()],
        Vec::new(),
        false,
        vec![443],
    );
    for host in [
        "blocked.invalid..",
        "blocked.invalid...",
        "a..b.example",
        ".a.example",
    ] {
        assert_eq!(open.judge_name(host), Err(Denial::HostName), "{host}");
    }
    assert!(open.judge_name("blocked.invalid.").is_ok(), "an allow rule");
}

/// REQ-SEC-006 and REQ-CFG-005 (RFC 019 D1): a rule is written without the
/// root dot or a leading dot; one with them is a configuration error, so
/// that rules and names meet at one canonical form.
#[test]
fn req_sec_006_a_rule_with_a_root_dot_is_a_configuration_error() {
    assert_eq!(parse_rule("blocked.invalid."), Err(InvalidRule));
    assert_eq!(parse_rule(".blocked.invalid"), Err(InvalidRule));
    assert_eq!(parse_rule("*.blocked.invalid."), Err(InvalidRule));
    assert!(parse_rule("blocked.invalid").is_ok());
    assert!(parse_rule("BLOCKED.Invalid").is_ok(), "lower-cased");
}

/// REQ-SEC-006 (RFC 019 D1): one constructor, one canonical form.
#[test]
fn req_sec_006_host_name_is_the_canonical_constructor() {
    assert_eq!(
        Host::from_name("Example.ORG."),
        Ok(Host::Name("example.org".to_owned()))
    );
    assert_eq!(Host::from_name("a."), Ok(Host::Name("a".to_owned())));
    for bad in ["", ".", "..", "a..", "a..b", ".a"] {
        assert_eq!(Host::from_name(bad), Err(Denial::HostName), "{bad:?}");
    }
    assert_eq!(
        Host::from_name("x.example").unwrap().name(),
        Some("x.example")
    );
    assert_eq!(Host::Ip("10.0.0.1".parse().unwrap()).name(), None);
}

proptest! {
    /// REQ-SEC-006 (RFC 019 D1): for a well-formed name, capitals and an
    /// added root dot do not change the judgment; a name with an empty
    /// label is always refused.
    #[test]
    fn req_sec_006_judgment_of_a_name_is_the_judgment_of_its_canonical_form(
        host in "[a-z0-9]{1,8}(\\.[a-z0-9]{1,8}){1,3}",
        deny_it in any::<bool>(),
    ) {
        // A name whose last label is all digits is not a rule (it is a
        // malformed address); then nothing is denied by rule.
        let rule = if deny_it { parse_rule(&host).ok() } else { None };
        let denied = rule.is_some();
        let deny = rule.into_iter().collect();
        let p = EgressPolicy::new(Vec::new(), deny, false, vec![443]);
        let plain = p.judge_name(&host);
        prop_assert_eq!(&plain, &p.judge_name(&host.to_uppercase()));
        prop_assert_eq!(&plain, &p.judge_name(&format!("{host}.")));
        prop_assert_eq!(&plain, &p.judge_name(&format!("{}.", host.to_uppercase())));
        let built_in = [".local", ".internal", ".arpa", ".localhost"]
            .iter()
            .any(|suffix| host.ends_with(suffix));
        prop_assert_eq!(plain.is_err(), denied || built_in);
        // An empty label is refused, however the rest is written.
        prop_assert_eq!(p.judge_name(&format!("{host}..")), Err(Denial::HostName));
        prop_assert_eq!(p.judge_name(&format!(".{host}")), Err(Denial::HostName));
        prop_assert_eq!(p.judge_name(&host.replacen('.', "..", 1)), Err(Denial::HostName));
    }

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

/// REQ-SEC-006 (RFC 020 D-6): which `allow` rules `check` calls wide: a
/// network with a prefix shorter than /16 (IPv4) or /32 (IPv6); not a host
/// name, a single address or a narrower network.
#[test]
fn req_sec_006_wide_allow_rules_are_those_wider_than_16_and_32() {
    for wide in [
        "0.0.0.0/0",
        "10.0.0.0/8",
        "172.16.0.0/12",
        "192.168.0.0/15",
        "::/0",
        "fc00::/7",
        "2001:db8::/31",
    ] {
        assert!(is_wide_network_rule(wide), "{wide}");
    }
    for narrow in [
        "192.168.0.0/16",
        "192.168.1.0/24",
        "10.1.2.3",
        "10.1.2.3/32",
        "2001:db8::/32",
        "2001:db8::1",
        "docs.rs",
        "*.example.org",
        "not a rule",
        "",
    ] {
        assert!(!is_wide_network_rule(narrow), "{narrow}");
    }
}

/// REQ-SEC-023 (RFC 020, TST-14): a rule written back as text is read as the
/// same rule, for a network, an address, a name and a subdomain pattern (the
/// round trip the `parse_rule` fuzz target checks on any text).
#[test]
fn req_sec_023_a_parsed_rule_round_trips_through_its_text() {
    for text in [
        "10.0.0.0/8",
        "fd00::/8",
        "203.0.113.7",
        "2001:db8::1",
        "wiki.corp.example",
        "*.corp.example",
        "  Mixed.Case.Example  ",
    ] {
        let rule = parse_rule(text).unwrap();
        assert_eq!(parse_rule(&rule.to_string()), Ok(rule.clone()), "{text}");
    }
    assert_eq!(
        parse_rule("*.Corp.Example").unwrap().to_string(),
        "*.corp.example"
    );
}
