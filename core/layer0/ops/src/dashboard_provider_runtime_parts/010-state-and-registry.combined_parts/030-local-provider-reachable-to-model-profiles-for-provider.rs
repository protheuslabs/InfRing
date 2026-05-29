
fn local_provider_reachable(provider_id: &str, row: &Value) -> bool {
    match provider_id {
        "ollama" => {
            let base_url = clean_text(
                row.get("base_url")
                    .and_then(Value::as_str)
                    .unwrap_or(&provider_base_url_default("ollama")),
                400,
            );
            probe_ollama_runtime_online(&base_url)
        }
        "claude-code" => command_exists("claude"),
        "local" => row
            .get("local_model_root")
            .and_then(Value::as_str)
            .map(|value| {
                let cleaned = clean_text(value, 4000);
                !cleaned.is_empty() && Path::new(&cleaned).exists()
            })
            .unwrap_or(false),
        _ => row
            .get("reachable")
            .and_then(Value::as_bool)
            .unwrap_or(true),
    }
}

pub fn provider_supports_chat(provider_id: &str, base_url: &str) -> bool {
    let cleaned = clean_text(base_url, 400);
    match provider_id {
        "openai" | "frontier_provider" | "google" | "groq" | "moonshot" | "xai" | "openrouter"
        | "deepseek" | "together" | "fireworks" | "perplexity" | "mistral" | "ollama"
        | "llama.cpp" => !cleaned.is_empty(),
        "claude-code" => true,
        "cohere" | "auto" => false,
        _ => !cleaned.is_empty(),
    }
}

pub fn auth_status_configured(raw: &str) -> bool {
    matches!(
        clean_text(raw, 40).to_ascii_lowercase().as_str(),
        "configured" | "set" | "ok"
    )
}

fn provider_api_key_env(provider_id: &str) -> String {
    provider_key_env_names(provider_id)
        .first()
        .map(|value| value.to_string())
        .unwrap_or_default()
}

fn provider_key_env_names(provider_id: &str) -> &'static [&'static str] {
    match provider_id {
        "openai" => &["OPENAI_API_KEY"],
        "frontier_provider" => &[
            "ANTHROPIC_API_KEY",
            "FRONTIER_PROVIDER_API_KEY",
            "CLAUDE_API_KEY",
        ],
        "google" => &["GEMINI_API_KEY", "GOOGLE_API_KEY"],
        "groq" => &["GROQ_API_KEY"],
        "moonshot" => &["MOONSHOT_API_KEY", "KIMI_API_KEY"],
        "xai" => &["XAI_API_KEY"],
        "openrouter" => &["OPENROUTER_API_KEY"],
        "deepseek" => &["DEEPSEEK_API_KEY"],
        "together" => &["TOGETHER_API_KEY"],
        "fireworks" => &["FIREWORKS_API_KEY"],
        "perplexity" => &["PERPLEXITY_API_KEY"],
        "mistral" => &["MISTRAL_API_KEY"],
        "cohere" => &["COHERE_API_KEY"],
        "minimax" => &["MINIMAX_API_KEY"],
        _ => &[],
    }
}

fn provider_key_from_env(provider_id: &str) -> Option<String> {
    for key in provider_key_env_names(provider_id) {
        if let Ok(value) = std::env::var(key) {
            let cleaned = clean_text(&value, 4096);
            if !cleaned.is_empty() {
                return Some(cleaned);
            }
        }
    }
    None
}

fn provider_key_from_store(root: &Path, provider_id: &str) -> Option<String> {
    load_secrets(root)
        .get("providers")
        .and_then(Value::as_object)
        .and_then(|obj| obj.get(provider_id))
        .and_then(Value::as_object)
        .and_then(|row| row.get("key").and_then(Value::as_str))
        .map(|raw| clean_text(raw, 4096))
        .filter(|value| !value.is_empty())
}

fn provider_key(root: &Path, provider_id: &str) -> Option<String> {
    provider_key_from_env(provider_id).or_else(|| provider_key_from_store(root, provider_id))
}

fn provider_base_url_default(provider_id: &str) -> String {
    match provider_id {
        "openai" => "https://api.openai.com/v1".to_string(),
        "frontier_provider" => "https://api.anthropic.com".to_string(),
        "google" => "https://generativelanguage.googleapis.com".to_string(),
        "groq" => "https://api.groq.com/openai/v1".to_string(),
        "moonshot" => "https://api.moonshot.ai/v1".to_string(),
        "xai" => "https://api.x.ai/v1".to_string(),
        "openrouter" => "https://openrouter.ai/api/v1".to_string(),
        "deepseek" => "https://api.deepseek.com/v1".to_string(),
        "together" => "https://api.together.xyz/v1".to_string(),
        "fireworks" => "https://api.fireworks.ai/inference/v1".to_string(),
        "perplexity" => "https://api.perplexity.ai".to_string(),
        "mistral" => "https://api.mistral.ai/v1".to_string(),
        "ollama" => "http://127.0.0.1:11434".to_string(),
        "llama.cpp" => "http://127.0.0.1:8080".to_string(),
        _ => String::new(),
    }
}

fn model_profiles_for_provider(provider_id: &str) -> Map<String, Value> {
    match provider_id {
        "openai" => serde_json::from_value(json!({
            "gpt-5": {"power_rating": 5, "cost_rating": 5, "param_count_billion": 70, "specialty": "general", "specialty_tags": ["general"], "deployment_kind": "api"},
            "gpt-5-mini": {"power_rating": 4, "cost_rating": 3, "param_count_billion": 0, "specialty": "speed", "specialty_tags": ["speed", "general"], "deployment_kind": "api"},
            "gpt-4o": {"power_rating": 4, "cost_rating": 3, "param_count_billion": 0, "specialty": "vision", "specialty_tags": ["vision", "general"], "deployment_kind": "api"}
        }))
        .unwrap_or_default(),
        "frontier_provider" => serde_json::from_value(json!({
            "claude-sonnet-4-20250514": {"power_rating": 4, "cost_rating": 4, "param_count_billion": 0, "specialty": "general", "specialty_tags": ["general"], "deployment_kind": "api"},
            "claude-opus-4-20250514": {"power_rating": 5, "cost_rating": 5, "param_count_billion": 0, "specialty": "reasoning", "specialty_tags": ["reasoning", "general"], "deployment_kind": "api"},
            "claude-3-7-sonnet-latest": {"power_rating": 4, "cost_rating": 4, "param_count_billion": 0, "specialty": "general", "specialty_tags": ["general", "coding"], "deployment_kind": "api"},
            "claude-3-5-haiku-latest": {"power_rating": 3, "cost_rating": 2, "param_count_billion": 0, "specialty": "speed", "specialty_tags": ["speed", "general"], "deployment_kind": "api"}
        }))
        .unwrap_or_default(),
        "google" => serde_json::from_value(json!({
            "gemini-2.5-pro": {"power_rating": 5, "cost_rating": 5, "param_count_billion": 0, "specialty": "vision", "specialty_tags": ["vision", "general"], "deployment_kind": "api", "context_window": 1048576},
            "gemini-2.5-flash": {"power_rating": 3, "cost_rating": 2, "param_count_billion": 0, "specialty": "vision", "specialty_tags": ["vision", "speed", "general"], "deployment_kind": "api", "context_window": 1048576}
        }))
        .unwrap_or_default(),
        "groq" => serde_json::from_value(json!({
            "llama-3.3-70b-versatile": {"power_rating": 4, "cost_rating": 3, "param_count_billion": 70, "specialty": "general", "specialty_tags": ["general"], "deployment_kind": "api", "context_window": 131072},
            "llama-3.1-8b-instant": {"power_rating": 2, "cost_rating": 1, "param_count_billion": 8, "specialty": "speed", "specialty_tags": ["speed", "general"], "deployment_kind": "api", "context_window": 131072}
        }))
        .unwrap_or_default(),
        "moonshot" => serde_json::from_value(json!({
            "kimi-k2.5": {"power_rating": 5, "cost_rating": 4, "param_count_billion": 0, "specialty": "reasoning", "specialty_tags": ["reasoning", "general"], "deployment_kind": "api", "context_window": 262144},
            "kimi-k2-thinking": {"power_rating": 5, "cost_rating": 4, "param_count_billion": 0, "specialty": "reasoning", "specialty_tags": ["reasoning", "general"], "deployment_kind": "api", "context_window": 262144},
            "kimi-k2": {"power_rating": 5, "cost_rating": 4, "param_count_billion": 1000, "specialty": "coding", "specialty_tags": ["coding", "general"], "deployment_kind": "api", "context_window": 262144}
        }))
        .unwrap_or_default(),
        "deepseek" => serde_json::from_value(json!({
            "deepseek-chat": {"power_rating": 4, "cost_rating": 2, "param_count_billion": 0, "specialty": "general", "specialty_tags": ["general", "coding"], "deployment_kind": "api", "context_window": 65536},
            "deepseek-reasoner": {"power_rating": 5, "cost_rating": 3, "param_count_billion": 0, "specialty": "reasoning", "specialty_tags": ["reasoning", "general"], "deployment_kind": "api", "context_window": 65536}
        }))
        .unwrap_or_default(),
        "openrouter" => serde_json::from_value(json!({
            "google/gemini-2.5-flash": {"power_rating": 3, "cost_rating": 2, "param_count_billion": 0, "specialty": "vision", "specialty_tags": ["vision", "speed", "general"], "deployment_kind": "api", "context_window": 1048576},
            "frontier_provider/claude-sonnet-4": {"power_rating": 4, "cost_rating": 4, "param_count_billion": 0, "specialty": "general", "specialty_tags": ["general"], "deployment_kind": "api", "context_window": 200000},
            "moonshotai/kimi-k2": {"power_rating": 5, "cost_rating": 4, "param_count_billion": 1000, "specialty": "coding", "specialty_tags": ["coding", "general"], "deployment_kind": "api", "context_window": 262144},
            "moonshotai/kimi-k2.5": {"power_rating": 5, "cost_rating": 4, "param_count_billion": 0, "specialty": "reasoning", "specialty_tags": ["reasoning", "general"], "deployment_kind": "api", "context_window": 262144}
        }))
        .unwrap_or_default(),
        "xai" => serde_json::from_value(json!({
            "grok-2": {"power_rating": 4, "cost_rating": 4, "param_count_billion": 0, "specialty": "general", "specialty_tags": ["general"], "deployment_kind": "api", "context_window": 131072}
        }))
        .unwrap_or_default(),
        "ollama" => serde_json::from_value(json!({
            "qwen2.5-coder:7b": {"power_rating": 3, "cost_rating": 1, "param_count_billion": 7, "specialty": "coding", "specialty_tags": ["coding", "general"], "deployment_kind": "ollama", "context_window": 131072},
            "qwen2.5-coder:latest": {"power_rating": 3, "cost_rating": 1, "param_count_billion": 7, "specialty": "coding", "specialty_tags": ["coding", "general"], "deployment_kind": "ollama", "context_window": 131072},
            "qwen2.5:3b": {"power_rating": 2, "cost_rating": 1, "param_count_billion": 3, "specialty": "general", "specialty_tags": ["general"], "deployment_kind": "ollama", "context_window": 131072},
            "qwen2.5:3b-instruct-q4_K_M": {"power_rating": 2, "cost_rating": 1, "param_count_billion": 3, "specialty": "general", "specialty_tags": ["general"], "deployment_kind": "ollama", "context_window": 131072},
            "qwen3:4b": {"power_rating": 2, "cost_rating": 1, "param_count_billion": 4, "specialty": "general", "specialty_tags": ["general"], "deployment_kind": "ollama", "context_window": 131072},
            "qwen3:8b": {"power_rating": 3, "cost_rating": 1, "param_count_billion": 8, "specialty": "general", "specialty_tags": ["general"], "deployment_kind": "ollama", "context_window": 131072},
            "qwen3:14b": {"power_rating": 3, "cost_rating": 1, "param_count_billion": 14, "specialty": "general", "specialty_tags": ["general"], "deployment_kind": "ollama", "context_window": 131072},
            "smallthinker:latest": {"power_rating": 2, "cost_rating": 1, "param_count_billion": 4, "specialty": "reasoning", "specialty_tags": ["reasoning", "general"], "deployment_kind": "ollama", "context_window": 131072},
            "tinyllama:latest": {"power_rating": 1, "cost_rating": 1, "param_count_billion": 1, "specialty": "speed", "specialty_tags": ["speed", "general"], "deployment_kind": "ollama", "context_window": 32768},
            "gemma3:4b": {"power_rating": 2, "cost_rating": 1, "param_count_billion": 4, "specialty": "general", "specialty_tags": ["general"], "deployment_kind": "ollama", "context_window": 131072},
            "phi:latest": {"power_rating": 2, "cost_rating": 1, "param_count_billion": 3, "specialty": "general", "specialty_tags": ["general"], "deployment_kind": "ollama", "context_window": 32768},
            "llama3.2:latest": {"power_rating": 2, "cost_rating": 1, "param_count_billion": 3, "specialty": "general", "specialty_tags": ["general"], "deployment_kind": "ollama", "context_window": 128000},
            "llama3.3:70b": {"power_rating": 4, "cost_rating": 1, "param_count_billion": 70, "specialty": "general", "specialty_tags": ["general"], "deployment_kind": "ollama", "context_window": 131072},
            "kimi-k2.5:cloud": {"power_rating": 5, "cost_rating": 2, "param_count_billion": 0, "specialty": "reasoning", "specialty_tags": ["reasoning", "general"], "deployment_kind": "cloud", "context_window": 262144},
            "kimi-k2.5:thinking": {"power_rating": 5, "cost_rating": 2, "param_count_billion": 0, "specialty": "reasoning", "specialty_tags": ["reasoning", "general"], "deployment_kind": "cloud", "context_window": 262144},
            "deepseek-v3.2:cloud": {"power_rating": 5, "cost_rating": 2, "param_count_billion": 0, "specialty": "coding", "specialty_tags": ["coding", "general"], "deployment_kind": "cloud", "context_window": 262144},
            "deepseek-v3.1:671b-cloud": {"power_rating": 5, "cost_rating": 2, "param_count_billion": 671, "specialty": "coding", "specialty_tags": ["coding", "general"], "deployment_kind": "cloud", "context_window": 262144},
            "qwen3-coder:480b-cloud": {"power_rating": 5, "cost_rating": 2, "param_count_billion": 480, "specialty": "coding", "specialty_tags": ["coding", "general"], "deployment_kind": "cloud", "context_window": 262144},
            "qwen3-vl:235b-cloud": {"power_rating": 5, "cost_rating": 2, "param_count_billion": 235, "specialty": "vision", "specialty_tags": ["vision", "general"], "deployment_kind": "cloud", "context_window": 262144},
            "gpt-oss:120b-cloud": {"power_rating": 5, "cost_rating": 2, "param_count_billion": 120, "specialty": "general", "specialty_tags": ["general"], "deployment_kind": "cloud", "context_window": 262144}
        }))
        .unwrap_or_default(),
        "claude-code" => serde_json::from_value(json!({
            "sonnet": {"power_rating": 4, "cost_rating": 2, "param_count_billion": 0, "specialty": "coding", "specialty_tags": ["coding", "general"], "deployment_kind": "local", "context_window": 200000}
        }))
        .unwrap_or_default(),
        _ => Map::new(),
    }
}
