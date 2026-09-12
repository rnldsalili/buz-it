use std::path::PathBuf;

use tauri::{WebviewUrl, WebviewWindowBuilder};

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

            let url = format!("http://127.0.0.1:{port}/board?k={host_key}")
                .parse()
                .map_err(|e| format!("invalid board URL: {e}"))?;
            WebviewWindowBuilder::new(app, "main", WebviewUrl::External(url))
                .title("Quiz Buzzer")
                .inner_size(1280.0, 720.0)
                .build()?;
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
