# Handoff 022b — Release preparation for 0.10.0

> **Amended 2026-11-11 after the first review:** items 11 to 14 are
> added. They go in one more commit on top of the architect's two
> commits (the audit report, and this amendment). Commits 1 and 2 stay as
> they are.

**Created 2026-11-11.** The owner authorized release 0.10.0 on 2026-11-11.
It carries RFC 021 (handoff 021a) and RFC 022 (handoff 022a). As with
020v, preparing a release is a change of the tree and goes through
implement → review.

**RFCs.** [`../../done/021-call-maxima-and-definition-changes.md`](../../done/021-call-maxima-and-definition-changes.md),
[`../../done/022-rmcp-advisories.md`](../../done/022-rmcp-advisories.md).
**Requirements.** REQ-MAIN-006 (every contract change announced), REQ-DEP-005.
**Procedure.** `docs/src/maintainers/releasing.md`, steps 1 and 2. This
handoff is those two steps. Step 3 and later steps belong to the architect
and the owner.
**Rules.** As 022a. Tests first wherever a test can fail first. Push only
after approval. Evidence goes to `.git-exclude/evidence/022b/`. Use
relative `cd` only. The tool-definition rule applies: if a definition
changes, stop and ask.
**Depends on.** `d2d096a` (021a and 022a closed; CI green on all 7 jobs).

**Purpose.** A user of 0.9.0 reads one section and learns three things:
what 0.10.0 fixes, what it changes on the wire, and what it adds, in the
words the pages use. The dependencies are those of the day, and the
version is bumped. Nothing else changes.

## Change scope

Two commits, in this order, each through CI:

### Commit 1: the dependency step of the cycle (procedure step 1)

| # | Work | Evidence |
|---|---|---|
| 1 | **`cargo update`**, the whole lock file. A dry run on 2026-11-11 showed 48 packages, among them one new crate, `utf_types 1.0.0`. **For every crate that is added or removed**, say which parent brings it in or drops it, and give its license. For every crate whose update **is more than a patch** (from 1.0, a new minor or major, such as `toml 1.1 → 1.2`; before 1.0, a new second number), read its changelog entries and write one line: "affects rypsok" or "does not affect rypsok", and why. **`rmcp` stays `=3.9.0`**. Pins with `=` are not moved | the lock-file diff; the list |
| 2 | **The pinned actions**, each at the latest release of that action, pinned by commit SHA with the version as a comment, as the procedure says. One line per action that moves: the old and new version, and anything in its release notes that touches us | the list |
| 3 | **The base image digest** in the `Dockerfile`: the current digest of the same tag. Say whether it moved | the old and new digest |
| 4 | **The release compiler.** `rustup update stable`, then `rustc --version`. On 2026-11-11 stable was 1.99.1, the version that `action.yml` already names. If stable is still 1.99.1, nothing changes: say so. If it moved, set the `toolchain:` line to the new version and run the gates with it | `rustc --version` |
| 5 | **The gates are green**, `cargo deny check` included. The fixture tests show the tool definitions unchanged (`tools_list.json`, `instructions.txt`). If a dependency changes a definition or an answer, **stop and report** | the gates |

### Commit 2: the release commit (procedure step 2)

| # | Work | Evidence |
|---|---|---|
| 6 | **The changelog, assembled.** `[Unreleased]` becomes `## [0.10.0] — <the date you prepare it> — The rmcp security fix; call maxima; tool-definition changes announced`. Then do (a) to (f) below | the diff; in the review request, every line you merged, changed or dropped, each with its reason |
| 7 | **Versions.** The workspace `version` and the `rypsok-core` dependency version become `0.10.0`. Run `cargo update --workspace`. `cargo publish --workspace --dry-run --locked` must be green | the dry run's output |
| 8 | **`tools/CURRENT_MILESTONE` stays `M8`.** 0.10.0 is not a milestone. Say so in the review request, and show that the coverage gate passes | the gate's output |
| 9 | **`ROADMAP.md`.** Two rows in the order table: `21 \| Call maxima and tool-definition changes (requests of the first consumer) \| 0.10.0` and `22 \| The rmcp advisories of November 2026 \| 0.10.0`. Add one sentence under "Release cycle": "0.10.0 is a release outside the milestones, decided by the owner on 2026-11-11; it carries RFC 021 and RFC 022." **Do not edit the cycle's rule itself:** that is the owner's | — |
| 10 | **A consistency pass, editing only what it finds.** (a) Every page that names an HTTP status for a refused or unknown request agrees with 3.9.0's answers: 200 for an unknown tool, 400 for a session-less POST of a handshake revision, 406 for `q=0`, 415 for a `null` id. Check the tools page, "Reading a response", troubleshooting, deployment, the integration guide and the stability page. (b) The limits page has LIM-O3, and it agrees with `limits.rs`. (c) The configuration page's description of `check` shows the maxima line exactly as `check` prints it. (d) Every "0.9.0" in `README.md` and `docs/src/` that means the *current* release (installation commands, version examples) becomes 0.10.0. Every "0.9.0" that means a past release stays. List each one you found, kept or changed. (e) Run `tools/regenerate-example-answers` in compare mode: the four answers must be identical, and `tests/examples.rs` must be green | findings fixed in place, and listed in the review request |

**Item 6, the changelog, in detail:**

- **(a) Read every line against the final tree** (after commit 1). The
  Security entry says "this build" and "this tree's the glibc build". Name
  the version instead: "0.10.0 (measured on a glibc build of the release
  tree)". Keep the measured numbers as they are. They come from the
  review, and you do not measure again.
- **(b) The call-maxima line must not mislead.** The current line says
  "the bounds were documented; they are now enforced over the whole call".
  A reader takes that to mean a 0.9.0 call could run past them. It could
  not: RFC 021's amendment found that both maxima already held by the way
  each pipeline is built. Say instead that they **already held, and are
  now also enforced by one outer bound and pinned by tests**, so that they
  stay true if a pipeline changes. Keep the clause "nothing changes for a
  call that ends in time; no new kind, reason or notice".
- **(c) Order.** `### Security` first: it is why a user should upgrade.
  Then `### Changed`, opened by the paragraph "**Contract changes**
  (announced as REQ-MAIN-006 requires; they are why this is a minor
  release):", as 0.9.0 has it. The wire differences of rmcp 3.9.0 follow,
  one line each, then the other Changed lines. Then `### Added`, then
  `### Internal` last. **No `### Tool definitions` heading:** none changed,
  and the release checks will confirm it against 0.9.0.
- **(d) A user-visible behavior that changed and has no line yet gets one.**
  Compare 0.9.0 with this tree for stdio: end of input after
  `server/discover`, and `SIGTERM` before any message. Do the handoff 022a
  tests describe a difference from 0.9.0, or only from rmcp 3.9.0 before
  the adaptation? If a user of 0.9.0 sees a difference, add a line. If not,
  say so in the review request, with your reason.
- **(e) Commit 1 gets a line** under `### Internal`: "Dependencies updated
  for the cycle", plus the compiler, if it moved. Name any crate whose
  update a user could notice. If there is none, name none.
- **(f) The comparison links** at the foot: `[0.10.0]` for `0.9.0...0.10.0`,
  and `[Unreleased]` for `0.10.0...HEAD`.

### Commit 3: after the first review

| # | Work | Test or evidence |
|---|---|---|
| 11 | **The Unicode data is kept (`icu_*` 2.4).** The architect re-checked it: five of the newly accepted code points map to ASCII letters, not to an `xn--` label (U+209D, U+209E and U+209F to `w`, `y` and `z`; U+1D6A6 and U+1DF95 to `ss`). That is safe only because the egress policy judges the name **after** the mapping (`url.host()`). Nothing pins this yet, so add a test in `egress/policy/tests.rs`: mapped spellings of refused names are refused with `Denial::HostName`. Use at least fullwidth `ｌｏｃａｌｈｏｓｔ`, `a.ｉnternal`, `a.locaℓ` (U+2113) and `metadata.google.ｉnternal`, each built from escapes (`\u{…}`), plus one of the five new code points on a public name, which is admitted under the mapped ASCII name. Product code is unchanged, so no mutation run is needed (the release checks mutate `egress-policy`). Show the test failing first: point it at the raw input instead of `url.host()`, or explain why it cannot fail | the test; the failing-first run |
| 12 | **The changelog, three lines.** (a) **Move the Unicode sentence** from `### Internal` to the "Other changes" of `### Changed`, because a user sees it. Say what a user sees: a `web_fetch` of such a URL is no longer refused as invalid. Check whether a `web_search` result with such a host is now shown where 0.9.0 dropped it as `url_invalid` (RFC 020, D-24), and say so if it is. Name the five code points that map to ASCII, and add that the egress policy judges the mapped name, so a refused name stays refused however it is written. The Internal dependency line keeps only a pointer. (b) **The `### Fixed` line:** 0.9.0 kept running "until its input closed **or its first message arrived**". (c) **The fuzz lock file** (item 13) gets a short clause in the Internal dependency line | the diff |
| 13 | **The fuzz lock file agrees with the main one.** `fuzz/Cargo.lock` is resolved apart from `Cargo.lock`. Today 29 crates that both share are at older versions there (`toml 1.1.8` against `1.2.1`, `encoding_rs 0.8.42` against `0.8.44`, `url`'s neighbours and others), so the release-check fuzzing tests parsers that are not the ones we ship. (a) **Align it:** run `cargo update` with `--manifest-path fuzz/Cargo.toml`, then use `--precise` wherever its resolver picks a different version than the main lock. (b) **Add the gate `tools/check-fuzz-lock`** (Python, like `check-limits`). It fails when the fuzz lock holds a `name version` for a crate that the main lock also has, but not at that version. Crates only the fuzz workspace has (`libfuzzer-sys`, `arbitrary`…) are not compared. Run it in CI's `rules` job, next to the other `tools/` gates. (c) **`releasing.md`, step 1** names `fuzz/Cargo.lock` and the gate. The name `tools/check-fuzz-lock` is fixed: the audit report cites it | the gate failing on the old fuzz lock and passing on the new one |
| 14 | **The review questions, answered** (no work): the `### Fixed` section, the intro paragraph and the THR-28/LIM-O3 line all stay. The release checks are the architect's to run, on the head after commit 3 | — |

## Explicit non-change scope

- No source change, except what commit 1 makes necessary. If an update
  breaks the build or a test, **stop and report**: the fix is a decision.
- No tag, no publish, no GitHub release, no security advisory.
- No dispatch of the release checks. The architect dispatches them on the
  pushed release commit (procedure step 3).
- RFC 021 and RFC 022 stay in `accepted/`. The architect moves them to
  `done/` after the release, as with RFC 020.

## Prohibited shortcuts

- No changelog line that paraphrases a handoff instead of describing what
  landed.
- No line kept just because it was written, if the tree no longer does
  what it says.
- No dependency held back without its reason written down.

## Required evidence and review request

1. The gates, after each of the two commits.
2. The lock-file diff and the lists of items 1 to 4.
3. The dry run of item 7.
4. The list of changelog lines merged, changed or dropped, and the
   consistency findings of item 10.
5. The review request at
   `.git-exclude/review-requests/022b-release-preparation/README.md`.

On approval the architect pushes. The architect then dispatches the release
checks on the release commit, writes the 0.10.0 readiness report with that
run and its `head_sha`, and asks the owner for the Go.
