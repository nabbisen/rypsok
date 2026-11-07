# Handoff 020t — Documents: the book of the release, the deployment page true, measured texts

**Created 2026-11-07.** A fix handoff of the audit (RFC 020, D3), wave 4,
documents and their workflow only. Owner decision **D-17** (publish the book
from the release tag), authorized on 2026-11-03. Findings: DEP-18 (the book
runs ahead of the releases), PRF-4, PRF-6, PRF-7 and PRF-9 (the 020e review),
N1 of the 020q review (which certificates the fetches trust), and one found
while this handoff was written (item 7, decision ids).

**RFC.** [`../../accepted/020-project-audit.md`](../../accepted/020-project-audit.md), RFC 017 (the book).
**Rules.** As 020w. Evidence goes to `.git-exclude/evidence/020t/`. No source
file under `crates/` changes, except tests that read documents.
**Depends on.** 020p, which changes `deployment.md`'s container section and
`getting-started.md` first.

## Why

The owner's principle for the interface applies to the book: a reader must
not be misled.
- **The book describes the wrong version.** Today it is published on every
  push to `main`, so it describes code that no release contains. A reader
  of the 0.8.0 binary reads about 0.9.0 behavior.
- **Some pages say what the audit measured to be untrue**, or leave out what
  an operator needs to know.

## Change scope

| # | Change | Test (fails before) |
|---|---|---|
| 1 | **D-17, the book from the release tag.** `docs.yml` triggers on `push: tags: ["[0-9]*"]` and on `workflow_dispatch` (from a tag or from `main`, the owner's choice, visibly), and **not** on pushes to `main`. CI's `rules` job keeps building the book on every push, so a broken book is still red at once. Remove `continue-on-error: true` from the deploy step: Pages is live, and a failed publish must be red. The deploy job keeps its own `pages: write` and `id-token: write` | Gate tests: `docs.yml`'s triggers are exactly a tag push and a dispatch; no `continue-on-error` in it; CI still runs `mdbook build docs` |
| 2 | **The first page says which release it describes.** `introduction.md` gets, under its title, the line `This book describes the development version of rypsok, not a release.` between the markers `<!-- release -->` and `<!-- /release -->`. Before `mdbook build`, `docs.yml` replaces the text between the markers. On a tag `X.Y.Z` it becomes `This book describes rypsok X.Y.Z.`. On a dispatch from `main` it becomes `This book describes the development version of rypsok at commit <short sha>, not a release.` A step fails the job if the markers are missing, so the line cannot be dropped silently | A gate test: the markers are in `introduction.md` and the workflow's step replaces them. Show locally the three outputs of the step's script |
| 3 | **The owner's setting, named.** The `github-pages` environment allows deployments only from the branches and tags its policy names. For a tag deploy, add the tag rule `[0-9]*` (Settings, Environments, `github-pages`, Deployment branches and tags). `releasing.md` names the setting and says that until it is set, the publish job of a tag fails red, which is visible. `releasing.md` also says a documentation-only fix is a patch release (D-17) | `req_doc_001` |
| 4 | **PRF-4, swap.** `deployment.md`: add `MemorySwapMax=0` to the unit beside `MemoryMax=256M`, and `--memory-swap 256m` to the `docker run` line (memory and swap together, so no swap). The sentence "turn a resource bomb … into a restart" stays true with those lines. Add one sentence: without them, a host with swap slows down instead of restarting the process (the audit measured 200 MiB moved to swap with no kill) | A gate test: the unit holds `MemorySwapMax=0` and the `docker run` line holds `--memory-swap 256m` |
| 5 | **N1 (020q review), the trust of outbound TLS.** `deployment.md`, a short section "Which certificates the fetches trust". The outbound clients trust the operating system's certificate store (no bundled roots). On Linux the locations are read from `SSL_CERT_FILE` and `SSL_CERT_DIR` when set. A minimal container needs a CA bundle: the documented distroless image has one. A private CA is added to the system store or through those variables. **Read the client code to state it exactly** (the `reqwest`/`rustls` features in `Cargo.toml`, and what the egress clients build), and say what you read in the review request. If what you find differs from this sentence, write what is true and tell me | — (read) |
| 6 | **PRF-6, PRF-9, PRF-7: measured texts.** (a) `engines/wikipedia.md`: the admission's burst of 10 means that from the eleventh search in quick succession the engine limiter makes a call wait, about 600 ms per call at the admitted 100 per minute. This is by design (REQ-CMP-002), it is bounded by the deadline, and it counts against LIM-S4. (b) `limits-and-defaults.md`, the basis of LIM-S4: "Measured: cold start to the first answer 729 ms at M1 and 715 ms at M8; rypsok's own share about 6 ms (RFC 020, 020e)". (c) `tools/measure-memory`: a `--defaults` mode that runs the delivered defaults (`fetches_in_flight` and `parsing_in_flight` at their defaults, the largest page `fetch.max_bytes` admits, several hosts), and prints beside its measurement the worst case that `rypsok check` derives for the same settings. The tool's header says which mode is the delivered configuration | (c): run the mode once and put the output in the evidence |
| 7 | **Decision ids a reader can resolve.** Two lists use `D-n`: the study's decisions (cited in `requirements.md` and the limits table) and the audit's (RFC 020). A bare `D-10` or `D-19` in the book names two different decisions. Every citation of an **audit** decision in the book reads `RFC 020, D-n` (`releasing.md` "D-14 as amended" is one). For the **study's** decisions, find where a reader can look them up. If no public page lists them, say so in the review request; do not invent a list | A test: no `D-<n>` in `docs/src` outside `requirements.md` without either `RFC 020` before it in the same parenthesis or sentence, or a link to the list of the study's decisions |
| 8 | Changelog `[Unreleased]`, under Changed: "The book is published from the release tag and says on its first page which release it describes"; the swap and certificate notes under Documentation (or the section the changelog uses for it) | — |

## Explicit non-change scope

- No Dependabot file (D-14 as amended by D-23).
- No change to the limits' values.
- No text about features that do not exist.

## Required evidence and review request

1. The gates.
2. The three outputs of item 2.
3. One `--defaults` run of item 6(c).
4. The review request at `.git-exclude/review-requests/020t-documents/README.md`.

After the push, nothing publishes until the 0.9.0 tag (or a dispatch). The
book stays as last published until then, and the architect names that in
the readiness report.
