# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Status (Last Updated: 2025-09-26)

### Security Hardening Progress
**Phase 1 - History File Access Hardening: ✅ COMPLETED**
- Implemented secure path sanitization for history audio files
- Replaced direct file path exposure with secure byte streaming via IPC
- Added restrictive CSP and narrowed asset protocol scope
- Fixed all clippy warnings
- Added unit tests for path sanitization (pending system library installation for full test execution)

**Phase 2 - Model Download Security: ⚙️ IN PROGRESS**
- Backend manifest loader + SHA-256 enforcement through `ModelManager`
- Safe tar extraction with canonicalised paths and symlink rejection
- Hardened HTTP client (timeouts, user-agent)
- TODO: surface checksum/extraction failures to UI and replace placeholder manifest digests with real release data

**Phase 3 - Runtime Hardening: 🔄 PENDING**
- Capability permissions audit
- Clipboard operation hardening
- Additional logging and monitoring

**Phase 4 - Continuous Security: 🔄 PENDING**
- CI/CD security pipeline setup
- Automated dependency auditing
- Security documentation

## Development Commands

**Prerequisites:**
- [Rust](https://rustup.rs/) (latest stable)
- [Bun](https://bun.sh/) package manager

**Core Development:**
```bash
# Install dependencies
bun install

# Run in development mode
bun run tauri dev
# If cmake error on macOS:
CMAKE_POLICY_VERSION_MINIMUM=3.5 bun run tauri dev

# Build for production
bun run tauri build

# Frontend only development
bun run dev        # Start Vite dev server
bun run build      # Build frontend (TypeScript + Vite)
bun run preview    # Preview built frontend
```

**Model Setup (Required for Development):**
```bash
# Create models directory
mkdir -p src-tauri/resources/models

# Download required VAD model
curl -o src-tauri/resources/models/silero_vad_v4.onnx https://blob.handy.computer/silero_vad_v4.onnx
```

## Architecture Overview

Handy is a cross-platform desktop speech-to-text application built with Tauri (Rust backend + React/TypeScript frontend).

### Core Components

**Backend (Rust - src-tauri/src/):**
- `lib.rs` - Main application entry point with Tauri setup, tray menu, and managers
- `managers/` - Core business logic managers:
  - `audio.rs` - Audio recording and device management
  - `model.rs` - Whisper model downloading and management  
  - `transcription.rs` - Speech-to-text processing pipeline
- `audio_toolkit/` - Low-level audio processing:
  - `audio/` - Device enumeration, recording, resampling 
  - `vad/` - Voice Activity Detection using Silero VAD
- `commands/` - Tauri command handlers for frontend communication
- `shortcut.rs` - Global keyboard shortcut handling
- `settings.rs` - Application settings management

**Frontend (React/TypeScript - src/):**
- `App.tsx` - Main application component with onboarding flow
- `components/settings/` - Settings UI components
- `components/model-selector/` - Model management interface
- `hooks/` - React hooks for settings and model management
- `lib/types.ts` - Shared TypeScript type definitions

### Key Architecture Patterns

**Manager Pattern:** Core functionality is organized into managers (Audio, Model, Transcription) that are initialized at startup and managed by Tauri's state system.

**Command-Event Architecture:** Frontend communicates with backend via Tauri commands, backend sends updates via events.

**Pipeline Processing:** Audio → VAD → Whisper → Text output with configurable components at each stage.

### Technology Stack

**Core Libraries:**
- `whisper-rs` - Local Whisper inference with GPU acceleration
- `cpal` - Cross-platform audio I/O  
- `vad-rs` - Voice Activity Detection
- `rdev` - Global keyboard shortcuts
- `rubato` - Audio resampling
- `rodio` - Audio playback for feedback sounds

**Platform-Specific Features:**
- macOS: Metal acceleration for Whisper, accessibility permissions
- Windows: Vulkan acceleration, code signing
- Linux: OpenBLAS + Vulkan acceleration

### Application Flow

1. **Initialization:** App starts minimized to tray, loads settings, initializes managers
2. **Model Setup:** First-run downloads preferred Whisper model (Small/Medium/Turbo/Large)
3. **Recording:** Global shortcut triggers audio recording with VAD filtering
4. **Processing:** Audio sent to Whisper model for transcription
5. **Output:** Text pasted to active application via system clipboard

### Settings System

Settings are stored using Tauri's store plugin with reactive updates:
- Keyboard shortcuts (configurable, supports push-to-talk)
- Audio devices (microphone/output selection)
- Model preferences (Small/Medium/Turbo/Large Whisper variants)
- Audio feedback and translation options

### Single Instance Architecture

The app enforces single instance behavior - launching when already running brings the settings window to front rather than creating a new process.

## Recent Changes (2025-09-24)

### Security Improvements
- **Path Traversal Protection**: Added `sanitize_history_path()` function with defense-in-depth validation including path component count check and canonicalization verification (src-tauri/src/managers/history.rs:320-335)
- **Secure Audio Streaming**: Replaced direct file path exposure with `stream_history_audio` command that returns byte arrays instead of filesystem paths
- **Frontend Blob Management**: Updated HistorySettings component to create scoped Blob URLs from streamed audio data with proper cleanup on unmount
- **CSP Hardening**: Restored strict Content Security Policy limiting connections to self and blob.handy.computer only
- **Asset Protocol Restriction**: Limited asset protocol scope to dist/** and resources/** directories with literal dot requirement

### Code Quality
- Fixed multiple clippy warnings across managers and overlay modules
- Improved error handling patterns using idiomatic Rust constructs
- Removed redundant default trait implementations in favor of derive macros

### Testing Infrastructure
- Added tempfile as dev dependency for isolated test fixtures
- Implemented comprehensive unit tests for path sanitization logic
- Note: Full test suite execution requires system libraries (libxi-dev, libx11-dev, libgtk-3-dev)
