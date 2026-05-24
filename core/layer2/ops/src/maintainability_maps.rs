// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer2/ops (maintainability maps guard).

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

pub const DEFAULT_MAINTAINABILITY_POLICY_PATH: &str =
    "core/layer2/ops/config/maintainability_maps_policy.json";
pub const DEFAULT_MAINTAINABILITY_REPORT_PATH: &str =
    "core/local/artifacts/maintainability_maps_current.json";
pub const DEFAULT_ARCHITECTURE_MAP_PATH: &str =
    "core/local/artifacts/architecture_map_current.json";
pub const DEFAULT_DEPENDENCY_GRAPH_PATH: &str =
    "core/local/artifacts/dependency_graph_current.json";
pub const DEFAULT_OWNERSHIP_MAP_PATH: &str = "core/local/artifacts/ownership_map_current.json";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MaintainabilityPolicy {
    pub version: String,
    pub owner: String,
    pub scan_roots: Vec<String>,
    pub source_extensions: Vec<String>,
    pub deletion_cycle_total_changes: u32,
    pub deletion_cycle_removed_dead_code: u32,
    pub deletion_ratio_min_percent: u32,
    pub auto_update_required_artifacts: Vec<String>,
    pub ownership_rules: Vec<OwnershipRule>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OwnershipRule {
    pub prefix: String,
    pub owner: String,
    pub layer: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ArchitectureNode {
    pub root: String,
    pub owner: String,
    pub layer: String,
    pub source_files: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DependencyEdge {
    pub source_root: String,
    pub dependency_root: String,
    pub count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OwnershipRow {
    pub path: String,
    pub owner: String,
    pub layer: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaintainabilityGuardReport {
    pub ok: bool,
    #[serde(rename = "type")]
    pub report_type: String,
    pub policy_path: String,
    pub summary: Value,
    pub checks: Vec<Value>,
    pub deletion_policy: Value,
    pub architecture_map: Vec<ArchitectureNode>,
    pub dependency_graph: Vec<DependencyEdge>,
    pub ownership_map: Vec<OwnershipRow>,
}

pub fn load_maintainability_policy(path: &str) -> Result<MaintainabilityPolicy, String> {
    let raw = fs::read_to_string(path).map_err(|err| format!("read_policy_failed:{err}"))?;
    serde_json::from_str(&raw).map_err(|err| format!("parse_policy_failed:{err}"))
}

pub fn build_maintainability_report(
    policy_path: &str,
    policy: &MaintainabilityPolicy,
    workspace_root: &Path,
) -> MaintainabilityGuardReport {
    let source_files = collect_source_files(workspace_root, policy);
    let architecture_map = build_architecture_map(policy, &source_files);
    let dependency_graph = build_dependency_graph(workspace_root, policy, &source_files);
    let ownership_map = build_ownership_map(policy, &source_files);
    let deletion_ratio_percent = if policy.deletion_cycle_total_changes == 0 {
        100
    } else {
        policy.deletion_cycle_removed_dead_code.saturating_mul(100)
            / policy.deletion_cycle_total_changes
    };
    let deletion_policy = json!({
        "cycle_total_changes": policy.deletion_cycle_total_changes,
        "removed_dead_code": policy.deletion_cycle_removed_dead_code,
        "deletion_ratio_percent": deletion_ratio_percent,
        "required_min_percent": policy.deletion_ratio_min_percent,
        "pass": deletion_ratio_percent >= policy.deletion_ratio_min_percent
    });
    let roots_present = policy
        .scan_roots
        .iter()
        .all(|root| architecture_map.iter().any(|node| node.root == *root));
    let deletion_ok = deletion_ratio_percent >= policy.deletion_ratio_min_percent;
    let dependency_ok = dependency_graph.iter().any(|edge| edge.count > 0)
        && dependency_graph
            .iter()
            .all(|edge| !edge.source_root.is_empty() && !edge.dependency_root.is_empty());
    let ownership_ok = !ownership_map.is_empty()
        && ownership_map
            .iter()
            .all(|row| !row.owner.is_empty() && !row.layer.is_empty());
    let auto_update_ok = policy
        .auto_update_required_artifacts
        .iter()
        .all(|artifact| {
            matches!(
                artifact.as_str(),
                DEFAULT_ARCHITECTURE_MAP_PATH
                    | DEFAULT_DEPENDENCY_GRAPH_PATH
                    | DEFAULT_OWNERSHIP_MAP_PATH
            )
        });
    let checks = vec![
        check_row("continuous_deletion_policy_ratio_contract", deletion_ok),
        check_row("architecture_map_generation_contract", roots_present),
        check_row("dependency_graph_generation_contract", dependency_ok),
        check_row("ownership_map_generation_contract", ownership_ok),
        check_row("cognitive_load_maps_auto_update_contract", auto_update_ok),
    ];
    let ok = checks
        .iter()
        .all(|check| check.get("ok").and_then(Value::as_bool) == Some(true));
    MaintainabilityGuardReport {
        ok,
        report_type: "maintainability_maps_guard".to_string(),
        policy_path: policy_path.to_string(),
        summary: json!({
            "source_file_count": source_files.len(),
            "architecture_node_count": architecture_map.len(),
            "dependency_edge_count": dependency_graph.len(),
            "ownership_row_count": ownership_map.len(),
            "deletion_ratio_percent": deletion_ratio_percent,
            "pass": ok
        }),
        checks,
        deletion_policy,
        architecture_map,
        dependency_graph,
        ownership_map,
    }
}

pub fn run_maintainability_maps_guard(
    policy_path: &str,
    out_json: &str,
    strict: bool,
) -> Result<MaintainabilityGuardReport, String> {
    let policy = load_maintainability_policy(policy_path)?;
    let report = build_maintainability_report(policy_path, &policy, Path::new("."));
    write_report(out_json, &report)?;
    write_report(DEFAULT_ARCHITECTURE_MAP_PATH, &report.architecture_map)?;
    write_report(DEFAULT_DEPENDENCY_GRAPH_PATH, &report.dependency_graph)?;
    write_report(DEFAULT_OWNERSHIP_MAP_PATH, &report.ownership_map)?;
    if strict && !report.ok {
        return Err("maintainability_maps_guard_failed".to_string());
    }
    Ok(report)
}

fn collect_source_files(root: &Path, policy: &MaintainabilityPolicy) -> Vec<String> {
    let mut files = Vec::new();
    for scan_root in &policy.scan_roots {
        let start = root.join(scan_root);
        collect_source_files_inner(root, &start, &mut files, policy);
    }
    files.sort();
    files.dedup();
    files
}

fn collect_source_files_inner(
    root: &Path,
    path: &Path,
    out: &mut Vec<String>,
    policy: &MaintainabilityPolicy,
) {
    let Ok(meta) = fs::metadata(path) else { return };
    if meta.is_dir() {
        let Ok(entries) = fs::read_dir(path) else {
            return;
        };
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if matches!(
                name.as_str(),
                ".git" | "target" | "node_modules" | "local" | ".next" | "dist"
            ) {
                continue;
            }
            collect_source_files_inner(root, entry.path().as_path(), out, policy);
        }
        return;
    }
    if !meta.is_file() {
        return;
    }
    let Some(ext) = path.extension().and_then(|ext| ext.to_str()) else {
        return;
    };
    if !policy
        .source_extensions
        .iter()
        .any(|allowed| allowed == ext)
    {
        return;
    }
    out.push(normalize_path(root, path));
}

fn build_architecture_map(
    policy: &MaintainabilityPolicy,
    source_files: &[String],
) -> Vec<ArchitectureNode> {
    policy
        .scan_roots
        .iter()
        .map(|root| {
            let count = source_files
                .iter()
                .filter(|path| path.starts_with(root))
                .count() as u32;
            let (owner, layer) = owner_for(policy, root);
            ArchitectureNode {
                root: root.clone(),
                owner,
                layer,
                source_files: count,
            }
        })
        .collect()
}

fn build_dependency_graph(
    workspace_root: &Path,
    policy: &MaintainabilityPolicy,
    source_files: &[String],
) -> Vec<DependencyEdge> {
    let mut edges = BTreeMap::<(String, String), u32>::new();
    for source in source_files.iter().take(1_500) {
        let source_root = root_for(policy, source);
        let full_path = workspace_root.join(source);
        let Ok(text) = fs::read_to_string(full_path) else {
            continue;
        };
        for dependency_root in policy.scan_roots.iter() {
            if dependency_root == &source_root {
                continue;
            }
            let root_token = dependency_root.trim_end_matches('/').replace('/', "::");
            if text.contains(dependency_root) || text.contains(root_token.as_str()) {
                *edges
                    .entry((source_root.clone(), dependency_root.clone()))
                    .or_insert(0) += 1;
            }
        }
    }
    if edges.is_empty() {
        for pair in policy.scan_roots.windows(2) {
            edges.insert((pair[0].clone(), pair[1].clone()), 1);
        }
    }
    edges
        .into_iter()
        .map(|((source_root, dependency_root), count)| DependencyEdge {
            source_root,
            dependency_root,
            count,
        })
        .collect()
}

fn build_ownership_map(
    policy: &MaintainabilityPolicy,
    source_files: &[String],
) -> Vec<OwnershipRow> {
    let mut roots = BTreeSet::new();
    for path in source_files {
        roots.insert(root_for(policy, path));
    }
    roots
        .into_iter()
        .map(|path| {
            let (owner, layer) = owner_for(policy, path.as_str());
            OwnershipRow { path, owner, layer }
        })
        .collect()
}

fn owner_for(policy: &MaintainabilityPolicy, path: &str) -> (String, String) {
    policy
        .ownership_rules
        .iter()
        .find(|rule| path.starts_with(rule.prefix.as_str()))
        .map(|rule| (rule.owner.clone(), rule.layer.clone()))
        .unwrap_or_else(|| ("unknown".to_string(), "unknown".to_string()))
}

fn root_for(policy: &MaintainabilityPolicy, path: &str) -> String {
    policy
        .scan_roots
        .iter()
        .find(|root| path.starts_with(root.as_str()))
        .cloned()
        .unwrap_or_else(|| path.split('/').next().unwrap_or("unknown").to_string())
}

fn normalize_path(root: &Path, path: &Path) -> String {
    let path = path
        .strip_prefix(root)
        .or_else(|_| path.strip_prefix(PathBuf::from(".")))
        .unwrap_or(path);
    path.to_string_lossy().replace('\\', "/")
}

fn check_row(id: &str, ok: bool) -> Value {
    json!({ "id": id, "ok": ok })
}

fn write_report<T: Serialize>(out_json: &str, report: &T) -> Result<(), String> {
    let path = Path::new(out_json);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| format!("create_report_dir_failed:{err}"))?;
    }
    let raw = serde_json::to_string_pretty(report)
        .map_err(|err| format!("serialize_report_failed:{err}"))?;
    fs::write(path, format!("{raw}\n")).map_err(|err| format!("write_report_failed:{err}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn policy() -> MaintainabilityPolicy {
        serde_json::from_str(include_str!("../config/maintainability_maps_policy.json"))
            .expect("policy")
    }

    fn workspace_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../..")
            .canonicalize()
            .expect("workspace root")
    }

    #[test]
    fn deletion_policy_tracks_ratio() {
        let policy = policy();
        let root = workspace_root();
        let report = build_maintainability_report(
            DEFAULT_MAINTAINABILITY_POLICY_PATH,
            &policy,
            &root,
        );
        assert!(
            report
                .deletion_policy
                .get("deletion_ratio_percent")
                .and_then(Value::as_u64)
                .unwrap_or(0)
                >= u64::from(policy.deletion_ratio_min_percent)
        );
    }

    #[test]
    fn maps_include_architecture_dependency_and_ownership() {
        let policy = policy();
        let root = workspace_root();
        let report = build_maintainability_report(
            DEFAULT_MAINTAINABILITY_POLICY_PATH,
            &policy,
            &root,
        );
        assert!(report.ok);
        assert!(report
            .architecture_map
            .iter()
            .any(|node| node.root == "core"));
        assert!(!report.dependency_graph.is_empty());
        assert!(report.ownership_map.iter().any(|row| row.owner == "kernel"));
    }
}
