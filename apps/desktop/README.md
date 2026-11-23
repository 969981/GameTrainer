# GameTrainer Desktop · M0 / M1 Read-only

React 19 + TypeScript + Vite 7 + Tailwind CSS 4 + shadcn/ui configuration + Tauri 2.

## Setup
Windows: install stable Rust/MSVC toolchain, WebView2, Node.js 22+, pnpm 10.

~~~bash
corepack enable
pnpm install
pnpm dev            # Browser only: shared fixture Mock
pnpm desktop        # Tauri 2 invokes Rust read-only GameRegistry
pnpm lint
pnpm typecheck
pnpm test
pnpm build
cargo fmt --all -- --check
cargo clippy -p trainer-core -p trainer-mgba --all-targets -- -D warnings
cargo test -p trainer-core -p trainer-mgba
~~~

Browser uses fixtures/m0-registry.json. Tauri uses the same fixture embedded in Rust, returned by get_catalog. A Tauri IPC error is shown as error, NEVER silently downgraded to browser Mock mode.

M0 browser exposes only Mock and never attaches. **Windows Tauri M1** adds explicit read-only mGBA process discovery/attach, partial Emerald ROM-profile checks, live Seed snapshots and detach. Never enables process write, patch or Hook. This code has passed Windows compile/unit tests but has **not yet been verified against a live mGBA / game ROM**.

To add a game in M0, add a descriptor to fixtures/m0-registry.json; there should be no changes to AppShell/global navigation. Rust GameAdapter registration and tests also allow adding a sixth game without rewriting registry.

## CI & manual verification

- [Latest green CI](https://github.com/969981/GameTrainer/actions/runs/45107272587): 16 Rust core tests, 10 React unit tests, 3 Chromium browser layouts, Windows Tauri command unit test and debug EXE build.
- [M1 runbook](../../docs/m1-readonly.md). Connect only to the exact supported mGBA executable; use an unmodified Emerald Rev0 that you are authorized to run.
- If browser-only (`pnpm dev`), live IPC is unavailable by design.
- The Windows EXE is a debug/testing artifact, not a released installer or evidence of WebView2 visual verification.
