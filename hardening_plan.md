# Hardened Handy Hardening Plan

## Objectives
- Remediate the high-severity vulnerabilities identified during code review (file path traversal and unsafe tar extraction).
- Reduce exposure of the renderer process and network surfaces used for model downloads.
- Provide a phased, testable roadmap that an engineer or automation agent can follow end-to-end.

## Current System Architecture (High-Level)
```
+-----------------+        +-----------------------+
| React/Vite UI   | <----> | Tauri IPC (invoke/emit)|
+-----------------+        +-----------+-----------+
                                      |
                                      v
                        +-----------------------------+
                        | Rust Backend (Tauri Command |
                        | handlers & managers)        |
                        +-----------+-----------------+
                                    |
          +-------------------------+-------------------------+
          v                         v                         v
  Audio Toolkit (CPAL/ VAD)   Model Manager (reqwest)   History Manager (SQLite)
                                    |
                                    v
                              File System (models/, recordings/, history.db)
```
Key notes:
- History audio files are exposed to the renderer via raw filesystem paths and the permissive `assetProtocol` scope.
- Model downloads are streamed directly from HTTPS without integrity checks; archives are extracted as-is.
- Renderer security controls (CSP, asset scope) are effectively disabled.

## Proposed Hardened Architecture
```
React UI <-> Tauri IPC (unchanged channel semantics)
    |                                     |
    | (asset protocol restricted to app)  |   Hardened Commands Layer
    |                                     v
    +------------------------------>  Secure History Service
                                         • validates filenames
                                         • streams audio via command

Model Manager (Rust)
    • verifies manifest checksums / size
    • validates archive entries before extract

Tauri Config
    • strict CSP
    • asset scope limited to `dist` & vetted resources
```
Supporting changes:
- Introduce a manifest (JSON or Rust constant) that stores expected SHA-256 hashes for shipped models; download verification enforces integrity before marking models usable.
- Upgrade tar extraction to canonicalise paths and reject traversal or symlink attacks.
- Tighten renderer permissions and add unit / integration tests for the new guards.

## Implementation Phases

### Phase 1 – Lock Down History File Access & Renderer Scope *(Status: Completed)*
**Goals**: Remove path traversal risk when serving history audio, restrict asset protocol exposure, and add regression tests.

**Implementation notes**
- Added `sanitize_history_path`/`read_history_audio_from` helpers in `src-tauri/src/managers/history.rs` to canonicalise filenames, reject traversal attempts, and read bytes from disk only after validating the target directory.
- Replaced the `get_audio_file_path` command with `stream_history_audio`, so the renderer never receives raw filesystem paths.
- Updated `HistorySettings.tsx` to request byte arrays over IPC, create scoped Blob URLs, and revoke them on cleanup to avoid leaks.
- Hardened `tauri.conf.json` with a restrictive CSP and narrowed asset protocol allowance to `dist/**` and curated resources.
- Added unit tests for the new path helper functions and introduced `tempfile` as a dev dependency to isolate fixtures.

| File | Outcome |
| ---- | ------- |
| `src-tauri/src/managers/history.rs` | Sanitises history filenames, provides safe streaming helper, and deletes entries only after validation. |
| `src-tauri/src/commands/history.rs` | Exposes `stream_history_audio` returning bytes and propagates sanitized read errors. |
| `src/components/settings/HistorySettings.tsx` | Streams audio via IPC, builds Blob URLs locally, and revokes URLs on unmount. |
| `src-tauri/tauri.conf.json` | Restores strict CSP and limits asset protocol scope to vetted directories. |

**Code Snippets**
```rust
// src-tauri/src/managers/history.rs
fn sanitize_history_path(recordings_dir: &Path, file_name: &str) -> Result<PathBuf> {
    let clean_name = Path::new(file_name)
        .file_name()
        .ok_or_else(|| anyhow!("invalid history filename"))?;

    let candidate = recordings_dir.join(clean_name);
    if !candidate.starts_with(recordings_dir) {
        bail!("history asset not found");
    }

    Ok(candidate)
}

pub fn read_history_audio(&self, file_name: &str) -> Result<Vec<u8>> {
    read_history_audio_from(&self.recordings_dir, file_name)
}
```
```typescript
// src/components/settings/HistorySettings.tsx
const getAudioUrl = useCallback(async (fileName: string) => {
  const bytes = await invoke<number[]>("stream_history_audio", { fileName });
  const blob = new Blob([new Uint8Array(bytes)], { type: "audio/wav" });
  return URL.createObjectURL(blob);
}, []);
```
```json
// src-tauri/tauri.conf.json
"security": {
  "csp": "default-src 'self'; img-src 'self'; style-src 'self' 'unsafe-inline'; script-src 'self'; media-src 'self' blob:; connect-src 'self' https://blob.handy.computer",
  "assetProtocol": {
    "enable": true,
    "scope": {
      "allow": ["dist/**", "../resources/**"],
      "requireLiteralLeadingDot": true
    }
  }
}
```

**Tests & Validation**
- ✅ Added Rust unit tests for `sanitize_history_path` and `read_history_audio_from`, covering traversal attempts and missing files.
- ⚠️ `cargo test` currently blocked because the system lacks X11 development packages (`libxi-dev`, `libx11-dev`, `libgtk-3-dev`, `pkg-config`). Install these before rerunning the test suite.
- ➡️ After installing prerequisites, execute `CARGO_HOME=$PWD/.cargo-home CARGO_TARGET_DIR=$PWD/target cargo test` inside `src-tauri`, then perform a manual history playback sanity check in the UI.

### Phase 2 – Secure Model Download & Extraction Pipeline *(Status: Backend implemented; UI surfacing & manifest data pending)*
**Goals**: Ensure downloaded models are authentic, correctly sized, and cannot escape their target directory when extracted.

| File | Planned Modifications |
| ---- | --------------------- |
| `src-tauri/src/managers/model.rs` | Introduce a `ModelManifest` struct containing expected SHA-256 hashes and total sizes. After download, verify content length, compute hash, and only then mark `is_downloaded = true`. Replace `archive.unpack` with manual iteration that canonicalises output paths and rejects `..` components and symlink entries. Add timeouts and custom user-agent to the `reqwest` client. |
| `src-tauri/src/settings.rs` | Optionally, expose a feature flag to allow unsigned models in debug builds only. |
| `src-tauri/src/commands/models.rs` | Surface download failures (hash mismatch, extraction errors) via structured errors to the UI. |
| `src/hooks/useModels.ts` & `src/components/model-selector/ModelSelector.tsx` | Update error handling to display checksum/extraction failures clearly to users. |
| `resources/models/manifest.json` (new) | JSON manifest containing entries `{ "id": ..., "sha256": ..., "size": ... }` consumed by the model manager. |

**Implementation notes**
- Added a `ModelManifest` loader that deserialises `resources/models/manifest.json`, validates each entry, and injects digests into `ModelManager` during initialisation.
- Downloads now stream into `*.partial` files, resume safely, and, on completion, run `verify_download` to enforce both byte-count and SHA-256 integrity before promoting artifacts.
- Hardened the HTTP client with explicit connect/read timeouts and a scoped user-agent string to reduce anonymous scraping and ensure consistent telemetry.
- Replaced `archive.unpack` with `extract_archive_securely`, which rejects symlinks, absolute/parent traversals, and enforces directory creation inside a temporary staging area before final promotion.
- Updated the extraction guard to explicitly treat `EntryType::Link` (hard links) as unsupported, matching the current `tar` crate API while keeping link-based payloads blocked.
- Added unit tests covering checksum verification, archive sanitisation, and successful extraction of well-formed archives (pending execution until GTK/X11 build deps are available).
- Populated `resources/models/manifest.json` with canonical CDN `content-length` values and SHA-256 digests (see `model_info.md` for the reference table and `generate_reference_hashes.sh` for the regeneration workflow recorded on 2025-09-26).
- `download_model`, `delete_model`, and `cancel_download` now emit structured Tauri errors with machine-readable codes (`checksum_mismatch`, `archive_error`, etc.), which the React hooks/components surface to users.

**Outstanding (Phase 2)**
- Update `src-tauri/src/settings.rs` once a debug-time bypass toggle is confirmed as necessary.

**Code Snippets**
```rust
// src-tauri/src/managers/model.rs
fn verify_download(path: &Path, expected: &ModelDigest) -> Result<()> {
    let metadata = std::fs::metadata(path)?;
    if metadata.len() != expected.size_bytes {
        anyhow::bail!("size mismatch: expected {} got {}", expected.size_bytes, metadata.len());
    }
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    std::io::copy(&mut file, &mut hasher)?;
    let actual = hex::encode(hasher.finalize());
    if actual != expected.sha256 {
        anyhow::bail!("hash mismatch for model {}", expected.model_id);
    }
    Ok(())
}
```
```rust
// Safe extraction loop
for entry in archive.entries()? {
    let mut entry = entry?;
    if entry.header().entry_type().is_symlink() {
        anyhow::bail!("symlinks are not permitted in model archives");
    }
    let dest = temp_extract_dir.join(entry.path()?);
    let canon = dest.canonicalize()?;
    if !canon.starts_with(&temp_extract_dir) {
        anyhow::bail!("archive entry escapes extraction directory");
    }
    entry.unpack(&canon)?;
}
```

**Tests & Validation (Required Actions)**
- Install Tauri Linux prerequisites so the Rust tests can build: `sudo apt-get install -y libxi-dev libx11-dev libgtk-3-dev pkg-config libglib2.0-dev`. Re-run `CARGO_HOME=$PWD/.cargo-home CARGO_TARGET_DIR=$PWD/target cargo test model::tests::` from `src-tauri/` and ensure the new checksum/extraction tests pass.
- When CDN artifacts change, rerun `generate_reference_hashes.sh` to refresh `resources/models/manifest.json` and capture the updated hashes in `model_info.md`.
- Validate error propagation once the UI wiring is complete: corrupt a downloaded model (flip a byte) and confirm the UI surfaces the checksum failure rather than silently completing.
- Exercise the tar-guard path manually by creating a `.tar.gz` containing `../escape.txt`; the download should abort and log a traversal rejection.
- For network behaviour, record that timeouts and resume headers are in place; when a mock server is available (wiremock/new fixture), add an automated test to assert retry/backoff semantics.

### Phase 3 – Renderer & Runtime Hardening Enhancements
**Goals**: Harden remaining surfaces, reduce permissions, and add monitoring hooks.

| File | Planned Modifications |
| ---- | --------------------- |
| `src-tauri/capabilities/*.json` | Remove redundant or duplicated permissions (multiple `autostart`) and ensure only required capabilities remain. Consider moving history/model commands into a narrower capability if using Tauri v2 capability system. *(Completed: duplicated entries removed; default scope limited to core/store/opener/resource read only).* |
| `src-tauri/src/lib.rs` | Add logging around denied operations and ensure `cancel_current_operation` covers new async flows. *(Completed: cancellation routine now emits structured log output.)*
| `src-tauri/src/clipboard.rs` | Wrap clipboard writes in timeout/error handling to avoid leaking previous clipboard contents on failure. *(Completed: clipboard restore now runs in a `finally` block with error logging.)*
| `src-tauri/src/audio_toolkit/audio/recorder.rs` | Ensure `run_consumer` clears buffers on shutdown and logs VAD anomalies; optional but part of defence-in-depth. *(Completed: consumer resets buffered samples and VAD state on shutdown or channel close.)*
| `README.md` / docs | Document new security assumptions (hash manifest, restricted asset protocol) for contributors.

**Code Snippet**
```json
// src-tauri/capabilities/desktop.json
{
  "identifier": "desktop-capability",
  "platforms": ["macOS", "windows", "linux"],
  "permissions": [
    "autostart:default",
    "global-shortcut:default",
    "updater:default"
  ]
}
```
```rust
// src-tauri/src/clipboard.rs
pub fn paste(text: String, app_handle: AppHandle) -> Result<(), String> {
    let clipboard = app_handle.clipboard();
    let original = clipboard.read_text().unwrap_or_default();
    clipboard.write_text(&text).map_err(|e| format!("clipboard write failed: {e}"))?;
    if let Err(e) = send_paste() {
        let _ = clipboard.write_text(&original);
        return Err(e);
    }
    clipboard
        .write_text(&original)
        .map_err(|e| format!("Failed to restore clipboard: {e}"))
}
```

**Tests & Validation**
- **Security lint**: Run `cargo clippy --all-targets -- -D warnings` and ensure no new warnings.
- **Clipboard regression**: Add unit/integration tests using `tauri-plugin-clipboard-manager` mocks if available.
- **Capability audit**: Verify `tauri-capability` CLI (or manual inspection) confirms no unused permissions.
- **Manual QA**: Validate shortcuts, autostart, updater flows after permission tightening.

### Phase 4 – Continuous Security Hygiene & Tooling
**Goals**: Institutionalise security checks to prevent regressions.

| File | Planned Modifications |
| ---- | --------------------- |
| `.github/workflows/security.yml` (new) | CI workflow running `cargo fmt --check`, `cargo clippy`, `cargo test`, `cargo audit`, and `npm audit --production`. |
| `package.json` / `Cargo.toml` | Add `cargo-audit` / `npm audit` helper scripts; update dependencies if high-risk advisories appear. |
| `docs/SECURITY.md` (new) | Document patch process, threat model highlights, and disclosure policy.

**Tests & Validation**
- **CI Pipeline**: Trigger workflow to ensure all steps pass; confirm failing advisories break the build.
- **Documentation Review**: Peer review SECURITY.md.

## Current Testing Status & Recommendations
- `cargo fmt` has been run; no formatting drift remains.
- Newly added unit tests for history path sanitisation compile, but running the full suite is blocked on missing system libraries for the `x11` crate (`libxi`/`libx11` headers).
- Recommended host setup before continuing: install Tauri's Linux prerequisites (`sudo apt-get install -y libxi-dev libx11-dev libgtk-3-dev pkg-config` or distro equivalents).
- After installing dependencies, rerun `CARGO_HOME=$PWD/.cargo-home CARGO_TARGET_DIR=$PWD/target cargo test` inside `src-tauri`, then exercise history playback in the UI to confirm blob streaming works end-to-end.
- Once tests succeed, proceed to Phase 2 work on model integrity verification.

## Execution Checklist
1. Create feature branches per phase; ensure phases can merge independently if required.
2. After each phase, run: `cargo fmt`, `cargo clippy`, `cargo test`, `bun run build`, and relevant manual checks.
3. Update CHANGELOG.md summarising security fixes per release.
4. Coordinate release with updated model manifests and communicate model verification requirements to users.

## Additional Notes
- Keep a backup copy of existing history files before deploying Phase 1 to avoid data loss if filenames fail sanitisation.
- Consider bundling a minimal audio history streaming API that supports pagination to avoid large memory loads.
- Evaluate signing model manifests with an offline key in a future iteration to provide tamper-evidence independent of TLS.
