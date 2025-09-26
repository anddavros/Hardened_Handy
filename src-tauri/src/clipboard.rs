use enigo::Enigo;
use enigo::Key;
use enigo::Keyboard;
use enigo::Settings;
use log::{error, warn};
use std::time::{Duration, Instant};
use tauri::AppHandle;
use tauri_plugin_clipboard_manager::ClipboardExt;

/// Sends a paste command (Cmd+V or Ctrl+V) using platform-specific virtual key codes.
/// This ensures the paste works regardless of keyboard layout (e.g., Russian, AZERTY, DVORAK).
fn send_paste() -> Result<(), String> {
    // Platform-specific key definitions
    #[cfg(target_os = "macos")]
    let (modifier_key, v_key_code) = (Key::Meta, Key::Other(9));
    #[cfg(target_os = "windows")]
    let (modifier_key, v_key_code) = (Key::Control, Key::Other(0x56)); // VK_V
    #[cfg(target_os = "linux")]
    let (modifier_key, v_key_code) = (Key::Control, Key::Unicode('v'));

    let mut enigo = Enigo::new(&Settings::default())
        .map_err(|e| format!("Failed to initialize Enigo: {}", e))?;

    // Press modifier + V
    enigo
        .key(modifier_key, enigo::Direction::Press)
        .map_err(|e| format!("Failed to press modifier key: {}", e))?;
    enigo
        .key(v_key_code, enigo::Direction::Press)
        .map_err(|e| format!("Failed to press V key: {}", e))?;

    // Release V + modifier (reverse order)
    enigo
        .key(v_key_code, enigo::Direction::Release)
        .map_err(|e| format!("Failed to release V key: {}", e))?;
    enigo
        .key(modifier_key, enigo::Direction::Release)
        .map_err(|e| format!("Failed to release modifier key: {}", e))?;

    Ok(())
}

pub fn paste(text: String, app_handle: AppHandle) -> Result<(), String> {
    let clipboard = app_handle.clipboard();
    let original_content = clipboard.read_text().unwrap_or_default();
    let start = Instant::now();

    let result = (|| {
        clipboard
            .write_text(&text)
            .map_err(|e| format!("Failed to write to clipboard: {}", e))?;

        std::thread::sleep(Duration::from_millis(40));

        send_paste()?;

        // Give the target application a moment to receive the paste event
        std::thread::sleep(Duration::from_millis(40));

        Ok::<(), String>(())
    })();

    // Always attempt to restore the clipboard, even if paste failed
    if let Err(err) = clipboard.write_text(&original_content) {
        warn!("Failed to restore clipboard contents: {}", err);
    }

    if let Err(err) = result {
        error!(
            "Clipboard paste failed after {:?}: {}",
            start.elapsed(),
            err
        );
        return Err(err);
    }

    Ok(())
}
