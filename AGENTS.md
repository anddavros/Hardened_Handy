# Repository Guidelines

## Project Structure & Module Organization
`src/` hosts the React + TypeScript UI, with feature-specific code grouped under `components/`, state helpers in `stores/`, shared hooks in `hooks/`, and pure utilities in `lib/`. Runtime overlays live in `overlay/`. The Rust core resides in `src-tauri/`, where `commands/` expose Tauri invocations, `managers/` hold long-lived services (history, models, audio), and `audio_toolkit/` provides reusable DSP helpers. Build output folders such as `dist/` and `target/` are generated artefacts; keep changes focused on source directories. Platform notes and dependency setup live in `BUILD.md`.

## Build, Test, and Development Commands
Run `bun install` after cloning to sync JavaScript dependencies. Use `bun run dev` for the Vite dev server when iterating on the settings UI. Launch the complete desktop app with `bun run tauri dev`, which compiles the Rust backend and opens the Tauri shell. Prepare a production bundle via `bun run build` (frontend) paired with `bun run tauri build` when you need distributable binaries. Rust-side checks: run `cargo fmt`, `cargo clippy --all-targets`, and `cargo test` from `src-tauri/`.

## Coding Style & Naming Conventions
Follow the existing two-space indentation, double quotes, and semicolon style in `.tsx`/`.ts` files; rely on your editor's Prettier-equivalent settings. Name React components in `PascalCase`, hooks in `useCamelCase`, and shared constants either `SCREAMING_SNAKE_CASE` or `camelCase` depending on scope. In Rust, keep modules small, prefer `?`-based error propagation, and organise helpers alongside their consuming module. Format with `cargo fmt` and fix lint issues surfaced by Clippy before committing.

## Testing Guidelines
Rust modules include lightweight unit tests near their implementations (`#[cfg(test)] mod tests`). Extend those blocks or add new ones when introducing business logic, and validate via `cargo test`. Critical UI flows should be exercised manually through `bun run tauri dev`; document scenario coverage in the pull request until automated frontend tests are added. Avoid committing large model assets—reference download steps in `BUILD.md` instead.

## Commit & Pull Request Guidelines
Craft commit messages in the imperative mood ("Add clipboard guard"), optionally prefixed with Conventional headers (`feat:`, `fix:`) as seen in history. Keep the subject under ~72 characters and include a focused body when context is needed. Pull requests must link related issues, describe user-facing changes, list the commands/tests executed, and attach screenshots or logs for UI or platform-specific updates. Call out any new permissions or settings mutations so reviewers can reason about security hardening.
