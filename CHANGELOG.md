# Changelog

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

This release contains plans and Mock examples only. No working game modification engine has been committed.
