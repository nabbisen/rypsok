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
  LIM-H3).

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
TasksMax=64
IPAddressDeny=any
IPAddressAllow=127.0.0.0/8 ::1 <the engines' addresses>
```

Every line below `ProtectSystem` confines the process to what rypsok
needs: a network socket, memory, no devices, no namespaces, no new
privileges and no executable memory. `MemoryMax` and `TasksMax` turn a
resource bomb that got past rypsok's own bounds into a restart instead
of a host-wide problem. The `IPAddressAllow` line names the proxy, the
engines and the sites the operator admits: replace `<the engines'
addresses>` with the addresses (or networks) of the engines you configured
and of any site you allow, and keep the line in step with `[egress]`.

## Binaries

Each release lists archives for Linux x86_64 (static), macOS arm64 and
Windows x86_64, and a file `SHA256SUMS`. Verify a download before running
it: on Linux, `sha256sum --check --ignore-missing SHA256SUMS` in the
directory that holds it; elsewhere, compute the archive's SHA-256 and
compare it with its line in `SHA256SUMS`. The sums show the file is
intact; they do not show who published it.

## Container

The repository's `Dockerfile` builds a static binary into a distroless
image that runs as a non-root user and reads `/etc/rypsok/rypsok.toml`:

```sh
docker build -t rypsok .
docker run --rm -p 127.0.0.1:8737:8737 \
  --read-only --cap-drop ALL --security-opt no-new-privileges \
  --memory 256m --pids-limit 64 \
  -v "$PWD/rypsok.toml:/etc/rypsok/rypsok.toml:ro" \
  -e RYPSOK_TOKEN_MIMLYS=… rypsok
```

The image is static and runs as a non-root user; the flags take away
what it does not need: a writable root, every capability, privilege
escalation, unbounded memory and processes.

Inside the container the bind must be `0.0.0.0:8737`, so callers are
required; publish the port on loopback or behind the proxy only. Restrict
the container's egress at the network level as the integration guide
describes.

## Tokens

Generate a token of at least 32 bytes (`openssl rand -base64 32`), give it
to rypsok by `token_env` or `token_file` and to the caller out of band.
Rotate by adding the new caller, switching the client, then removing the
old one; each change is a restart.

## State

Nothing persists across a restart: circuit states, counters, the robots.txt
cache and the strict-mode set start empty.
