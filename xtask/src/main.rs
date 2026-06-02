use anyhow::{anyhow, bail, Context, Result};
use infring_agent_surface::{
    default_template_dir, run_runtime_lane, scaffold_template, NativeToolCall,
    NativeToolDispatcher, RuntimeLaneRequest, TemplateKind, TemplateScaffoldOptions,
};
use nursery_runtime::{
    build_specialist_training_plan, containment_permissions_from_value, seed_manifest_from_value,
};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

const INFRING_DETACH_XTASK_CONTRACT_ID: &str = "V6-INFRING-DETACH-001.7";

fn main() -> Result<()> {
    let mut args = env::args().skip(1).collect::<Vec<_>>();
    if args.is_empty() {
        print_usage();
        bail!("xtask_missing_command");
    }

    let command = args.remove(0);
    match command.as_str() {
        "infring-detach-bootstrap" => run_infring_detach_bootstrap(&args),
        "verify-infring-detach" => run_verify_infring_detach(&args),
        "emit-nursery-plan" => run_emit_nursery_plan(&args),
        "infring-new" => run_infring_new(&args),
        "infring-agent-run" => run_infring_agent_run(&args),
        "native-file-tool-smoke" => run_native_file_tool_smoke(&args),
        "help" | "--help" | "-h" => {
            print_usage();
            Ok(())
        }
        other => {
            print_usage();
            bail!("xtask_unknown_command:{other}")
        }
    }
}

fn print_usage() {
    println!("xtask commands:");
    println!(
        "  cargo run -p xtask -- infring-detach-bootstrap [--source-root=..] [--apply=1|0] [--strict=1|0] [--max-copy-mb=2048]"
    );
    println!(
        "  cargo run -p xtask -- verify-infring-detach [--root=. ]  # validates assimilated artifacts"
    );
    println!(
        "  cargo run -p xtask -- emit-nursery-plan --seed=<seed_manifest.json> --permissions=<permissions.json> --out=<plan.json>"
    );
    println!(
        "  cargo run -p xtask -- infring-new --template=single-agent|swarm|rag|voice --name=<project-name> [--out=<dir>] [--force=1|0]"
    );
    println!(
        "  cargo run -p xtask -- infring-agent-run --name=<agent> --prompt=<text|@file> [--workflow=<workflow_id>] [--preamble=<text|@file>] [--provider=local-echo|ollama] [--model=kimi-k2.6:cloud] [--pack=research,web-ops,lead-gen,social-signal,issue-ops] [--tool=web.search,web.fetch] [--lifespan=3600] [--schedule-interval=<seconds>] [--schedule-max-runs=<n>] [--permissions=<json|@file>] [--permissions-template=parent|admin|user] [--parent-permissions=<json|@file>] [--success-criteria=<json|@file>] [--wasm-policy=<json|@file>] [--voice=<json|@file>] [--receipt-merkle=1|0] [--receipt-merkle-seed=<seed>] [--prev-receipt-root=<hash>]"
    );
    println!(
        "  cargo run -p xtask -- native-file-tool-smoke [--root=<dir>]  # directly exercises native file_write/read/patch without a provider"
    );
}

fn parse_flag_map(args: &[String]) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    for arg in args {
        if !arg.starts_with("--") {
            continue;
        }
        if let Some((k, v)) = arg[2..].split_once('=') {
            out.insert(k.trim().to_string(), v.trim().to_string());
        } else {
            out.insert(arg[2..].trim().to_string(), "1".to_string());
        }
    }
    out
}

fn parse_bool(raw: Option<&String>, default: bool) -> bool {
    let Some(v) = raw else {
        return default;
    };
    matches!(
        v.trim().to_ascii_lowercase().as_str(),
        "1" | "true" | "yes" | "on"
    )
}

fn parse_u64(raw: Option<&String>, default: u64) -> u64 {
    raw.and_then(|v| v.parse::<u64>().ok()).unwrap_or(default)
}

fn parse_u32(raw: Option<&String>) -> Option<u32> {
    raw.and_then(|v| v.parse::<u32>().ok())
}

fn resolve_workspace_root() -> Result<PathBuf> {
    let cwd = env::current_dir().context("xtask_current_dir_failed")?;
    if cwd.join("Cargo.toml").exists() {
        return Ok(cwd);
    }
    let mut probe = cwd.as_path();
    while let Some(parent) = probe.parent() {
        if parent.join("Cargo.toml").exists() {
            return Ok(parent.to_path_buf());
        }
        probe = parent;
    }
    Err(anyhow!("xtask_workspace_root_not_found"))
}

fn run_infring_detach_bootstrap(args: &[String]) -> Result<()> {
    let flags = parse_flag_map(args);
    let root = resolve_workspace_root()?;
    let source_root = flags
        .get("source-root")
        .cloned()
        .unwrap_or_else(|| "..".to_string());
    let apply = parse_bool(flags.get("apply"), true);
    let strict = parse_bool(flags.get("strict"), true);
    let max_copy_mb = parse_u64(flags.get("max-copy-mb"), 2048);

    let contract_ids = [
        "V6-INFRING-DETACH-001.1",
        "V6-INFRING-DETACH-001.2",
        "V6-INFRING-DETACH-001.3",
        "V6-INFRING-DETACH-001.4",
    ];

    let payload = json!({
        "source_root": source_root,
        "max_assimilation_copy_mb": max_copy_mb,
    })
    .to_string();

    let mut executed = Vec::<Value>::new();
    for id in contract_ids {
        let output = Command::new("cargo")
            .current_dir(&root)
            .arg("run")
            .arg("--quiet")
            .arg("--package")
            .arg("infring-ops-core")
            .arg("--bin")
            .arg("infring-ops")
            .arg("--")
            .arg("runtime-systems")
            .arg("run")
            .arg(format!("--id={id}"))
            .arg(format!("--apply={}", if apply { 1 } else { 0 }))
            .arg(format!("--strict={}", if strict { 1 } else { 0 }))
            .arg(format!("--payload-json={payload}"))
            .output()
            .with_context(|| format!("xtask_infring_detach_exec_failed:{id}"))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            return Err(anyhow!(
                "xtask_infring_detach_contract_failed:{id}:stdout={stdout}:stderr={stderr}"
            ));
        }

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let parsed =
            serde_json::from_str::<Value>(&stdout).unwrap_or_else(|_| json!({"raw": stdout}));
        executed.push(json!({"id": id, "result": parsed}));
    }

    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "ok": true,
            "contract_id": INFRING_DETACH_XTASK_CONTRACT_ID,
            "executed_contracts": executed,
            "source_root": source_root,
            "apply": apply,
            "strict": strict,
            "max_copy_mb": max_copy_mb,
        }))
        .expect("encode")
    );
    Ok(())
}

fn run_verify_infring_detach(args: &[String]) -> Result<()> {
    let flags = parse_flag_map(args);
    let root = flags
        .get("root")
        .map(PathBuf::from)
        .unwrap_or(resolve_workspace_root()?);

    let required = [
        "config/infring_assimilation/cron/jobs.json",
        "config/infring_assimilation/nursery/containment/permissions.json",
        "config/infring_assimilation/nursery/manifests/seed_manifest.json",
        "config/infring_assimilation/llm/model_registry.json",
        "local/state/nursery/promotion/specialist_training_plan.json",
        "local/state/llm_runtime/model_registry.json",
    ];

    let mut missing = Vec::<String>::new();
    let mut present = Vec::<String>::new();
    for rel in required {
        let path = root.join(rel);
        if path.exists() {
            present.push(rel.to_string());
        } else {
            missing.push(rel.to_string());
        }
    }

    let out = json!({
        "ok": missing.is_empty(),
        "contract_id": INFRING_DETACH_XTASK_CONTRACT_ID,
        "root": root.display().to_string(),
        "present": present,
        "missing": missing,
    });
    println!("{}", serde_json::to_string_pretty(&out).expect("encode"));

    if !out.get("ok").and_then(Value::as_bool).unwrap_or(false) {
        bail!("xtask_infring_detach_verify_missing_required_artifacts");
    }
    Ok(())
}

fn run_emit_nursery_plan(args: &[String]) -> Result<()> {
    let flags = parse_flag_map(args);
    let seed_path = flags
        .get("seed")
        .map(PathBuf::from)
        .ok_or_else(|| anyhow!("xtask_missing_seed_path"))?;
    let permissions_path = flags
        .get("permissions")
        .map(PathBuf::from)
        .ok_or_else(|| anyhow!("xtask_missing_permissions_path"))?;
    let out_path = flags
        .get("out")
        .map(PathBuf::from)
        .ok_or_else(|| anyhow!("xtask_missing_out_path"))?;

    let seed_raw = read_json_file(&seed_path)?;
    let permissions_raw = read_json_file(&permissions_path)?;

    let seed = seed_manifest_from_value(&seed_raw);
    let permissions = containment_permissions_from_value(&permissions_raw);
    let generated_at = now_isoish();
    let plan = build_specialist_training_plan(&generated_at, &seed, &permissions);

    if let Some(parent) = out_path.parent() {
        fs::create_dir_all(parent).with_context(|| {
            format!("xtask_emit_nursery_plan_mkdir_failed:{}", parent.display())
        })?;
    }
    fs::write(
        &out_path,
        serde_json::to_string_pretty(&plan).expect("encode nursery plan"),
    )
    .with_context(|| {
        format!(
            "xtask_emit_nursery_plan_write_failed:{}",
            out_path.display()
        )
    })?;

    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "ok": true,
            "contract_id": INFRING_DETACH_XTASK_CONTRACT_ID,
            "seed_path": seed_path.display().to_string(),
            "permissions_path": permissions_path.display().to_string(),
            "out_path": out_path.display().to_string(),
            "specialist_count": plan.specialists.len(),
            "max_train_minutes": plan.max_train_minutes,
        }))
        .expect("encode")
    );
    Ok(())
}

fn run_infring_new(args: &[String]) -> Result<()> {
    let flags = parse_flag_map(args);
    let template_raw = flags
        .get("template")
        .map(|value| value.as_str())
        .unwrap_or("single-agent");
    let template = TemplateKind::parse(template_raw)
        .ok_or_else(|| anyhow!("xtask_invalid_template:{template_raw}"))?;
    let name = flags
        .get("name")
        .cloned()
        .unwrap_or_else(|| "infring-agent-app".to_string());
    let root = resolve_workspace_root()?;
    let out = flags
        .get("out")
        .map(PathBuf::from)
        .unwrap_or_else(|| default_template_dir(&root));
    let force = parse_bool(flags.get("force"), false);

    let result = scaffold_template(
        template,
        &TemplateScaffoldOptions {
            name: name.clone(),
            out_dir: out.clone(),
            force,
        },
    )
    .with_context(|| format!("xtask_infring_new_failed:name={name}:out={}", out.display()))?;

    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "ok": true,
            "template": result.template,
            "root": result.root,
            "created_files": result.created_files,
            "name": name,
            "out_dir": out.display().to_string(),
            "force": force,
        }))
        .expect("encode")
    );
    Ok(())
}

fn run_infring_agent_run(args: &[String]) -> Result<()> {
    let xtask_started = Instant::now();
    let parse_started = Instant::now();
    let flags = parse_flag_map(args);
    let parse_flags_ms = elapsed_ms(parse_started);
    let name = flags
        .get("name")
        .cloned()
        .unwrap_or_else(|| "infring-agent".to_string());
    let prompt_started = Instant::now();
    let prompt = parse_text_flag(flags.get("prompt"), "prompt")?
        .ok_or_else(|| anyhow!("xtask_missing_prompt"))?;
    let preamble = parse_text_flag(flags.get("preamble"), "preamble")?;
    let prompt_load_ms = elapsed_ms(prompt_started);
    let workflow_started = Instant::now();
    let workflow_context = load_workflow_context(flags.get("workflow"))?;
    let workflow_load_ms = elapsed_ms(workflow_started);
    let preamble_started = Instant::now();
    let preamble = combine_preamble(preamble, workflow_context.as_ref().map(|row| row.0.clone()));
    let preamble_combine_ms = elapsed_ms(preamble_started);
    let provider = flags.get("provider").cloned();
    let model = flags.get("model").cloned();
    let lifespan_seconds = parse_u64(flags.get("lifespan"), 3600);
    let schedule_interval_seconds = flags
        .get("schedule-interval")
        .and_then(|value| value.parse::<u64>().ok());
    let schedule_max_runs = parse_u32(flags.get("schedule-max-runs"));
    let mut packs = csv_tokens(flags.get("pack"));
    add_workflow_default_packs(&mut packs, workflow_context.as_ref().map(|row| &row.1));
    let tools = csv_tokens(flags.get("tool"));
    let mut permissions_manifest = parse_json_flag(flags.get("permissions"))?;
    let mut permissions_template = flags
        .get("permissions-template")
        .map(|value| value.trim().to_ascii_lowercase())
        .filter(|value| !value.is_empty());
    if permissions_template.is_none() && permissions_manifest.is_none() {
        permissions_template = workflow_context
            .as_ref()
            .and_then(|row| row.1.get("native_permission_template"))
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_ascii_lowercase);
    }
    let parent_permissions_manifest = parse_json_flag(flags.get("parent-permissions"))?;
    if permissions_template.is_some() || parent_permissions_manifest.is_some() {
        let mut merged = permissions_manifest.unwrap_or_else(|| json!({}));
        if !merged.is_object() {
            merged = json!({});
        }
        if let Some(template) = &permissions_template {
            if let Some(object) = merged.as_object_mut() {
                object.insert("template".to_string(), json!(template));
                if template == "parent" {
                    object.insert("inherit_parent".to_string(), json!(true));
                }
            }
        }
        permissions_manifest = Some(merged);
    }
    let wasm_sandbox = parse_json_flag(flags.get("wasm-policy"))?;
    let voice_session = parse_json_flag(flags.get("voice"))?;
    let receipt_merkle = if parse_bool(flags.get("receipt-merkle"), false) {
        Some(json!({
            "enabled": true,
            "seed": flags.get("receipt-merkle-seed").cloned()
        }))
    } else {
        None
    };
    let previous_receipt_root = flags.get("prev-receipt-root").cloned();
    let success_criteria_override = parse_json_flag(flags.get("success-criteria"))?;

    let mut metadata = json!({
        "source": "xtask.infring-agent-run",
        "workflow": workflow_context
            .as_ref()
            .map(|row| row.1.clone())
            .unwrap_or(Value::Null),
        "parent_permissions_manifest": parent_permissions_manifest
    });
    let workflow_success_criteria = workflow_context
        .as_ref()
        .and_then(|row| row.1.get("native_success_criteria").cloned())
        .filter(|value| !value.is_null());
    let native_success_criteria = match (workflow_success_criteria, success_criteria_override) {
        (Some(mut base), Some(override_value)) => {
            if let (Some(base_object), Some(override_object)) =
                (base.as_object_mut(), override_value.as_object())
            {
                for (key, value) in override_object {
                    base_object.insert(key.clone(), value.clone());
                }
                Some(base)
            } else {
                Some(override_value)
            }
        }
        (Some(base), None) => Some(base),
        (None, Some(override_value)) => Some(override_value),
        (None, None) => None,
    };
    if let Some(criteria) = native_success_criteria {
        if let Some(object) = metadata.as_object_mut() {
            object.insert("native_success_criteria".to_string(), criteria);
        }
    }

    let request_build_started = Instant::now();
    let request = RuntimeLaneRequest {
        name,
        preamble,
        initial_prompt: prompt,
        provider,
        model,
        tools,
        capability_packs: packs,
        lifespan_seconds: Some(lifespan_seconds),
        metadata,
        permissions_manifest,
        wasm_sandbox,
        voice_session,
        receipt_merkle,
        previous_receipt_root,
        schedule_interval_seconds,
        schedule_max_runs,
    };
    let request_build_ms = elapsed_ms(request_build_started);
    let runtime_lane_started = Instant::now();
    let response = match run_runtime_lane(request) {
        Ok(response) => response,
        Err(error) => {
            let runtime_lane_ms = elapsed_ms(runtime_lane_started);
            let envelope = json!({
                "ok": false,
                "contract": {
                    "source": "xtask.infring-agent-run",
                    "terminal_status": "runner_error"
                },
                "receipt": {
                    "type": "runtime_lane_receipt",
                    "status": "runner_error",
                    "error": error.to_string(),
                    "native_tool_receipts": []
                },
                "trace_summary": {
                    "status": "runner_error"
                },
                "xtask_outer_timing_ms": {
                    "parse_flags_ms": parse_flags_ms,
                    "prompt_load_ms": prompt_load_ms,
                    "workflow_load_ms": workflow_load_ms,
                    "preamble_combine_ms": preamble_combine_ms,
                    "request_build_ms": request_build_ms,
                    "runtime_lane_ms": runtime_lane_ms,
                    "xtask_total_ms": elapsed_ms(xtask_started)
                },
                "output": "",
                "error": error.to_string()
            });
            println!(
                "{}",
                serde_json::to_string_pretty(&envelope).expect("encode runtime lane error")
            );
            return Err(anyhow!("xtask_infring_agent_run_failed:{error}"));
        }
    };
    let runtime_lane_ms = elapsed_ms(runtime_lane_started);
    let mut response_value =
        serde_json::to_value(response).expect("encode runtime lane value");
    if let Some(object) = response_value.as_object_mut() {
        object.insert(
            "xtask_outer_timing_ms".to_string(),
            json!({
                "parse_flags_ms": parse_flags_ms,
                "prompt_load_ms": prompt_load_ms,
                "workflow_load_ms": workflow_load_ms,
                "preamble_combine_ms": preamble_combine_ms,
                "request_build_ms": request_build_ms,
                "runtime_lane_ms": runtime_lane_ms,
                "xtask_total_ms": elapsed_ms(xtask_started)
            }),
        );
    }

    println!(
        "{}",
        serde_json::to_string_pretty(&response_value).expect("encode runtime lane")
    );
    Ok(())
}

fn elapsed_ms(started: Instant) -> u64 {
    started.elapsed().as_millis().min(u64::MAX as u128) as u64
}

fn run_native_file_tool_smoke(args: &[String]) -> Result<()> {
    let flags = parse_flag_map(args);
    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0);
    let root = flags
        .get("root")
        .map(PathBuf::from)
        .unwrap_or_else(|| env::temp_dir().join(format!("infring-native-file-smoke-{now_ms}")));
    fs::create_dir_all(&root)
        .with_context(|| format!("native_file_tool_smoke_create_root_failed:{}", root.display()))?;
    let target = root.join("hello.py");
    let dispatcher = NativeToolDispatcher::new(&[
        "file_write".to_string(),
        "file_read".to_string(),
        "file_patch".to_string(),
    ]);
    let calls = vec![
        NativeToolCall {
            id: "smoke_write".to_string(),
            name: "file_write".to_string(),
            args: json!({
                "path": target.display().to_string(),
                "content": "print(\"hello native tools\")\n",
                "overwrite": true
            }),
        },
        NativeToolCall {
            id: "smoke_read_after_write".to_string(),
            name: "file_read".to_string(),
            args: json!({
                "path": target.display().to_string(),
                "start_line": 1,
                "end_line": 20
            }),
        },
        NativeToolCall {
            id: "smoke_patch".to_string(),
            name: "file_patch".to_string(),
            args: json!({
                "path": target.display().to_string(),
                "old": "print(\"hello native tools\")",
                "new": "print(\"hello patched tools\")",
                "allow_multiple": false
            }),
        },
        NativeToolCall {
            id: "smoke_read_after_patch".to_string(),
            name: "file_read".to_string(),
            args: json!({
                "path": target.display().to_string(),
                "start_line": 1,
                "end_line": 20
            }),
        },
    ];
    let receipts = calls
        .into_iter()
        .map(|call| dispatcher.dispatch(call))
        .collect::<Vec<_>>();
    let final_content = fs::read_to_string(&target).unwrap_or_default();
    let ok = receipts.iter().all(|receipt| receipt.status == "ok")
        && final_content.trim() == "print(\"hello patched tools\")";
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "ok": ok,
            "root": root,
            "target": target,
            "final_content": final_content,
            "receipts": receipts,
        }))
        .expect("encode native file tool smoke")
    );
    if ok {
        Ok(())
    } else {
        bail!("native_file_tool_smoke_failed")
    }
}

fn csv_tokens(raw: Option<&String>) -> Vec<String> {
    raw.map(|value| {
        value
            .split(',')
            .map(|token| token.trim().to_string())
            .filter(|token| !token.is_empty())
            .collect::<Vec<_>>()
    })
    .unwrap_or_default()
}

fn parse_json_flag(raw: Option<&String>) -> Result<Option<Value>> {
    let Some(value) = raw else {
        return Ok(None);
    };
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    if let Some(path) = trimmed.strip_prefix('@') {
        let bytes =
            fs::read(path).with_context(|| format!("xtask_json_flag_read_failed:{path}"))?;
        let parsed = serde_json::from_slice::<Value>(&bytes)
            .with_context(|| format!("xtask_json_flag_parse_failed:{path}"))?;
        return Ok(Some(parsed));
    }
    let parsed = serde_json::from_str::<Value>(trimmed)
        .with_context(|| format!("xtask_json_flag_parse_failed:inline:{trimmed}"))?;
    Ok(Some(parsed))
}

fn parse_text_flag(raw: Option<&String>, name: &str) -> Result<Option<String>> {
    let Some(value) = raw else {
        return Ok(None);
    };
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    if let Some(path) = trimmed.strip_prefix('@') {
        let text =
            fs::read_to_string(path).with_context(|| format!("xtask_text_flag_read_failed:{name}:{path}"))?;
        return Ok(Some(text));
    }
    Ok(Some(value.clone()))
}

fn combine_preamble(base: Option<String>, workflow: Option<String>) -> Option<String> {
    match (base, workflow) {
        (Some(base), Some(workflow)) if !base.trim().is_empty() => {
            Some(format!("{}\n\n{}", base.trim(), workflow.trim()))
        }
        (None, Some(workflow)) => Some(workflow),
        (Some(base), None) => Some(base),
        _ => None,
    }
}

fn add_workflow_default_packs(packs: &mut Vec<String>, workflow: Option<&Value>) {
    let Some(workflow) = workflow else {
        return;
    };
    let Some(default_packs) = workflow
        .get("native_capability_packs")
        .and_then(Value::as_array)
    else {
        return;
    };
    for pack in default_packs {
        let Some(pack) = pack.as_str().map(str::trim).filter(|pack| !pack.is_empty()) else {
            continue;
        };
        if !packs.iter().any(|existing| existing == pack) {
            packs.push(pack.to_string());
        }
    }
}

fn load_workflow_context(raw_workflow_id: Option<&String>) -> Result<Option<(String, Value)>> {
    let Some(raw_workflow_id) = raw_workflow_id else {
        return Ok(None);
    };
    let workflow_id = raw_workflow_id.trim();
    if workflow_id.is_empty() {
        return Ok(None);
    }
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .ok_or_else(|| anyhow!("xtask_workspace_root_unavailable"))?;
    let registry_path =
        workspace_root.join("orchestration/src/control_plane/workflows/workflow_registry.json");
    let registry = read_json_file(&registry_path)?;
    let workflow_entry = registry
        .get("workflows")
        .and_then(Value::as_array)
        .and_then(|items| {
            items.iter().find(|entry| {
                entry
                    .get("workflow_id")
                    .and_then(Value::as_str)
                    .map(|value| value == workflow_id)
                    .unwrap_or(false)
            })
        })
        .cloned()
        .ok_or_else(|| anyhow!("xtask_workflow_not_found:{workflow_id}"))?;
    let source_path = workflow_entry
        .get("source_path")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("xtask_workflow_source_path_missing:{workflow_id}"))?;
    let workflow_path = workspace_root.join(source_path);
    let workflow_spec = read_json_file(&workflow_path)?;
    let stages = workflow_spec
        .get("stages")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>()
                .join(", ")
        })
        .unwrap_or_default();
    let children = workflow_spec
        .pointer("/workflow_composition_contract/child_workflow_calls")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| {
                    let capability = item.get("capability").and_then(Value::as_str)?;
                    let child_id = item.get("workflow_id").and_then(Value::as_str)?;
                    Some(format!("{capability}->{child_id}"))
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let description = workflow_spec
        .get("description")
        .and_then(Value::as_str)
        .unwrap_or("");
    let native_tooling = workflow_spec
        .get("native_runtime_tooling")
        .cloned()
        .unwrap_or(Value::Null);
    let native_capability_packs = native_tooling
        .get("capability_packs")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let native_permission_template = native_tooling
        .get("permission_template")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("");
    let native_success_criteria = workflow_spec
        .get("native_execution_success_contract")
        .and_then(|contract| contract.get("native_success_criteria"))
        .cloned()
        .unwrap_or(Value::Null);
    let native_runtime_prompt_policy = workflow_spec
        .get("native_runtime_prompt_policy")
        .cloned()
        .unwrap_or(Value::Null);
    let success_summary = compact_success_criteria_summary(&native_success_criteria);
    let public_reasoning_contract = workflow_spec
        .get("public_reasoning_trace_contract")
        .cloned()
        .unwrap_or(Value::Null);
    let reasoning_summary = compact_public_reasoning_summary(&public_reasoning_contract);
    let completion_evidence_contract = workflow_spec
        .get("completion_evidence_contract")
        .cloned()
        .unwrap_or(Value::Null);
    let persistence_safety_contract = workflow_spec
        .get("coding_persistence_safety_contract")
        .cloned()
        .unwrap_or(Value::Null);
    let persistence_summary = compact_persistence_safety_summary(&persistence_safety_contract);
    let preamble = format!(
        "Native Infring execution brief v3\nWorkflow: {workflow_id}\nSource: {source_path}\nPurpose: {}\nStages: {}\nChildren: {}\nNative packs: {}\nPermission template: {}\nEvidence: {}\nReasoning: {}\nPersistence: {}\nRules: execute the concrete local task with native tools; keep claims receipt-backed; run requested validation; return a structured blocker instead of completion when evidence cannot be produced.",
        compact_workflow_brief_fragment(description, 240),
        compact_workflow_brief_fragment(&stages, 180),
        compact_workflow_brief_fragment(&children.join(", "), 220),
        compact_workflow_brief_fragment(&native_capability_packs.join(", "), 160),
        compact_workflow_brief_fragment(native_permission_template, 80),
        compact_workflow_brief_fragment(&success_summary, 260),
        compact_workflow_brief_fragment(&reasoning_summary, 160),
        compact_workflow_brief_fragment(&persistence_summary, 160)
    );
    let metadata = json!({
        "native_execution_brief_version": "native_execution_brief_v2",
        "workflow_id": workflow_id,
        "source_path": source_path,
        "runtime_selectable": workflow_entry
            .get("runtime_selectable")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        "promotion_status": workflow_entry
            .get("promotion_status")
            .and_then(Value::as_str)
            .unwrap_or(""),
        "tier": workflow_entry
            .get("tier")
            .and_then(Value::as_str)
            .unwrap_or(""),
        "child_workflow_calls": children,
        "native_capability_packs": native_capability_packs,
        "native_permission_template": native_permission_template,
        "native_success_criteria": native_success_criteria,
        "native_runtime_prompt_policy": native_runtime_prompt_policy,
        "public_reasoning_trace_contract": public_reasoning_contract,
        "completion_evidence_contract": completion_evidence_contract,
        "coding_persistence_safety_contract": persistence_safety_contract,
    });
    Ok(Some((preamble, metadata)))
}

fn compact_workflow_brief_fragment(value: &str, max_chars: usize) -> String {
    let compact = value.split_whitespace().collect::<Vec<_>>().join(" ");
    if compact.chars().count() <= max_chars {
        return compact;
    }
    let mut out = compact.chars().take(max_chars).collect::<String>();
    out.push_str("...");
    out
}

fn compact_success_criteria_summary(criteria: &Value) -> String {
    if !criteria.is_object() {
        return "none declared".to_string();
    }
    let mut parts = Vec::<String>::new();
    if criteria
        .get("requires_native_tool_use")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        parts.push("requires native tool use".to_string());
    }
    if criteria
        .get("requires_successful_discovery_receipt")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        parts.push("requires successful file_list/file_stat discovery receipt".to_string());
    }
    if criteria
        .get("requires_successful_mutation_receipt")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        parts.push("requires successful file_write/file_patch receipt".to_string());
    }
    if let Some(limit) = criteria
        .get("max_context_only_turns")
        .and_then(Value::as_u64)
    {
        parts.push(format!("max context-only tool turns {limit}"));
    }
    if criteria
        .get("micro_direct_write_enabled")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        parts.push("allows micro direct-write lane".to_string());
    }
    if criteria
        .get("synthesize_final_after_successful_micro_mutation")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        parts.push("synthesizes final after successful micro mutation".to_string());
    }
    if criteria
        .get("completion_evidence_required_for_multi_requirement_tasks")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        parts.push("requires completion evidence for multi-requirement tasks".to_string());
    }
    if criteria
        .get("synthesize_completion_evidence_on_finalization_timeout")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        parts.push("synthesizes completion evidence on finalization timeout".to_string());
    }
    if criteria
        .get("repair_uncovered_requirements_before_final")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        parts.push("repairs uncovered requirements before final".to_string());
    }
    if let Some(limit) = criteria
        .get("completion_evidence_repair_max_turns")
        .and_then(Value::as_u64)
    {
        parts.push(format!("completion evidence repair max turns {limit}"));
    }
    if let Some(limit) = criteria
        .get("empty_tool_retry_limit")
        .and_then(Value::as_u64)
    {
        parts.push(format!("empty-tool retry limit {limit}"));
    }
    if criteria
        .get("force_discovery_first_turn")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        parts.push("forces discovery-first native tool turn".to_string());
    }
    if criteria
        .get("force_read_first_turn")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        parts.push("forces read-first native tool turn".to_string());
    }
    if let Some(limit) = criteria
        .get("provider_timeout_seconds")
        .and_then(Value::as_u64)
    {
        parts.push(format!("provider timeout {limit}s"));
    }
    if let Some(limit) = criteria
        .get("native_wall_timeout_seconds")
        .and_then(Value::as_u64)
    {
        parts.push(format!("native loop wall timeout {limit}s"));
    }
    if criteria
        .get("partial_progress_on_timeout")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        parts.push("returns partial progress on timeout".to_string());
    }
    if let Some(limit) = criteria
        .get("partial_recovery_max_turns")
        .and_then(Value::as_u64)
    {
        parts.push(format!("partial recovery max turns {limit}"));
    }
    if let Some(limit) = criteria
        .get("recovery_provider_timeout_seconds")
        .and_then(Value::as_u64)
    {
        parts.push(format!("recovery provider timeout {limit}s"));
    }
    if parts.is_empty() {
        "declared but non-restrictive".to_string()
    } else {
        parts.join("; ")
    }
}

fn compact_public_reasoning_summary(contract: &Value) -> String {
    if !contract.is_object() {
        return "none declared".to_string();
    }
    let role = contract
        .get("workflow_role")
        .and_then(Value::as_str)
        .unwrap_or("unspecified_role");
    let emits = contract
        .get("emits")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>()
                .join(",")
        })
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "public_reasoning_trace_v1".to_string());
    let visibility = contract
        .get("parent_visibility")
        .or_else(|| contract.pointer("/presentation_policy/default_visible_mode"))
        .and_then(Value::as_str)
        .unwrap_or("rollup_plus_refs");
    let redaction = contract
        .get("redaction_policy")
        .and_then(Value::as_str)
        .unwrap_or("no_hidden_chain_of_thought");
    format!("role={role}; emits={emits}; visibility={visibility}; redaction={redaction}")
}

fn compact_persistence_safety_summary(contract: &Value) -> String {
    if !contract.is_object() {
        return "none declared".to_string();
    }
    let requirements = contract
        .get("requirements")
        .or_else(|| contract.get("implementation_guidance"))
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .take(4)
                .collect::<Vec<_>>()
                .join("; ")
        })
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| {
            "load helpers must treat missing and empty persistence files as empty state".to_string()
        });
    format!("file-backed persistence safety: {requirements}")
}

fn read_json_file(path: &Path) -> Result<Value> {
    let bytes = fs::read(path).with_context(|| format!("xtask_read_failed:{}", path.display()))?;
    serde_json::from_slice::<Value>(&bytes)
        .with_context(|| format!("xtask_parse_json_failed:{}", path.display()))
}

fn now_isoish() -> String {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("unix:{ts}")
}
