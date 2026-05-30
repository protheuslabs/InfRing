
pub fn run_constitution_guardian(repo_root: &Path, argv: &[String]) -> (Value, i32) {
    let parsed = parse_args(argv);
    let cmd = parsed
        .positional
        .first()
        .map(|v| v.to_ascii_lowercase())
        .unwrap_or_else(|| "status".to_string());
    let policy = load_constitution_policy(repo_root, &parsed);
    let paths = constitution_paths(repo_root, &policy);
    let _ = fs::create_dir_all(&paths.proposals_dir);
    let _ = fs::create_dir_all(&paths.history_dir);

    match cmd.as_str() {
        "init-genesis" => {
            if !paths.constitution.exists() {
                return (
                    json!({
                        "ok": false,
                        "type": "constitution_genesis",
                        "error": "constitution_missing",
                        "constitution_path": normalize_rel(paths.constitution.to_string_lossy())
                    }),
                    1,
                );
            }
            let force = bool_flag(&parsed, "force", false);
            if paths.genesis.exists() && !force {
                return (
                    json!({
                        "ok": true,
                        "type": "constitution_genesis",
                        "already_initialized": true,
                        "genesis_path": normalize_rel(paths.genesis.to_string_lossy())
                    }),
                    0,
                );
            }
            let constitution_sha = match sha256_hex_file(&paths.constitution) {
                Ok(v) => v,
                Err(err) => {
                    return (
                        json!({"ok": false, "type": "constitution_genesis", "error": clean(err, 220)}),
                        1,
                    )
                }
            };
            let genesis = json!({
                "type": "constitution_genesis",
                "ts": now_iso(),
                "constitution_path": normalize_rel(paths.constitution.to_string_lossy()),
                "constitution_sha256": constitution_sha,
                "genesis_id": format!("genesis_{}", &sha256_hex_bytes(now_iso().as_bytes())[0..12])
            });
            if let Err(err) = write_json_atomic(&paths.genesis, &genesis) {
                return (
                    json!({"ok": false, "type": "constitution_genesis", "error": clean(err, 220)}),
                    1,
                );
            }
            let _ = append_jsonl(
                &paths.events,
                &json!({"ts": now_iso(), "type": "constitution_genesis_initialized"}),
            );
            (
                json!({"ok": true, "type": "constitution_genesis", "genesis": genesis}),
                0,
            )
        }
        "propose-change" => {
            let candidate_file = clean(
                flag(&parsed, "candidate-file")
                    .or_else(|| flag(&parsed, "candidate_file"))
                    .unwrap_or(""),
                420,
            );
            let proposer = clean(
                flag(&parsed, "proposer-id")
                    .or_else(|| flag(&parsed, "proposer_id"))
                    .unwrap_or(""),
                120,
            );
            let reason = clean(flag(&parsed, "reason").unwrap_or(""), 400);
            if candidate_file.is_empty() || proposer.is_empty() || reason.is_empty() {
                return (
                    json!({"ok": false, "type": "constitution_propose_change", "error": "candidate_file_proposer_id_reason_required"}),
                    1,
                );
            }
            let candidate_abs = resolve_runtime_or_state(repo_root, &candidate_file);
            if !candidate_abs.exists() {
                return (
                    json!({"ok": false, "type": "constitution_propose_change", "error": "candidate_file_missing"}),
                    1,
                );
            }
            let proposal_id = clean(
                flag(&parsed, "proposal-id")
                    .or_else(|| flag(&parsed, "proposal_id"))
                    .unwrap_or(&format!(
                        "ccp_{}",
                        &sha256_hex_bytes(now_iso().as_bytes())[0..10]
                    )),
                120,
            );
            let proposal_dir = paths.proposals_dir.join(&proposal_id);
            let candidate_copy = proposal_dir.join("candidate_constitution.md");
            if let Some(parent) = candidate_copy.parent() {
                let _ = fs::create_dir_all(parent);
            }
            if let Err(err) = fs::copy(&candidate_abs, &candidate_copy) {
                return (
                    json!({"ok": false, "type": "constitution_propose_change", "error": clean(format!("copy_candidate_failed:{err}"), 220)}),
                    1,
                );
            }
            let candidate_sha = sha256_hex_file(&candidate_copy).unwrap_or_default();
            let proposal = json!({
                "proposal_id": proposal_id,
                "status": "pending_approval",
                "created_at": now_iso(),
                "proposer_id": proposer,
                "reason": reason,
                "candidate_file": normalize_rel(candidate_copy.to_string_lossy()),
                "candidate_sha256": candidate_sha,
                "approvals": [],
                "veto": null,
                "gauntlet": null,
                "activated_at": null
            });
            if let Err(err) = save_proposal(&paths, &proposal_id, &proposal) {
                return (
                    json!({"ok": false, "type": "constitution_propose_change", "error": clean(err, 220)}),
                    1,
                );
            }
            let _ = append_jsonl(
                &paths.events,
                &json!({"ts": now_iso(), "type": "constitution_proposal_created", "proposal_id": proposal_id}),
            );
            (
                json!({"ok": true, "type": "constitution_propose_change", "proposal": proposal}),
                0,
            )
        }
        "approve-change" => {
            let proposal_id = clean(
                flag(&parsed, "proposal-id")
                    .or_else(|| flag(&parsed, "proposal_id"))
                    .unwrap_or(""),
                120,
            );
            let approver_id = clean(
                flag(&parsed, "approver-id")
                    .or_else(|| flag(&parsed, "approver_id"))
                    .unwrap_or(""),
                120,
            );
            let approval_note = clean(
                flag(&parsed, "approval-note")
                    .or_else(|| flag(&parsed, "approval_note"))
                    .unwrap_or(""),
                500,
            );
            if proposal_id.is_empty()
                || approver_id.is_empty()
                || approval_note.len() < policy.min_approval_note_chars
            {
                return (
                    json!({"ok": false, "type": "constitution_approve_change", "error": "proposal_id_approver_id_and_approval_note_required"}),
                    1,
                );
            }
            let mut proposal = match load_proposal(&paths, &proposal_id) {
                Some(v) if v.is_object() => v,
                _ => {
                    return (
                        json!({"ok": false, "type": "constitution_approve_change", "error": "proposal_missing"}),
                        1,
                    )
