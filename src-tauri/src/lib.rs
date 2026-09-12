use std::path::PathBuf;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![])
        .setup(|app| {
            let host_key = uuid::Uuid::new_v4().simple().to_string();
            let port: u16 = std::env::var("QUIZ_BUZZER_PORT")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(7423);
            let static_dir = resolve_static_dir();

            let (listener, router) = tauri::async_runtime::block_on(
                quiz_buzzer_core::bind_server(quiz_buzzer_core::ServerConfig {
                    port,
                    host_key: host_key.clone(),
                    static_dir,
                }),
            )?;
            let port = listener.local_addr()?.port();

            tauri::async_runtime::spawn(async move {
                if let Err(err) = axum::serve(listener, router).await {
                    eprintln!("quiz buzzer server error: {err}");
                }
            });

            let window = app
                .get_webview_window("main")
                .ok_or("missing webview window 'main'")?;
            let url = format!("http://127.0.0.1:{port}/board?k={host_key}");
            navigate_to_board(&window, &url);
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running Quiz Buzzer");
}

fn resolve_static_dir() -> PathBuf {
    let from_cwd = std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("web/dist");
    if from_cwd.is_dir() {
        return from_cwd;
    }
    let from_manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../web/dist");
    if from_manifest.is_dir() {
        return from_manifest;
    }
    from_cwd
}

fn navigate_to_board(window: &tauri::WebviewWindow, url: &str) {
    if let Ok(parsed) = url.parse() {
        if window.navigate(parsed).is_ok() {
            return;
        }
    }
    let _ = window.eval(&format!("window.location.replace({url:?})"));
}
