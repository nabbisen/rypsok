# Tutorial 4: network mode

**What you will have at the end.** rypsok listening over HTTP behind a
reverse proxy, two callers with their own tokens and their own rate
limits, and a runtime connecting to it.

**What you need.** Tutorial 1 done, a reverse proxy that terminates TLS
(any of the usual ones), and a way to give each caller its token.

## 1. The configuration

```toml
{{#include ../../../examples/configs/network-mode.toml}}
```

- `bind` off loopback (`0.0.0.0`, or an address) requires callers. The
  default, `127.0.0.1:8737`, needs none: on loopback every request is the
  caller `local`.
- `allowed_hosts` lists the names the proxy forwards in the `Host`
  header. A bind on `0.0.0.0` or `::` requires this list. Any other
  `Host` is refused with 400.
- A caller has an identifier and a token read from the environment or a
  file. A token is at least 32 bytes. Generate one:

```sh
openssl rand -base64 32
```

## 2. Start and check

```sh
export RYPSOK_TOKEN_MIMLYS=...   # the token of the caller "mimlys"
rypsok check --config rypsok.toml
rypsok serve --transport http --config rypsok.toml
```

The log's first line is `listening` with the address and the number of
callers. rypsok does not terminate TLS: put the proxy in front, forward
`/mcp` to `127.0.0.1:8737` on the rypsok host, and let the proxy carry
the TLS certificate.

## 3. Call it

Every request is a `POST` to `/mcp` with the bearer token and the three
headers of the protocol revision beside the JSON body:

```sh
curl -sS https://rypsok.example/mcp \
  -H "Authorization: Bearer $RYPSOK_TOKEN_MIMLYS" \
  -H 'Content-Type: application/json' -H 'Accept: application/json' \
  -H 'MCP-Protocol-Version: 2026-07-28' -H 'Mcp-Method: tools/call' -H 'Mcp-Name: engine_status' \
  -d '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"engine_status","arguments":{},
       "_meta":{"io.modelcontextprotocol/protocolVersion":"2026-07-28",
                "io.modelcontextprotocol/clientInfo":{"name":"probe","version":"0"},
                "io.modelcontextprotocol/clientCapabilities":{}}}}'
```

The answer is the same JSON the stdio transport gives. What a status
means: 401 the token is missing or wrong; 403 the request carried an
`Origin` that is not allowed (browsers never call rypsok, so none is
allowed unless listed); 404 a path other than `/mcp`; 413 a body above
the limit; 400 a `Host` not in the list or a missing protocol header.

## 4. Connect a runtime

```json
{{#include ../../../examples/hosts/http.json}}
```

Each caller has its own rate limits; one caller that exhausts its limit
does not slow another.

## 5. Run it as a service

The [deployment page](../reference/deployment.md) has a systemd unit with
the token as a credential and the outbound traffic restricted at the
operating-system level, and the container image.

Next: [Tutorial 5: the scraping switch](./05-scraping.md).
