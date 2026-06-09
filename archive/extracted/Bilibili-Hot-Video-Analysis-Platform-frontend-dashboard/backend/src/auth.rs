use axum::extract::Request;
use axum::http::header::AUTHORIZATION;
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::Response;
use serde_json::json;

use crate::config::AppConfig;

pub async fn auth_middleware(
    axum::extract::State(config): axum::extract::State<AppConfig>,
    request: Request,
    next: Next,
) -> Result<Response, (StatusCode, axum::Json<serde_json::Value>)> {
    if !config.auth_enabled() {
        return Ok(next.run(request).await);
    }

    let token = config.api_token.as_ref().unwrap();

    let auth_header = request
        .headers()
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok());

    match auth_header {
        Some(header) if header.starts_with("Bearer ") => {
            let request_token = &header[7..];
            if request_token == token.as_str() {
                Ok(next.run(request).await)
            } else {
                Err((
                    StatusCode::UNAUTHORIZED,
                    axum::Json(json!({
                        "code": 401,
                        "message": "invalid token",
                        "data": null
                    })),
                ))
            }
        }
        _ => Err((
            StatusCode::UNAUTHORIZED,
            axum::Json(json!({
                "code": 401,
                "message": "missing or invalid Authorization header, expected: Bearer <token>",
                "data": null
            })),
        )),
    }
}
