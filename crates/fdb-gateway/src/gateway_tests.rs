//! Rate-limiting and security-header unit tests for the gateway composition root.
//!
//! Relocated from `main.rs` (p16 file-size split) — behavior unchanged. These
//! tests build their own minimal `Router`s mirroring the layers applied in
//! `bootstrap::run()`; they do not depend on any other relocated module.

// ─── Rate-limiting unit tests ────────────────────────────────────────────────

#[cfg(test)]
mod rate_limit_tests {
    use axum::{
        body::Body,
        extract::ConnectInfo,
        http::{Request, StatusCode},
        routing::get,
        Router,
    };
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};
    use tower::ServiceExt as _;
    use tower_governor::{governor::GovernorConfigBuilder, GovernorLayer};

    /// Build a minimal test app with `per_second` / `burst` rate limits applied.
    fn rate_limited_app(per_second: u64, burst: u32) -> Router {
        let config = GovernorConfigBuilder::default()
            .per_second(per_second)
            .burst_size(burst)
            .finish()
            .expect("GovernorConfig");
        Router::new()
            .route("/ping", get(|| async { "pong" }))
            .layer(GovernorLayer::new(config))
    }

    /// Construct a plain GET request with a `ConnectInfo<SocketAddr>` extension so
    /// that `PeerIpKeyExtractor` can resolve the peer address without a TCP listener.
    fn make_request(path: &str) -> Request<Body> {
        let peer = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 1234);
        let mut req = Request::builder().uri(path).body(Body::empty()).unwrap();
        req.extensions_mut().insert(ConnectInfo(peer));
        req
    }

    /// `GovernorConfigBuilder` produces a valid config for the default parameters.
    #[test]
    fn governor_config_builds_without_panic() {
        let config = GovernorConfigBuilder::default()
            .per_second(100)
            .burst_size(10)
            .finish();
        assert!(config.is_some(), "expected Some(GovernorConfig), got None");
    }

    /// Config with burst = 0 must return None (zero is invalid per tower_governor docs).
    #[test]
    fn governor_config_rejects_zero_burst() {
        let config = GovernorConfigBuilder::default()
            .per_second(10)
            .burst_size(0)
            .finish();
        assert!(config.is_none(), "expected None for burst_size=0");
    }

    /// When FLINT_RATE_LIMIT_REST=0 the gate in main() bypasses the layer and all
    /// requests are served normally.  Model that logic here without a live server.
    #[tokio::test]
    async fn rate_limiting_disabled_when_rps_zero() {
        let rest_rps: u64 = 0; // simulates FLINT_RATE_LIMIT_REST=0

        // Mirror the if/else in main() — no GovernorLayer when disabled.
        let app: Router = if rest_rps > 0 {
            let cfg = GovernorConfigBuilder::default()
                .per_second(1)
                .burst_size(1)
                .finish()
                .expect("cfg");
            Router::new()
                .route("/ping", get(|| async { "pong" }))
                .layer(GovernorLayer::new(cfg))
        } else {
            Router::new().route("/ping", get(|| async { "pong" }))
        };

        // Five consecutive requests should all succeed when rate limiting is off.
        for _ in 0..5_u8 {
            let res = app.clone().oneshot(make_request("/ping")).await.unwrap();
            assert_eq!(res.status(), StatusCode::OK);
        }
    }

    /// After the burst bucket is exhausted the next request must receive 429.
    #[tokio::test]
    async fn returns_429_when_limit_exceeded() {
        // 1 req/s sustained, burst of 1 → the second immediate request is rejected.
        let app = rate_limited_app(1, 1);

        let res1 = app.clone().oneshot(make_request("/ping")).await.unwrap();
        assert_eq!(
            res1.status(),
            StatusCode::OK,
            "first request should succeed"
        );

        let res2 = app.clone().oneshot(make_request("/ping")).await.unwrap();
        assert_eq!(
            res2.status(),
            StatusCode::TOO_MANY_REQUESTS,
            "second immediate request should be rate-limited"
        );
    }
}

// ─── Security-header unit tests ───────────────────────────────────────────────

#[cfg(test)]
mod security_header_tests {
    use axum::http::{HeaderName, HeaderValue};
    use axum::{
        body::Body,
        http::{Request, StatusCode},
        routing::get,
        Router,
    };
    use tower::ServiceExt as _;
    use tower_http::set_header::SetResponseHeaderLayer;

    /// Build a minimal test app with the three security header layers applied,
    /// mirroring the layers added in `main()`.
    fn secure_app() -> Router {
        Router::new()
            .route("/healthz", get(|| async { "ok" }))
            .route("/a2ui/v1/components", get(|| async { "[]" }))
            .layer(SetResponseHeaderLayer::if_not_present(
                HeaderName::from_static("x-content-type-options"),
                HeaderValue::from_static("nosniff"),
            ))
            .layer(SetResponseHeaderLayer::if_not_present(
                HeaderName::from_static("x-frame-options"),
                HeaderValue::from_static("DENY"),
            ))
            .layer(SetResponseHeaderLayer::if_not_present(
                HeaderName::from_static("referrer-policy"),
                HeaderValue::from_static("strict-origin-when-cross-origin"),
            ))
    }

    /// All three security headers must be present and have the expected values
    /// on a plain GET /healthz response.
    #[tokio::test]
    async fn security_headers_present_on_healthz() {
        let app = secure_app();
        let req = Request::builder()
            .uri("/healthz")
            .body(Body::empty())
            .unwrap();

        let res = app.oneshot(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        let headers = res.headers();
        assert_eq!(
            headers
                .get("x-content-type-options")
                .and_then(|v| v.to_str().ok()),
            Some("nosniff"),
            "X-Content-Type-Options must be 'nosniff'"
        );
        assert_eq!(
            headers.get("x-frame-options").and_then(|v| v.to_str().ok()),
            Some("DENY"),
            "X-Frame-Options must be 'DENY'"
        );
        assert_eq!(
            headers.get("referrer-policy").and_then(|v| v.to_str().ok()),
            Some("strict-origin-when-cross-origin"),
            "Referrer-Policy must be 'strict-origin-when-cross-origin'"
        );
    }

    /// p16-c006 reconcile (p9-c005 gap): the same three security headers must
    /// also be present on `GET /a2ui/v1/components`, not just `/healthz` — the
    /// layers are applied blanket to the whole router in `main()`, so this
    /// proves that generalizes beyond the one route the original test covered.
    #[tokio::test]
    async fn security_headers_present_on_a2ui_components() {
        let app = secure_app();
        let req = Request::builder()
            .uri("/a2ui/v1/components")
            .body(Body::empty())
            .unwrap();

        let res = app.oneshot(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        let headers = res.headers();
        assert_eq!(
            headers
                .get("x-content-type-options")
                .and_then(|v| v.to_str().ok()),
            Some("nosniff"),
            "X-Content-Type-Options must be 'nosniff'"
        );
        assert_eq!(
            headers.get("x-frame-options").and_then(|v| v.to_str().ok()),
            Some("DENY"),
            "X-Frame-Options must be 'DENY'"
        );
        assert_eq!(
            headers.get("referrer-policy").and_then(|v| v.to_str().ok()),
            Some("strict-origin-when-cross-origin"),
            "Referrer-Policy must be 'strict-origin-when-cross-origin'"
        );
    }

    /// A handler that pre-sets X-Content-Type-Options should NOT be overwritten
    /// by the `if_not_present` layer — the handler's value wins.
    #[tokio::test]
    async fn if_not_present_does_not_overwrite_handler_header() {
        use axum::body::Body as AxumBody;
        use axum::http::Response as AxumResponse;

        let app = Router::new()
            .route(
                "/custom",
                get(|| async {
                    AxumResponse::builder()
                        .header("x-content-type-options", "custom-value")
                        .body(AxumBody::empty())
                        .unwrap()
                }),
            )
            .layer(SetResponseHeaderLayer::if_not_present(
                HeaderName::from_static("x-content-type-options"),
                HeaderValue::from_static("nosniff"),
            ));

        let req = Request::builder()
            .uri("/custom")
            .body(Body::empty())
            .unwrap();

        let res = app.oneshot(req).await.unwrap();
        assert_eq!(
            res.headers()
                .get("x-content-type-options")
                .and_then(|v| v.to_str().ok()),
            Some("custom-value"),
            "if_not_present must not overwrite a header already set by the handler"
        );
    }
}
