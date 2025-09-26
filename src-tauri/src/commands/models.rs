use crate::managers::model::{ModelInfo, ModelManager};
use crate::managers::transcription::TranscriptionManager;
use crate::settings::{get_settings, write_settings};
use anyhow::Error;
use serde::Serialize;
use std::sync::Arc;
use tauri::{AppHandle, State};

#[derive(Debug, Serialize)]
pub struct ModelCommandError {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

impl ModelCommandError {
    fn new(code: impl Into<String>, message: impl Into<String>, detail: Option<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            detail,
        }
    }
}

fn classify_download_error(error: Error) -> ModelCommandError {
    let detail_text = error.to_string();
    let lower = detail_text.to_ascii_lowercase();

    if lower.contains("hash mismatch") {
        ModelCommandError::new(
            "checksum_mismatch",
            "Downloaded model failed checksum verification.",
            Some(detail_text.clone()),
        )
    } else if lower.contains("size mismatch") {
        ModelCommandError::new(
            "size_mismatch",
            "Downloaded model size did not match the expected value.",
            Some(detail_text.clone()),
        )
    } else if lower.contains("failed to extract archive")
        || lower.contains("unsupported link")
        || lower.contains("unsupported path component")
    {
        ModelCommandError::new(
            "archive_error",
            "Model archive failed safety checks during extraction.",
            Some(detail_text.clone()),
        )
    } else if lower.contains("failed to request model")
        || lower.contains("http ")
        || lower.contains("timeout")
    {
        ModelCommandError::new(
            "network_error",
            "Network request for the model failed. Please retry or check connectivity.",
            Some(detail_text.clone()),
        )
    } else {
        ModelCommandError::new(
            "download_failed",
            "Model download failed. See details for more information.",
            Some(detail_text),
        )
    }
}

#[tauri::command]
pub async fn get_available_models(
    model_manager: State<'_, Arc<ModelManager>>,
) -> Result<Vec<ModelInfo>, String> {
    Ok(model_manager.get_available_models())
}

#[tauri::command]
pub async fn get_model_info(
    model_manager: State<'_, Arc<ModelManager>>,
    model_id: String,
) -> Result<Option<ModelInfo>, String> {
    Ok(model_manager.get_model_info(&model_id))
}

#[tauri::command]
pub async fn download_model(
    model_manager: State<'_, Arc<ModelManager>>,
    model_id: String,
) -> Result<(), ModelCommandError> {
    model_manager
        .download_model(&model_id)
        .await
        .map_err(|e| classify_download_error(&e))
}

#[tauri::command]
pub async fn delete_model(
    model_manager: State<'_, Arc<ModelManager>>,
    model_id: String,
) -> Result<(), ModelCommandError> {
    model_manager
        .delete_model(&model_id)
        .map_err(|e| classify_download_error(&e))
}

#[tauri::command]
pub async fn set_active_model(
    app_handle: AppHandle,
    model_manager: State<'_, Arc<ModelManager>>,
    transcription_manager: State<'_, Arc<TranscriptionManager>>,
    model_id: String,
) -> Result<(), String> {
    // Check if model exists and is available
    let model_info = model_manager
        .get_model_info(&model_id)
        .ok_or_else(|| format!("Model not found: {}", model_id))?;

    if !model_info.is_downloaded {
        return Err(format!("Model not downloaded: {}", model_id));
    }

    // Load the model in the transcription manager
    transcription_manager
        .load_model(&model_id)
        .map_err(|e| e.to_string())?;

    // Update settings
    let mut settings = get_settings(&app_handle);
    settings.selected_model = model_id.clone();
    write_settings(&app_handle, settings);

    Ok(())
}

#[tauri::command]
pub async fn get_current_model(app_handle: AppHandle) -> Result<String, String> {
    let settings = get_settings(&app_handle);
    Ok(settings.selected_model)
}

#[tauri::command]
pub async fn get_transcription_model_status(
    transcription_manager: State<'_, Arc<TranscriptionManager>>,
) -> Result<Option<String>, String> {
    Ok(transcription_manager.get_current_model())
}

#[tauri::command]
pub async fn is_model_loading(
    transcription_manager: State<'_, Arc<TranscriptionManager>>,
) -> Result<bool, String> {
    // Check if transcription manager has a loaded model
    let current_model = transcription_manager.get_current_model();
    Ok(current_model.is_none())
}

#[tauri::command]
pub async fn has_any_models_available(
    model_manager: State<'_, Arc<ModelManager>>,
) -> Result<bool, String> {
    let models = model_manager.get_available_models();
    Ok(models.iter().any(|m| m.is_downloaded))
}

#[tauri::command]
pub async fn has_any_models_or_downloads(
    model_manager: State<'_, Arc<ModelManager>>,
) -> Result<bool, String> {
    let models = model_manager.get_available_models();
    // Return true if any models are downloaded OR if any downloads are in progress
    Ok(models.iter().any(|m| m.is_downloaded))
}

#[tauri::command]
pub async fn cancel_download(
    model_manager: State<'_, Arc<ModelManager>>,
    model_id: String,
) -> Result<(), ModelCommandError> {
    model_manager
        .cancel_download(&model_id)
        .map_err(|e| classify_download_error(&e))
}

#[tauri::command]
pub async fn get_recommended_first_model() -> Result<String, String> {
    // Recommend Parakeet V3 model for first-time users - fastest and most accurate
    Ok("parakeet-tdt-0.6b-v3".to_string())
}
