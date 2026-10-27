# Tutorial 5: the scraping switch

**What you will have at the end.** A general web engine that needs no
account or key, and a clear understanding of what you accepted to get it.

**What you need.** Tutorial 1 done.

## What a scraping driver is

Wikipedia and Brave offer interfaces meant for programs, and their terms
say what a program may do. Some providers offer none: the only way to
search them is to read the page a person would see. rypsok calls a driver
that does this a scraping driver. It ships with one, `duckduckgo`, which
reads DuckDuckGo's HTML endpoint.

Three things rypsok does not do for it: it does not pretend to be a
browser (it sends its own User-Agent, as it does everywhere), it does not
work around a challenge or a block (a challenge page is reported as the
engine failing with reason `blocked`), and it does not consult the
endpoint's robots.txt at run time (the state of that file is recorded in
the engine's record, and your switch covers it).

## 1. Turn the switch on

```toml
{{#include ../../../examples/configs/scraping.toml}}
```

Without `[scraping] enabled = true`, `check` refuses the file with one
sentence: `engines.ddg: type "duckduckgo" scrapes a human-facing
interface; it runs only with [scraping] enabled = true, at the operator's
risk under the provider's terms`. Turning the switch on is your
acceptance of those terms; rypsok logs `scraping_enabled` with the
engine's name at startup so that it is never silent.

## 2. Search

Run the pipe of Tutorial 1 with this file. Two engines answer; DuckDuckGo
is asked at most ten times a minute, one request at a time, and its
results are never cached.

## 3. When the provider says no

DuckDuckGo challenges requests it takes for bots. When it does, the search
comes back `partial` with the notice `{"code": "engine_unavailable",
"engine": "ddg", "reason": "blocked"}`, and after a few such answers
`engine_status` shows the engine `open`: rypsok stops asking for a while,
then tries once. There is nothing to configure to get around it; that is
the provider's decision, and the honest answer is to report it.

## What you can do instead

An engine that offers a real interface is always the better route: Brave
with a key (Tutorial 2), or a search service you run yourself. Both keep
the terms where you can read them.

That was the last tutorial. [Reading a response](../reading-a-response.md)
turns to the agent's side, and [Troubleshooting](../troubleshooting.md) is
there when something does not work.
