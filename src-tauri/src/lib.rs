mod assets;

use tauri::{WebviewUrl, WebviewWindowBuilder};

fn app_context<R: tauri::Runtime>() -> tauri::Context<R> {
    tauri::generate_context!()
}

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
            let assets = assets::router(app.asset_resolver());

            let (listener, router) = tauri::async_runtime::block_on(
                quiz_buzzer_core::bind_server_with_assets(port, host_key.clone(), assets),
            )?;
            let port = listener.local_addr()?.port();

            tauri::async_runtime::spawn(async move {
                if let Err(err) = axum::serve(
                    listener,
                    router.into_make_service_with_connect_info::<std::net::SocketAddr>(),
                )
                .await
                {
                    eprintln!("Buz It server error: {err}");
                }
            });

            let url = format!("http://127.0.0.1:{port}/board?k={host_key}")
                .parse()
                .map_err(|e| format!("invalid board URL: {e}"))?;
            WebviewWindowBuilder::new(app, "main", WebviewUrl::External(url))
                .title("Buz It")
                .inner_size(1280.0, 720.0)
                .build()?;
            Ok(())
        })
        .run(app_context())
        .expect("error while running Buz It");
}

#[cfg(test)]
mod packaging_tests {
    use axum::{
        body::{to_bytes, Body},
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;

    #[tokio::test]
    async fn bundled_assets_work_without_a_static_directory() {
        let app = tauri::test::mock_builder()
            .build(super::app_context())
            .unwrap();
        let router = crate::assets::router(app.asset_resolver());
        for (path, content_type) in [
            ("/", "text/html"),
            ("/board?k=test", "text/html"),
            ("/theme.js", "text/javascript"),
            ("/favicon.svg", "image/svg+xml"),
            ("/lockout.wav", "audio/x-wav"),
            ("/fonts/barlow-condensed-bold.ttf", "application/font-sfnt"),
        ] {
            let response = router
                .clone()
                .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::OK, "{path}");
            assert!(
                response.headers()["content-type"]
                    .to_str()
                    .unwrap()
                    .starts_with(content_type),
                "{path}: {:?}",
                response.headers()
            );
            assert!(!to_bytes(response.into_body(), usize::MAX)
                .await
                .unwrap()
                .is_empty());
        }
        for path in [
            "/missing.js",
            "/missing",
            "/host",
            "/host.html",
            "/board/",
            "/../Cargo.toml",
        ] {
            let response = router
                .clone()
                .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::NOT_FOUND, "{path}");
        }
        let response = router
            .oneshot(
                Request::builder()
                    .method("HEAD")
                    .uri("/board")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert!(to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap()
            .is_empty());
    }
}
