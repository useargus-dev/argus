use tauri::AppHandle;

use crate::crypto::{decrypt_totp_secret, verify_totp_code};
use crate::db::meta;
use crate::error::{AppError, AppResult};
use crate::state::AppState;
use crate::util::biometry;

pub struct SecondFactorProof<'a> {
    pub totp_code: Option<&'a str>,
    pub use_biometric: bool,
}

pub fn verify_second_factor(
    app: &AppHandle,
    state: &tauri::State<'_, AppState>,
    proof: SecondFactorProof<'_>,
) -> AppResult<()> {
    let second_factor = meta::read_second_factor_type()?.to_lowercase();

    if second_factor == "totp" {
        let code = proof.totp_code.ok_or_else(|| {
            AppError::message("VALIDATION_ERROR", "TOTP code required")
        })?;
        let secret_enc = meta::read_totp_secret_enc()?;
        let value_key = {
            let inner = state
                .0
                .lock()
                .map_err(|_| AppError::message("LOCK_ERROR", "state poisoned"))?;
            inner
                .value_key()
                .ok_or_else(|| AppError::message("NOT_SIGNED_IN", "not signed in"))?
        };
        let secret_b32 = decrypt_totp_secret(&value_key, &secret_enc)?;
        if !verify_totp_code(&secret_b32, code)? {
            return Err(AppError::message("AUTH_FAILED", "invalid TOTP code"));
        }
        Ok(())
    } else if second_factor == "biometric" {
        if !proof.use_biometric {
            return Err(AppError::message(
                "VALIDATION_ERROR",
                "biometric verification required",
            ));
        }
        biometry::verify_user("Verify your identity for Argus", app)?;
        Ok(())
    } else {
        Err(AppError::message("AUTH_FAILED", "unknown second factor"))
    }
}
