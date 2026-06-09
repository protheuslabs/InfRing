#[cfg(test)]
mod health_tests {
    use super::*;

    #[test]
    fn dashboard_health_response_ok_accepts_2xx_status_codes() {
        assert!(dashboard_health_response_ok(
            b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n{}"
        ));
        assert!(dashboard_health_response_ok(
            b"HTTP/1.1 204 No Content\r\nContent-Type: application/json\r\n\r\n"
        ));
    }

    #[test]
    fn dashboard_health_response_ok_rejects_non_2xx_status_codes() {
        assert!(!dashboard_health_response_ok(
            b"HTTP/1.1 503 Service Unavailable\r\nContent-Type: text/plain\r\n\r\noffline"
        ));
    }

    #[test]
    fn dashboard_health_snapshot_classifies_listener_timeout_as_wedge() {
        let snapshot = dashboard_health_snapshot_from_response(b"", true, false);
        assert!(!snapshot.healthy);
        assert_eq!(snapshot.reason, "dashboard_healthz_timeout");
        assert!(snapshot.listener_reachable);
        assert!(snapshot.timed_out);
        assert!(snapshot.is_wedged());
        assert_eq!(
            snapshot.to_json()["reason"].as_str(),
            Some("dashboard_healthz_timeout")
        );
    }

    #[test]
    fn dashboard_health_snapshot_classifies_non_2xx_without_wedge() {
        let snapshot = dashboard_health_snapshot_from_response(
            b"HTTP/1.1 503 Service Unavailable\r\nContent-Type: text/plain\r\n\r\noffline",
            false,
            false,
        );
        assert!(!snapshot.healthy);
        assert_eq!(snapshot.reason, "dashboard_healthz_non_2xx");
        assert_eq!(snapshot.status_code, Some(503));
        assert!(!snapshot.is_wedged());
    }

    #[test]
    fn dashboard_web_tooling_response_ready_accepts_auth_signals() {
        assert!(dashboard_web_tooling_response_ready(
            b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n{\"ok\":true,\"any_present\":true}"
        ));
        assert!(dashboard_web_tooling_response_ready(
            b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n{\"ok\":true,\"readiness\":\"ready\"}"
        ));
    }

    #[test]
    fn dashboard_web_tooling_response_ready_rejects_missing_auth_signals() {
        assert!(!dashboard_web_tooling_response_ready(
            b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n{\"ok\":true,\"auth_sources\":[]}"
        ));
    }
}
