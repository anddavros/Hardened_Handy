# Documentation Update Plan

## Executive Summary
This plan addresses critical documentation gaps identified during the codebase review, focusing on security implementation documentation, version alignment, and missing architectural details.

## Priority 1: Critical Security Updates (Immediate)

### 1.1 Model Manifest SHA-256 Checksums
**File:** `src-tauri/resources/models/manifest.json`
**Issue:** Contains placeholder SHA-256 hashes (all zeros, ones, twos, etc.)
**Action Required:**
- Replace placeholder hashes with actual SHA-256 checksums for each model
- Verify checksums match the actual model files hosted at blob.handy.computer
- Current placeholders:
  - small: `0000000000000000000000000000000000000000000000000000000000000000`
  - medium: `1111111111111111111111111111111111111111111111111111111111111111`
  - turbo: `2222222222222222222222222222222222222222222222222222222222222222`
  - large: `3333333333333333333333333333333333333333333333333333333333333333`
  - parakeet: `4444444444444444444444444444444444444444444444444444444444444444`

### 1.2 Phase 2 Security Status Update
**File:** `CLAUDE.md`
**Current Status:** Shows "⚙️ IN PROGRESS"
**Actual Implementation:** Substantially complete based on code review
**Action Required:**
- Update Phase 2 status to reflect actual implementation
- Document completed security features:
  - ✅ ModelManifest class with digest validation (model.rs:79-124)
  - ✅ SHA-256 verification for downloads (model.rs:126-165)
  - ✅ Secure tar extraction with path sanitization (model.rs:167-230)
  - ✅ Symlink/hardlink rejection in archives (model.rs:213-217)
  - ✅ Hardened HTTP client with timeouts (model.rs:533-538)
  - ✅ Size validation before and after download
  - ✅ Atomic extraction with temporary directories
- Add remaining TODO items if any

## Priority 2: Documentation Completeness

### 2.1 Version History Updates
**File:** `CHANGELOG.md`
**Issue:** Missing v0.4.0 and v0.5.0 entries
**Current Version:** v0.5.0 (per Cargo.toml)
**Last Documented:** v0.3.0 (2025-07-11)
**Action Required:**
- Add v0.4.0 entry with:
  - Security hardening Phase 1 completion
  - History file access hardening
  - Path traversal protection
  - CSP implementation
- Add v0.5.0 entry with:
  - Model download security (Phase 2)
  - SHA-256 verification
  - Secure tar extraction
  - Test infrastructure improvements

### 2.2 Complete BUILD.md Instructions
**File:** `BUILD.md`
**Issue:** Truncated at line 57
**Action Required:**
- Complete VAD model download instructions:
  ```bash
  mkdir -p src-tauri/resources/models
  curl -o src-tauri/resources/models/silero_vad_v4.onnx \
    https://blob.handy.computer/silero_vad_v4.onnx
  ```
- Add verification step for downloaded model
- Include troubleshooting section

### 2.3 Create SECURITY.md
**File:** `SECURITY.md` (new)
**Purpose:** Document security architecture and threat model
**Content Structure:**
```markdown
# Security Architecture

## Threat Model
- Local file system attacks
- Malicious model files
- Path traversal attempts
- Supply chain attacks

## Implemented Mitigations

### Phase 1: History File Hardening
- Path sanitization (history.rs:320-335)
- Secure byte streaming
- CSP restrictions

### Phase 2: Model Download Security
- SHA-256 verification
- Safe tar extraction
- Hardened HTTP client

### Phase 3: Runtime Hardening (Planned)
- Capability permissions
- Clipboard hardening

## Security Testing
- Unit tests for path sanitization
- Archive extraction tests
- Checksum verification tests

## Reporting Security Issues
[Contact information and process]
```

## Priority 3: Architectural Documentation

### 3.1 Update Architecture Section
**File:** `CLAUDE.md` or `README.md`
**Additions Required:**
- Security components architecture:
  - ModelManifest system
  - Verification pipeline
  - Safe extraction process
- Data flow diagrams for:
  - Model download and verification
  - History file access
  - Audio streaming

### 3.2 Testing Documentation
**File:** `TESTING.md` (new) or section in `BUILD.md`
**Content Required:**
- System library requirements:
  ```bash
  # Linux dependencies for testing
  sudo apt install libxi-dev libx11-dev libgtk-3-dev
  ```
- Test execution commands:
  ```bash
  cargo test
  cargo test --workspace
  ```
- Test categories:
  - Unit tests (path sanitization, verification)
  - Integration tests
  - Security tests

### 3.3 API Documentation
**Location:** Code comments or separate API.md
**Focus Areas:**
- ModelManager public API
- Security boundaries
- Error handling patterns

## Implementation Timeline

### Week 1 (Critical Security)
- [ ] Day 1-2: Obtain and validate actual SHA-256 checksums for models
- [ ] Day 2-3: Update manifest.json with real checksums
- [ ] Day 3-4: Update CLAUDE.md Phase 2 status
- [ ] Day 4-5: Complete BUILD.md instructions

### Week 2 (Documentation Completeness)
- [ ] Day 1-2: Update CHANGELOG.md with v0.4.0 and v0.5.0
- [ ] Day 3-4: Create comprehensive SECURITY.md
- [ ] Day 5: Review and validate all updates

### Week 3 (Architecture & Testing)
- [ ] Day 1-2: Expand architecture documentation
- [ ] Day 3-4: Create testing documentation
- [ ] Day 5: Final review and cross-reference check

## Validation Checklist

### Pre-Release Requirements
- [ ] All model checksums are real SHA-256 hashes
- [ ] CLAUDE.md accurately reflects implementation status
- [ ] CHANGELOG.md is up to date with current version
- [ ] BUILD.md provides complete setup instructions
- [ ] SECURITY.md documents all security measures
- [ ] Test documentation includes all dependencies

### Quality Checks
- [ ] No placeholder or dummy values remain
- [ ] All file paths and commands are verified working
- [ ] Documentation is consistent across all files
- [ ] Version numbers align (Cargo.toml, CHANGELOG, README)
- [ ] Security features are accurately documented

## Notes for Implementation

1. **Model Checksum Priority**: The placeholder checksums are a critical security issue. Without real checksums, the verification system provides no actual security.

2. **Phase Status Accuracy**: CLAUDE.md should reflect that Phase 2 is substantially complete, with only UI error surfacing and manifest updates remaining.

3. **Version Documentation**: The two-version gap in CHANGELOG.md suggests significant work has been done without documentation updates.

4. **Security Transparency**: Creating SECURITY.md will help users understand the threat model and trust the implementation.

5. **Testing Requirements**: The system library requirements for testing should be prominently documented to avoid confusion.

## Review Process

1. Technical review by security team
2. User documentation review for clarity
3. Cross-reference all version numbers
4. Validate all commands and paths
5. Test all setup instructions on fresh systems

---

*Generated: 2025-09-26*
*Priority: HIGH - Security-critical documentation gaps identified*