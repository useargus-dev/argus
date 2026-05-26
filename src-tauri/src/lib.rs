mod commands;
pub mod crypto;
pub mod db;
mod error;
mod ipc;
mod register;
mod sessions;
mod state;
mod util;

use state::AppState;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_biometry::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .manage(AppState::default())
        .manage(ipc::IpcRuntime::default())
        .setup(|app| {
            #[cfg(desktop)]
            {
                use tauri::menu::{Menu, MenuItem};
                use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
                use tauri::Manager;

                let open_i = MenuItem::with_id(app, "open", "Open Argus", true, None::<&str>)?;
                let requests_i = MenuItem::with_id(app, "requests", "Approvals", true, None::<&str>)?;
                let quit_i = MenuItem::with_id(app, "quit", "Sign out", true, None::<&str>)?;
                let menu = Menu::with_items(app, &[&open_i, &requests_i, &quit_i])?;
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
                        } else if event.id.as_ref() == "requests" {
                            show_requests_window(app);
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
                            let signed_in = app
                                .state::<AppState>()
                                .0
                                .lock()
                                .map(|i| i.is_signed_in())
                                .unwrap_or(false);
                            if signed_in {
                                show_requests_window(app);
                            } else if let Some(w) = app.get_webview_window("main") {
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
                let hide_to_tray = should_run_in_background(&window.app_handle());
                if hide_to_tray {
                    api.prevent_close();
                    let _ = window.hide();
                }
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
            commands::buckets::list_buckets,
            commands::buckets::create_bucket,
            commands::buckets::delete_bucket,
            commands::buckets::set_bucket_active,
            commands::buckets::get_bucket_token,
            commands::buckets::list_bucket_mappings,
            commands::buckets::upsert_bucket_mapping,
            commands::buckets::delete_bucket_mapping,
            commands::settings::get_settings,
            commands::settings::set_setting,
            commands::clients::is_signed_in,
            commands::clients::show_main_window,
            commands::clients::list_pending_client_access,
            commands::clients::respond_to_client_access,
            commands::clients::pending_client_access_count,
            commands::clients::list_grants,
            commands::clients::revoke_grant,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn should_run_in_background(app: &tauri::AppHandle) -> bool {
    use tauri::Manager;
    let state = app.state::<AppState>();
    let inner = match state.0.lock() {
        Ok(g) => g,
        Err(_) => return true,
    };
    if !inner.is_signed_in() {
        return false;
    }
    let pool = match inner.db.as_ref() {
        Some(p) => p,
        None => return true,
    };
    let conn = match pool.lock() {
        Ok(c) => c,
        Err(_) => return true,
    };
    crate::db::settings::get_or_default(&conn, "run_in_background", "1")
        .map(|v| v == "1")
        .unwrap_or(true)
}

fn show_requests_window(app: &tauri::AppHandle) {
    use tauri::Manager;
    if let Some(w) = app.get_webview_window("requests") {
        let _ = w.show();
        let _ = w.set_focus();
    } else {
        let width = 400.0_f64;
        let height = 480.0_f64;
        let margin = 16.0_f64;

        let mut builder = tauri::WebviewWindowBuilder::new(
            app,
            "requests",
            tauri::WebviewUrl::App("/requests".into()),
        )
        .title("Argus — Approvals")
        .inner_size(width, height)
        .min_inner_size(360.0, 300.0)
        .resizable(true)
        .always_on_top(true);

        if let Some(monitor) = app.primary_monitor().ok().flatten() {
            let screen = monitor.size();
            let scale = monitor.scale_factor();
            let screen_w = screen.width as f64 / scale;
            let screen_h = screen.height as f64 / scale;
            let x = screen_w - width - margin;
            let y = screen_h - height - margin - 48.0;
            builder = builder.position(x, y);
        }

        let _ = builder.build();
    }
}
