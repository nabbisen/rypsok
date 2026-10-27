# Tutorial 1: your first search

**What you will have at the end.** A running rypsok that answers a
`web_search` call, an answer you have read line by line, and a host entry
that lets your agent runtime start rypsok itself.

**What you need.** The `rypsok` binary and a shell. No key, no account:
this tutorial uses Wikipedia, which needs none.

## 1. Write the configuration

Save this as `rypsok.toml`:

```toml
{{#include ../../../examples/configs/first-search.toml}}
```

Check it:

```sh
rypsok check --config rypsok.toml
```

You should see `configuration from --config rypsok.toml is valid`, the
effective settings, and the line
`engine wikipedia: type=wikipedia enabled=true ... no credential needed`.

## 2. Talk to rypsok without a host

rypsok speaks the Model Context Protocol over standard input and output:
one JSON message per line. Two lines are enough for a first search. Save
them as `first-search.jsonl`:

```json
{{#include ../../../examples/calls/first-search.jsonl}}
```

The first line asks rypsok what it offers; the second calls `web_search`.
The `_meta` object names the protocol revision and your client; every
call carries it. Now pipe the file in:

```sh
rypsok serve --config rypsok.toml < first-search.jsonl 2>/dev/null
```

rypsok answers each line with one line, then stops because its input
ended. The `2>/dev/null` hides the logs; drop it to see them.

## 3. Read the answer

The second answer looks like this (the search was answered by a fixed
Wikipedia sample when this page was built; the titles you see come from
Wikipedia today, the shape is the same):

```json
{{#include ../../../examples/calls/web-search-ok.json}}
```

Reading it from the top:

- `isError: false`: the call succeeded and `structuredContent` holds the
  answer. The same JSON is repeated as text in `content[0].text` for
  hosts that read text only.
- `outcome: "ok"`: every engine asked answered. The other outcomes are
  `partial` (results, but an engine failed) and `empty` (an engine
  answered, nothing remained).
- `notices: []`: nothing to tell you. When rypsok cut a snippet, skipped
  an engine or dropped a result, a notice says so, from a fixed list.
- `results`: at most `max_results`, deduplicated and ranked. `title`,
  `url` and `snippet` come from the engine and are cleaned and bounded;
  `date` is present when the engine supplied one.

Everything in `title`, `snippet` and `url` is text from another site.
rypsok never treats it as an instruction, and neither should your agent:
[Reading a response](../reading-a-response.md) says how to tell it.

## 4. Let your runtime start rypsok

A runtime that speaks MCP starts rypsok as a child process from an entry
like this one (the key names vary by runtime; the command and arguments
are what matters):

```json
{{#include ../../../examples/hosts/stdio.json}}
```

Restart the runtime, list the tools, and you should see `web_search`,
`web_fetch` and `engine_status`.

## If it did not work

- `check` reported a problem: it names the key. Fix the line it names.
- No second line came back: look at the logs (drop `2>/dev/null`). A line
  with `engine_failed` names the engine and the class of failure, for
  example `timeout` when Wikipedia could not be reached.
- The answer is `empty` for a query that should match: Wikipedia's search
  is literal; try fewer words.
- The runtime does not list the tools: run the pipe of step 2 by hand
  first; if that works, the problem is the host entry's path or arguments.

More in [Troubleshooting](../troubleshooting.md). Next:
[Tutorial 2: a keyed engine](./02-keyed-engine.md).
