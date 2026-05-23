use tauri::AppHandle;
use tauri_plugin_biometry::{AuthOptions, BiometryExt};

use crate::error::{AppError, AppResult};

pub fn verify_user(reason: &str, app: &AppHandle) -> AppResult<()> {
    let options = AuthOptions {
        allow_device_credential: Some(true),
        cancel_title: Some("Cancel".into()),
        fallback_title: Some("Use passcode".into()),
        title: Some("Argus".into()),
        subtitle: Some("Verify your identity".into()),
        confirmation_required: Some(false),
    };

    app.biometry()
        .authenticate(reason.into(), options)
        .map_err(|e| AppError::message("AUTH_FAILED", e.to_string()))
}
