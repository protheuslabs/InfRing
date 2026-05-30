                "ok": false,
                "type": "constitution_guardian",
                "error": "unknown_command",
                "usage": [
                    "constitution-guardian init-genesis [--force=1|0]",
                    "constitution-guardian propose-change --candidate-file=<path> --proposer-id=<id> --reason=<text>",
                    "constitution-guardian approve-change --proposal-id=<id> --approver-id=<id> --approval-note=<text>",
                    "constitution-guardian veto-change --proposal-id=<id> --veto-by=<id> --note=<text>",
                    "constitution-guardian run-gauntlet --proposal-id=<id> [--critical-failures=<n>] [--evidence=<text>]",
                    "constitution-guardian activate-change --proposal-id=<id> --approver-id=<id> --approval-note=<text>",
                    "constitution-guardian enforce-inheritance --actor=<id> --target=<id>",
                    "constitution-guardian emergency-rollback --note=<text>",
                    "constitution-guardian status"
                ]
            }),
            2,
        ),
    }
}
