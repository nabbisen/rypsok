# Tutorial 2: a keyed engine

**What you will have at the end.** Two engines answering each search,
Wikipedia and Brave Search, with the key kept out of the file.

**What you need.** Tutorial 1 done, and a Brave Search subscription token.

## 1. Add the engine

```toml
{{#include ../../../examples/configs/keyed-engine.toml}}
```

Three things to notice:

- `api_key_env` names the environment variable that holds the key.
  `api_key_file` is the alternative: a file whose content is the key.
  Writing the key itself into the file (`api_key = "..."`) is refused at
  startup, so a configuration file is safe to share.
- `categories` says what each engine answers. A search names a category
  (`general` by default); only engines serving it are asked.
- `preference` orders engines when results tie; lower comes first.

## 2. Check twice

Without the key:

```sh
rypsok check --config rypsok.toml
```

The Brave line says `inactive: environment variable BRAVE_API_KEY is unset
or empty`, and the exit code is still 0: an engine without its key is
inactive, not an error, so the same file runs on a machine without the
key. Now with it:

```sh
export BRAVE_API_KEY=...
rypsok check --config rypsok.toml
```

The Brave line now ends with `credential resolved`.

## 3. Search, and read the engines' part

Run the pipe of Tutorial 1 again. Two engines answer; a result that both
returned appears once, ranked higher. Ask for the engines explicitly with
`engine_status`:

```json
{{#include ../../../examples/calls/engine-status.json}}
```

Each engine's `state` is `closed` (in use), `open` (failing lately and
skipped for a while), `half_open` (one probe in flight) or `disabled`
(by configuration or for lack of a key). `latency_ms` is the median of
its recent requests.

## 4. When Brave refuses the key

A wrong or expired key gives every search the outcome `partial` with the
notice `{"code": "engine_unavailable", "engine": "brave", "reason":
"auth_failure"}`, and Wikipedia's results. rypsok keeps asking Brave, so
that a corrected key takes effect at the next search without a restart
of anything but rypsok itself (it reads the key at startup).

## What Brave's terms mean for you

Brave's results may be used for inference and must not be stored beyond
the call or redistributed; rypsok therefore never caches them, whatever
the cache setting. Wikipedia's results may be cached. Each engine's
record of its provider's terms is in the maintainers' part of this book.

Next: [Tutorial 3: reading a page](./03-reading-a-page.md).
