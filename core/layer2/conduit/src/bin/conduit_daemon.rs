// SPDX-License-Identifier: Apache-2.0
use conduit::{
    run_stdio_once, validate_conduit_contract_budget, ConduitPolicy, ConduitSecurityContext,
    KernelLaneCommandHandler, RegistryPolicyGate,
};
use std::env;
use std::io::{self, BufReader};

fn main() {
    if let Err(err) = run() {
        eprintln!("conduit_daemon_error:{err}");
        std::process::exit(1);
    }
}

fn run() -> io::Result<()> {
    let policy = load_policy()?;
    validate_conduit_contract_budget(policy.bridge_message_budget_max)
        .map_err(|reason| io::Error::new(io::ErrorKind::InvalidData, reason))?;
    let signing_key_id = env_or_default("CONDUIT_SIGNING_KEY_ID", "conduit-msg-k1");
    let signing_secret = env_or_default("CONDUIT_SIGNING_SECRET", "conduit-dev-signing-secret");
    let token_key_id = env_or_default("CONDUIT_TOKEN_KEY_ID", "conduit-token-k1");
    let token_secret = env_or_default("CONDUIT_TOKEN_SECRET", "conduit-dev-token-secret");

    assert_not_dev_fallback_secret_in_production(&signing_secret, "CONDUIT_SIGNING_SECRET")?;
    assert_not_dev_fallback_secret_in_production(&token_secret, "CONDUIT_TOKEN_SECRET")?;

    let gate = RegistryPolicyGate::new(policy.clone());
    let mut security = ConduitSecurityContext::from_policy(
        &policy,
        signing_key_id,
        signing_secret,
        token_key_id,
        token_secret,
    );
    let mut handler = KernelLaneCommandHandler;

    let stdin = io::stdin();
    let mut reader = BufReader::new(stdin.lock());
    let stdout = io::stdout();
    let mut writer = stdout.lock();

    while run_stdio_once(&mut reader, &mut writer, &gate, &mut security, &mut handler)? {}
    Ok(())
}

fn load_policy() -> io::Result<ConduitPolicy> {
    if let Ok(path) = env::var("CONDUIT_POLICY_PATH") {
        ConduitPolicy::from_path(path)
    } else {
        Ok(ConduitPolicy::default())
    }
}

fn env_or_default(key: &str, default: &str) -> String {
    env::var(key).unwrap_or_else(|_| default.to_string())
}

fn is_production_mode() -> bool {
    match env::var("INFRING_RELEASE_CHANNEL")
        .unwrap_or_else(|_| "development".to_string())
        .to_ascii_lowercase()
        .as_str()
    {
        "release" | "prod" | "production" => true,
        _ => false,
    }
}

fn is_dev_fallback_secret(value: &str) -> bool {
    matches!(value, "conduit-dev-signing-secret" | "conduit-dev-token-secret")
}

fn assert_not_dev_fallback_secret_in_production(
    value: &str,
    variable: &str,
) -> io::Result<()> {
    if is_production_mode() && is_dev_fallback_secret(value) {
        Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "{} cannot use development fallback secret in production mode",
                variable
            ),
        ))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{load_policy, run};
    use conduit::ConduitPolicy;
    use std::env;
    use std::fs;
    use std::sync::{Mutex, OnceLock};

    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    fn clear_policy_path_env() {
        env::remove_var("CONDUIT_POLICY_PATH");
    }

    fn with_release_channel_release_mode<F>(value: &str, action: F)
    where
        F: FnOnce(),
    {
        let prior = env::var("INFRING_RELEASE_CHANNEL").ok();
        env::set_var("INFRING_RELEASE_CHANNEL", value);
        action();
        match prior {
            Some(previous) => env::set_var("INFRING_RELEASE_CHANNEL", previous),
            None => env::remove_var("INFRING_RELEASE_CHANNEL"),
        }
    }

    fn with_env_var_reset<K, V, F>(key: K, value: V, action: F)
    where
        K: AsRef<str>,
        V: AsRef<str>,
        F: FnOnce(),
    {
        let prior = env::var(key.as_ref()).ok();
        env::set_var(key.as_ref(), value.as_ref());
        action();
        match prior {
            Some(previous) => env::set_var(key.as_ref(), previous),
            None => env::remove_var(key.as_ref()),
        }
    }

    #[test]
    fn load_policy_uses_default_when_env_unset() {
        let _guard = env_lock().lock().expect("env lock");
        clear_policy_path_env();
        let policy = load_policy().expect("default policy");
        assert_eq!(
            policy.bridge_message_budget_max,
            conduit::MAX_CONDUIT_MESSAGE_TYPES
        );
    }

    #[test]
    fn load_policy_reads_policy_file_from_env_path() {
        let _guard = env_lock().lock().expect("env lock");
        let temp = tempfile::tempdir().expect("tempdir");
        let policy_path = temp.path().join("policy.json");
        let mut policy = ConduitPolicy::default();
        policy.bridge_message_budget_max = 10;
        fs::write(
            &policy_path,
            serde_json::to_string(&policy).expect("serialize policy"),
        )
        .expect("write policy");
        env::set_var("CONDUIT_POLICY_PATH", &policy_path);
        let policy = load_policy().expect("policy from file");
        assert_eq!(policy.bridge_message_budget_max, 10);
        clear_policy_path_env();
    }

    #[test]
    fn load_policy_fails_for_missing_file_path() {
        let _guard = env_lock().lock().expect("env lock");
        env::set_var(
            "CONDUIT_POLICY_PATH",
            "/tmp/infring_conduit_policy_missing_file_for_test.json",
        );
        let err = load_policy().expect_err("missing path must fail");
        assert_eq!(err.kind(), std::io::ErrorKind::NotFound);
        clear_policy_path_env();
    }

    #[test]
    fn load_policy_fails_for_invalid_json() {
        let _guard = env_lock().lock().expect("env lock");
        let temp = tempfile::tempdir().expect("tempdir");
        let policy_path = temp.path().join("policy.json");
        fs::write(&policy_path, "{ invalid json").expect("write invalid json");
        env::set_var("CONDUIT_POLICY_PATH", &policy_path);
        let err = load_policy().expect_err("invalid json must fail");
        assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
        clear_policy_path_env();
    }

    #[test]
    fn run_fails_fast_when_signing_secret_is_dev_fallback_in_production() {
        let _guard = env_lock().lock().expect("env lock");
        clear_policy_path_env();
        with_release_channel_release_mode("production", || {
            with_env_var_reset("CONDUIT_SIGNING_SECRET", "conduit-dev-signing-secret", || {
                let err = run().expect_err("dev fallback signing secret must fail");
                assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
            });
        });
    }

    #[test]
    fn run_fails_fast_when_token_secret_is_dev_fallback_in_production() {
        let _guard = env_lock().lock().expect("env lock");
        clear_policy_path_env();
        with_release_channel_release_mode("production", || {
            with_env_var_reset("CONDUIT_TOKEN_SECRET", "conduit-dev-token-secret", || {
                let err = run().expect_err("dev fallback token secret must fail");
                assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
            });
        });
    }

    #[test]
    fn run_fails_fast_when_policy_budget_is_invalid() {
        let _guard = env_lock().lock().expect("env lock");
        let temp = tempfile::tempdir().expect("tempdir");
        let policy_path = temp.path().join("policy.json");
        let mut policy = ConduitPolicy::default();
        policy.bridge_message_budget_max = 0;
        fs::write(
            &policy_path,
            serde_json::to_string(&policy).expect("serialize policy"),
        )
        .expect("write policy");
        env::set_var("CONDUIT_POLICY_PATH", &policy_path);
        let err = run().expect_err("invalid budget must fail");
        assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
        clear_policy_path_env();
    }
}
