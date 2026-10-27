Diff Checker Native V2

Own extension: almatkai, pinned commit b9a47b252ba9f8704bd83802ede71f407de27c49,
https://github.com/almatkai/diff-checker — MIT. The original LICENSE contains
literal backslash-n sequences; fixtures/own/LICENSE retains those exact bytes.
LICENSE-MIT presents the same terms with normal newlines.

Original Raycast extension: author destiner; contributor hayk_hayrapetian.
https://github.com/raycast/extensions/tree/6734d85992290066fd31a7f5bee3cca3d3545a1a/extensions/diff-checker
MIT repository terms retained in fixtures/raycast/ROOT-LICENSE. The icon is
unmodified, 201232 bytes, SHA256
5ffb435db77bbfb870dae78abdf32dc641cd83a2704b130a5b574e55dca580b6.

The text algorithm in src/diff.rs is a Rust adaptation of jsdiff 8.0.3,
Copyright (c) 2009-2015, Kevin Decker <kpdecker@gmail.com>. That ACTUAL pinned
npm distribution declares BSD-3-Clause, not MIT. Its complete LICENSE is
retained at LICENSE-BSD-3-CLAUSE and fixtures/diff-8.0.3/package/LICENSE.
The separate JSON Myers/backtrack algorithm adapts the own MIT source.

Tezbar beta SDK, native sessions, API docs/schema and renderer fixtures:
fd300e276356587726566026f072563eb9c34cb1, Apache-2.0, retained byte-exact.
See LICENSE-APACHE-2.0 and fixtures/beta/LICENSE. These fixtures are unmodified;
production extension code is in src/, test-only Tauri/registry/router stubs
are in fixtures/harness/. The aggregate source package is
MIT AND BSD-3-Clause AND Apache-2.0.

Cargo dependencies are resolved by Cargo.lock, not vendored or relicensed.
ryu-js supplies ECMAScript number formatting; serde/serde_json carry the Native
SDK envelope, and getrandom creates ephemeral capability tokens. The optional
Node test toolchain is locked in tools/package-lock.json. It executes original
TS/TSX and the actual beta renderer; none of these Node packages is a production
runtime dependency. Their own distributed license notices remain applicable.
