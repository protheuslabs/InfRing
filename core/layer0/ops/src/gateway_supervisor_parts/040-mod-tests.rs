
#[cfg(test)]
mod tests {
    #[cfg(target_os = "linux")]
    use super::render_systemd_service;
    #[cfg(target_os = "macos")]
    use super::{launchctl_state, render_launchd_plist};
    use super::{
        run_command_with_timeout, shell_quote, trim_text, watchdog_args, GatewaySupervisorConfig,
    };
    use std::path::Path;

    #[test]
    fn shell_quote_escapes_single_quotes() {
        let input = "a'b";
        let quoted = shell_quote(input);
        assert_eq!(quoted, "'a'\\''b'");
    }

    #[test]
    fn trim_text_caps_output() {
        let raw = "x".repeat(20);
        let trimmed = trim_text(raw, 8);
        assert_eq!(trimmed.len(), 8);
    }

    #[test]
    fn supervisor_command_timeout_returns_bounded_payload() {
        let result = run_command_with_timeout(
            "sh",
            &[
                "-c".to_string(),
                "sleep 2".to_string(),
            ],
            None,
            250,
        );
        assert_eq!(result.get("ok").and_then(serde_json::Value::as_bool), Some(false));
        assert_eq!(
            result.get("timed_out").and_then(serde_json::Value::as_bool),
            Some(true)
        );
        assert_eq!(
            result.get("error").and_then(serde_json::Value::as_str),
            Some("supervisor_command_timeout")
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn launchctl_state_extracts_running_state() {
        let stdout = "service = {\n    state = running\n}";
        assert_eq!(launchctl_state(stdout).as_deref(), Some("running"));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn launchctl_state_handles_missing_state() {
        let stdout = "service = {\n    pid = 42\n}";
        assert!(launchctl_state(stdout).is_none());
    }

    #[test]
    fn watchdog_args_include_watchdog_command() {
        let cfg = GatewaySupervisorConfig {
            host: "127.0.0.1".to_string(),
            port: 4173,
            team: "ops".to_string(),
            refresh_ms: 2000,
            ready_timeout_ms: 36000,
            watchdog_interval_ms: 2000,
            node_binary: "/usr/bin/node".to_string(),
        };
        let args = watchdog_args(Path::new("/tmp/infring-ops"), &cfg);
        assert_eq!(args.get(1).map(String::as_str), Some("daemon-control"));
        assert_eq!(args.get(2).map(String::as_str), Some("watchdog"));
        assert!(args
            .iter()
            .any(|row| row == "--dashboard-watchdog-interval-ms=2000"));
        assert!(args.iter().any(|row| row == "--node-binary=/usr/bin/node"));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn launchd_plist_includes_bootstrap_environment() {
        let cfg = GatewaySupervisorConfig {
            host: "127.0.0.1".to_string(),
            port: 4173,
            team: "ops".to_string(),
            refresh_ms: 2000,
            ready_timeout_ms: 36000,
            watchdog_interval_ms: 2000,
            node_binary: "/usr/bin/node".to_string(),
        };
        let args = watchdog_args(Path::new("/tmp/infring-ops"), &cfg);
        let plist = render_launchd_plist(
            Path::new("/tmp/workspace"),
            Path::new("/tmp/watchdog.log"),
            "ai.infring.gateway",
            &args,
        );
        assert!(plist.contains("<key>EnvironmentVariables</key>"));
        assert!(plist.contains("<key>INFRING_OPS_ALLOW_STALE</key>"));
        assert!(plist.contains("<key>INFRING_NPM_ALLOW_STALE</key>"));
        assert!(plist.contains("<key>INFRING_NPM_BINARY</key>"));
        assert!(plist.contains("/tmp/infring-ops"));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn systemd_service_contains_restart_and_watchdog() {
        let cfg = GatewaySupervisorConfig {
            host: "127.0.0.1".to_string(),
            port: 4173,
            team: "ops".to_string(),
            refresh_ms: 2000,
            ready_timeout_ms: 36000,
            watchdog_interval_ms: 2000,
            node_binary: "/usr/bin/node".to_string(),
        };
        let service = render_systemd_service(
            Path::new("/tmp/workspace"),
            &cfg,
            Path::new("/tmp/infring-ops"),
        );
        assert!(service.contains("Restart=always"));
        assert!(service.contains("daemon-control watchdog"));
    }
}
