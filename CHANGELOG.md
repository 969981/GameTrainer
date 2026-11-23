# Changelog

## 2026-11-23 — M4 FireRed / LeafGreen v2.0.4 guarded GDB writes

- Added 16 original ROM SHA1 profiles, Rust `trainer-frlg`, GDB-paused capture/wall transactions and SHA256-pinned 116-byte user-provided experimental wild Seed Hook.
- Added separate M4 Tauri IPC, SQLite live audit and a React FireRed/LeafGreen runtime panel, including real guest diff preview, confirmation and verified restoration. M3/M4 sessions are mutually exclusive.
- Catalog has two new FireRed/LeafGreen game entries (nine titles total). Public repository does not distribute the original GPL binary or any ROM.
- **No real mGBA ROM gameplay or abnormal restore acceptance test has been performed.** See [M4](docs/m4-frlg-v204-live.md).

## 2026-11-23 — M3 opt-in GDB live patches (CI green, emulator unverified)

- Verified [CI #45118655477](https://github.com/969981/GameTrainer/actions/runs/45118655477): 50 Rust tests, 15 UI tests, Chromium 3-width layouts, and Windows debug EXE.
- Fixed strict AXV/AXP ROM family address selection and repeat full-ROM SHA1 before each preview and apply.
- Added mocked GDB stop/read/write/readback/continue integration and fail-closed SQLite path management.

- Introduce localhost GDB RSP transport, full ROM SHA1 allowlist, guarded two-span Capture modification and four-ROM experimental BattleCrit Hook with a user-supplied SHA256-verified source payload.
- Add Tauri M3 scoped IPC and React preview/confirm/apply/restore/history UI; SQLite records real intents separately from M2 fixtures.
- Fishing/Feebas/Teleport live hooks remain disabled; real Windows+mGBA+retail ROM acceptance still outstanding.
- See [M3 experimental implementation](docs/m3-live-gdb-experimental.md).

## 2026-11-23 — RS v4.6 advanced feature security migration review

- Reviewed capture, fishing/Feebas, teleport/PokéFinder and experimental BattleCrit sources and installed/uninstall flow.
- Added security assessment, ADR-0004, 7-feature deny-live Manifest and 4 regression assertions; no production patch, injector or live-write capability.
- Targeted upstream tests locally: 47 passed / 2 skipped; full suite cannot pass without historical dist/*.CT artifacts.
- See [RS migration security review](docs/gen3-rs-migration-security-review.md).

## 2026-11-23 — Ruby/Sapphire Gen3 separate adapters
- Added 26-version source-backed Ruby/Sapphire header/anchor read-only Rust profiles and mGBA Win32 provider.
- Added two Gen3 game descriptors, feature research pages, Rust profile catalog and Seed/Timer viewer.
- Extended fixture-only M2 seed transactions to Ruby/Sapphire with distinct game_id/profile_id/SQLite audit.
- Ruby/Sapphire CI [#45111652638](https://github.com/969981/GameTrainer/actions/runs/45111652638) is green: 44 Rust tests, 11 UI tests, 3-width Chromium screenshot runs and Windows EXE.
- Real game writes and BattleCrit experiment remain disabled; see [source evidence](docs/gen3-ruby-sapphire.md).

## 2026-11-23 — M2 safe mutation prototype (Fixture only)

- CI verified: 32 Rust unit tests, React unit/build, Playwright 1100/1366/1500px browser layout and Windows Tauri M2 fixture command/EXE.
- [Full green CI](https://github.com/969981/GameTrainer/actions/runs/45109373457); live game writes remain disabled.
- Add Rust PatchManager: short one-time preview, identity validation, conflict checks, read-back, owner-scoped restore.
- Add SQLite AuditStore: WAL+FULL sync, write intent before mutation and separated fixture/live history.
- Add Tauri fixture commands and React preview/confirm/apply/restore screen. Actual mGBA memory remains read-only.
- Document [M2 design](docs/m2-safe-patch.md) and [ADR-0002](docs/decisions/0002-fixture-only-mutations.md).

## 2026-11-23 — M0 automated validation and M1 read-only prototype

- Windows Tauri icon-resource issue fixed; `cargo check`, desktop Rust `get_catalog` test, debug EXE `cargo build` succeeded.
- M0: Chromium Playwright 1100/1366/1500px screenshots, cross-game navigation and no horizontal overflow — all passed.
- M1: add `trainer-mgba` with six retail Emerald Rev0 profiles, partial ROM anchor verification and 9 synthesized read-only Seed/session unit tests.
- M1: add `trainer-runtime-win` Win32 read-only discovery/mapping, guarded Tauri IPC and read-only React RNG panel.
- [All three CI jobs passed: #45107272587](https://github.com/969981/GameTrainer/actions/runs/45107272587) — Rust 16 tests, UI 10 tests, Chromium E2E, Windows EXE artifact.
- Live Windows WebView2 GUI, true front-end-to-Tauri IPC flow, actual mGBA ROM memory reads and full ROM CRC are not validated yet. No memory writes or hooks.

## 2026-11-23 — M0 Mock platform scaffold

- Add Rust `trainer-core` typed GameRegistry and Tauri 2 read-only commands.
- Add pnpm/React/TypeScript/Vite/Tailwind workspace and shadcn/ui-style base components.
- Implement V3 game library, context-aware feature navigation, module/runtime pages, detached session inspector, theme switcher and local developer-journal draft.
- Five mocked platform families: Gen3, Gen4, Gen5, Gen6, Windows PC Native; verify a sixth game can be registered without AppShell changes.
- Add Rust 7 tests, frontend 9 tests and GitHub Actions CI; both jobs passed at run #45104751458.
- Limitations: no real target attach, ROM validation, Hook or memory mutation; Windows Tauri GUI not yet manually verified.

## 2026-11-23 — Initial planning
- Initialize GameTrainer Studio project repository.
- Add AGENTS.md as AI implementation contract.
- Document modular cross-game architecture (Runtime / Game / Feature / UI).
- Define Pokémon Gen3–Gen6, Windows PC Native expansion and version safety.
- Document V3 Dark Control UI information architecture, components and responsive layout.
- Add logging, SQLite audit and optional Obsidian development journal designs.
- Provide roadmap, test matrix, ADR, mock manifests, and theme tokens.

This repository contains a Windows-compilable desktop shell and read-only emulator connection prototype. No ROM memory mutation/Hook has been implemented or validated.
