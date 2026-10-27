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
the metadata in its body; without them the answer is 400:

```sh
curl -sS http://127.0.0.1:8737/mcp \
  -H 'Content-Type: application/json' -H 'Accept: application/json' \
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
Environment=RYPSOK_TOKEN_MIMLYS=…        # or LoadCredential= and token_file
DynamicUser=yes
IPAddressDeny=any
IPAddressAllow=127.0.0.0/8 ::1 93.184.216.0/24
NoNewPrivileges=yes
ProtectSystem=strict
```

## Container

The repository's `Dockerfile` builds a static binary into a distroless
image that runs as a non-root user and reads `/etc/rypsok/rypsok.toml`:

```sh
docker build -t rypsok .
docker run --rm -p 127.0.0.1:8737:8737 \
  -v "$PWD/rypsok.toml:/etc/rypsok/rypsok.toml:ro" \
  -e RYPSOK_TOKEN_MIMLYS=… rypsok
```

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
