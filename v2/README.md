# Diff Checker — Native V2

Offline Rust ports of **Check Diff** (`check-diff`) and **Compare JSON**
(`compare-json`), based on the own extension at
`b9a47b252ba9f8704bd83802ede71f407de27c49`. No JavaScript engine, network,
persistent storage, subprocess or payload logging is used by the extension.
Original author: **destiner**; contributor: **hayk_hayrapetian**. See
[NOTICE.md](NOTICE.md) and [provenance.json](provenance.json).

## Behavior

Check Diff keeps Original Text (`original`) and Changed Text (`changed`) drafts.
Find Difference leaves identical inputs on the form with “Files are identical”.
Other inputs open Detail with the source's exact raw markdown and `getDiffText`
output: `>>>` additions, `<<<` removals, six spaces on nonempty equal lines,
and blank lines left blank. This directly ports the locked **diff 8.0.3** line
path algorithm, including its tie selection and CRLF/end-newline behavior.
There is no additional text copy action. Back restores both drafts.

Compare JSON has the source description, live validation and Compare, Format
JSONs, Swap JSONs and the two field-specific Paste Clipboard actions. Format
handles each valid nonempty field independently and preserves invalid fields.
Its source feedback titles appear inline. Swap and Paste deliberately retain
existing field errors, as the pinned hook does; typing validates the changed
field again. An empty clipboard read does nothing; malformed responses and read
failures preserve the drafts and report an inline failure. Submit validates
Original before Modified and accepts all JSON roots, including primitives and
null. Rust syntax errors use the stable field message **Invalid JSON** instead
of engine-dependent JavaScript SyntaxError wording. Empty submit uses **JSON is
required**; empty live input has no error.

JSON formatting follows JSON.parse + JSON.stringify with two spaces: insertion
order, numeric index keys first, duplicate keys' last value in their original
position, binary64 rounding, -0 → 0, overflow → null, and UTF-16 keys/strings
including escaped lone surrogates. It does not alphabetize object keys. Raw
unpaired UTF-16 text cannot cross the beta SDK's UTF-8 string transport; use
JSON `\ud800` escapes for such JSON data. There is no lossy surrogate replacement.

The JSON diff directly ports the own `src/lib/diff.ts` Myers/backtrack (a separate
algorithm from jsdiff), edit order, counters, context 3, absorbed gaps below 4,
skip wording, identical markdown and full/compact code fences. Detail metadata
has Added Lines, Removed Lines and Total Changed tags in beta's supported
success/error/warning colors. Copy Full Diff, Copy Compact Diff, Copy Formatted
Original and Copy Formatted Modified recompute from committed inputs; supplied
event content is ignored. Copy acknowledgements require an object with
`ok: true`, no non-null `error`, and absent or true `success`. Failures remain
retryable in Detail, with committed results and form drafts intact. Compare
Again / Back restores the form.

All source shortcut descriptors are retained (cmd-shift F/S/1/2 on the form;
cmd-shift C, cmd 1/2, cmd R on Detail). **The pinned beta renderer does not bind
arbitrary action shortcuts or display them in its action overlay.** Actions
remain available in the existing action menu; beta supplies its own Enter,
Back and Cmd+K controls. This package does not claim working platform keyboard
shortcuts or introduce a custom shortcut layer. It uses real beta Form/Detail
nodes, not list substitutes, toast/HUD channels, or an executable JSON loader.
The beta Markdown renderer highlights diff code when supported, but colored
individual diff lines are not guaranteed; metadata colors are verified.

`root.props.rawMarkdown` always retains exact source markdown. Ordinary displayed
markdown is identical. For content containing longer backtick runs, display
uses longer fences; raw CR is displayed as the literal `\r` so markdown cannot
reinterpret it as a new line outside a prefixed code block. These render-only
changes prevent embedded fence/image injection. Copy outputs remain exact.
The host Markdown component also supplies its own generic code-block copy
button; this package adds no text-copy action, and its committed JSON copy
routes are the four actions above.

## Bounds and session lifecycle

Each input is at most **1 MiB UTF-8**, each diff side at most **20,000 lines**,
and each generated formatted/diff output at most **8 MiB**. JSON nesting is at
most **128**. Each algorithm has **4,000,000 work units**, charging diagonal
steps, token comparisons and arena nodes or all trace snapshot cells *before*
allocation. Lines are interned before Myers comparisons. Limits use checked
size calculations and bounded line counts. These are generous finite bounds,
not unbounded parity claims: adversarial disjoint/repeated lines may be rejected
well below the byte/line ceilings. Invalid or over-budget comparisons remain
retryable on the form. Format leaves a field untouched if its formatted expansion
would exceed the 1 MiB input ceiling and reports the limit inline, while still
formatting the other eligible field. Trace storage is bounded by the work budget rather than
quadratic unchecked allocation.

IDs are ephemeral per-session capabilities, validated before effects. Field IDs
and callback IDs stay stable during typing. Typing responses omit controlled
values, so actual beta reconciliation retains queued local drafts; backend form
actions use serialized field callbacks, while submit accepts the renderer's
complete current draft. IDs rotate on Form/Detail/Back and search transitions.
Dispose removes session data; relaunch starts fresh. Stale, fabricated,
cross-command and old-session actions are rejected before clipboard operations.

## Compiled-in integration

This package is **not** dynamically loadable. Vendor this directory at the stable
host path `src-tauri/native-extensions/diff-checker`. Do not depend on an external
checkout path. The API reference is Tezbar beta
`fd300e276356587726566026f072563eb9c34cb1`; exact fixtures are included.

1. Add to the host's `src-tauri/Cargo.toml`:

   ```toml
   tezbar-diff-checker = { path = "native-extensions/diff-checker" }
   ```

2. Register the concrete production adapter in
   `src-tauri/src/native_core/extensions/mod.rs`:

   ```rust
   #[path = "../../../native-extensions/diff-checker/src/tezbar_adapter.rs"]
   mod v2_diff_checker;
   ```

3. In `v2_runtime.rs`, add `"diff-checker"` to the accepted slug match, route its
   `commands` arm to `super::v2_diff_checker::commands()`, and add its handler arm:

   ```rust
   "diff-checker" => {
       super::v2_diff_checker::handle(ctx, command, operation, payload, state).await
   }
   ```

4. Append [manifest.json](manifest.json) as a bundled entry in `v2_catalog.json`.
   Its canonical ID is `com.tezbar.native.diff-checker`, replacing
   `raycast.diff-checker`. Runtime is Rust for both commands; macOS and Windows
   verification remain **not-run**. The strict schema has no root author field,
   so author/contributor credit is in the description and this notice.
5. Copy the unmodified `fixtures/own/assets/icon.png` to the host's
   **`src/renderer/public/extensions/icons/diff-checker.png`**. The manifest URL
   is `/extensions/icons/diff-checker.png` (201232 bytes, SHA256 in provenance).
6. Rebuild the native app. The manifest/marker alone cannot supply Rust dispatch.
   The beta native-session layer continues to enforce installation and serialize
   adapter state; no new storage or profile migration is needed.

The adapter compiles unchanged alongside the included byte-exact beta common
and SDK. Tests also execute byte-exact beta `native.rs`; only Tauri, installed
registry and compiled registration are test stubs. No host files are modified
by this package or its validation tools.

## Reproduce validation

From this directory, with Rust stable and 1.85.0, and Node 22+:

```sh
cargo test --all-targets --locked
cargo fmt --all --check
cargo clippy --all-targets --locked -- -D warnings
cargo +1.85.0 test --all-targets --locked
cargo +1.85.0 clippy --all-targets --locked -- -D warnings
cargo check --all-targets --target x86_64-pc-windows-gnu --locked
cargo clippy --all-targets --target x86_64-pc-windows-gnu --locked -- -D warnings
npm ci --prefix tools --ignore-scripts
npm test --prefix tools
python3 tools/check-package.py
```

The Node oracle executes byte-exact **own TS/TSX**, actual **diff 8.0.3**, and beta
Form/Detail/Markdown/reconciliation helpers. It does not rewrite expected
algorithms in JS. Seeded randomized/edge fixtures cover 10,249 comparisons,
including numeric/key/Unicode cases and compact chunks. TSX tests execute the
source form hooks/actions with test-only Raycast/clipboard bindings. Rust tests
cover all copy routes, strict acknowledgements, retries, field actions, bounds
and native-session lifecycle/error envelopes. No test accesses a real OS
clipboard or user profile; beta's original common/SDK storage tests use isolated
temporary directories.

`check-package.py` creates a Cargo archive, verifies every provenance fixture and
license (including npm's `dist` files), extracts it, restores the archive's own
`Cargo.toml.orig` (Cargo normalizes away path-only local dev dependencies), then
recreates local test manifests from the included `fixtures/harness` templates
(Cargo excludes nested packages), then runs its self-contained Rust tests and fixture/schema checks. No external host
or source checkout is required. Node packages and Cargo registry dependencies
must be fetched normally if not already cached. The published Git file set is
also checked for accidentally ignored fixtures. `v2/.gitignore` explicitly
unignores the pinned npm `dist` directory; no force-add is needed.

Completed checks are recorded in [verification.json](verification.json).
Real macOS/Windows UI, keyboard and clipboard QA has **not run**. Cross-compiling
and DOM/render tests do not establish real desktop verification.
