# Model Information for Handy Project (Updated)

> **Scope.** Replace placeholder hashes with **verifiable SHA‑256** values and add a manifest snippet + Linux verification commands. Based on the prior Handy doc.  

---

## **Model Overview (with SHA‑256)**

> **Note:** SHA‑256 values below are **canonical upstream hashes** for the exact filenames; they should match your CDN files if those are byte‑identical mirrors. If you repacked or re‑quantized, compute hashes locally and update the manifest accordingly.

| Model ID | Model Name | Filename | CDN URL | Approx. Size | **SHA‑256** (canonical) |
|---|---|---|---|---:|---|
| `small` | Whisper Small | `ggml-small.bin` | https://blob.handy.computer/ggml-small.bin | ~488 MB | `1be3a9b2063867b937e64e2ec7483364a79917e157fa98c5d94b5c1fffea987b` |
| `medium` | Whisper Medium (Q4_1) | `whisper-medium-q4_1.bin` *(aka `ggml-medium-q4_1.bin`)* | https://blob.handy.computer/whisper-medium-q4_1.bin | ~492 MB | `79283fc1f9fe12ca3248543fbd54b73292164d8df5a16e095e2bceeaaabddf57` |
| `turbo` | Whisper Large v3 Turbo | `ggml-large-v3-turbo.bin` | https://blob.handy.computer/ggml-large-v3-turbo.bin | ~1.62 GB | `1fc70f774d38eb169993ac391eea357ef47c88757ef72ee5943879b7e8e2bc69` |
| `large` | Whisper Large v3 (Q5_0) | `ggml-large-v3-q5_0.bin` | https://blob.handy.computer/ggml-large-v3-q5_0.bin | ~1.08 GB | `d75795ecff3f83b5faa89d1900604ad8c780abd5739fae406de19f23ecd98ad1` |
| `parakeet-tdt-0.6b-v3` | Parakeet TDT 0.6B V3 INT8 *(archive)* | `parakeet-v3-int8.tar.gz` | https://blob.handy.computer/parakeet-v3-int8.tar.gz | 478 517 071 B | `43d37191602727524a7d8c6da0eef11c4ba24320f5b4730f1a2497befc2efa77` |

**Parakeet INT8 component hashes (inside the tarball after extraction):**

- `encoder-model.int8.onnx` → `6139d2fa7e1b086097b277c7149725edbab89cc7c7ae64b23c741be4055aff09`  
- `decoder_joint-model.int8.onnx` → `eea7483ee3d1a30375daedc8ed83e3960c91b098812127a0d99d1c8977667a70`  
- `nemo128.onnx` → `a9fde1486ebfcc08f328d75ad4610c67835fea58c73ba57e3209a6f6cf019e9f`  

> **Size note:** Upstream Parakeet INT8 components sum to ~670 MB extracted; the **tar.gz size** depends on compression and metadata and may be higher.

---

## **Model Details**

### Whisper Small
- **ID**: `small`
- **Filename**: `ggml-small.bin`
- **SHA‑256**: `1be3a9b2063867b937e64e2ec7483364a79917e157fa98c5d94b5c1fffea987b`
- **Use**: quick transcription; CPU‑friendly.

### Whisper Medium (Q4_1)
- **ID**: `medium`
- **Filename**: `whisper-medium-q4_1.bin` *(aka `ggml-medium-q4_1.bin`)*
- **SHA‑256**: `79283fc1f9fe12ca3248543fbd54b73292164d8df5a16e095e2bceeaaabddf57`
- **Use**: balanced accuracy/speed.

### Whisper Large v3 Turbo
- **ID**: `turbo`
- **Filename**: `ggml-large-v3-turbo.bin`
- **SHA‑256**: `1fc70f774d38eb169993ac391eea357ef47c88757ef72ee5943879b7e8e2bc69`
- **Use**: high‑quality, optimized performance.

### Whisper Large v3 (Q5_0)
- **ID**: `large`
- **Filename**: `ggml-large-v3-q5_0.bin`
- **SHA‑256**: `d75795ecff3f83b5faa89d1900604ad8c780abd5739fae406de19f23ecd98ad1`
- **Use**: max accuracy; slower.

### Parakeet V3 INT8 (archive + components)
- **ID**: `parakeet-tdt-0.6b-v3`
- **Tarball**: `parakeet-v3-int8.tar.gz` → SHA‑256 `43d37191602727524a7d8c6da0eef11c4ba24320f5b4730f1a2497befc2efa77` (measured from https://blob.handy.computer on 2025‑09‑26).  
- **Component SHA‑256** *(verify after extraction)*:
  - `encoder-model.int8.onnx` → `6139d2fa7e1b086097b277c7149725edbab89cc7c7ae64b23c741be4055aff09`
  - `decoder_joint-model.int8.onnx` → `eea7483ee3d1a30375daedc8ed83e3960c91b098812127a0d99d1c8977667a70`
  - `nemo128.onnx` → `a9fde1486ebfcc08f328d75ad4610c67835fea58c73ba57e3209a6f6cf019e9f`

---

## **Manifest snippet** (`src-tauri/resources/models/manifest.json`)

```json
{
  "models": [
    {
      "id": "small",
      "name": "Whisper Small",
      "filename": "ggml-small.bin",
      "url": "https://blob.handy.computer/ggml-small.bin",
      "sha256": "1be3a9b2063867b937e64e2ec7483364a79917e157fa98c5d94b5c1fffea987b"
    },
    {
      "id": "medium",
      "name": "Whisper Medium (Q4_1)",
      "filename": "whisper-medium-q4_1.bin",
      "url": "https://blob.handy.computer/whisper-medium-q4_1.bin",
      "sha256": "79283fc1f9fe12ca3248543fbd54b73292164d8df5a16e095e2bceeaaabddf57"
    },
    {
      "id": "turbo",
      "name": "Whisper Large v3 Turbo",
      "filename": "ggml-large-v3-turbo.bin",
      "url": "https://blob.handy.computer/ggml-large-v3-turbo.bin",
      "sha256": "1fc70f774d38eb169993ac391eea357ef47c88757ef72ee5943879b7e8e2bc69"
    },
    {
      "id": "large",
      "name": "Whisper Large v3 (Q5_0)",
      "filename": "ggml-large-v3-q5_0.bin",
      "url": "https://blob.handy.computer/ggml-large-v3-q5_0.bin",
      "sha256": "d75795ecff3f83b5faa89d1900604ad8c780abd5739fae406de19f23ecd98ad1"
    },
    {
      "id": "parakeet-tdt-0.6b-v3",
      "name": "Parakeet V3 INT8 (archive)",
      "filename": "parakeet-v3-int8.tar.gz",
      "url": "https://blob.handy.computer/parakeet-v3-int8.tar.gz",
      "sha256": "43d37191602727524a7d8c6da0eef11c4ba24320f5b4730f1a2497befc2efa77",
      "components": [
        {"path": "encoder-model.int8.onnx", "sha256": "6139d2fa7e1b086097b277c7149725edbab89cc7c7ae64b23c741be4055aff09"},
        {"path": "decoder_joint-model.int8.onnx", "sha256": "eea7483ee3d1a30375daedc8ed83e3960c91b098812127a0d99d1c8977667a70"},
        {"path": "nemo128.onnx", "sha256": "a9fde1486ebfcc08f328d75ad4610c67835fea58c73ba57e3209a6f6cf019e9f"}
      ]
    }
  ]
}
```

---

## **Linux verification commands**

**Option A — download to disk and verify**

```bash
# Whisper Small
curl -L -o ggml-small.bin https://blob.handy.computer/ggml-small.bin
echo "1be3a9b2063867b937e64e2ec7483364a79917e157fa98c5d94b5c1fffea987b  ggml-small.bin" | sha256sum -c -

# Whisper Medium (Q4_1)
curl -L -o whisper-medium-q4_1.bin https://blob.handy.computer/whisper-medium-q4_1.bin
echo "79283fc1f9fe12ca3248543fbd54b73292164d8df5a16e095e2bceeaaabddf57  whisper-medium-q4_1.bin" | sha256sum -c -

# Whisper Large v3 Turbo
curl -L -o ggml-large-v3-turbo.bin https://blob.handy.computer/ggml-large-v3-turbo.bin
echo "1fc70f774d38eb169993ac391eea357ef47c88757ef72ee5943879b7e8e2bc69  ggml-large-v3-turbo.bin" | sha256sum -c -

# Whisper Large v3 (Q5_0)
curl -L -o ggml-large-v3-q5_0.bin https://blob.handy.computer/ggml-large-v3-q5_0.bin
echo "d75795ecff3f83b5faa89d1900604ad8c780abd5739fae406de19f23ecd98ad1  ggml-large-v3-q5_0.bin" | sha256sum -c -
```

**Option B — stream and inspect (no file saved)**

```bash
curl -L https://blob.handy.computer/ggml-large-v3-turbo.bin | sha256sum
# Compare the printed hash to: 1fc70f774d38eb169993ac391eea357ef47c88757ef72ee5943879b7e8e2bc69
```

**Verify Parakeet archive + contents**

```bash
# 1) Download the tarball and compute its SHA-256 (record this value into the manifest)
curl -L -o parakeet-v3-int8.tar.gz https://blob.handy.computer/parakeet-v3-int8.tar.gz
echo "43d37191602727524a7d8c6da0eef11c4ba24320f5b4730f1a2497befc2efa77  parakeet-v3-int8.tar.gz" | sha256sum -c -

# 2) Extract safely and verify component files
mkdir -p parakeet-tdt-0.6b-v3-int8 && tar -xzf parakeet-v3-int8.tar.gz -C parakeet-tdt-0.6b-v3-int8
cd parakeet-tdt-0.6b-v3-int8

# Adjust paths below if the archive structure differs
cat > checks.txt <<'EOF'
6139d2fa7e1b086097b277c7149725edbab89cc7c7ae64b23c741be4055aff09  encoder-model.int8.onnx
eea7483ee3d1a30375daedc8ed83e3960c91b098812127a0d99d1c8977667a70  decoder_joint-model.int8.onnx
a9fde1486ebfcc08f328d75ad4610c67835fea58c73ba57e3209a6f6cf019e9f  nemo128.onnx
EOF
sha256sum -c checks.txt
```

---

## **References**

- Whisper **Small** (`ggml-small.bin`) — Hugging Face “Large File Pointer Details”. https://huggingface.co/ggerganov/whisper.cpp/blob/main/ggml-small.bin  
- Whisper **Medium (Q4_1)** (`ggml/whisper-medium-q4_1`) — Pomni repo commit showing LFS SHA‑256 + size. https://huggingface.co/Pomni/whisper-medium-ggml-allquants/commit/f45b7052306b3c6e6a68a541159a8578880daedb  
- Whisper **Large v3 Turbo** (`ggml-large-v3-turbo.bin`) — Hugging Face “Large File Pointer Details”. https://huggingface.co/ggerganov/whisper.cpp/blob/main/ggml-large-v3-turbo.bin  
- Whisper **Large v3 (Q5_0)** (`ggml-large-v3-q5_0.bin`) — Hugging Face “Large File Pointer Details”. https://huggingface.co/ggerganov/whisper.cpp/blob/main/ggml-large-v3-q5_0.bin  
- Parakeet V3 **INT8 components** — Hugging Face files with “Large File Pointer Details”:  
  - encoder INT8 — https://huggingface.co/istupakov/parakeet-tdt-0.6b-v3-onnx/blob/main/encoder-model.int8.onnx  
  - decoder‑joint INT8 — https://huggingface.co/istupakov/parakeet-tdt-0.6b-v3-onnx/blob/main/decoder_joint-model.int8.onnx  
  - nemo128 — https://huggingface.co/istupakov/parakeet-tdt-0.6b-v3-onnx/blob/main/nemo128.onnx  

---

*Last updated: 2025‑09‑26*  
*Maintainer: Handy Project*  
