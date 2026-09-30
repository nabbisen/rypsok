# Glossary

**Status:** Baseline 1.0 (2026-09-30).

One name for each thing, one meaning for each name. Where mimlys or MCP uses
a word differently, the difference is stated, and rypsok documents use the
word only in the sense given here.

| Term | Meaning in rypsok |
|---|---|
| **rypsok** | This product: a headless meta-search service for AI agents. It works as an independent service; mimlys is one of its clients |
| **Caller** | Whoever invokes a tool: a model through its host, or a runtime such as mimlys. Callers are untrusted |
| **Host** | The program that connects a model to rypsok: an MCP client, an agent runtime, a desktop application |
| **Operator** | Whoever configures and starts rypsok. Operators are trusted to the extent of their configuration |
| **Tool** | One of the three operations rypsok offers: `web_search`, `web_fetch`, `engine_status` |
| **Call** | One invocation of a tool. A call ends in exactly one outcome |
| **Outcome** | The single classification of a completed call: `ok`, `partial`, `empty`, or one error kind |
| **Notice** | A statement by rypsok about a call that succeeded: an engine was skipped, a snippet was shortened, characters were removed, results were dropped |
| **Engine** | A configured search source with a stable identifier, visible to operator and caller. One driver may serve several engines |
| **Driver** | The code that implements one kind of engine: request construction, parsing, error classification |
| **Provider** | The organization or service that operates the endpoint behind an engine |
| **Admission record** | The recorded result of checking a provider's terms before its driver exists: endpoint, permitted use, rate limits, attribution, storage rules |
| **Category** | A fixed name that selects a set of engines: `general`, `news`, `science`, `code` |
| **Result** | One entry returned by `web_search`: title, URL, snippet, and optional date and engines |
| **Envelope** | The fields of a response that rypsok itself asserts: `outcome`, `notices`, and the control fields the external design lists for each tool (`truncated`, `next_start_char`, `engines`). Every other field is external data |
| **External data** | Text or identifiers that originate outside rypsok: titles, snippets, page content, URLs. Never an instruction |
| **Content boundary** | The set of controls that keeps external data recognizable as data: structural separation, encoding, exclusion of hidden content, removal of invisible characters |
| **Egress policy** | The rules that decide whether rypsok may connect to a network destination. Two profiles: agent-directed and engine |
| **Agent-directed request** | An outbound request whose destination the caller chose: a fetch, and the robots.txt read that precedes it |
| **Engine request** | An outbound request to a configured engine endpoint |
| **Deadline** | The absolute time by which a search or fetch returns, with or without complete results |
| **Circuit** | The health state of an engine: closed (in use), open (skipped during a cooldown), half-open (one probe request) |
| **Comparison key** | The internal, normalized form of a URL used only to detect duplicates. Never returned, never fetched |
| **Continuation** | Reading the next part of a page: `start_char` in the request, `next_start_char` in the response. No state is kept between calls |
| **Profile of a tool** | Its declared effect on local state, the data that leaves the host, the maximum duration, and the maximum output |
| **Milestone** | M0 to M5 of `ROADMAP.md`. Every requirement names the milestone in which it is verified |

## Words with another meaning elsewhere

| Word | mimlys | MCP | rypsok |
|---|---|---|---|
| Capability | An explicit authorization for a resource or operation | A protocol feature that a client or server declares | Not used |
| Session | The holder of a task's capability set | Not present in the current revision | Not used; isolation is per caller identity or per process |
| Observation | Structured information returned from an action | — | Not used |
| Policy | Several named policies of a task or of the runtime | — | Not used as a noun of its own; rypsok has an egress policy and configuration |
| Provider | An implementation behind an interface, such as `ToolProvider` | — | The organization behind an engine |
