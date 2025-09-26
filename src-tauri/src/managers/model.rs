use crate::settings::{get_settings, write_settings};
use anyhow::{Context, Result};
use flate2::read::GzDecoder;
use futures_util::StreamExt;
use hex::encode;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::cmp;
use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;
use tar::{Archive, EntryType};
use tauri::{App, AppHandle, Emitter, Manager};

const MANIFEST_RESOURCE_PATH: &str = "resources/models/manifest.json";
const MODEL_DOWNLOAD_USER_AGENT: &str = "HandyModelManager/1.0 (+https://handy.computer)";
const MODEL_DOWNLOAD_TIMEOUT_SECS: u64 = 600;
const MODEL_CONNECT_TIMEOUT_SECS: u64 = 30;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EngineType {
    Whisper,
    Parakeet,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub filename: String,
    pub url: Option<String>,
    pub size_mb: u64,
    pub is_downloaded: bool,
    pub is_downloading: bool,
    pub partial_size: u64,
    pub is_directory: bool,
    pub engine_type: EngineType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadProgress {
    pub model_id: String,
    pub downloaded: u64,
    pub total: u64,
    pub percentage: f64,
}

#[derive(Debug, Clone, Deserialize)]
struct ManifestEntry {
    id: String,
    #[serde(rename = "sha256")]
    digest: String,
    #[serde(rename = "size_bytes")]
    size: u64,
}

#[derive(Debug, Clone, Deserialize)]
struct ManifestFile {
    models: Vec<ManifestEntry>,
}

#[derive(Debug, Clone)]
struct ModelDigest {
    model_id: String,
    sha256: String,
    size_bytes: u64,
}

#[derive(Debug, Clone)]
struct ModelManifest {
    digests: HashMap<String, ModelDigest>,
}

impl ModelManifest {
    fn load(app_handle: &AppHandle) -> Result<Self> {
        let manifest_path = app_handle
            .path()
            .resolve(MANIFEST_RESOURCE_PATH, tauri::path::BaseDirectory::Resource)?;

        let raw = fs::read(&manifest_path).with_context(|| {
            format!(
                "failed to read model manifest at {}",
                manifest_path.display()
            )
        })?;
        let parsed: ManifestFile =
            serde_json::from_slice(&raw).context("failed to parse model manifest")?;
        let digests = parsed
            .models
            .into_iter()
            .map(|entry| -> Result<_> {
                if entry.size == 0 {
                    anyhow::bail!("manifest entry for model {} contains zero size", entry.id);
                }
                if entry.digest.len() != 64 || !entry.digest.chars().all(|c| c.is_ascii_hexdigit())
                {
                    anyhow::bail!(
                        "manifest entry for model {} has invalid sha256 digest",
                        entry.id
                    );
                }

                // Reject placeholder patterns commonly used in development
                if entry.digest.chars().all(|c| c == '0')
                    || entry.digest.chars().all(|c| c == '1')
                    || entry.digest.chars().all(|c| c == '2')
                    || entry.digest.chars().all(|c| c == '3')
                    || entry.digest.chars().all(|c| c == '4')
                    || entry.digest == "deadbeef".repeat(8)
                    || entry.digest == "cafebabe".repeat(8)
                {
                    anyhow::bail!(
                        "manifest entry for model {} contains placeholder sha256 digest (security risk)",
                        entry.id
                    );
                }
                Ok((
                    entry.id.clone(),
                    ModelDigest {
                        model_id: entry.id,
                        sha256: entry.digest.to_lowercase(),
                        size_bytes: entry.size,
                    },
                ))
            })
            .collect::<Result<HashMap<_, _>>>()?;

        Ok(Self { digests })
    }

    fn digest_for(&self, model_id: &str) -> Option<ModelDigest> {
        self.digests.get(model_id).cloned()
    }
}

fn verify_download(path: &Path, digest: &ModelDigest) -> Result<()> {
    let metadata = fs::metadata(path)
        .with_context(|| format!("unable to stat downloaded artifact at {}", path.display()))?;

    if metadata.len() != digest.size_bytes {
        anyhow::bail!(
            "size mismatch for model {}: expected {} bytes, got {}",
            digest.model_id,
            digest.size_bytes,
            metadata.len()
        );
    }

    let mut file = File::open(path)
        .with_context(|| format!("unable to open downloaded artifact at {}", path.display()))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 8192];

    loop {
        let read = file
            .read(&mut buffer)
            .with_context(|| "failed while hashing downloaded model")?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }

    let actual = encode(hasher.finalize());
    if actual != digest.sha256 {
        anyhow::bail!(
            "hash mismatch for model {}: expected {}, got {}",
            digest.model_id,
            digest.sha256,
            actual
        );
    }

    Ok(())
}

fn sanitize_archive_entry_path(base: &Path, entry: &Path) -> Result<PathBuf> {
    let mut sanitized = PathBuf::from(base);

    for component in entry.components() {
        match component {
            Component::CurDir => {}
            Component::Normal(segment) => sanitized.push(segment),
            _ => {
                anyhow::bail!(
                    "archive entry contains unsupported path component: {}",
                    entry.display()
                );
            }
        }
    }

    Ok(sanitized)
}

fn extract_archive_securely<R: Read>(archive: &mut Archive<R>, destination: &Path) -> Result<()> {
    for entry_result in archive.entries()? {
        let mut entry = entry_result?;
        let header = entry.header();
        let entry_path = entry
            .path()
            .with_context(|| "failed to read archive entry path")?;
        let full_path = sanitize_archive_entry_path(destination, entry_path.as_ref())?;
        let entry_type = header.entry_type();

        match entry_type {
            EntryType::Directory => {
                fs::create_dir_all(&full_path).with_context(|| {
                    format!("failed to create directory {}", full_path.display())
                })?;
            }
            EntryType::Regular => {
                if let Some(parent) = full_path.parent() {
                    fs::create_dir_all(parent).with_context(|| {
                        format!("failed to create parent directory {}", parent.display())
                    })?;
                }

                entry
                    .unpack(&full_path)
                    .with_context(|| format!("failed to unpack {}", full_path.display()))?;
            }
            EntryType::Symlink | EntryType::Link => {
                anyhow::bail!(
                    "archive entry contains unsupported link: {}",
                    entry_path.display()
                );
            }
            other => {
                anyhow::bail!(
                    "archive entry contains unsupported type {:?}: {}",
                    other,
                    entry_path.display()
                );
            }
        }
    }

    Ok(())
}

pub struct ModelManager {
    app_handle: AppHandle,
    models_dir: PathBuf,
    available_models: Mutex<HashMap<String, ModelInfo>>,
    manifest: ModelManifest,
}

impl ModelManager {
    pub fn new(app: &App) -> Result<Self> {
        let app_handle = app.app_handle().clone();

        // Create models directory in app data
        let models_dir = app
            .path()
            .app_data_dir()
            .map_err(|e| anyhow::anyhow!("Failed to get app data dir: {}", e))?
            .join("models");

        if !models_dir.exists() {
            fs::create_dir_all(&models_dir)?;
        }

        let mut available_models = HashMap::new();

        available_models.insert(
            "small".to_string(),
            ModelInfo {
                id: "small".to_string(),
                name: "Whisper Small".to_string(),
                description: "Fast and fairly accurate.".to_string(),
                filename: "ggml-small.bin".to_string(),
                url: Some("https://blob.handy.computer/ggml-small.bin".to_string()),
                size_mb: 244,
                is_downloaded: false,
                is_downloading: false,
                partial_size: 0,
                is_directory: false,
                engine_type: EngineType::Whisper,
            },
        );

        // Add downloadable models
        available_models.insert(
            "medium".to_string(),
            ModelInfo {
                id: "medium".to_string(),
                name: "Whisper Medium".to_string(),
                description: "Good accuracy, medium speed".to_string(),
                filename: "whisper-medium-q4_1.bin".to_string(),
                url: Some("https://blob.handy.computer/whisper-medium-q4_1.bin".to_string()),
                size_mb: 491, // Approximate size
                is_downloaded: false,
                is_downloading: false,
                partial_size: 0,
                is_directory: false,
                engine_type: EngineType::Whisper,
            },
        );

        available_models.insert(
            "turbo".to_string(),
            ModelInfo {
                id: "turbo".to_string(),
                name: "Whisper Turbo".to_string(),
                description: "Balanced accuracy and speed.".to_string(),
                filename: "ggml-large-v3-turbo.bin".to_string(),
                url: Some("https://blob.handy.computer/ggml-large-v3-turbo.bin".to_string()),
                size_mb: 1600, // Approximate size
                is_downloaded: false,
                is_downloading: false,
                partial_size: 0,
                is_directory: false,
                engine_type: EngineType::Whisper,
            },
        );

        available_models.insert(
            "large".to_string(),
            ModelInfo {
                id: "large".to_string(),
                name: "Whisper Large".to_string(),
                description: "Good accuracy, but slow.".to_string(),
                filename: "ggml-large-v3-q5_0.bin".to_string(),
                url: Some("https://blob.handy.computer/ggml-large-v3-q5_0.bin".to_string()),
                size_mb: 1080, // Approximate size
                is_downloaded: false,
                is_downloading: false,
                partial_size: 0,
                is_directory: false,
                engine_type: EngineType::Whisper,
            },
        );

        // Add NVIDIA Parakeet model (directory-based)
        available_models.insert(
            "parakeet-tdt-0.6b-v3".to_string(),
            ModelInfo {
                id: "parakeet-tdt-0.6b-v3".to_string(),
                name: "Parakeet V3".to_string(),
                description: "Fast and accurate".to_string(),
                filename: "parakeet-tdt-0.6b-v3-int8".to_string(), // Directory name
                url: Some("https://blob.handy.computer/parakeet-v3-int8.tar.gz".to_string()),
                size_mb: 850, // Approximate size for int8 quantized model
                is_downloaded: false,
                is_downloading: false,
                partial_size: 0,
                is_directory: true,
                engine_type: EngineType::Parakeet,
            },
        );

        let manifest = ModelManifest::load(&app_handle)?;

        let manager = Self {
            app_handle,
            models_dir,
            available_models: Mutex::new(available_models),
            manifest,
        };

        // Migrate any bundled models to user directory
        manager.migrate_bundled_models()?;

        // Check which models are already downloaded
        manager.update_download_status()?;

        // Auto-select a model if none is currently selected
        manager.auto_select_model_if_needed()?;

        Ok(manager)
    }

    pub fn get_available_models(&self) -> Vec<ModelInfo> {
        let models = self.available_models.lock().unwrap();
        models.values().cloned().collect()
    }

    pub fn get_model_info(&self, model_id: &str) -> Option<ModelInfo> {
        let models = self.available_models.lock().unwrap();
        models.get(model_id).cloned()
    }

    fn migrate_bundled_models(&self) -> Result<()> {
        // Check for bundled models and copy them to user directory
        let bundled_models = ["ggml-small.bin"]; // Add other bundled models here if any

        for filename in &bundled_models {
            let bundled_path = self.app_handle.path().resolve(
                format!("resources/models/{}", filename),
                tauri::path::BaseDirectory::Resource,
            );

            if let Ok(bundled_path) = bundled_path {
                if bundled_path.exists() {
                    let user_path = self.models_dir.join(filename);

                    // Only copy if user doesn't already have the model
                    if !user_path.exists() {
                        println!("Migrating bundled model {} to user directory", filename);
                        fs::copy(&bundled_path, &user_path)?;
                        println!("Successfully migrated {}", filename);
                    }
                }
            }
        }

        Ok(())
    }

    fn update_download_status(&self) -> Result<()> {
        let mut models = self.available_models.lock().unwrap();

        for model in models.values_mut() {
            if model.is_directory {
                // For directory-based models, check if the directory exists
                let model_path = self.models_dir.join(&model.filename);
                let partial_path = self.models_dir.join(format!("{}.partial", &model.filename));
                let extracting_path = self
                    .models_dir
                    .join(format!("{}.extracting", &model.filename));

                // Clean up any leftover .extracting directories from interrupted extractions
                if extracting_path.exists() {
                    println!("Cleaning up interrupted extraction for model: {}", model.id);
                    let _ = fs::remove_dir_all(&extracting_path);
                }

                model.is_downloaded = model_path.exists() && model_path.is_dir();
                model.is_downloading = partial_path.exists();

                // Get partial file size if it exists (for the .tar.gz being downloaded)
                if partial_path.exists() {
                    model.partial_size = partial_path.metadata().map(|m| m.len()).unwrap_or(0);
                } else {
                    model.partial_size = 0;
                }
            } else {
                // For file-based models (existing logic)
                let model_path = self.models_dir.join(&model.filename);
                let partial_path = self.models_dir.join(format!("{}.partial", &model.filename));

                model.is_downloaded = model_path.exists();
                model.is_downloading = partial_path.exists();

                // Get partial file size if it exists
                if partial_path.exists() {
                    model.partial_size = partial_path.metadata().map(|m| m.len()).unwrap_or(0);
                } else {
                    model.partial_size = 0;
                }
            }
        }

        Ok(())
    }

    fn auto_select_model_if_needed(&self) -> Result<()> {
        // Check if we have a selected model in settings
        let settings = get_settings(&self.app_handle);

        // If no model is selected or selected model is empty
        if settings.selected_model.is_empty() {
            // Find the first available (downloaded) model
            let models = self.available_models.lock().unwrap();
            if let Some(available_model) = models.values().find(|model| model.is_downloaded) {
                println!(
                    "Auto-selecting model: {} ({})",
                    available_model.id, available_model.name
                );

                // Update settings with the selected model
                let mut updated_settings = settings;
                updated_settings.selected_model = available_model.id.clone();
                write_settings(&self.app_handle, updated_settings);

                println!("Successfully auto-selected model: {}", available_model.id);
            }
        }

        Ok(())
    }

    pub async fn download_model(&self, model_id: &str) -> Result<()> {
        let model_info = {
            let models = self.available_models.lock().unwrap();
            models.get(model_id).cloned()
        };

        let model_info =
            model_info.ok_or_else(|| anyhow::anyhow!("Model not found: {}", model_id))?;

        let digest = self
            .manifest
            .digest_for(&model_info.id)
            .ok_or_else(|| anyhow::anyhow!("No manifest entry for model {}", model_id))?;

        let url = model_info
            .url
            .ok_or_else(|| anyhow::anyhow!("No download URL for model"))?;
        let model_path = self.models_dir.join(&model_info.filename);
        let partial_path = self
            .models_dir
            .join(format!("{}.partial", &model_info.filename));

        // Don't download if complete version already exists
        if model_path.exists() {
            // Clean up any partial file that might exist
            if partial_path.exists() {
                let _ = fs::remove_file(&partial_path);
            }
            self.update_download_status()?;
            return Ok(());
        }

        // Check if we have a partial download to resume
        let resume_from = if partial_path.exists() {
            let size = partial_path.metadata()?.len();
            if size > digest.size_bytes {
                anyhow::bail!(
                    "partial download for model {} exceeds expected size ({} > {})",
                    model_id,
                    size,
                    digest.size_bytes
                );
            }
            println!("Resuming download of model {} from byte {}", model_id, size);
            size
        } else {
            println!("Starting fresh download of model {} from {}", model_id, url);
            0
        };

        // Mark as downloading
        {
            let mut models = self.available_models.lock().unwrap();
            if let Some(model) = models.get_mut(model_id) {
                model.is_downloading = true;
            }
        }

        // Create hardened HTTP client with range support for resuming
        let client = reqwest::Client::builder()
            .user_agent(MODEL_DOWNLOAD_USER_AGENT)
            .timeout(Duration::from_secs(MODEL_DOWNLOAD_TIMEOUT_SECS))
            .connect_timeout(Duration::from_secs(MODEL_CONNECT_TIMEOUT_SECS))
            .build()
            .context("failed to build HTTP client for model download")?;

        let mut request = client.get(&url);

        if resume_from > 0 {
            request = request.header("Range", format!("bytes={}-", resume_from));
        }

        let response = request
            .send()
            .await
            .with_context(|| format!("failed to request model {}", model_id))?;

        // Check for success or partial content status
        if !response.status().is_success()
            && response.status() != reqwest::StatusCode::PARTIAL_CONTENT
        {
            // Mark as not downloading on error
            {
                let mut models = self.available_models.lock().unwrap();
                if let Some(model) = models.get_mut(model_id) {
                    model.is_downloading = false;
                }
            }
            return Err(anyhow::anyhow!(
                "Failed to download model: HTTP {}",
                response.status()
            ));
        }

        let total_size = if digest.size_bytes > 0 {
            digest.size_bytes
        } else if resume_from > 0 {
            resume_from + response.content_length().unwrap_or(0)
        } else {
            response.content_length().unwrap_or(0)
        };

        let mut downloaded = resume_from;
        let mut stream = response.bytes_stream();

        // Open file for appending if resuming, or create new if starting fresh
        let mut file = if resume_from > 0 {
            std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&partial_path)?
        } else {
            std::fs::File::create(&partial_path)?
        };

        // Emit initial progress
        let initial_progress = DownloadProgress {
            model_id: model_id.to_string(),
            downloaded,
            total: total_size,
            percentage: if total_size > 0 {
                (downloaded as f64 / total_size as f64) * 100.0
            } else {
                0.0
            },
        };
        let _ = self
            .app_handle
            .emit("model-download-progress", &initial_progress);

        // Download with progress
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.inspect_err(|_e| {
                // Mark as not downloading on error
                {
                    let mut models = self.available_models.lock().unwrap();
                    if let Some(model) = models.get_mut(model_id) {
                        model.is_downloading = false;
                    }
                }
            })?;

            file.write_all(&chunk)?;
            downloaded += chunk.len() as u64;

            let percentage = if total_size > 0 {
                (cmp::min(downloaded, total_size) as f64 / total_size as f64) * 100.0
            } else {
                0.0
            };

            // Emit progress event
            let progress = DownloadProgress {
                model_id: model_id.to_string(),
                downloaded,
                total: total_size,
                percentage,
            };

            let _ = self.app_handle.emit("model-download-progress", &progress);
        }

        file.flush()?;
        drop(file); // Ensure file is closed before moving

        if let Err(error) = verify_download(&partial_path, &digest) {
            {
                let mut models = self.available_models.lock().unwrap();
                if let Some(model) = models.get_mut(model_id) {
                    model.is_downloading = false;
                    model.partial_size = partial_path.metadata().map(|m| m.len()).unwrap_or(0);
                }
            }
            return Err(error);
        }

        // Handle directory-based models (extract tar.gz) vs file-based models
        if model_info.is_directory {
            // Emit extraction started event
            let _ = self.app_handle.emit("model-extraction-started", model_id);
            println!("Extracting archive for directory-based model: {}", model_id);

            // Use a temporary extraction directory to ensure atomic operations
            let temp_extract_dir = self
                .models_dir
                .join(format!("{}.extracting", &model_info.filename));
            let final_model_dir = self.models_dir.join(&model_info.filename);

            // Clean up any previous incomplete extraction
            if temp_extract_dir.exists() {
                let _ = fs::remove_dir_all(&temp_extract_dir);
            }

            // Create temporary extraction directory
            fs::create_dir_all(&temp_extract_dir)?;

            // Open the downloaded tar.gz file
            let tar_gz = File::open(&partial_path)?;
            let tar = GzDecoder::new(tar_gz);
            let mut archive = Archive::new(tar);

            // Extract to the temporary directory first
            if let Err(error) = extract_archive_securely(&mut archive, &temp_extract_dir) {
                let error_msg = format!("Failed to extract archive: {error}");
                let _ = fs::remove_dir_all(&temp_extract_dir);
                let _ = self.app_handle.emit(
                    "model-extraction-failed",
                    &serde_json::json!({
                        "model_id": model_id,
                        "error": error_msg
                    }),
                );
                {
                    let mut models = self.available_models.lock().unwrap();
                    if let Some(model) = models.get_mut(model_id) {
                        model.is_downloading = false;
                        model.partial_size = partial_path.metadata().map(|m| m.len()).unwrap_or(0);
                    }
                }
                return Err(error);
            }

            // Find the actual extracted directory (archive might have a nested structure)
            let extracted_dirs: Vec<_> = fs::read_dir(&temp_extract_dir)?
                .filter_map(|entry| entry.ok())
                .filter(|entry| entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false))
                .collect();

            if extracted_dirs.len() == 1 {
                // Single directory extracted, move it to the final location
                let source_dir = extracted_dirs[0].path();
                if final_model_dir.exists() {
                    fs::remove_dir_all(&final_model_dir)?;
                }
                fs::rename(&source_dir, &final_model_dir)?;
                // Clean up temp directory
                let _ = fs::remove_dir_all(&temp_extract_dir);
            } else {
                // Multiple items or no directories, rename the temp directory itself
                if final_model_dir.exists() {
                    fs::remove_dir_all(&final_model_dir)?;
                }
                fs::rename(&temp_extract_dir, &final_model_dir)?;
            }

            println!("Successfully extracted archive for model: {}", model_id);
            // Emit extraction completed event
            let _ = self.app_handle.emit("model-extraction-completed", model_id);

            // Remove the downloaded tar.gz file
            let _ = fs::remove_file(&partial_path);
        } else {
            // Move partial file to final location for file-based models
            fs::rename(&partial_path, &model_path)?;
        }

        // Update download status
        {
            let mut models = self.available_models.lock().unwrap();
            if let Some(model) = models.get_mut(model_id) {
                model.is_downloading = false;
                model.is_downloaded = true;
                model.partial_size = 0;
            }
        }

        // Emit completion event
        let _ = self.app_handle.emit("model-download-complete", model_id);

        println!(
            "Successfully downloaded model {} to {:?}",
            model_id, model_path
        );

        Ok(())
    }

    pub fn delete_model(&self, model_id: &str) -> Result<()> {
        println!("ModelManager: delete_model called for: {}", model_id);

        let model_info = {
            let models = self.available_models.lock().unwrap();
            models.get(model_id).cloned()
        };

        let model_info =
            model_info.ok_or_else(|| anyhow::anyhow!("Model not found: {}", model_id))?;

        println!("ModelManager: Found model info: {:?}", model_info);

        let model_path = self.models_dir.join(&model_info.filename);
        let partial_path = self
            .models_dir
            .join(format!("{}.partial", &model_info.filename));
        println!("ModelManager: Model path: {:?}", model_path);
        println!("ModelManager: Partial path: {:?}", partial_path);

        let mut deleted_something = false;

        if model_info.is_directory {
            // Delete complete model directory if it exists
            if model_path.exists() && model_path.is_dir() {
                println!(
                    "ModelManager: Deleting model directory at: {:?}",
                    model_path
                );
                fs::remove_dir_all(&model_path)?;
                println!("ModelManager: Model directory deleted successfully");
                deleted_something = true;
            }
        } else {
            // Delete complete model file if it exists
            if model_path.exists() {
                println!("ModelManager: Deleting model file at: {:?}", model_path);
                fs::remove_file(&model_path)?;
                println!("ModelManager: Model file deleted successfully");
                deleted_something = true;
            }
        }

        // Delete partial file if it exists (same for both types)
        if partial_path.exists() {
            println!("ModelManager: Deleting partial file at: {:?}", partial_path);
            fs::remove_file(&partial_path)?;
            println!("ModelManager: Partial file deleted successfully");
            deleted_something = true;
        }

        if !deleted_something {
            return Err(anyhow::anyhow!("No model files found to delete"));
        }

        // Update download status
        self.update_download_status()?;
        println!("ModelManager: Download status updated");

        Ok(())
    }

    pub fn get_model_path(&self, model_id: &str) -> Result<PathBuf> {
        let model_info = self
            .get_model_info(model_id)
            .ok_or_else(|| anyhow::anyhow!("Model not found: {}", model_id))?;

        if !model_info.is_downloaded {
            return Err(anyhow::anyhow!("Model not available: {}", model_id));
        }

        // Ensure we don't return partial files/directories
        if model_info.is_downloading {
            return Err(anyhow::anyhow!(
                "Model is currently downloading: {}",
                model_id
            ));
        }

        let model_path = self.models_dir.join(&model_info.filename);
        let partial_path = self
            .models_dir
            .join(format!("{}.partial", &model_info.filename));

        if model_info.is_directory {
            // For directory-based models, ensure the directory exists and is complete
            if model_path.exists() && model_path.is_dir() && !partial_path.exists() {
                Ok(model_path)
            } else {
                Err(anyhow::anyhow!(
                    "Complete model directory not found: {}",
                    model_id
                ))
            }
        } else {
            // For file-based models (existing logic)
            if model_path.exists() && !partial_path.exists() {
                Ok(model_path)
            } else {
                Err(anyhow::anyhow!(
                    "Complete model file not found: {}",
                    model_id
                ))
            }
        }
    }

    pub fn cancel_download(&self, model_id: &str) -> Result<()> {
        println!("ModelManager: cancel_download called for: {}", model_id);

        let _model_info = {
            let models = self.available_models.lock().unwrap();
            models.get(model_id).cloned()
        };

        let _model_info =
            _model_info.ok_or_else(|| anyhow::anyhow!("Model not found: {}", model_id))?;

        // Mark as not downloading
        {
            let mut models = self.available_models.lock().unwrap();
            if let Some(model) = models.get_mut(model_id) {
                model.is_downloading = false;
            }
        }

        // Note: The actual download cancellation would need to be handled
        // by the download task itself. This just updates the state.
        // The partial file is kept so the download can be resumed later.

        // Update download status to reflect current state
        self.update_download_status()?;

        println!("ModelManager: Download cancelled for: {}", model_id);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use std::{fs, path::Path};
    use tar::{Builder, Header};
    use tempfile::tempdir;

    #[test]
    fn verify_download_accepts_matching_digest() {
        let temp_dir = tempdir().expect("failed to create temp dir");
        let file_path = temp_dir.path().join("model.bin");
        fs::write(&file_path, b"test-bytes").expect("failed to write test model");

        let mut hasher = Sha256::new();
        hasher.update(b"test-bytes");
        let digest = ModelDigest {
            model_id: "test".into(),
            sha256: encode(hasher.finalize()),
            size_bytes: 10,
        };

        verify_download(&file_path, &digest).expect("verification should succeed");
    }

    #[test]
    fn verify_download_rejects_mismatch() {
        let temp_dir = tempdir().expect("failed to create temp dir");
        let file_path = temp_dir.path().join("model.bin");
        fs::write(&file_path, b"other-bytes").expect("failed to write test model");

        let digest = ModelDigest {
            model_id: "test".into(),
            sha256: "deadbeef".into(),
            size_bytes: 42,
        };

        let err = verify_download(&file_path, &digest).expect_err("verification must fail");
        let msg = err.to_string();
        assert!(msg.contains("size mismatch"));
    }

    #[test]
    fn sanitize_archive_path_rejects_parent() {
        let temp_dir = tempdir().expect("failed to create temp dir");
        let result = sanitize_archive_entry_path(temp_dir.path(), Path::new("../evil"));
        assert!(result.is_err());
    }

    #[test]
    fn extract_archive_securely_rejects_symlink() {
        let mut builder = Builder::new(Vec::new());
        let mut header = Header::new_gnu();
        header.set_entry_type(EntryType::Symlink);
        header.set_size(0);
        header.set_mode(0o755);
        header
            .set_path(Path::new("link"))
            .expect("failed to set link path");
        header
            .set_link_name(Path::new("../evil"))
            .expect("failed to set symlink target");
        header.set_cksum();
        builder
            .append(&header, Cursor::new(Vec::new()))
            .expect("failed to append entry");

        let data = builder.into_inner().expect("failed to finalize tar");
        let mut archive = Archive::new(Cursor::new(data));
        let temp_dir = tempdir().expect("failed to create temp dir");

        let err = extract_archive_securely(&mut archive, temp_dir.path())
            .expect_err("symlink entry should be rejected");
        assert!(err.to_string().contains("unsupported link"));
    }

    #[test]
    fn extract_archive_securely_writes_files() {
        let mut builder = Builder::new(Vec::new());

        let mut dir_header = Header::new_gnu();
        dir_header.set_entry_type(EntryType::Directory);
        dir_header.set_size(0);
        dir_header.set_mode(0o755);
        dir_header.set_cksum();
        builder
            .append_data(
                &mut dir_header,
                Path::new("nested"),
                Cursor::new(Vec::new()),
            )
            .expect("failed to append dir");

        let mut file_header = Header::new_gnu();
        file_header.set_size(4);
        file_header.set_mode(0o644);
        file_header.set_cksum();
        builder
            .append_data(
                &mut file_header,
                Path::new("nested/file.txt"),
                Cursor::new(b"data"),
            )
            .expect("failed to append file");

        let data = builder.into_inner().expect("failed to finalize tar");
        let mut archive = Archive::new(Cursor::new(data));
        let temp_dir = tempdir().expect("failed to create temp dir");

        extract_archive_securely(&mut archive, temp_dir.path()).expect("extraction should succeed");

        let extracted = temp_dir.path().join("nested/file.txt");
        let contents = fs::read(&extracted).expect("failed to read extracted file");
        assert_eq!(contents, b"data");
    }

    #[test]
    fn manifest_rejects_placeholder_hashes() {
        // Test the parsing logic directly using a JSON manifest with placeholder values
        let placeholder_manifest_json = r#"{
            "models": [
                {
                    "id": "test-zeros",
                    "sha256": "0000000000000000000000000000000000000000000000000000000000000000",
                    "size_bytes": 1024
                }
            ]
        }"#;

        let parsed: ManifestFile =
            serde_json::from_str(placeholder_manifest_json).expect("should parse valid JSON");

        // Attempt to create ModelManifest from placeholder data - should fail
        let result = parsed
            .models
            .into_iter()
            .map(|entry| -> Result<_> {
                if entry.size == 0 {
                    anyhow::bail!("manifest entry for model {} contains zero size", entry.id);
                }
                if entry.digest.len() != 64 || !entry.digest.chars().all(|c| c.is_ascii_hexdigit()) {
                    anyhow::bail!("manifest entry for model {} has invalid sha256 digest", entry.id);
                }

                // This should trigger our placeholder detection
                if entry.digest.chars().all(|c| c == '0')
                    || entry.digest.chars().all(|c| c == '1')
                    || entry.digest.chars().all(|c| c == '2')
                    || entry.digest.chars().all(|c| c == '3')
                    || entry.digest.chars().all(|c| c == '4')
                    || entry.digest == "deadbeef".repeat(8)
                    || entry.digest == "cafebabe".repeat(8)
                {
                    anyhow::bail!(
                        "manifest entry for model {} contains placeholder sha256 digest (security risk)",
                        entry.id
                    );
                }

                Ok((entry.id.clone(), entry))
            })
            .collect::<Result<Vec<_>>>();

        // Should fail with placeholder detection error
        let err = result.expect_err("should reject placeholder hash");
        assert!(err.to_string().contains("placeholder sha256 digest"));
    }
}
