mod api;
pub mod crypto;
mod infra;
pub use infra::db as db;
mod error;
mod ipc;
mod proxy;
mod register;
mod sessions;
mod state;
mod messages;
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
        .manage(proxy::ProxyRuntime::default())
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
                            let _ = api::auth::sign_out(app.clone(), state);
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
            api::auth::has_account,
            api::auth::prepare_totp_setup,
            api::auth::verify_biometric,
            api::auth::register_validate,
            api::auth::register_finalize,
            api::auth::sign_in,
            api::auth::sign_out,
            api::auth::unlock_app,
            api::auth::lock_app,
            api::auth::get_scope_status,
            api::auth::get_profile,
            api::account::update_profile,
            api::account::get_second_factor_status,
            api::account::enroll_totp,
            api::account::enroll_biometric,
            api::account::set_active_second_factor,
            api::auth::get_second_factor_type,
            api::elevate::elevate_vault,
            api::elevate::lock_vault,
            api::secrets::search_secrets,
            api::secrets::get_secret,
            api::secrets::create_secret,
            api::secrets::update_secret,
            api::secrets::delete_secret,
            api::buckets::list_buckets,
            api::buckets::create_bucket,
            api::buckets::delete_bucket,
            api::buckets::set_bucket_active,
            api::buckets::set_bucket_proxy_enabled,
            api::buckets::get_bucket_token,
            api::buckets::list_bucket_mappings,
            api::buckets::upsert_bucket_mapping,
            api::buckets::delete_bucket_mapping,
            api::settings::get_settings,
            api::settings::set_setting,
            api::clients::is_signed_in,
            api::clients::show_main_window,
            api::clients::list_pending,
            api::clients::respond_access,
            api::clients::pending_count,
            api::clients::list_grants,
            api::clients::revoke_grant,
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
    crate::infra::db::settings::get_or_default(&conn, "run_in_background", "1")
        .map(|v| v == "1")
        .unwrap_or(true)
}

fn show_requests_window(app: &tauri::AppHandle) {
    use tauri::{Manager, UserAttentionType};

    if let Some(w) = app.get_webview_window("requests") {
        let app = app.clone();
        let _ = w.run_on_main_thread(move || {
            if let Some(w) = app.get_webview_window("requests") {
                if w.is_minimized().unwrap_or(false) {
                    let _ = w.unminimize();
                }
                let _ = w.show();
                let _ = w.set_always_on_top(true);
                let _ = w.set_focus();
                let _ = w.request_user_attention(Some(UserAttentionType::Informational));
            }
        });
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
