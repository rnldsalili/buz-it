#[tokio::main]
async fn main() {
    let port: u16 = std::env::var("QUIZ_BUZZER_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(7423);
    let key = uuid::Uuid::new_v4().simple().to_string();
    eprintln!("host key {key}");
    eprintln!("board http://127.0.0.1:{port}/board?k={key}");
    quiz_buzzer_core::start_server(quiz_buzzer_core::ServerConfig {
        port,
        host_key: key,
        static_dir: std::path::PathBuf::from("web/dist"),
    })
    .await
    .unwrap();
}
