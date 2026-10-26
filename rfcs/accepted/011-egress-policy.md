# RFC 011 — Egress policy and SSRF defense: the agent-directed profile

**Status.** Accepted (2026-10-25) — implementer may start after M2 closes
(release 0.2.0).
**Tracks.** Milestone M3; requirements REQ-SEC-005 (agent-directed half),
REQ-SEC-006, REQ-SEC-007, REQ-SEC-008, REQ-SEC-009, REQ-SEC-021,
REQ-FETCH-007, REQ-FETCH-010, REQ-RES-008, REQ-TEST-005, REQ-DEP-003;
external design §8, §16 (I1, I3); threats THR-05, THR-06, THR-07, THR-09,
THR-14; decisions D-09, D-18, D-34.
**Touches.** `rypsok-core::egress` (new `policy`, `resolver`, `strict`
modules and the agent client), `rypsok-core::config` (validation of
`[egress]` and `fetch.ports`), the limits table (two rows), the operator
documentation.
**Handoff.** [`../handoffs/011-egress-policy/README.md`](../handoffs/011-egress-policy/README.md)

## Summary

One policy object, built once from configuration, answers two questions
and nothing else: may this URL be requested, and may this address be
connected to. The second question is asked by the HTTP client's own
resolver at connect time, so the address that was judged is the address
that is dialed, for every hop. Redirects are never followed by the client;
the fetch pipeline (RFC 012) asks the policy again for every target. A
strict mode narrows destinations to what rypsok itself returned. No caller
parameter reaches the policy.

## Design

### The policy (REQ-SEC-006, REQ-SEC-009)

```rust
pub struct EgressPolicy {
    allow: Vec<Rule>,          // [egress].allow, operator
    deny: Vec<Rule>,           // [egress].deny, operator
    allow_http: bool,          // [egress].allow_http
    ports: Vec<u16>,           // [fetch].ports (LIM-F8)
}

pub enum Rule { Network(IpNet), Host(HostPattern) }   // "10.0.0.0/8", "wiki.corp.example", "*.corp.example"

pub enum Denial { Scheme, CredentialInUrl, Port, Address, HostName }

pub struct Target { pub url: Url, pub host: Host, pub port: u16 }  // host: Name(String) | Ip(IpAddr)

impl EgressPolicy {
    pub fn judge_url(&self, url: &Url) -> Result<Target, Denial>;   // before any I/O
    pub fn judge_address(&self, host: &Host, ip: IpAddr) -> Result<(), Denial>;  // at connect time
}
```

`judge_url`, in order: the scheme is `https`, or `http` when `allow_http`
(the `http` → `https` retry of REQ-FETCH-010 is the pipeline's, RFC 012;
the policy sees the URL the pipeline decided to request); no userinfo
(`CredentialInUrl`, REQ-FETCH-007); the port, explicit or default, is in
`ports`; the host is a name or an address literal. The `url` crate parses
every address literal notation of the WHATWG algorithm (decimal, octal,
hexadecimal, fewer than four parts, IPv6, IPv4-mapped IPv6) into one
canonical `Host::Ipv4` or `Host::Ipv6`, so a literal is judged by
`judge_address` right away, before any request. A host name that is a
single label, ends in `.localhost`, `.local`, `.internal`, `.arpa` or is
`localhost` is refused (`HostName`): such names resolve inside the host, not
on the public Internet.

`judge_address` canonicalizes an IPv4-mapped IPv6 address (`::ffff:a.b.c.d`)
to IPv4 and then applies, in this order:

1. operator `deny` rules (a match refuses);
2. operator `allow` rules (a match admits; this is how a private mirror or
   a test server is reached, at the operator's responsibility);
3. the built-in refused ranges below (a match refuses);
4. otherwise admitted.

A `Host` rule matches the name the URL carried (exact, or `*.` prefix for
subdomains); a `Network` rule matches the address.

**Built-in refused ranges** (REQ-SEC-006; a constant table with the RFC
that defines each range, reviewed with the threat register):

| Family | Ranges |
|---|---|
| IPv4 | `0.0.0.0/8` (this network), `10.0.0.0/8`, `100.64.0.0/10` (carrier-grade NAT; includes Alibaba metadata `100.100.100.200`), `127.0.0.0/8`, `169.254.0.0/16` (link-local; includes cloud metadata `169.254.169.254`), `172.16.0.0/12`, `192.0.0.0/24` (IETF protocol assignments), `192.0.2.0/24`, `192.88.99.0/24` (6to4 relay), `192.168.0.0/16`, `198.18.0.0/15` (benchmarking), `198.51.100.0/24`, `203.0.113.0/24`, `224.0.0.0/4` (multicast), `240.0.0.0/4` (reserved, includes broadcast) |
| IPv6 | `::/128` (unspecified), `::1/128`, `::ffff:0:0/96` (mapped; judged as IPv4 after canonicalization, never reached as IPv6), `64:ff9b::/96` and `64:ff9b:1::/48` (NAT64), `100::/64` (discard), `2001::/23` (IETF assignments: Teredo `2001::/32`, benchmarking `2001:2::/48`, documentation `2001:db8::/32`, ORCHID), `2002::/16` (6to4), `fc00::/7` (unique local; includes AWS metadata `fd00:ec2::254`), `fe80::/10` (link-local), `ff00::/8` (multicast) |

A globally routable unicast address that is in none of these is admitted.
The table is data; adding a range is a one-line change with a test.

### Connect-time enforcement (REQ-SEC-007, THR-06)

The agent client is one `reqwest::Client` for all fetches and robots.txt
reads, built by `egress::agent_client`:

- `dns_resolver(Arc<JudgingResolver>)`: the resolver performs the system
  lookup (`tokio::net::lookup_host`), applies `judge_address` to **every**
  returned address, drops refused ones, and fails the lookup with
  `Denial::Address` when none remains. hyper connects only to the addresses
  the resolver returned, so the judged address is the dialed address. The
  lookup result is not cached by rypsok; each request resolves again, and a
  DNS answer that changes between two requests is judged again
  (rebinding across requests cannot reach a refused address; rebinding
  within one connection is impossible because the connection is made to
  the judged address).
- address literals never reach the resolver; `judge_url` judged them.
- `redirect(Policy::none())`: every redirect comes back to the pipeline.
- `no_proxy()`: environment proxies are ignored. `[egress].proxy` applies to
  engine traffic only (REQ-RES-008); a proxy for agent-directed traffic is
  not offered in M3, because rypsok cannot verify that it enforces the same
  profile (REQ-SEC-007). The configuration page says so.
- `https_only(!allow_http)`, `cookie_store(false)`, no default headers but
  `User-Agent` (REQ-CMP-004) and `Accept`; `connect_timeout(LIM-F6)`,
  `timeout(LIM-F4)`; the method is GET and the pipeline sets no other
  header (REQ-SEC-009).
- `tcp_keepalive` and `pool_idle_timeout` short (30 s): a pooled connection
  was made to a judged address and may be reused; an idle one is dropped.

Redirect targets pass `judge_url` and, at connect time, `judge_address`
exactly like the first request (REQ-SEC-008); RFC 012 owns the same-host
rule and the `redirected` error.

### Strict mode (REQ-SEC-021, D-09)

`egress::strict::ReturnedUrls`: a process-wide set of comparison keys
(RFC 008) of every URL `web_search` returned in this process, bounded to
LIM-F14 entries with least-recently-added eviction, behind a `Mutex` held
for nanoseconds. The search pipeline records each returned URL after the
boundary; the fetch pipeline, when `[fetch].strict_mode = true`, requires
the requested URL's comparison key to be in the set, or the URL to match an
operator `allow` rule; otherwise `denied` with reason `strict_mode` before
any I/O. Same-host redirect targets are exempt (they were not returned but
belong to the approved site); cross-host targets are not followed anyway.
Nothing persists across a restart (REQ-RES-009).

### Configuration (RFC 004 keys, validation added)

| Key | Validation |
|---|---|
| `egress.allow`, `egress.deny` | Each entry parses as a CIDR (`ipnet`) or a host pattern (`name` or `*.name`, lowercase, no port); the error names the key and the index: `egress.allow[2]: not a network or a host pattern` |
| `egress.allow_http` | boolean |
| `fetch.ports` | 1–65535; a port other than 80 and 443 needs an `egress.allow` rule (unchanged) |
| `fetch.strict_mode` | boolean |

### Logging (RFC 009)

A refusal logs one `warn` event `refused` with `tool`, `reason` (`egress`,
`strict_mode`, `credential_in_url`, `scheme`) and `denial` (the `Denial`
name); never the URL, never the address. An admitted request logs the
`host` field (REQ-OBS-005 allows the host after the policy passed).

### Deployment egress control (REQ-DEP-003)

`docs/src/reference/integration-guide.md` gains a section "Restricting
egress at the operating-system level": a systemd unit with
`IPAddressDeny=any` plus `IPAddressAllow=` for the engine hosts and the
sites the operator allows; a container with `--network` on a dedicated
bridge and firewall rules; the note that the policy is a second line of
defense, not a substitute.

### Tests (REQ-TEST-005)

- `egress::policy::tests`: a table of every refused range in IPv4 and IPv6
  with one address inside and one outside; literals in decimal, octal,
  hexadecimal and short forms (`http://2130706433/`, `http://0177.0.0.1/`,
  `http://0x7f000001/`, `http://127.1/`) all judged as `127.0.0.1`;
  IPv4-mapped IPv6; credentials in the URL; forbidden ports; host-name
  refusals; deny over allow over built-in order; property test: every
  address the policy admits is a global unicast address as `IpAddr` sees it.
- `egress::resolver::tests`: a resolver whose answers change between calls
  (rebinding) never yields a refused address; a mixed answer yields only
  the admitted addresses.
- Integration tests reach `wiremock` only through an injected
  `[egress] allow = ["127.0.0.1/32"]` and `allow_http = true` in the test
  configuration; the policy code under test is the production policy.
- Protocol tests: a fetch of a loopback URL without the rule is `denied`
  with reason `egress`; strict mode refuses an unknown URL and admits one
  that `web_search` returned in the same session.

## Amendment to the baseline

The limits table gains two rows, applied with this RFC:

| ID | Parameter | Default | Range | Basis |
|---|---|---:|---|---|
| LIM-F14 | URLs remembered for strict mode | 10000 | operator | REQ-SEC-021; bounded memory |
| LIM-F15 | robots.txt entries kept in memory | 1000 origins | operator | REQ-CMP-005; bounded memory |

## Alternatives considered

- **Validate, then let the client resolve again.** Rejected: the check-to-
  connect gap is THR-06 itself.
- **A client per request pinned with `resolve_to_addrs`.** Rejected: a
  client per request rebuilds TLS state and a pool on every call; the
  judging resolver gives the same guarantee with one client.
- **Honoring `[egress].proxy` for fetches.** Rejected in M3: rypsok cannot
  know that the proxy applies the profile; an operator who needs it can
  restrict egress at the operating-system level (REQ-DEP-003).
- **An allow-only policy.** Rejected by D-09: open by default with built-in
  refusals and operator lists; strict mode exists for the closed stance.

## Open questions

- Whether `.arpa` and `.local` should be refused by name or left to the
  address judgment alone. Decided here: refused by name; they never name a
  public site.

## Amendment to RFC 011 (2026-10-27, M3 review)

- The resolver's lookup is a trait (`Lookup`) with the system resolver as
  the default; tests script DNS answers without a network, and the fetch
  service accepts a lookup for the same purpose. The policy judges every
  address whatever the lookup.
- Refusal logging lives in the fetch pipeline, the first place an
  agent-directed request is made: `refused` with `reason` and `denial`,
  never a URL, host or address; the resolver logs `address_refused` at
  `debug` with the denial name.
- A host whose last label is all digits is not a rule (neither an address
  nor a name), so a typo is refused at startup.
- A deny rule by name yields the denial `host_name`, by network `address`;
  both are the error reason `egress`.
- `*.name` does not match `name` itself; the page says so.
- Same-host redirect targets are exempt from strict mode in the fetch
  pipeline (RFC 012); the strict-mode key is the comparison key of the
  URL's returned form.
