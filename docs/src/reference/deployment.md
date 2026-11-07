# Deployment

rypsok runs in one of two modes, with the same configuration file and the
same egress policy.

## Local mode: a child process over stdio

The host starts `rypsok serve --transport stdio` and speaks MCP on its
standard input and output; nothing listens on the network. This is the
mode for a host on the same machine, and the default.

## Network mode: Streamable HTTP

`rypsok serve --transport http` listens on `[service.http].bind`, loopback
by default, and serves MCP on `/mcp`. The 2026-07-28 revision is served
statelessly; the handshake-based revisions keep their sessions in memory
until `legacy_revisions = false`, which refuses them.

A stateless request carries the standard headers of the revision beside
the metadata in its body; without them the answer is 400. Its `Accept`
header lists both `application/json` and `text/event-stream` (the protocol
requires a client to list both; with one the answer is 406):

```sh
curl -sS http://127.0.0.1:8737/mcp \
  -H 'Content-Type: application/json' -H 'Accept: application/json, text/event-stream' \
  -H 'MCP-Protocol-Version: 2026-07-28' -H 'Mcp-Method: tools/call' -H 'Mcp-Name: engine_status' \
  -d '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"engine_status","arguments":{},
       "_meta":{"io.modelcontextprotocol/protocolVersion":"2026-07-28",
                "io.modelcontextprotocol/clientInfo":{"name":"probe","version":"0"},
                "io.modelcontextprotocol/clientCapabilities":{}}}}'
```

- **Off loopback, callers are required.** Each caller has an identifier
  and a token read from the environment or a file, presented as
  `Authorization: Bearer <token>`; rate limits apply per caller.
- **TLS is terminated in front of rypsok**, by a reverse proxy or a
  tunnel. List the name the proxy forwards in `allowed_hosts`; a bind on
  `0.0.0.0` or `::` requires that list.
- `Host` and `Origin` are validated; a foreign `Host` is refused, and any
  `Origin` is refused unless listed.
- Connections, request bodies and idle time are bounded (LIM-H1 to
  LIM-H3). A connection that sends nothing is closed after the idle
  time too. Every connection holds a file descriptor: set the descriptor
  limit (`LimitNOFILE` in the unit, `--ulimit nofile` for a container) to at
  least `max_connections` + 64. Below that, `accept` fails when the
  descriptors run out; the listener then waits 100 ms between attempts and
  logs `accept_failed` at most once per 10 s.

A systemd unit for network mode, with the egress restrictions of the
[integration guide](./integration-guide.md#restricting-egress-at-the-operating-system-level):

```ini
[Service]
ExecStart=/usr/local/bin/rypsok serve --transport http --config /etc/rypsok/rypsok.toml
LoadCredential=rypsok-mimlys:/etc/credentials/rypsok-mimlys   # token_file = "/run/credentials/rypsok.service/rypsok-mimlys"
DynamicUser=yes
NoNewPrivileges=yes
ProtectSystem=strict
ProtectHome=yes
PrivateTmp=yes
PrivateDevices=yes
ProtectKernelTunables=yes
ProtectControlGroups=yes
RestrictAddressFamilies=AF_INET AF_INET6
RestrictNamespaces=yes
LockPersonality=yes
MemoryDenyWriteExecute=yes
SystemCallFilter=@system-service
SystemCallArchitectures=native
CapabilityBoundingSet=
AmbientCapabilities=
UMask=077
MemoryMax=256M
MemorySwapMax=0
TasksMax=64
LimitNOFILE=128   # max_connections (default 64) + 64
IPAddressDeny=any
IPAddressAllow=127.0.0.0/8 ::1 <the engines' addresses>
```

Every line below `ProtectSystem` confines the process to what rypsok
needs: a network socket, memory, no devices, no namespaces, no new
privileges and no executable memory. `MemoryMax`, `MemorySwapMax=0` and
`TasksMax` turn a resource bomb that got past rypsok's own bounds into a
restart instead of a host-wide problem. Without `MemorySwapMax=0` a host with
swap slows down instead of restarting the process: the audit measured 200 MiB
of the process moved to swap with no kill, because `MemoryMax` alone lets the
kernel swap out before it kills. The `IPAddressAllow` line names the proxy, the
engines and the sites the operator admits: replace `<the engines'
addresses>` with the addresses (or networks) of the engines you configured
and of any site you allow, and keep the line in step with `[egress]`.

### Does the configuration fit `MemoryMax`?

`rypsok check` prints the worst-case memory of your settings and warns when
it is above the 256 MiB of this unit. The estimate is

```text
13 MiB
+ limits.fetches_in_flight × fetch.max_bytes
+ limits.parsing_in_flight × (4.5 × fetch.max_bytes + 13 MiB)
+ service.http.max_sessions × 60 KiB   (only while legacy_revisions is on)
+ cache.max_entries × cache.max_entry_bytes   (only when the cache is on)
```

The constants are measured: the idle process takes 10 to 12 MiB; one parse
of a page costs 34 MiB at 5 MiB and 99 MiB at 20 MiB (a line through both
gives 4.34 per byte of page and 12.4 MiB at none); a session takes 58.5 KiB.
They were measured on the musl release build of 0.8.0 (audit 020e, rounded
up) and are rows LIM-M1 to LIM-M5 of the
[limits table](../maintainers/limits-and-defaults.md). The result is an
estimate, not a guarantee: with the defaults it says 119 MiB, where four
5 MiB pages at once, from four hosts, measured 98 to 107 MiB without
sessions on the musl release build (`tools/measure-memory --defaults`, which
prints the estimate beside its measurement). The estimate is of the musl
binaries of the release and the image. A glibc build, which `cargo install`
makes on Linux, measured 171 to 187 MiB in the same run: above the estimate
and inside the 256 MiB of the unit, with less room than the estimate shows. The ceilings
`fetch.max_bytes` (10 MiB) and `service.http.max_sessions` (2048) each fit
the unit with the other at its default (184 and 224 MiB); both at once
(289 MiB) do not, and `check` warns. In a container, `--memory` takes the
place of `MemoryMax` in the same comparison, and `--memory-swap` set to the same
value takes the place of `MemorySwapMax=0` (memory and swap together, so no
swap).

## Binaries

Each release lists archives for Linux x86_64 and aarch64 (both static),
macOS arm64 and Windows x86_64, two container images (below), and a file
`SHA256SUMS` that names all six. Verify a download before running it, in
two steps:

1. **Integrity.** On Linux, `sha256sum --check --ignore-missing SHA256SUMS`
   in the directory that holds the file; elsewhere, compute the file's
   SHA-256 and compare it with its line in `SHA256SUMS`. This shows the file
   is intact. It does not show who made it: anyone who can replace the file
   can replace `SHA256SUMS` beside it.
2. **Origin.** `gh attestation verify <file> --repo nabbisen/rypsok`. Each
   release file carries a provenance attestation that GitHub signs when this
   repository's release workflow builds it from a tag. The command checks the
   signature and that the file is the one the workflow built. It needs the
   GitHub CLI and a network.

## Container

Each release holds an image for Linux x86_64 and one for Linux aarch64:
`rypsok-X.Y.Z-image-amd64.tar.gz` and `rypsok-X.Y.Z-image-arm64.tar.gz`. There
is no registry; the image is a file of the release, and it contains the very
binary of the release's Linux archive of the same architecture. Verify the
file as described under Binaries, then load it. The image is distroless, runs
as a non-root user and reads `/etc/rypsok/rypsok.toml`:

```sh
docker load < rypsok-X.Y.Z-image-amd64.tar.gz
docker run --rm -p 127.0.0.1:8737:8737 \
  --read-only --cap-drop ALL --security-opt no-new-privileges \
  --memory 256m --memory-swap 256m --pids-limit 64 --ulimit nofile=128:128 \
  -v "$PWD/rypsok.toml:/etc/rypsok/rypsok.toml:ro" \
  -e RYPSOK_TOKEN_MIMLYS=… rypsok:X.Y.Z-amd64
```

(`rypsok:X.Y.Z-arm64` on aarch64.) To make an image yourself, build the static
binary for `x86_64-unknown-linux-musl` (or `aarch64-unknown-linux-musl`),
copy it to `dist/linux-amd64/rypsok` (or `dist/linux-arm64/rypsok`) and run
`docker build -t rypsok .`; the `Dockerfile` compiles nothing and only copies
that file onto the pinned base image. An image made so is your own build, not
the release's.

The image is static and runs as a non-root user; the flags take away
what it does not need: a writable root, every capability, privilege
escalation, unbounded memory (and swap) and processes.

Inside the container the bind must be `0.0.0.0:8737`, so callers are
required; publish the port on loopback or behind the proxy only. Restrict
the container's egress at the network level as the integration guide
describes.

## Which certificates the fetches trust

Every outbound HTTPS connection, to the engines and to the pages `web_fetch`
reads, is checked against the operating system's certificate store. rypsok
bundles no root certificates of its own. The code that does this is the
platform verifier of the TLS library (`reqwest` with the `rustls-no-provider`
feature, which uses `rustls-platform-verifier`; no `webpki-roots`).

- **Linux and other Unix systems** (not macOS): the roots are read from the
  system locations, usually `/etc/ssl/certs`. When `SSL_CERT_FILE` or
  `SSL_CERT_DIR` is set (`SSL_CERT_DIR` is a colon-separated list), roots are
  read **only** from there and the system locations are not read. A private
  certificate authority is therefore added either to the system store
  (`update-ca-certificates`, `update-ca-trust`) or through these variables,
  and in the second case the file must hold the public roots as well, for
  example a copy of the system bundle with the private root appended.
- **macOS and Windows:** the system's own verifier and store; the variables do
  nothing. Add a private authority to the system store.
- **A minimal container needs a CA bundle.** The documented distroless image
  has one (`/etc/ssl/certs/ca-certificates.crt`). An image of your own that
  starts from `scratch` has none, and rypsok then does not start: it exits
  with status 1 and says "the HTTP client could not be built". The same
  happens when `SSL_CERT_FILE` names a file that does not exist. A broken
  trust configuration is thus found at start, not at the first fetch.

Nothing in the configuration file changes this. A certificate that fails
verification is, like any failed connection, a fetch that ends `unavailable`
with the reason `network`; the response does not say why, so check the
store first when every HTTPS fetch fails.

## Tokens

Generate a token of at least 32 bytes (`openssl rand -base64 32`), give it
to rypsok by `token_env` or `token_file` and to the caller out of band.
Rotate by adding the new caller, switching the client, then removing the
old one; each change is a restart.

## State

Nothing persists across a restart: circuit states, counters, the robots.txt
cache and the strict-mode set start empty.
