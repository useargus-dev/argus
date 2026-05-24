use std::collections::HashMap;

use serde::Deserialize;
use tauri::State;

use crate::db::settings;
use crate::error::AppError;
use crate::state::AppState;
use crate::util::session;

const ALLOWED_KEYS: &[&str] = &[
    "auto_lock_minutes",
    "lock_on_screen_lock",
    "run_in_background",
    "notify_client_access",
    "expiry_notify_days",
];

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> Result<HashMap<String, String>, String> {
    session::with_db(&state, |conn, _inner| {
        let mut map = HashMap::new();
        for key in ALLOWED_KEYS {
            let default = match *key {
                "auto_lock_minutes" => "30",
                "lock_on_screen_lock" | "run_in_background" | "notify_client_access" => "1",
                "expiry_notify_days" => "7",
                _ => "",
            };
            let value = settings::get_or_default(conn, key, default)?;
            map.insert(key.to_string(), value);
        }
        Ok(map)
    })
    .map_err(|e| String::from(e))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetSettingRequest {
    pub key: String,
    pub value: String,
}

#[tauri::command]
pub fn set_setting(
    state: State<'_, AppState>,
    req: SetSettingRequest,
) -> Result<(), String> {
    if !ALLOWED_KEYS.contains(&req.key.as_str()) {
        return Err(String::from(AppError::message(
            "VALIDATION_ERROR",
            "setting key not allowed",
        )));
    }

    session::with_db(&state, |conn, _inner| settings::set(conn, &req.key, &req.value))
        .map_err(|e| String::from(e))
}
