//! The egress policy's judgment of any URL (RFC 011, RFC 019 D1; RFC 020,
//! TST-14). A URL the policy admits is `https` (or `http` when plain HTTP is
//! allowed), has no userinfo, has a port of the policy's set, and names a
//! host that the policy admits again in its canonical form.
#![no_main]
use std::net::IpAddr;

use libfuzzer_sys::fuzz_target;
use rypsok_core::egress::{EgressPolicy, Host};
use url::Url;

fuzz_target!(|data: &[u8]| {
    let Some((&flags, rest)) = data.split_first() else { return };
    let Ok(text) = std::str::from_utf8(rest) else { return };
    let Ok(url) = Url::parse(text) else { return };
    let allow_http = flags & 1 == 1;
    let policy = EgressPolicy::new(Vec::new(), Vec::new(), allow_http, vec![80, 443]);
    let Ok(target) = policy.judge_url(&url) else { return };

    let scheme = target.url.scheme();
    assert!(scheme == "https" || (scheme == "http" && allow_http), "{scheme}");
    assert!(target.url.username().is_empty() && target.url.password().is_none());
    assert!([80, 443].contains(&target.port), "{}", target.port);

    // The canonical form of the host is judged again, and the verdict is the
    // same: admitted.
    let host_text = match &target.host {
        Host::Name(name) => name.clone(),
        Host::Ip(IpAddr::V4(v4)) => v4.to_string(),
        Host::Ip(IpAddr::V6(v6)) => format!("[{v6}]"),
    };
    if let Host::Ip(ip) = &target.host {
        assert!(policy.judge_address(&target.host, *ip).is_ok(), "{ip}");
    }
    let again = Url::parse(&format!("{scheme}://{host_text}:{}/", target.port))
        .expect("a canonical host is a URL host");
    assert!(
        policy.judge_url(&again).is_ok(),
        "{url} was admitted, its canonical form {again} is not"
    );
});
