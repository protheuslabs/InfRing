// Layer ownership: Core Layer 2 (Scheduling + Execution) - agent runtime surface coordination.
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CapabilityPackSpec {
    pub id: String,
    pub description: String,
    pub tools: Vec<String>,
    pub default_interval_seconds: u64,
    pub default_max_runs: Option<u32>,
    pub required_permissions: Vec<String>,
    pub autonomy_profile: String,
}

pub trait StaticCapabilityPack {
    fn spec() -> CapabilityPackSpec;
}

pub struct ResearchCapabilityPack;
pub struct WebOpsCapabilityPack;
pub struct LeadGenCapabilityPack;
pub struct SocialSignalCapabilityPack;
pub struct IssueOpsCapabilityPack;
pub struct LocalCodingFilesCapabilityPack;

impl StaticCapabilityPack for ResearchCapabilityPack {
    fn spec() -> CapabilityPackSpec {
        CapabilityPackSpec {
            id: "research".to_string(),
            description: "Research and synthesis flow with web + memory reads".to_string(),
            tools: vec![
                "web.search".to_string(),
                "web.fetch".to_string(),
                "memory.read".to_string(),
                "summary.synthesize".to_string(),
            ],
            default_interval_seconds: 600,
            default_max_runs: None,
            required_permissions: vec!["memory.read".to_string(), "web.search".to_string()],
            autonomy_profile: "continuous".to_string(),
        }
    }
}

impl StaticCapabilityPack for WebOpsCapabilityPack {
    fn spec() -> CapabilityPackSpec {
        CapabilityPackSpec {
            id: "web-ops".to_string(),
            description: "Operational health checks for tooling and workflow routes".to_string(),
            tools: vec![
                "web.search".to_string(),
                "web.fetch".to_string(),
                "tool.health_probe".to_string(),
                "receipt.inspect".to_string(),
            ],
            default_interval_seconds: 300,
            default_max_runs: None,
            required_permissions: vec!["web.search".to_string(), "web.fetch".to_string()],
            autonomy_profile: "continuous".to_string(),
        }
    }
}

impl StaticCapabilityPack for LeadGenCapabilityPack {
    fn spec() -> CapabilityPackSpec {
        CapabilityPackSpec {
            id: "lead-gen".to_string(),
            description: "Lead generation campaign flow with bounded outreach cadences".to_string(),
            tools: vec![
                "web.search".to_string(),
                "web.fetch".to_string(),
                "crm.lead_upsert".to_string(),
                "message.compose".to_string(),
                "memory.write".to_string(),
            ],
            default_interval_seconds: 900,
            default_max_runs: Some(96),
            required_permissions: vec![
                "web.search".to_string(),
                "memory.write".to_string(),
                "outreach.send".to_string(),
            ],
            autonomy_profile: "campaign".to_string(),
        }
    }
}

impl StaticCapabilityPack for SocialSignalCapabilityPack {
    fn spec() -> CapabilityPackSpec {
        CapabilityPackSpec {
            id: "social-signal".to_string(),
            description: "Signal monitoring across public channels with summary rollups"
                .to_string(),
            tools: vec![
                "web.search".to_string(),
                "web.fetch".to_string(),
                "social.monitor".to_string(),
                "summary.synthesize".to_string(),
                "memory.write".to_string(),
            ],
            default_interval_seconds: 420,
            default_max_runs: None,
            required_permissions: vec![
                "web.search".to_string(),
                "social.monitor".to_string(),
                "memory.write".to_string(),
            ],
            autonomy_profile: "continuous".to_string(),
        }
    }
}

impl StaticCapabilityPack for IssueOpsCapabilityPack {
    fn spec() -> CapabilityPackSpec {
        CapabilityPackSpec {
            id: "issue-ops".to_string(),
            description: "Runtime issue escalation and GitHub report orchestration".to_string(),
            tools: vec![
                "tool.health_probe".to_string(),
                "receipt.inspect".to_string(),
                "github.issue.create".to_string(),
                "memory.read".to_string(),
                "memory.write".to_string(),
            ],
            default_interval_seconds: 300,
            default_max_runs: Some(480),
            required_permissions: vec![
                "memory.read".to_string(),
                "memory.write".to_string(),
                "github.issue.create".to_string(),
            ],
            autonomy_profile: "incident".to_string(),
        }
    }
}

impl StaticCapabilityPack for LocalCodingFilesCapabilityPack {
    fn spec() -> CapabilityPackSpec {
        CapabilityPackSpec {
            id: "local-coding-files".to_string(),
            description: "Native local coding file discovery, read, write, patch, and validation command tools"
                .to_string(),
            tools: vec![
                "file_list".to_string(),
                "file_stat".to_string(),
                "file_read".to_string(),
                "file_read_many".to_string(),
                "file_write".to_string(),
                "file_patch".to_string(),
                "command_resolve".to_string(),
                "command_run".to_string(),
            ],
            default_interval_seconds: 3600,
            default_max_runs: None,
            required_permissions: vec![
                "file.read".to_string(),
                "file.write".to_string(),
                "file.patch".to_string(),
                "command.resolve".to_string(),
                "command.run".to_string(),
            ],
            autonomy_profile: "local_coding".to_string(),
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct CapabilityPackCatalog {
    packs: BTreeMap<String, CapabilityPackSpec>,
}

impl CapabilityPackCatalog {
    pub fn new() -> Self {
        let mut catalog = Self::default();
        catalog.register(ResearchCapabilityPack::spec());
        catalog.register(WebOpsCapabilityPack::spec());
        catalog.register(LeadGenCapabilityPack::spec());
        catalog.register(SocialSignalCapabilityPack::spec());
        catalog.register(IssueOpsCapabilityPack::spec());
        catalog.register(LocalCodingFilesCapabilityPack::spec());
        catalog
    }

    pub fn register(&mut self, pack: CapabilityPackSpec) {
        self.packs.insert(pack.id.clone(), pack);
    }

    pub fn get(&self, id: &str) -> Option<&CapabilityPackSpec> {
        self.packs.get(id)
    }

    pub fn all(&self) -> Vec<CapabilityPackSpec> {
        self.packs.values().cloned().collect()
    }

    pub fn default_interval_for_pack(&self, id: &str) -> Option<u64> {
        self.get(id).map(|pack| pack.default_interval_seconds)
    }

    pub fn default_max_runs_for_packs(&self, pack_ids: &[String]) -> Option<u32> {
        let mut chosen: Option<u32> = None;
        for pack_id in pack_ids {
            let Some(spec) = self.get(pack_id) else {
                continue;
            };
            if let Some(limit) = spec.default_max_runs {
                chosen = Some(chosen.map(|current| current.min(limit)).unwrap_or(limit));
            }
        }
        chosen
    }

    pub fn required_permissions_for_packs(&self, pack_ids: &[String]) -> Vec<String> {
        let mut out = BTreeSet::<String>::new();
        for pack_id in pack_ids {
            let Some(spec) = self.get(pack_id) else {
                continue;
            };
            for permission in &spec.required_permissions {
                let token = permission.trim().to_ascii_lowercase();
                if !token.is_empty() {
                    out.insert(token);
                }
            }
        }
        out.into_iter().collect()
    }

    pub fn autonomy_profiles_for_packs(&self, pack_ids: &[String]) -> Vec<String> {
        let mut out = BTreeSet::<String>::new();
        for pack_id in pack_ids {
            let Some(spec) = self.get(pack_id) else {
                continue;
            };
            let token = spec.autonomy_profile.trim().to_ascii_lowercase();
            if !token.is_empty() {
                out.insert(token);
            }
        }
        out.into_iter().collect()
    }

    pub fn expand_tools(&self, pack_ids: &[String], explicit_tools: &[String]) -> Vec<String> {
        let mut out = BTreeSet::<String>::new();
        for tool in explicit_tools {
            let token = tool.trim();
            if !token.is_empty() {
                out.insert(token.to_string());
            }
        }
        for pack_id in pack_ids {
            if let Some(pack) = self.get(pack_id) {
                for tool in &pack.tools {
                    out.insert(tool.clone());
                }
            }
        }
        out.into_iter().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_registers_autonomy_packs() {
        let catalog = CapabilityPackCatalog::new();
        assert!(catalog.get("lead-gen").is_some());
        assert!(catalog.get("social-signal").is_some());
        assert!(catalog.get("issue-ops").is_some());
        assert!(catalog.get("local-coding-files").is_some());
    }

    #[test]
    fn catalog_collects_permissions_and_profiles_across_packs() {
        let catalog = CapabilityPackCatalog::new();
        let pack_ids = vec!["lead-gen".to_string(), "issue-ops".to_string()];
        let permissions = catalog.required_permissions_for_packs(&pack_ids);
        assert!(permissions.iter().any(|item| item == "github.issue.create"));
        let profiles = catalog.autonomy_profiles_for_packs(&pack_ids);
        assert!(profiles.iter().any(|item| item == "campaign"));
        assert!(profiles.iter().any(|item| item == "incident"));
    }
}
