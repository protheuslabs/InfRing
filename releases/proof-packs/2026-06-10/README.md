# Release Proof Pack

- version: 2026-06-10
- pack_root: /Users/jay/.openclaw/workspace/releases/proof-packs/2026-06-10
- required_missing: 0
- stale_artifacts: 1
- failed_artifacts: 15
- historical_snapshot_artifacts: 1
- summary_consistency_failures: 0
- category_required_missing_sum: 0
- category_artifact_count_sum: 225
- category_required_total_sum: 143
- release_blocking_issue_count: 17
- top_blocker_count: 20
- primary_blocker_class: required_failed_artifacts
- primary_blocker_artifact: artifacts/web_tooling_context_soak_report_latest.json
- primary_blocker_action: repair failing required artifact artifacts/web_tooling_context_soak_report_latest.json
- primary_blocker_dedupe_key: release_proof_pack:required_failed_artifacts:artifacts/web_tooling_context_soak_report_latest.json
- primary_blocker_priority_score: 800
- primary_blocker_owner: core/kernel
- primary_blocker_target_layer: kernel
- primary_blocker_escalation_tier: release_blocker
- primary_blocker_release_gate_effect: blocks_release_until_closed
- primary_blocker_operator_next_step: repair failing required artifact artifacts/web_tooling_context_soak_report_latest.json; rerun release proof-pack assembly and confirm this dedupe key disappears
- primary_blocker_triage_queue: release_blockers
- primary_blocker_lifecycle_state: candidate_open
- primary_blocker_source_artifact_count: 2
- primary_blocker_closure_verification_command: node client/runtime/lib/ts_entrypoint.ts tests/tooling/scripts/ci/release_proof_pack_assemble.ts --strict=0
- top_blockers_actionable: true
- top_blocker_actionability_failure_count: 0
- top_blocker_action_count: 3

| artifact | category | required | exists | sha256 |
| --- | --- | :---: | :---: | --- |
| artifacts/web_tooling_context_soak_report_latest.json | workload_and_quality | yes | yes | 86479220df24865432372677f8f56f85bcd0a1977524abb30bfd1bdcf6b5d0eb |
| artifacts/web_tooling_reliability_latest.json | workload_and_quality | yes | yes | ee4dd89174ca9cdf4ea13130e0c0c8228c55b379cf1611250d4c2b67e493aded |
| artifacts/workflow_failure_recovery_latest.json | workload_and_quality | yes | yes | 2cd84ac8dd1fef4cee3ad1d1e233fb65e3f8353ddbf662af341c141060a0b026 |
| artifacts/workspace_tooling_context_soak_report_latest.json | workload_and_quality | yes | yes | a70b0b2be9e6cd018368875110252aa498b18f6cf02d539fe1460bfda92720e4 |
| artifacts/workspace_tooling_soak_report_latest.json | workload_and_quality | yes | yes | a70b0b2be9e6cd018368875110252aa498b18f6cf02d539fe1460bfda92720e4 |
| client/runtime/local/state/release/scorecard/release_scorecard.json | workload_and_quality | yes | yes | cd419763de8ce7255f4e0da4cd22eb7ba72d0ee5b8696347963432e659da0a48 |
| core/local/artifacts/agent_surface_status_guard_current.json | release_governance | yes | yes | 1c603202e051f803648bff575717de4b0cead0b57cab81c06a563290635e9c1b |
| core/local/artifacts/agentic_loop_doctrine_guard_current.json | release_governance | yes | yes | ea7f57333ef1f717c75dc9c269d2003ed5a3925f0516e1b875dd7af92fd87f41 |
| core/local/artifacts/architecture_nexus_required_artifact_guard_current.json | release_governance | yes | yes | dd73cd58a5868b898d4668ed68060aa1b68a2bd74843b514e7b1ebb8d8865c02 |
| core/local/artifacts/binary_size_profile_policy_guard_current.json | runtime_proof | yes | yes | 6fc933f3dc584418dacc0b1e0455a86ecc11894f380e8717f51d39d60a7c54e2 |
| core/local/artifacts/capability_proof_burden_guard_current.json | release_governance | yes | yes | f227d4632b01496defb6c7886c7a8108d5d1797f719e3c0ae5724e0e1fe55ad0 |
| core/local/artifacts/chat_rendering_experience_guard_current.json | workload_and_quality | yes | yes | e0941787e4b7303f8cd4b68fb53e1eaae35727d45b397838415a1693eb42e469 |
| core/local/artifacts/client_deauthority_closure_guard_current.json | workload_and_quality | yes | yes | fc0f9c5c2ce788f70b3beda918db48017b30e6facac64eecada06677ba56f3e7 |
| core/local/artifacts/dx_public_facade_closure_guard_current.json | workload_and_quality | yes | yes | 6913ae501abd3c91cb06da71e248f914b411ccfa2acb45bb25489c7939591d50 |
| core/local/artifacts/effective_loc_metric_contract_guard_current.json | workload_and_quality | yes | yes | 42f622042b6838bcfdf28e8c075099e27642af61124c4fe865b7a33a8e02713f |
| core/local/artifacts/effective_loc_metric_current.json | workload_and_quality | yes | yes | 271f200b839d25e589601358f708e4e5045c479bd2c7920ebdcd5348fb65363f |
| core/local/artifacts/eval_adversarial_guard_current.json | workload_and_quality | yes | yes | 4ee027faf1014e50fb9fdd513bc3dc1d36686f29f699cb510861cf0b784d57af |
| core/local/artifacts/eval_agent_chat_monitor_guard_current.json | workload_and_quality | yes | yes | ad8a260f0b905ad3af55f63a0e85d2a7ed58c9ac6ec78db3d0bde77dece11b96 |
| core/local/artifacts/eval_autopilot_guard_current.json | workload_and_quality | yes | yes | b39c4bd70312d949a0898fe1f1aa05d003ce7b5aba5913d0276880be56b8bf20 |
| core/local/artifacts/eval_feedback_router_current.json | workload_and_quality | yes | yes | 9971cd8f711d62b885b747ac5ce83aa8eb6415eeffb321243419505846f54ef9 |
| core/local/artifacts/eval_issue_filing_guard_current.json | workload_and_quality | yes | yes | 9b3be9fa7633d2e6199fc387512cf81c68256967efb2c3ca7e0a5a36922e06cc |
| core/local/artifacts/eval_issue_resolution_current.json | workload_and_quality | yes | yes | 1de99531b1b48c1379311568234d6650829e3d60a8e1f1bdb55fdc689cea2941 |
| core/local/artifacts/eval_judge_human_agreement_current.json | workload_and_quality | yes | yes | 46c361a627685682ae9f1f5726bd25aae7f3ebaf13501fed4ec1f3ee358cbfc2 |
| core/local/artifacts/eval_monitor_slo_current.json | workload_and_quality | yes | yes | c91ff975420a19dd5dfc97063e0802d2cfd497c57157ba3554ec8a0332e4260a |
| core/local/artifacts/eval_quality_gate_v1_current.json | workload_and_quality | yes | yes | 70ee37849f139dd83f57bd756c8705aa03f44949175f0bc9ebb37192ce659c87 |
| core/local/artifacts/eval_quality_metrics_current.json | workload_and_quality | yes | yes | 904f91c3729c688f67e4e2ac4e9e377eee035e8dec2103caadf43f95f05a6b1b |
| core/local/artifacts/eval_regression_guard_current.json | workload_and_quality | yes | yes | 5aa71d4df0215cc255f4b91f8ab8bc624a50d11b8cb08f59961e920c1e82a18d |
| core/local/artifacts/eval_reviewer_feedback_weekly_current.json | workload_and_quality | yes | yes | 2f24b808c466f59e5d7f47bd0cd7afefb2cb3f2e2685d754a870977b1b9cd72e |
| core/local/artifacts/eval_runtime_authority_guard_current.json | workload_and_quality | yes | yes | 564d5534c54a1e4e838a3fe85fa8d79bcd2c79c5daf74725003a19619a7daaf9 |
| core/local/artifacts/gateway_graduation_status_snapshot_current.json | adapter_and_orchestration | yes | yes | e99e2045f67ac21669788eafe1f540f948a8b7e1104735084f3d16ba0f2c6e4a |
| core/local/artifacts/gateway_manifest.json | adapter_and_orchestration | yes | yes | a9e16d2f89ca7dc72122034b0ae1407fd7546fb6f0e71ca5857261a137e4355c |
| core/local/artifacts/gateway_quarantine_recovery_proof_current.json | adapter_and_orchestration | yes | yes | 7a88ac7be7e8a10c678e9c900a8d3996940deca37e03cbe1817b58970c4ae8e0 |
| core/local/artifacts/gateway_runtime_chaos_gate_current.json | adapter_and_orchestration | yes | yes | be19fefe3569db4ff3d74ebe11cbf404e34a9dc0d8368319aacff5d5b34dcd0f |
| core/local/artifacts/gateway_status_manifest_current.json | adapter_and_orchestration | yes | yes | a9e16d2f89ca7dc72122034b0ae1407fd7546fb6f0e71ca5857261a137e4355c |
| core/local/artifacts/gateway_support_levels_current.json | adapter_and_orchestration | yes | yes | d5cb5b38f80219f157528fdf2c2bf25abd52647f5f0eab380f1bdb136ed4a6fc |
| core/local/artifacts/gateway_support_matrix_current.json | adapter_and_orchestration | yes | yes | 94f32178d99e1fd31cf62c9992dc226deccdb3dc526ccbb1c81a1cd6a1cadbc7 |
| core/local/artifacts/gem_feedback_closure_guard_current.json | workload_and_quality | yes | yes | d140644fbfe3488edb04682fa58fd2b245a34bbe9ad13d6fd4104c09e081d8ce |
| core/local/artifacts/gem_live_provider_smoke_current.json | workload_and_quality | yes | yes | 61efc795b41c0656a3891cfe3d96e74d47435993f1104c093562ed5f9d47bff1 |
| core/local/artifacts/gem_memory_durability_current.json | workload_and_quality | yes | yes | 4579c41956bfd9d091bc0f0a6b6cd5391affc1bab56cd280bd2d391ad6361147 |
| core/local/artifacts/gem_subagent_route_contract_current.json | workload_and_quality | yes | yes | 6a6cf2a8e2405ee6627bae67d61f92b63922d0d31678122cdb56f90fdf6ab25a |
| core/local/artifacts/guard_registry_ownership_current.json | release_governance | yes | yes | 72d5deb6dba3afa45d602c2fb88d5814b3aeb7b7b4070756b8cefdf005ca9bc1 |
| core/local/artifacts/guard_registry_ownership_guard_current.json | release_governance | yes | yes | bee7b3b6599f3303e918860e46ff96aed84f2cb12f35aca5d1d4c776a5d8649c |
| core/local/artifacts/incident_governance_closure_guard_current.json | release_governance | yes | yes | bc45411350253b6d0db7997a8b959cf42f04791e25b3b8f6fa45329bbccd4800 |
| core/local/artifacts/incident_operations_governance_gate_current.json | release_governance | yes | yes | a1c8c10b602291cfe5b72a09a2ce36cb9dc731b04d65871579484a95f3f7bbf8 |
| core/local/artifacts/installer_reliability_closure_guard_current.json | release_governance | yes | yes | 25ed34410edfddc737da34a6162cd78ac0848c9fe67fb5f4bac7fd8811806bae |
| core/local/artifacts/issue_candidate_backlog_current.json | release_governance | yes | yes | 72d49962ce6b823f0a41f2e25beacad3d333d39b2a4a78dbd3dc5a4f72051269 |
| core/local/artifacts/issue_candidate_contract_guard_current.json | release_governance | yes | yes | 7f123932cdd0eb49e2cd439e0984fc13fd1ec05ce6163c54fb43a6275d17c071 |
| core/local/artifacts/kernel_nexus_coupling_guard_current.json | release_governance | yes | yes | aa212aead4498dd0850c1488ef61be650cd281990c36c70111c6a8537ee72e24 |
| core/local/artifacts/kernel_sentinel_auto_run_current.json | release_governance | yes | yes | 43d4a04cae8ab73c4321c96c77ef9312fe4f42c7dd435123975e0999740a12bc |
| core/local/artifacts/knowledge_graph_query_acceleration_closure_guard_current.json | workload_and_quality | yes | yes | f6d539c8fe99a3fbe496d1a4f23cb3b5c959e4985a8527e31ef88c50c1c4ccc3 |
| core/local/artifacts/layer2_lane_parity_guard_current.json | adapter_and_orchestration | yes | yes | 1d8af264129649bf2a74d42a14643af3b4984c0a8c0c6032b18a56d5f67868f3 |
| core/local/artifacts/layer2_parity_matrix.json | adapter_and_orchestration | yes | yes | f313dc6c3acb94166a32fdc8f2074279d98d70b868f51d00354484f34e826b5c |
| core/local/artifacts/layer2_receipt_replay_current.json | adapter_and_orchestration | yes | yes | 78750f839a5c3590d787ee1df0b1f454735b12b63077d2d2e093a0d0516c2867 |
| core/local/artifacts/layer3_contract_guard_current.json | release_governance | yes | yes | fee3f15033f580f2300d77cf235b6db3773b80e6d5659677fb4ab08c25a2cad0 |
| core/local/artifacts/memory_continuity_closure_guard_current.json | workload_and_quality | yes | yes | 872fae805a68ade5ca9e9da9dfa25ebe50b88a8af45d286ff09dbd7e74ee52c5 |
| core/local/artifacts/memory_runtime_security_closure_guard_current.json | runtime_proof | yes | yes | e04f2d475a3453c13363872217b7ec2e76c5d6f28b1fad5e13f7e293f76b4bbd |
| core/local/artifacts/node_critical_path_inventory_current.json | release_governance | yes | yes | 1fc7d371221be866562e8b833b991327e585c4d92590d733299eff289cf52d5c |
| core/local/artifacts/orchestration_gateway_fallback_guard_current.json | adapter_and_orchestration | yes | yes | aeead617a3bd649a8cc08de796a5fbcc23cb130b814b0196eb7a71c59d7e6223 |
| core/local/artifacts/orchestration_planner_quality_guard_current.json | adapter_and_orchestration | yes | yes | c2315f72aa95d7a66502a189285056928d6fff6d73a099040a77b03044412b19 |
| core/local/artifacts/orchestration_quality_closure_guard_current.json | release_governance | yes | yes | 9fa218d4d08848dad1f57b80d5626654b0fcdfa87b3e0104898d887fdf449691 |
| core/local/artifacts/orchestration_runtime_quality_guard_current.json | adapter_and_orchestration | yes | yes | ef75fa114f9c16e186cd360b0c95f60c681b2ac4e4819a0f8897643a3758695f |
| core/local/artifacts/orchestration_workflow_contract_guard_current.json | adapter_and_orchestration | yes | yes | b0607a7142ba781123bbef465a5d9c76f22f0c7e4597e11baee9bc78df6c4c82 |
| core/local/artifacts/parity_end_to_end_replay_current.json | workload_and_quality | yes | yes | 01d08d614f31fc5430ba02d0c1d58ab40402fbfd1863b89ebbb52e204c4574fc |
| core/local/artifacts/parity_release_gate_current.json | workload_and_quality | yes | yes | e9746305fc4c5d3c508477f589830e25aa8011b3dadbb168a108a2716ce2c0f8 |
| core/local/artifacts/parity_trend_current.json | workload_and_quality | yes | yes | 67d68a159d2c8cfa588f9b58cea1f45cfaca1de6b3f348003266b5fb77b172ea |
| core/local/artifacts/production_readiness_closure_gate_current.json | release_governance | yes | yes | 89f52c6641493f3685ab07110f1ba68a105e9185481f8b41bb6b40984b806824 |
| core/local/artifacts/production_release_gate_closure_audit_current.json | release_governance | yes | yes | 0b8a3ab036a1115306f02f59400447844b3818c6e7fafa58df71dbed548b44d0 |
| core/local/artifacts/proof_pack_artifact_size_guard_current.json | release_governance | yes | yes | ee47c71c358e820e299a8087102a9dab196e2b552c4e999517ce698530e9709a |
| core/local/artifacts/queue_backpressure_policy_gate_current.json | runtime_proof | yes | yes | 51f1de2afddd29fbedb2eb5341db7410fbfb5c981ede002bb98bacf3bdb35229 |
| core/local/artifacts/real_work_workflow_proof_current.json | workload_and_quality | yes | yes | d2ed182f468c2f443574816bba27a1a66f3498bde121cdb7922dee27259b14bc |
| core/local/artifacts/real_work_workflow_proof_guard_current.json | workload_and_quality | yes | yes | 93411ffad70d9c8b91bbcf4c22956ecbb2e13cd153d8da9fd761d3a3ad906f6a |
| core/local/artifacts/release_contract_gate_current.json | release_governance | yes | yes | e11278e4d9526ee6cdeb8787f4debf1290dc5734942ff87dd50e9717283f059b |
| core/local/artifacts/release_policy_gate_current.json | release_governance | yes | yes | f4e65284e74d07e548502093204d5e784907ef02feb405c3fd74e4e7d4b6dcb9 |
| core/local/artifacts/release_proof_checksums_current.json | runtime_proof | yes | yes | a2d203cbbd9f8f6e4962bfb342495e2d20c168ebe8c34542d8844bfa4f3972a2 |
| core/local/artifacts/repo_entropy_scorecard_current.json | release_governance | yes | yes | b613a0cc38dd784017285896c47fe4bfb7b331036f68b25979224088637ef9ad |
| core/local/artifacts/repo_entropy_scorecard_guard_current.json | release_governance | yes | yes | 334c465e4b20fa40fa6c3cc97aceb57d301479d8d15a9cf099c19a1dc42c784f |
| core/local/artifacts/runtime_boundedness_72h_evidence_current.json | runtime_proof | yes | yes | 8c3c271c40bd679470b31f99d9df7a5674bf75a0fd13b96339c999dae73dbc53 |
| core/local/artifacts/runtime_boundedness_inspect_pure_current.json | runtime_proof | yes | yes | 140193707a785acb4d6df4d204c55154e01f53f21422c4da156573ee74b35596 |
| core/local/artifacts/runtime_boundedness_inspect_rich_current.json | runtime_proof | yes | yes | 0839163dc4a299eb4ae88946217aff07ca9cc0305b779d0ef359e055c6ebd5c3 |
| core/local/artifacts/runtime_boundedness_inspect_tiny-max_current.json | runtime_proof | yes | yes | 010818c7eeb6e0236ffd080681f902fd7a1e614e5f2ddf9c29f4c20bfa03b98a |
| core/local/artifacts/runtime_boundedness_profiles_current.json | runtime_proof | yes | yes | 32a30a7a9145ee4dc9c82538f9e09ea3c244653dcaa9fd1fa556cf5f98d49479 |
| core/local/artifacts/runtime_boundedness_release_gate_current.json | runtime_proof | yes | yes | a27057f809d7a3c51a51c2b8f6cde75f8f304498aab1eb28b1a6bc5046b5ef1d |
| core/local/artifacts/runtime_closure_board_guard_current.json | release_governance | yes | yes | 18c81637281547d011ded79bc4bf40e7040b0dba64054c7f124d347d05199c0e |
| core/local/artifacts/runtime_closure_feature_alignment_guard_current.json | release_governance | yes | yes | 30843480390dc0ce739110fd1a1ca5b8a88d8b1d5233872e0313c58641e2cc02 |
| core/local/artifacts/runtime_multi_day_soak_evidence_current.json | runtime_proof | yes | yes | 1aa08ff2cc8f0238ab75a17fa16e86626daabadc5fbbf85609b4cbf464af8a41 |
| core/local/artifacts/runtime_proof_empirical_minimum_contract_current.json | runtime_proof | yes | yes | 35bf7e04ea79ca18cba45db5b50382776c80c23a167592cb10a4d423903ebaf1 |
| core/local/artifacts/runtime_proof_empirical_profile_coverage_current.json | runtime_proof | yes | yes | 2d75a0b431ba79aa4949bf4b3fb1ea21c8021d32cd3830f7f7af6b3a360502d2 |
| core/local/artifacts/runtime_proof_empirical_profile_gate_current.json | runtime_proof | yes | yes | 0660ed47f68611728099472ae62447a210b05a98ed68172842c37776a6da67f2 |
| core/local/artifacts/runtime_proof_empirical_profile_gate_failures_current.json | runtime_proof | yes | yes | b14d79700d3d888fe8b81426f6b96f570782d8a6c6a0993a4d42082f47f48e61 |
| core/local/artifacts/runtime_proof_empirical_profile_readiness_current.json | runtime_proof | yes | yes | d0467d7710f3ab1b49be231f80fafeb8bb74916d786d02b56149cd4273427db2 |
| core/local/artifacts/runtime_proof_empirical_release_evidence_current.json | runtime_proof | yes | yes | 8d59e21e0ab7269755120754ca48ed613c01691ffbc8001e18395adbc2d52043 |
| core/local/artifacts/runtime_proof_empirical_source_matrix_current.json | runtime_proof | yes | yes | 5636c5f306eea2defd62823b115a496e7d9f610d7fc60cc960d57119c1f9a2eb |
| core/local/artifacts/runtime_proof_empirical_trends_current.json | runtime_proof | yes | yes | 7c164a732f7958a8aa59cd2ae4de56f613b0f9ca7dbc29cd387706c2398e9901 |
| core/local/artifacts/runtime_proof_harness_pure_current.json | runtime_proof | yes | yes | d16c7bb67393e0f7262a9a8fe9c216ab93ca2fe9a4c040c30eba0f6d1238d679 |
| core/local/artifacts/runtime_proof_harness_rich_current.json | runtime_proof | yes | yes | 631d0da5facf348b40f24cd180fa9f5862a774a8b29b7f11975b61cd88c5acfb |
| core/local/artifacts/runtime_proof_harness_tiny-max_current.json | runtime_proof | yes | yes | a4586ac8b52c319d640f0896f1dc775e6cdf8b111fc0e28cc2a5b6a362dd428a |
| core/local/artifacts/runtime_proof_reality_guard_current.json | runtime_proof | yes | yes | 24bc155b79074bf99d7f7e62392d22130bd6c71684f7a8e3a28e05710e2bfcfc |
| core/local/artifacts/runtime_proof_release_gate_pure_current.json | runtime_proof | yes | yes | 2dff617b4764491ec97d0a7e9cbea237191f3171f5ad6b059d6d77e91f0c5b60 |
| core/local/artifacts/runtime_proof_release_gate_rich_current.json | runtime_proof | yes | yes | aacdc682861b4cf0da945a1f34994365f9b85e00ae5778cf762711e89c17ee39 |
| core/local/artifacts/runtime_proof_release_gate_tiny-max_current.json | runtime_proof | yes | yes | d3fa250df7bbfb16ee93f9cd5b24c1a654bd5bcab8fb61129aaa97bfb6152df5 |
| core/local/artifacts/runtime_proof_release_metrics_pure_current.json | runtime_proof | yes | yes | 18e56815aeb0bdd4b58c36d7440356af1dd30735030369ca0735078396378d23 |
| core/local/artifacts/runtime_proof_release_metrics_rich_current.json | runtime_proof | yes | yes | e6fe803ea76b55d1b08b5c671730f7ed26b8b773a3e16fa4326787ee58cbc850 |
| core/local/artifacts/runtime_proof_release_metrics_tiny-max_current.json | runtime_proof | yes | yes | bdf76f96d7987dae349c1bf73cabf36fb5b1a88529a24d81b2aa276aeaa3a6cb |
| core/local/artifacts/runtime_proof_synthetic_canary_current.json | runtime_proof | yes | yes | 677e03070d51df60d7da011e630984e6e57da6c5f76ecd11cc486f7d6678c01b |
| core/local/artifacts/runtime_proof_verify_current.json | runtime_proof | yes | yes | 2527cb65d331fcd182d431736de620f8a0ae4bf84cc97974f5a99b45eb6c0203 |
| core/local/artifacts/runtime_soak_scenarios_current.json | runtime_proof | yes | yes | a4dbb0498573bfa82a93c1cc8554558c0bd42c98cc88bf2bf19e39de44214a5f |
| core/local/artifacts/runtime_trusted_core_report_current.json | release_governance | yes | yes | b68822679c9e704c68ad9833aad7a57481d6a5899dd2532653876e00812726d9 |
| core/local/artifacts/rust_core_file_size_gate_current.json | release_governance | yes | yes | 74bcc616341495ad89f14a4b63f971749d7a928ea189e4d0afb19a1b1cbe677b |
| core/local/artifacts/self_modification_guard_current.json | release_governance | yes | yes | 4cd4f793edc78a2d60225c5bae28b2fe9be8e2710ebbcab788923b83711ba5ad |
| core/local/artifacts/shell_truth_leak_guard_current.json | release_governance | yes | yes | 6f7de146d56cb09b41947920cfecdb978e43c44159dd4cfabe9e56639818ae35 |
| core/local/artifacts/srs_same_revision_guard_current.json | release_governance | yes | yes | c4f772b70cb9726ea400813fbe16df2a360c488f528a48ac4a2beae83eaf9c33 |
| core/local/artifacts/srs_todo_section_guard_current.json | release_governance | yes | yes | 85127b8ba265898d733c5a45a0b08c8d92f567621d61bd02c5539b2ce5ed6cf3 |
| core/local/artifacts/support_bundle_latest.json | release_governance | yes | yes | a379893fe96a0217583e7ba0d6cc662f72106a7d6b1fc5970869abf359483066 |
| core/local/artifacts/terminology_transition_inventory_current.json | release_governance | yes | yes | b2f29560448aa37da10fb63b66805e9fa548ba71529b68bf868208af62df9801 |
| core/local/artifacts/test_maturity_registry_current.json | release_governance | yes | yes | a09c57d43ec3b958ad80081eb6b159665c244c6f7b9825eae342872b0dea1808 |
| core/local/artifacts/test_maturity_retirement_backlog_current.json | release_governance | yes | yes | 45e7e66b956ee1a1393e6b3fad010d0f40442ebcae53761295b6f85d92848441 |
| core/local/artifacts/tool_route_decision_current.json | adapter_and_orchestration | yes | yes | f16e975fd65f9baf7e7443720b0900621ff70f051e332856d7aa8da60f5babe2 |
| core/local/artifacts/tooling_task_fabric_closure_guard_current.json | release_governance | yes | yes | 97f1e0fb983a939d91c627b5ace34102195238f37d515a77753700476f38bff8 |
| core/local/artifacts/transport_convergence_guard_current.json | release_governance | yes | yes | c93d7647454d739aadff546ad725202ecda5aad2578521c7dcfb4ee244880f2a |
| core/local/artifacts/transport_spawn_audit_current.json | release_governance | yes | yes | 565717974feaf6b6c39bf374a887080f61831302a33931556c55a1767be1feca |
| core/local/artifacts/trust_zone_guard_current.json | release_governance | yes | yes | 3e3f3dd498a36bb379c9c90264eccd306c30781df2113d59804add974e7a4597 |
| core/local/artifacts/typed_probe_contract_matrix_guard_current.json | adapter_and_orchestration | yes | yes | ffec91d1c6b508eb9c5271c9e196ef38ecbff7d5a827b379d6301559ee8646c5 |
| core/local/artifacts/web_conduit_openclaw_media_closure_guard_current.json | workload_and_quality | yes | yes | 3b9d2ccc0fcea40818dc0500db075c1129240f0e00deea30fe7a777d4837f27f |
| core/local/artifacts/web_retrieval_reliability_closure_guard_current.json | workload_and_quality | yes | yes | 06ad1272c925f4d28b48f015a058a9ea56b144cb4a754804082b965169f11790 |
| core/local/artifacts/web_tooling_reliability_current.json | workload_and_quality | yes | yes | cf860326a6ce52289307fd83f17d400e695d3ae1540e02a2afd83c319b37e1dc |
| core/local/artifacts/windows_install_reliability_current.json | release_governance | yes | yes | 20407a10e8b780b63b567d7430270ee4bc9821f1dee0990dc03ed72c5912e40c |
| core/local/artifacts/windows_installer_contract_guard_current.json | release_governance | yes | yes | fc06c2174cc356d4821cffe27743aba939f652365810a6e20084983843661e9f |
| core/local/artifacts/workflow_failure_recovery_current.json | workload_and_quality | yes | yes | bd2a0970dc69b3f86da46dda6d56db46527bfc2508135874ea63770c08737f19 |
| core/local/artifacts/workspace_tooling_context_soak_current.json | workload_and_quality | yes | yes | 00af52807ccba933828904e3f376619e25d9c7af7b752b0015fc13045ad85dfc |
| core/local/artifacts/workspace_tooling_context_soak_report_latest.json | workload_and_quality | yes | yes | 00af52807ccba933828904e3f376619e25d9c7af7b752b0015fc13045ad85dfc |
| core/local/artifacts/workspace_tooling_release_proof_current.json | workload_and_quality | yes | yes | 685214aaed7e65b1e5f61be35396653b49caa658b6ce3095dfda34f065272cc9 |
| local/state/kernel_sentinel/automation_candidates.jsonl | ungrouped | yes | yes | cd714130436ee0b0f417233ec097992a5fb084514b13a731264b1ea3b42b34a8 |
| local/state/kernel_sentinel/daily_report.md | ungrouped | yes | yes | 54524c58d90f559a7aa5a186199c0d4afdea016e2354d1480023d2b3ebabc883 |
| local/state/kernel_sentinel/feedback_inbox.jsonl | ungrouped | yes | yes | 3d5dfb6bf81f9c4bed607db20bd2a954dfaea09f8a39f57f3c1837885abefff1 |
| local/state/kernel_sentinel/issues.jsonl | ungrouped | yes | yes | 1f6ac827257ee6a527e8bd0bca7854dbb40b44f8ee73d035628f1a53043ef0b5 |
| local/state/kernel_sentinel/kernel_sentinel_report_current.json | ungrouped | yes | yes | 611342e4fd2aebb1ba964be963440df55557c0eb8a13e13d03063cd5ecedfdd8 |
| local/state/kernel_sentinel/kernel_sentinel_verdict.json | ungrouped | yes | yes | a0143217d4854e915d8a8d7d3a786750b4cd0e1c21abaf64a0bddadb391e2c61 |
| local/state/kernel_sentinel/rsi_readiness_summary_current.json | ungrouped | yes | yes | 12e51a581ff5c994ce7566a36fb15f47d1311ab08421d307a660993ccb5221e6 |
| local/state/kernel_sentinel/sentinel_trend_report_current.json | ungrouped | yes | yes | 24ee28af6ad2bbc63757b28f2923ea341cab04e8fdc4abe7b8039cad34acea21 |
| local/state/kernel_sentinel/suggestions.jsonl | ungrouped | yes | yes | 75ed28cefca4be40b05f7960f31de65a8b1ad4dd43d74f8c8837bb1651e7977b |
| local/state/kernel_sentinel/top_system_holes_current.json | ungrouped | yes | yes | 457bad97854d29b89e6bb1814b37d2e52d6c804406ea4da938f892433f99e3e4 |
| local/state/kernel_sentinel/trend_history.jsonl | ungrouped | yes | yes | d85b0aa8e9350b6c9713dbed9b42f1fd3d5ae7008321a04cb3fb289b13e04bd5 |
| validation/reports/client_archive/benchmark_matrix_run_latest.json | workload_and_quality | yes | yes | bb326350837a01b527fc7f9ca25736255cdb8405004c76a1f57858534c2a6d95 |
| local/workspace/reports/RUNTIME_PROOF_RELEASE_GATE_RICH_CURRENT.md | ungrouped | no | no | missing |
| local/workspace/reports/LAYER2_LANE_PARITY_GUARD_CURRENT.md | ungrouped | no | no | missing |
| local/workspace/reports/LAYER2_RECEIPT_REPLAY_CURRENT.md | ungrouped | no | no | missing |
| local/workspace/reports/RUNTIME_TRUSTED_CORE_REPORT_CURRENT.md | ungrouped | no | no | missing |
| local/workspace/reports/GATEWAY_RUNTIME_CHAOS_GATE_CURRENT.md | ungrouped | no | yes | 0b2efd5fc9191e88282c500f771478e73b3f31745e9d350f546d36fb6840cd4c |
| local/workspace/reports/GATEWAY_STATUS_MANIFEST_CURRENT.md | ungrouped | no | yes | f52c22c9b8bbe71913e5de0f66841c0074d76f79f5179db38ceddcec983b7c55 |
| local/workspace/reports/GATEWAY_GRADUATION_STATUS_SNAPSHOT_CURRENT.md | ungrouped | no | yes | 79c416337805a9fd1c3872793cecd90d3f4b55eefedd09e6845af8dc7eb6836c |
| local/workspace/reports/LAYER3_CONTRACT_GUARD_CURRENT.md | ungrouped | no | no | missing |
| local/workspace/reports/NODE_CRITICAL_PATH_INVENTORY_CURRENT.md | ungrouped | no | no | missing |
| local/workspace/reports/AGENT_SURFACE_STATUS_GUARD_CURRENT.md | ungrouped | no | yes | 41463613e6c990aad1f710f7020477346a60f0fd3f74f88a2ca9214c7598b6df |
| core/local/artifacts/agent_runtime_socket_live_gateway_guard_current.json | ungrouped | no | yes | 2c36259232ed727694723be809d2d9157a1d9a697a1a39b8bbabf98c211b8390 |
| core/local/artifacts/agent_runtime_socket_disposable_gateway_guard_current.json | ungrouped | no | yes | ace97c03f123f017d199ced38b47a7391620c6af09ff57b7fe0f98ace3f4c586 |
| local/workspace/reports/RUNTIME_BOUNDEDNESS_INSPECT_RICH_CURRENT.md | ungrouped | no | no | missing |
| local/workspace/reports/RUNTIME_BOUNDEDNESS_INSPECT_PURE_CURRENT.md | ungrouped | no | no | missing |
| local/workspace/reports/RUNTIME_BOUNDEDNESS_INSPECT_TINY-MAX_CURRENT.md | ungrouped | no | no | missing |
| local/workspace/reports/RUNTIME_PROOF_EMPIRICAL_RELEASE_EVIDENCE_CURRENT.md | ungrouped | no | no | missing |
| local/workspace/reports/RUNTIME_PROOF_EMPIRICAL_TRENDS_CURRENT.md | ungrouped | no | no | missing |
| local/workspace/reports/RUNTIME_PROOF_EMPIRICAL_PROFILE_COVERAGE_CURRENT.md | ungrouped | no | no | missing |
| local/workspace/reports/RUNTIME_PROOF_EMPIRICAL_SOURCE_MATRIX_CURRENT.md | ungrouped | no | no | missing |
| local/workspace/reports/RUNTIME_PROOF_EMPIRICAL_PROFILE_GATE_CURRENT.md | ungrouped | no | no | missing |
| local/workspace/reports/RUNTIME_PROOF_EMPIRICAL_PROFILE_GATE_FAILURES_CURRENT.md | ungrouped | no | no | missing |
| local/workspace/reports/RUNTIME_PROOF_EMPIRICAL_PROFILE_READINESS_CURRENT.md | ungrouped | no | no | missing |
| local/workspace/reports/RUNTIME_PROOF_EMPIRICAL_MINIMUM_CONTRACT_CURRENT.md | ungrouped | no | no | missing |
| local/workspace/reports/RUNTIME_SOAK_SCENARIOS_CURRENT.md | ungrouped | no | no | missing |
| local/workspace/reports/RUNTIME_PROOF_REALITY_GUARD_CURRENT.md | ungrouped | no | no | missing |
| local/workspace/reports/WEB_TOOLING_RELIABILITY_CURRENT.md | ungrouped | no | no | missing |
| local/workspace/reports/WORKFLOW_FAILURE_RECOVERY_CURRENT.md | ungrouped | no | no | missing |
| local/workspace/reports/WORKSPACE_TOOLING_CONTEXT_SOAK_CURRENT.md | ungrouped | no | no | missing |
| local/workspace/reports/WORKSPACE_TOOLING_RELEASE_PROOF_CURRENT.md | ungrouped | no | yes | 2a13c1c195f311e0322949bb9ccc3b1c296088fccd925237580e16db7bceaef9 |
| local/workspace/reports/AGENT_RUNTIME_TASK_HARNESS_REPORT_CURRENT.md | ungrouped | no | yes | d323c30eab0fc4c3db4c12f291d5fafd3baf5928f8b70bdd0cee331f3044f8e2 |
| core/local/state/ops/runtime_proof_empirical_history.jsonl | ungrouped | no | yes | 91747629dcbdea287477300b0f48585624c431adf1659fad43f6d95212de507d |
| local/workspace/reports/RELEASE_SCORECARD_CURRENT.md | ungrouped | no | no | missing |
| local/workspace/reports/PRODUCTION_RELEASE_GATE_CLOSURE_AUDIT_CURRENT.md | ungrouped | no | no | missing |
| local/workspace/reports/SHELL_TRUTH_LEAK_GUARD_CURRENT.md | ungrouped | no | no | missing |
| local/workspace/reports/TERMINOLOGY_TRANSITION_INVENTORY_CURRENT.md | ungrouped | no | no | missing |
| local/workspace/reports/SRS_SAME_REVISION_GUARD_CURRENT.md | ungrouped | no | no | missing |
| local/workspace/reports/RUNTIME_CLOSURE_BOARD_GUARD_CURRENT.md | ungrouped | no | no | missing |
| local/workspace/reports/RUNTIME_CLOSURE_FEATURE_ALIGNMENT_GUARD_CURRENT.md | ungrouped | no | no | missing |
| local/workspace/reports/CAPABILITY_PROOF_BURDEN_GUARD_CURRENT.md | ungrouped | no | yes | 37ab8f6f8f3fb3fede808ac76702e830bd39ecc2a467bee85bf477e5c45d665d |
| local/workspace/reports/WINDOWS_INSTALL_RELIABILITY_CURRENT.md | ungrouped | no | yes | a7e9e7e580f443f8348ebdd869fed27bdf45b4309c7d5addb2a15994296501af |
| local/workspace/reports/WINDOWS_INSTALLER_CONTRACT_GUARD_CURRENT.md | ungrouped | no | yes | 8947df0e0922290d996049f6bf07d03a13d566cd41ad5ab8f926f07ad979cb29 |
| local/workspace/reports/SRS_TODO_SECTION_GUARD_CURRENT.md | ungrouped | no | yes | a4c1f9a9ea2f1689d84f018f9e8a986f5388025cd750970ec24a87ce48f722d1 |
| local/workspace/reports/EVAL_QUALITY_METRICS_CURRENT.md | ungrouped | no | yes | 2f29ddbe9cb78c751fecb8ccf9e40a95ccfdbb63e2cda61252ed85c7761b0177 |
| local/workspace/reports/EVAL_MONITOR_SLO_CURRENT.md | ungrouped | no | yes | a5c382e21464de0a13e6d7e19ec2329927a5a949f02badef01842f1a0e0a9f26 |
| local/workspace/reports/EVAL_REVIEWER_FEEDBACK_WEEKLY_CURRENT.md | ungrouped | no | yes | a7a15f2b131fd848eea8bf4b5d5243b2f7095ae245584c900a0a80d6cdd5f373 |
| local/workspace/reports/EVAL_JUDGE_HUMAN_AGREEMENT_CURRENT.md | ungrouped | no | yes | f9f606121e081435ca1200c2ebb5da47a75c45b286108fa76fe3076afd60d67b |
| local/workspace/reports/EVAL_ADVERSARIAL_GUARD_CURRENT.md | ungrouped | no | yes | 8dd243f5c455f3824513b883856ce4760c2c65c06eb261e3ea5e8519b1596c70 |
| local/workspace/reports/EVAL_ISSUE_FILING_GUARD_CURRENT.md | ungrouped | no | yes | a555a6c52b1eb4054ed4dc6ff20637b14cf43172eecdfdb11c1d1b5cdbb90169 |
| local/workspace/reports/EVAL_ISSUE_RESOLUTION_CURRENT.md | ungrouped | no | yes | 34204c03b8d8339980a1deff74ab668a610771529a7a0db4e3ee85fd59af974d |
| local/workspace/reports/EVAL_QUALITY_GATE_V1_CURRENT.md | ungrouped | no | yes | e6c6852ed1a80efdbf9cc2f2b031df7d73f33f557ee492ed4e3c082dbf70e610 |
| local/workspace/reports/EVAL_RUNTIME_AUTHORITY_GUARD_CURRENT.md | ungrouped | no | yes | b5d5d0a62ed49fb7d45f9350e63b843dc621ff0776ee18c40591916c335dbf09 |
| local/workspace/reports/EVAL_REGRESSION_GUARD_CURRENT.md | ungrouped | no | yes | 133c8596af5de0a7bcb8ce3c008f625272bcc77aeb2939f9c2e115bec75c7f80 |
| local/workspace/reports/EVAL_FEEDBACK_ROUTER_CURRENT.md | ungrouped | no | yes | 39c09c0eb8fa577c06653937bb5853427776218f8f4a6e1c49581c357112f570 |
| local/workspace/reports/KERNEL_NEXUS_COUPLING_GUARD_CURRENT.md | ungrouped | no | yes | 204bea7d80728a1a7eb88d7161801b7f7db5a6dbc6695116bdec494f90ceddfb |
| local/workspace/reports/ARCHITECTURE_NEXUS_REQUIRED_ARTIFACT_GUARD_CURRENT.md | ungrouped | no | yes | be1feb5a7e5f21b1d65883ada9f2786859397c34665676d25d6eddbd264f47bf |
| local/workspace/reports/PARITY_END_TO_END_REPLAY_CURRENT.md | ungrouped | no | no | missing |
| local/workspace/reports/PARITY_TREND_CURRENT.md | ungrouped | no | no | missing |
| local/workspace/reports/PARITY_RELEASE_GATE_CURRENT.md | ungrouped | no | no | missing |
| local/state/ops/parity/parity_trend_history.jsonl | ungrouped | no | no | missing |
| local/workspace/reports/GEM_LIVE_PROVIDER_SMOKE_CURRENT.md | ungrouped | no | yes | 07d4a5fe2812758bc59dee32d32a79de8520c4e70da37d67de5e7247498782b8 |
| local/workspace/reports/GEM_MEMORY_DURABILITY_CURRENT.md | ungrouped | no | yes | 74147701c1ef80bf16dfa3a176d395f0527b1e3b9b8bf53c94593142372c8608 |
| local/workspace/reports/GEM_SUBAGENT_ROUTE_CONTRACT_CURRENT.md | ungrouped | no | yes | d975b9d0074d7d89d152b1acc244f07438d972213a682f92bb0127cbd368feaa |
| local/workspace/reports/GEM_FEEDBACK_CLOSURE_GUARD_CURRENT.md | ungrouped | no | yes | b48d2878acf7d255118e2d58bef7bb60839663fa910d302a9196a4e22f0313ec |
| local/workspace/reports/EVAL_AGENT_CHAT_MONITOR_GUARD_CURRENT.md | ungrouped | no | yes | 332e37149f8cfe3d517603647336bf93be7d2fcda597fd5eba86b68d77769682 |
| local/state/ops/gem_live_provider_smoke/latest.json | ungrouped | no | yes | 61efc795b41c0656a3891cfe3d96e74d47435993f1104c093562ed5f9d47bff1 |
| local/state/ops/gem_memory_durability/latest.json | ungrouped | no | yes | 4579c41956bfd9d091bc0f0a6b6cd5391affc1bab56cd280bd2d391ad6361147 |
| local/state/ops/gem_subagent_route_contract/latest.json | ungrouped | no | yes | 6a6cf2a8e2405ee6627bae67d61f92b63922d0d31678122cdb56f90fdf6ab25a |
| local/state/ops/gem_feedback_closure_guard/latest.json | ungrouped | no | yes | d140644fbfe3488edb04682fa58fd2b245a34bbe9ad13d6fd4104c09e081d8ce |
| local/state/ops/eval_agent_chat_monitor/latest.json | ungrouped | no | yes | ad8a260f0b905ad3af55f63a0e85d2a7ed58c9ac6ec78db3d0bde77dece11b96 |
| local/state/ops/eval_agent_chat_monitor/issue_drafts_latest.json | ungrouped | no | yes | 51784c7bd4bceedd02992ab8e6117cb224dc6b4f2f4320e3f853ad3df755c4b3 |
| client/runtime/local/state/ui/infring_dashboard/troubleshooting/eval_issue_resolution_panel.json | ungrouped | no | yes | 6fa46e6ca3b0d5abf788f19e6c048a01c3d58d807380065e574d9e8164ba0ba5 |
| local/state/ops/eval_quality_gate_v1/history.json | ungrouped | no | yes | 711a143877ec7a4ee7111aebf1c57f89c1bda6cc518a861cd332e648b960be06 |
| artifacts/eval_regression_guard_latest.json | ungrouped | no | yes | 5aa71d4df0215cc255f4b91f8ab8bc624a50d11b8cb08f59961e920c1e82a18d |
| artifacts/eval_feedback_router_latest.json | ungrouped | no | yes | 9971cd8f711d62b885b747ac5ce83aa8eb6415eeffb321243419505846f54ef9 |
| local/state/ops/eval_autopilot/latest.json | ungrouped | no | yes | b39c4bd70312d949a0898fe1f1aa05d003ce7b5aba5913d0276880be56b8bf20 |
| local/workspace/reports/EVAL_AUTOPILOT_GUARD_CURRENT.md | ungrouped | no | yes | b9a01dae9f6f29ff1e7c81420be97a7648bc027f9e96cc9314e1b4d1e63c58fc |
| local/workspace/reports/ISSUE_CANDIDATE_CONTRACT_GUARD_CURRENT.md | ungrouped | no | no | missing |
| local/workspace/reports/ISSUE_CANDIDATE_BACKLOG_CURRENT.md | ungrouped | no | yes | 16a56143b1b674a9a01a24f317ab1a5adf68c0fb25ea63f1c35ea87c87679a86 |
| local/workspace/reports/ORCHESTRATION_QUALITY_CLOSURE_GUARD_CURRENT.md | ungrouped | no | no | missing |
| local/workspace/reports/TOOLING_TASK_FABRIC_CLOSURE_GUARD_CURRENT.md | ungrouped | no | no | missing |
| local/workspace/reports/TOOL_ROUTE_MISDIRECTION_GUARD_CURRENT.md | ungrouped | no | no | missing |
| local/workspace/reports/CHAT_RENDERING_EXPERIENCE_GUARD_CURRENT.md | ungrouped | no | yes | b6e2cd87ac0fded6b9004b4be255dfbd92a7d30eef9064f48bc12ef1cbaf11c5 |

## Category summary
- workload_and_quality: present=43/43;required=43/43;required_missing=0;required_completeness=1.000;required_min=1.000
- release_governance: present=41/41;required=41/41;required_missing=0;required_completeness=1.000;required_min=1.000
- runtime_proof: present=32/32;required=32/32;required_missing=0;required_completeness=1.000;required_min=1.000
- adapter_and_orchestration: present=16/16;required=16/16;required_missing=0;required_completeness=1.000;required_min=1.000
- ungrouped: present=56/93;required=11/11;required_missing=0;required_completeness=1.000

## Operator summary
- pass: false
- primary_blocker: artifacts/web_tooling_context_soak_report_latest.json
- issue_candidate_ready: true
- next_actions: 28

## Issue candidate
- title: Release proof-pack is not release-ready
- severity: release_blocking
- fingerprint: release_proof_pack:2026-06-10:artifacts/web_tooling_context_soak_report_latest.json|artifacts/web_tooling_reliability_latest.json|client/runtime/local/state/release/scorecard/release_scorecard.json|core/local/artifacts/eval_quality_gate_v1_current.json|core/local/artifacts/kernel_sentinel_auto_run_current.json|core/local/artifacts/production_readiness_closure_gate_current.json|core/local/artifacts/release_contract_gate_current.json|core/local/artifacts/runtime_proof_verify_current.json|core/local/artifacts/runtime_trusted_core_report_current.json|core/local/artifacts/rust_core_file_size_gate_current.json|core/local/artifacts/srs_todo_section_guard_current.json|core/local/artifacts/support_bundle_latest.json|core/local/artifacts/transport_spawn_audit_current.json|local/state/kernel_sentinel/kernel_sentinel_report_current.json|local/state/kernel_sentinel/kernel_sentinel_verdict.json|core/local/artifacts/rust_core_file_size_gate_current.json|local/state/kernel_sentinel/kernel_sentinel_verdict.json|optional_artifacts:local/state/kernel_sentinel/kernel_sentinel_report_current.json|optional_artifacts:local/state/kernel_sentinel/kernel_sentinel_verdict.json|optional_artifacts:local/state/kernel_sentinel/feedback_inbox.jsonl|optional_artifacts:local/state/kernel_sentinel/trend_history.jsonl|optional_artifacts:local/state/kernel_sentinel/sentinel_trend_report_current.json|optional_artifacts:local/state/kernel_sentinel/rsi_readiness_summary_current.json|optional_artifacts:local/state/kernel_sentinel/top_system_holes_current.json|optional_artifacts:local/state/kernel_sentinel/issues.jsonl|optional_artifacts:local/state/kernel_sentinel/suggestions.jsonl|optional_artifacts:local/state/kernel_sentinel/automation_candidates.jsonl|optional_artifacts:local/state/kernel_sentinel/daily_report.md
- next_actions: 28

## Top blockers
- release_blocking: required_failed_artifacts artifacts/web_tooling_context_soak_report_latest.json -> repair failing required artifact artifacts/web_tooling_context_soak_report_latest.json
- release_blocking: required_failed_artifacts artifacts/web_tooling_reliability_latest.json -> repair failing required artifact artifacts/web_tooling_reliability_latest.json
- release_blocking: required_failed_artifacts client/runtime/local/state/release/scorecard/release_scorecard.json -> repair failing required artifact client/runtime/local/state/release/scorecard/release_scorecard.json
- release_blocking: required_failed_artifacts core/local/artifacts/eval_quality_gate_v1_current.json -> repair failing required artifact core/local/artifacts/eval_quality_gate_v1_current.json
- release_blocking: required_failed_artifacts core/local/artifacts/kernel_sentinel_auto_run_current.json -> repair failing required artifact core/local/artifacts/kernel_sentinel_auto_run_current.json
- release_blocking: required_failed_artifacts core/local/artifacts/production_readiness_closure_gate_current.json -> repair failing required artifact core/local/artifacts/production_readiness_closure_gate_current.json
- release_blocking: required_failed_artifacts core/local/artifacts/release_contract_gate_current.json -> repair failing required artifact core/local/artifacts/release_contract_gate_current.json
- release_blocking: required_failed_artifacts core/local/artifacts/runtime_proof_verify_current.json -> repair failing required artifact core/local/artifacts/runtime_proof_verify_current.json
- release_blocking: required_failed_artifacts core/local/artifacts/runtime_trusted_core_report_current.json -> repair failing required artifact core/local/artifacts/runtime_trusted_core_report_current.json
- release_blocking: required_failed_artifacts core/local/artifacts/rust_core_file_size_gate_current.json -> repair failing required artifact core/local/artifacts/rust_core_file_size_gate_current.json

## Manifest hygiene
- duplicate_warnings: 11
- optional_artifacts: local/state/kernel_sentinel/kernel_sentinel_report_current.json count=1; remove optional duplicate because this path is already required evidence
- optional_artifacts: local/state/kernel_sentinel/kernel_sentinel_verdict.json count=1; remove optional duplicate because this path is already required evidence
- optional_artifacts: local/state/kernel_sentinel/feedback_inbox.jsonl count=1; remove optional duplicate because this path is already required evidence
- optional_artifacts: local/state/kernel_sentinel/trend_history.jsonl count=1; remove optional duplicate because this path is already required evidence
- optional_artifacts: local/state/kernel_sentinel/sentinel_trend_report_current.json count=1; remove optional duplicate because this path is already required evidence
- optional_artifacts: local/state/kernel_sentinel/rsi_readiness_summary_current.json count=1; remove optional duplicate because this path is already required evidence
- optional_artifacts: local/state/kernel_sentinel/top_system_holes_current.json count=1; remove optional duplicate because this path is already required evidence
- optional_artifacts: local/state/kernel_sentinel/issues.jsonl count=1; remove optional duplicate because this path is already required evidence
- optional_artifacts: local/state/kernel_sentinel/suggestions.jsonl count=1; remove optional duplicate because this path is already required evidence
- optional_artifacts: local/state/kernel_sentinel/automation_candidates.jsonl count=1; remove optional duplicate because this path is already required evidence
- optional_artifacts: local/state/kernel_sentinel/daily_report.md count=1; remove optional duplicate because this path is already required evidence

