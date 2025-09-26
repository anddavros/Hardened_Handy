#!/bin/bash

# Script to download models ONCE and generate reference SHA-256 hashes
# These hashes will then be used as the "source of truth" for verification
# Run this in a trusted environment to establish your reference checksums

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TEMP_DIR="$SCRIPT_DIR/temp_models_reference"
MANIFEST_FILE="$SCRIPT_DIR/src-tauri/resources/models/manifest.json"

echo "🔒 SECURITY NOTICE:"
echo "This script downloads models and generates reference SHA-256 hashes."
echo "Only run this in a trusted network environment."
echo "These hashes will become your security reference for verification."
echo
read -p "Continue? (y/N): " -n 1 -r
echo
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo "Aborted."
    exit 1
fi

# Create temporary directory for downloads
mkdir -p "$TEMP_DIR"

echo "Downloading models and generating reference SHA-256 hashes..."
echo "This may take a while depending on your internet connection..."
echo

# Model definitions matching the Rust code
declare -A MODELS
declare -A URLS

MODELS["small"]="ggml-small.bin"
URLS["small"]="https://blob.handy.computer/ggml-small.bin"

MODELS["medium"]="whisper-medium-q4_1.bin"
URLS["medium"]="https://blob.handy.computer/whisper-medium-q4_1.bin"

MODELS["turbo"]="ggml-large-v3-turbo.bin"
URLS["turbo"]="https://blob.handy.computer/ggml-large-v3-turbo.bin"

MODELS["large"]="ggml-large-v3-q5_0.bin"
URLS["large"]="https://blob.handy.computer/ggml-large-v3-q5_0.bin"

MODELS["parakeet-tdt-0.6b-v3"]="parakeet-v3-int8.tar.gz"
URLS["parakeet-tdt-0.6b-v3"]="https://blob.handy.computer/parakeet-v3-int8.tar.gz"

# Start JSON output
echo "{" > "$MANIFEST_FILE.tmp"
echo '  "models": [' >> "$MANIFEST_FILE.tmp"

FIRST=true
REFERENCE_FILE="$SCRIPT_DIR/MODEL_REFERENCE_HASHES.md"

echo "# Model Reference Hashes" > "$REFERENCE_FILE"
echo "Generated on: $(date -Iseconds)" >> "$REFERENCE_FILE"
echo "Source: blob.handy.computer" >> "$REFERENCE_FILE"
echo >> "$REFERENCE_FILE"
echo "| Model ID | Filename | SHA-256 | Size (bytes) |" >> "$REFERENCE_FILE"
echo "|----------|----------|---------|--------------|" >> "$REFERENCE_FILE"

for MODEL_ID in small medium turbo large parakeet-tdt-0.6b-v3; do
    FILENAME="${MODELS[$MODEL_ID]}"
    URL="${URLS[$MODEL_ID]}"
    FILEPATH="$TEMP_DIR/$FILENAME"

    echo "Processing $MODEL_ID ($FILENAME)..."

    # Download with verification
    echo "  Downloading from $URL..."
    if ! curl -L --fail --connect-timeout 30 --max-time 600 -o "$FILEPATH" "$URL"; then
        echo "  ❌ DOWNLOAD FAILED for $FILENAME - SECURITY RISK!"
        echo "  This could indicate network issues or compromised source."
        echo "  Aborting to prevent insecure manifest generation."
        rm -rf "$TEMP_DIR"
        rm -f "$MANIFEST_FILE.tmp"
        exit 1
    fi

    # Verify download completed successfully
    if [ ! -f "$FILEPATH" ] || [ ! -s "$FILEPATH" ]; then
        echo "  ❌ DOWNLOAD VERIFICATION FAILED for $FILENAME"
        exit 1
    fi

    # Calculate SHA-256 hash
    echo "  Calculating SHA-256..."
    if command -v sha256sum >/dev/null 2>&1; then
        HASH=$(sha256sum "$FILEPATH" | cut -d' ' -f1)
    elif command -v shasum >/dev/null 2>&1; then
        HASH=$(shasum -a 256 "$FILEPATH" | cut -d' ' -f1)
    else
        echo "  ❌ No SHA-256 utility found (need sha256sum or shasum)"
        exit 1
    fi

    # Get file size in bytes
    if command -v stat >/dev/null 2>&1; then
        SIZE=$(stat -c%s "$FILEPATH" 2>/dev/null || stat -f%z "$FILEPATH" 2>/dev/null)
    else
        SIZE=$(wc -c < "$FILEPATH")
    fi

    echo "  ✅ Hash: $HASH"
    echo "  ✅ Size: $SIZE bytes ($(numfmt --to=iec-i --suffix=B $SIZE))"

    # Add to reference file
    echo "| $MODEL_ID | $FILENAME | $HASH | $SIZE |" >> "$REFERENCE_FILE"

    # Add to JSON (with comma handling)
    if [ "$FIRST" = true ]; then
        FIRST=false
    else
        echo "," >> "$MANIFEST_FILE.tmp"
    fi

    echo -n '    {
      "id": "'$MODEL_ID'",
      "sha256": "'$HASH'",
      "size_bytes": '$SIZE'
    }' >> "$MANIFEST_FILE.tmp"

    echo
done

# Close JSON
echo >> "$MANIFEST_FILE.tmp"
echo '  ]' >> "$MANIFEST_FILE.tmp"
echo '}' >> "$MANIFEST_FILE.tmp"

# Move temp file to final location
mv "$MANIFEST_FILE.tmp" "$MANIFEST_FILE"

echo "✅ REFERENCE MANIFEST GENERATED"
echo "📄 Manifest: $MANIFEST_FILE"
echo "📋 Reference: $REFERENCE_FILE"
echo
echo "🔒 SECURITY NOTES:"
echo "1. These hashes are now your reference checksums"
echo "2. Store MODEL_REFERENCE_HASHES.md securely"
echo "3. Future downloads will be verified against these hashes"
echo "4. If hashes don't match, it indicates file corruption or tampering"
echo
echo "Generated manifest:"
cat "$MANIFEST_FILE"

echo
echo "Clean up downloaded files? (saves space, keeps hashes)"
read -p "Remove $TEMP_DIR ($(du -sh "$TEMP_DIR" | cut -f1))? (Y/n): " -n 1 -r
echo
if [[ ! $REPLY =~ ^[Nn]$ ]]; then
    rm -rf "$TEMP_DIR"
    echo "✅ Cleaned up temporary downloads"
fi