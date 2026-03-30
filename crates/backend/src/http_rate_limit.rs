use axum::Json;
use axum::body::Body;
use axum::http::{Method, StatusCode};
use axum::response::{IntoResponse, Response};
use governor::middleware::NoOpMiddleware;
use serde_json::json;
use std::time::Duration;
use tower_governor::GovernorLayer;
use tower_governor::errors::GovernorError;
use tower_governor::governor::GovernorConfigBuilder;
use tower_governor::key_extractor::PeerIpKeyExtractor;

const LOGIN_REQUESTS_PER_MINUTE: u32 = 10;
const REGISTER_REQUESTS_PER_MINUTE: u32 = 5;
const RATE_LIMIT_WINDOW_SECONDS: u64 = 60;

type HttpRateLimitLayer = GovernorLayer<PeerIpKeyExtractor, NoOpMiddleware, Body>;

pub fn login_rate_limit_layer() -> HttpRateLimitLayer {
    build_rate_limit_layer(LOGIN_REQUESTS_PER_MINUTE)
}

pub fn register_rate_limit_layer() -> HttpRateLimitLayer {
    build_rate_limit_layer(REGISTER_REQUESTS_PER_MINUTE)
}

fn build_rate_limit_layer(requests_per_minute: u32) -> HttpRateLimitLayer {
    let replenish_every =
        Duration::from_secs(RATE_LIMIT_WINDOW_SECONDS / u64::from(requests_per_minute));

    let mut builder = GovernorConfigBuilder::default();
    builder
        .period(replenish_every)
        .burst_size(requests_per_minute)
        .methods(vec![Method::POST]);

    let config = builder
        .finish()
        .expect("rate limit config must use non-zero values");

    GovernorLayer::new(config).error_handler(governor_error_response)
}

fn governor_error_response(error: GovernorError) -> Response {
    match error {
        GovernorError::TooManyRequests { wait_time, headers } => {
            let mut response = (
                StatusCode::TOO_MANY_REQUESTS,
                Json(json!({
                    "error": "Too many requests",
                    "retry_after_seconds": wait_time,
                })),
            )
                .into_response();

            if let Some(headers) = headers {
                response.headers_mut().extend(headers);
            }

            response
        }
        GovernorError::UnableToExtractKey => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "error": "Unable to determine client address for rate limiting",
            })),
        )
            .into_response(),
        GovernorError::Other { code, msg, headers } => {
            let mut response = (
                code,
                Json(json!({
                    "error": msg.unwrap_or_else(|| "Rate limiter error".to_string()),
                })),
            )
                .into_response();

            if let Some(headers) = headers {
                response.headers_mut().extend(headers);
            }

            response
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{login_rate_limit_layer, register_rate_limit_layer};
    use axum::body::{Body, to_bytes};
    use axum::extract::ConnectInfo;
    use axum::http::{Request, StatusCode};
    use axum::routing::post;
    use axum::{Json, Router};
    use serde_json::{Value, json};
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};
    use tower::ServiceExt;

    fn request_with_ip(path: &str, ip: IpAddr) -> Request<Body> {
        let mut request = Request::builder()
            .method("POST")
            .uri(path)
            .body(Body::empty())
            .expect("request must be built");
        request
            .extensions_mut()
            .insert(ConnectInfo(SocketAddr::new(ip, 3000)));
        request
    }

    #[tokio::test]
    async fn login_limit_returns_429_with_retry_after() {
        let app = Router::new().route(
            "/api/auth/login",
            post(|| async { Json(json!({ "ok": true })) }).route_layer(login_rate_limit_layer()),
        );

        let ip = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));

        for _ in 0..10 {
            let response = app
                .clone()
                .oneshot(request_with_ip("/api/auth/login", ip))
                .await
                .expect("request must succeed");
            assert_eq!(response.status(), StatusCode::OK);
        }

        let response = app
            .oneshot(request_with_ip("/api/auth/login", ip))
            .await
            .expect("request must succeed");

        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
        let retry_after = response
            .headers()
            .get("retry-after")
            .expect("retry-after must be present")
            .to_str()
            .expect("retry-after must be ascii")
            .parse::<u64>()
            .expect("retry-after must be an integer");
        assert!((1..=6).contains(&retry_after));

        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body must be readable");
        let json: Value = serde_json::from_slice(&body).expect("body must be valid json");
        assert_eq!(json["error"], "Too many requests");
        assert_eq!(json["retry_after_seconds"], retry_after);
    }

    #[tokio::test]
    async fn register_limit_is_tracked_per_ip() {
        let app = Router::new().route(
            "/api/auth/register",
            post(|| async { Json(json!({ "ok": true })) }).route_layer(register_rate_limit_layer()),
        );

        let first_ip = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1));
        let second_ip = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 2));

        for _ in 0..5 {
            let response = app
                .clone()
                .oneshot(request_with_ip("/api/auth/register", first_ip))
                .await
                .expect("request must succeed");
            assert_eq!(response.status(), StatusCode::OK);
        }

        let limited = app
            .clone()
            .oneshot(request_with_ip("/api/auth/register", first_ip))
            .await
            .expect("request must succeed");
        assert_eq!(limited.status(), StatusCode::TOO_MANY_REQUESTS);
        let retry_after = limited
            .headers()
            .get("retry-after")
            .expect("retry-after must be present")
            .to_str()
            .expect("retry-after must be ascii")
            .parse::<u64>()
            .expect("retry-after must be an integer");
        assert!((1..=12).contains(&retry_after));

        let unaffected = app
            .oneshot(request_with_ip("/api/auth/register", second_ip))
            .await
            .expect("request must succeed");
        assert_eq!(unaffected.status(), StatusCode::OK);
    }
}
