mod commands;
pub mod crypto;
pub mod db;
mod error;
mod register;
mod state;
mod util;

use state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_biometry::init())
        .plugin(tauri_plugin_opener::init())
        .manage(AppState::default())
        .setup(|app| {
            #[cfg(desktop)]
            {
                use tauri::menu::{Menu, MenuItem};
                use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
                use tauri::Manager;

                let open_i = MenuItem::with_id(app, "open", "Open Argus", true, None::<&str>)?;
                let quit_i = MenuItem::with_id(app, "quit", "Sign out", true, None::<&str>)?;
                let menu = Menu::with_items(app, &[&open_i, &quit_i])?;
                let Some(icon) = app.default_window_icon().cloned() else {
                    return Ok(());
                };
                let _tray = TrayIconBuilder::new()
                    .icon(icon)
                    .menu(&menu)
                    .on_menu_event(|app, event| {
                        if event.id.as_ref() == "open" {
                            if let Some(w) = app.get_webview_window("main") {
                                let _ = w.show();
                                let _ = w.set_focus();
                            }
                        } else if event.id.as_ref() == "quit" {
                            let state = app.state::<AppState>();
                            let _ = commands::auth::sign_out(app.clone(), state);
                        }
                    })
                    .on_tray_icon_event(|tray, event| {
                        if let TrayIconEvent::Click {
                            button: MouseButton::Left,
                            button_state: MouseButtonState::Up,
                            ..
                        } = event
                        {
                            let app = tray.app_handle();
                            if let Some(w) = app.get_webview_window("main") {
                                let _ = w.show();
                                let _ = w.set_focus();
                            }
                        }
                    })
                    .build(app)?;
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::auth::has_account,
            commands::auth::prepare_totp_setup,
            commands::auth::verify_biometric,
            commands::auth::register_validate,
            commands::auth::register_finalize,
            commands::auth::sign_in,
            commands::auth::sign_out,
            commands::auth::unlock_app,
            commands::auth::lock_app,
            commands::auth::get_scope_status,
            commands::auth::get_profile,
            commands::account_settings::update_profile,
            commands::account_settings::get_second_factor_status,
            commands::account_settings::enroll_totp,
            commands::account_settings::enroll_biometric,
            commands::account_settings::set_active_second_factor,
            commands::auth::get_second_factor_type,
            commands::elevation::elevate_vault,
            commands::elevation::lock_vault,
            commands::secrets::search_secrets,
            commands::secrets::get_secret,
            commands::secrets::create_secret,
            commands::secrets::update_secret,
            commands::secrets::delete_secret,
            commands::settings::get_settings,
            commands::settings::set_setting,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
