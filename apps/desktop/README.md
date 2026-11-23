# GameTrainer Desktop · M0

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
cargo clippy -p trainer-core --all-targets -- -D warnings
cargo test -p trainer-core
~~~

Browser uses fixtures/m0-registry.json. Tauri uses the same fixture embedded in Rust, returned by get_catalog. A Tauri IPC error is shown as error, NEVER silently downgraded to browser Mock mode.

M0 exposes no process attach/read/write, memory scanner, Hook or game modification APIs. All game/feature statuses are Mock/Planned; no real-time or write-ready claims.

To add a game in M0, add a descriptor to fixtures/m0-registry.json; there should be no changes to AppShell/global navigation. Rust GameAdapter registration and tests also allow adding a sixth game without rewriting registry.
