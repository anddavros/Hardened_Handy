#!/bin/bash

# Script to download models and generate SHA-256 hashes for manifest.json
# This creates the production manifest with real cryptographic hashes

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TEMP_DIR="$SCRIPT_DIR/temp_models"
MANIFEST_FILE="$SCRIPT_DIR/src-tauri/resources/models/manifest.json"

# Create temporary directory for downloads
mkdir -p "$TEMP_DIR"

echo "Downloading models and generating SHA-256 hashes..."
echo "This may take a while depending on your internet connection..."
echo

# Declare associative arrays for model info
declare -A MODELS
declare -A URLS

# Model definitions matching the Rust code
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
for MODEL_ID in small medium turbo large parakeet-tdt-0.6b-v3; do
    FILENAME="${MODELS[$MODEL_ID]}"
    URL="${URLS[$MODEL_ID]}"
    FILEPATH="$TEMP_DIR/$FILENAME"

    echo "Processing $MODEL_ID ($FILENAME)..."

    # Download if not already present
    if [ ! -f "$FILEPATH" ]; then
        echo "  Downloading from $URL..."
        curl -L -o "$FILEPATH" "$URL" || {
            echo "  Failed to download $FILENAME - using placeholder"
            # Add placeholder entry if download fails
            if [ "$FIRST" = true ]; then
                FIRST=false
            else
                echo "," >> "$MANIFEST_FILE.tmp"
            fi
            echo -n '    {
      "id": "'$MODEL_ID'",
      "sha256": "0000000000000000000000000000000000000000000000000000000000000000",
      "size_bytes": 0
    }' >> "$MANIFEST_FILE.tmp"
            continue
        }
    else
        echo "  Using cached file..."
    fi

    # Calculate SHA-256 hash
    echo "  Calculating SHA-256..."
    HASH=$(sha256sum "$FILEPATH" | cut -d' ' -f1)

    # Get file size in bytes
    SIZE=$(stat -c%s "$FILEPATH" 2>/dev/null || stat -f%z "$FILEPATH" 2>/dev/null)

    echo "  Hash: $HASH"
    echo "  Size: $SIZE bytes"

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

echo "✅ Manifest generated at: $MANIFEST_FILE"
echo
echo "Generated manifest:"
cat "$MANIFEST_FILE"

echo
echo "Optional: To clean up downloaded models ($(du -sh "$TEMP_DIR" | cut -f1)):"
echo "  rm -rf $TEMP_DIR"