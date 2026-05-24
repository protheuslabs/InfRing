#!/usr/bin/env node
/* eslint-disable no-console */
import fs from 'node:fs';
import path from 'node:path';
import { execFileSync } from 'node:child_process';

const ROOT = process.cwd();
const DEFAULT_ALLOWLIST = 'validation/conformance/contracts/primitive_first_hardcoding_allowlist.json';
const DEFAULT_PRIMITIVE_REGISTRY = 'validation/conformance/contracts/primitive_capability_registry.json';
const DEFAULT_REGRESSION_GATES = 'orchestration/src/control_plane/workflows/lab/composites/coding/coding_workflow_regression_gates.json';
const DEFAULT_OUT_JSON = 'core/local/artifacts/primitive_first_doctrine_guard_current.json';
const DEFAULT_OUT_MD = 'local/workspace/reports/PRIMITIVE_FIRST_DOCTRINE_GUARD_CURRENT.md';

type AllowlistEntry = {
  id: string;
  owner: string;
  path: string;
  kind: string;
  match: string;
  allowed_boundary: string;
  reason: string;
  why_not_primitive: string;
  review_by: string;
};

type Allowlist = {
  schema_version: string;
  policy_path: string;
  entry_contract?: {
    required_fields?: string[];
    allowed_boundaries?: string[];
  };
  entries?: AllowlistEntry[];
};

type PrimitiveRegistryEntry = {
  id: string;
  name: string;
  kind: string;
  owner_domain: string;
  layer: string;
  path: string;
  contract_surface: string;
  extension_surface: string;
  promotion_status: string;
};

type PrimitiveRegistry = {
  schema_version: string;
  policy_path: string;
  entry_contract?: {
    required_fields?: string[];
    allowed_kinds?: string[];
    allowed_promotion_statuses?: string[];
  };
  primitives?: PrimitiveRegistryEntry[];
};

type RegressionGateContract = {
  version: string;
  lower_level_regression_policy?: {
    required_for_higher_level_changes?: boolean;
    failure_class?: string;
    rule?: string;
    stable_lower_level_floor_gate_ids?: string[];
    promotion_blockers?: string[];
    review_question?: string;
  };
  gates?: Array<{ gate_id?: string; status?: string }>;
};

type Args = {
  strict: boolean;
  allowlistPath: string;
  registryPath: string;
  regressionGatesPath: string;
  scanKind: string;
  outJson: string;
  outMarkdown: string;
  includeControlledViolation: boolean;
};

type Violation = {
  kind: string;
  path: string;
  line?: number;
  detail: string;
  excerpt?: string;
};

type Rule = {
  kind: string;
  detail: string;
  pattern: RegExp;
};

const PRODUCTION_ROOTS = [
  'core/',
  'orchestration/',
  'client/runtime/',
  'shell/',
  'apps/',
  'adapters/',
];

const SCANNED_EXTENSIONS = new Set([
  '.rs',
  '.ts',
  '.tsx',
  '.js',
  '.jsx',
  '.json',
  '.toml',
  '.yaml',
  '.yml',
]);

const REQUIRED_DOC_TOKENS: Record<string, string[]> = {
  'docs/workspace/primitive_first_system_doctrine.md': [
    'Status: hard repo-wide policy',
    'Production behavior must be primitive first.',
    'Hardcoding behavior for a specific case is forbidden',
    'Hardcoding is allowed inside explicit test and eval boundaries.',
    'Coding workflow anti-hardcoding law',
    'primitive_capability_registry.json',
  ],
  'docs/workspace/REAL_WORK_FIRST.md': [
    'Primitive-First System Doctrine',
    'primitive_first_system_doctrine.md',
    'production hardcoding of specific cases',
  ],
  'docs/workspace/codex_enforcer.md': [
    'primitive_first_system_doctrine.md',
    'Hardcoding behavior for specific cases is forbidden in production paths.',
    'special_case_promotion_policy.md',
  ],
  'docs/workspace/primitive_capability_registry_policy.md': [
    'Status: active repo-wide policy',
    'validation/conformance/contracts/primitive_capability_registry.json',
    'primitive_composition_boundary_violation',
  ],
  'docs/workspace/special_case_promotion_policy.md': [
    'Status: active repo-wide policy',
    'Special cases are not automatically bad. Hidden special cases in production are bad.',
    'Promote into the smallest correct target',
  ],
};

const RULES: Rule[] = [
  {
    kind: 'prompt_phrase_runtime_branch',
    detail: 'Production code must not branch directly on user prompt phrasing; route through a declared workflow/tool contract or primitive.',
    pattern:
      /\b(if|else if|match)\b[^\n]{0,220}\b(prompt|user_?prompt|initial_prompt|original_prompt|raw_prompt|prompt_lower)\b[^\n]{0,220}\b(contains|includes|starts_with|startsWith|ends_with|endsWith|==|===|matches)\b[^\n]{0,160}["'`]/i,
  },
  {
    kind: 'benchmark_or_eval_runtime_branch',
    detail: 'Production code must not branch on benchmark, eval, fixture, golden, harness, or numbered level identity.',
    pattern:
      /\b(if|else if|match|switch)\b[^\n]{0,220}\b(benchmark|eval[_-]?level|evalLevel|fixture|golden|harness)\b/i,
  },
  {
    kind: 'case_specific_workflow_literal',
    detail: 'Official workflow/tool contracts must not contain named eval/demo cases unless represented as a general lane or fixture-only contract.',
    pattern:
      /\b(hello world|task router|benchmark case|golden case|fixture shape|demo object|eval case|eval level|level[ _-]?[0-9]+ case|level[ _-]?[0-9]+ fixture|level[ _-]?[0-9]+ eval|level[0-9]+[_-])\b/i,
  },
  {
    kind: 'magic_verifier_runtime_branch',
    detail: 'Production code must not branch on magic verifier/output phrases; expose verifier requirements as eval assertions or contracts.',
    pattern:
      /\b(if|else if|match|switch)\b[^\n]{0,220}\b(verifier|rubric|grader|expected_output|expectedOutput)\b[^\n]{0,220}["'`]/i,
  },
  {
    kind: 'fixture_derived_coding_synthesis',
    detail: 'Production coding runtime must not synthesize domain implementation bodies from fixture, test, probe, or benchmark evidence; use declared primitives, model edit plans, and generic file tools.',
    pattern:
      /\b(probe_derived|fixture_derived|test_derived|benchmark_derived)[A-Za-z0-9_]*(patch|implementation|synthesis|edit)|\b[A-Za-z0-9_]*(infer|derive|synthesi[sz]e)[A-Za-z0-9_]*(expression|implementation|function_body|domain_code)\b/i,
  },
  {
    kind: 'production_domain_code_template',
    detail: 'Production runtime must not contain generated domain-code templates for eval-like implementation bodies; reusable code generation must be a registered primitive with cross-case tests or stay in eval fixtures.',
    pattern:
      /\bappend_functions\b|\breturn\s+\{expression\}|\bruntime_lane_try_[A-Za-z0-9_]*evidence_synthesis\b/i,
  },
];

const CODING_SYNTHESIS_RULE_KINDS = new Set([
  'fixture_derived_coding_synthesis',
  'production_domain_code_template',
]);

function normalizePath(value: string): string {
  return value.replace(/\\/g, '/').replace(/^\.\//, '');
}

function abs(relPath: string): string {
  return path.resolve(ROOT, relPath);
}

function readText(relPath: string): string {
  return fs.readFileSync(abs(relPath), 'utf8');
}

function readJson<T>(relPath: string): T {
  return JSON.parse(readText(relPath)) as T;
}

function readFlag(argv: string[], name: string): string | undefined {
  const prefix = `--${name}=`;
  const found = argv.find((arg) => arg.startsWith(prefix));
  return found ? found.slice(prefix.length) : undefined;
}

function parseBool(value: string | undefined, fallback: boolean): boolean {
  if (value === undefined) return fallback;
  return ['1', 'true', 'yes', 'on'].includes(value.toLowerCase());
}

function parseArgs(argv: string[]): Args {
  const out = readFlag(argv, 'out') || DEFAULT_OUT_JSON;
  return {
    strict: parseBool(readFlag(argv, 'strict'), true),
    allowlistPath: readFlag(argv, 'allowlist') || DEFAULT_ALLOWLIST,
    registryPath: readFlag(argv, 'registry') || DEFAULT_PRIMITIVE_REGISTRY,
    regressionGatesPath: readFlag(argv, 'regression-gates') || DEFAULT_REGRESSION_GATES,
    scanKind: readFlag(argv, 'scan-kind') || 'repo',
    outJson: readFlag(argv, 'out-json') || out,
    outMarkdown: readFlag(argv, 'out-markdown') || DEFAULT_OUT_MD,
    includeControlledViolation: parseBool(readFlag(argv, 'include-controlled-violation'), false),
  };
}

function trackedFiles(): string[] {
  return execFileSync('git', ['ls-files'], { cwd: ROOT, encoding: 'utf8' })
    .split(/\r?\n/)
    .map(normalizePath)
    .filter(Boolean);
}

function isProductionRoot(file: string): boolean {
  return PRODUCTION_ROOTS.some((root) => file.startsWith(root));
}

function isAllowedTestEvalBoundary(file: string): boolean {
  const base = path.basename(file);
  return (
    file.startsWith('tests/') ||
    file.startsWith('validation/evals/') ||
    file.startsWith('validation/benchmarks/') ||
    file.startsWith('validation/regression/') ||
    file.includes('/fixtures/') ||
    file.includes('/__fixtures__/') ||
    file.includes('/testdata/') ||
    file.includes('/goldens/') ||
    base.endsWith('_test.rs') ||
    base.endsWith('_tests.rs') ||
    base.endsWith('.test.ts') ||
    base.endsWith('.spec.ts') ||
    base.startsWith('eval_') ||
    file.includes('/eval_')
  );
}

function shouldScanFile(file: string): boolean {
  if (!isProductionRoot(file)) return false;
  if (isAllowedTestEvalBoundary(file)) return false;
  if (file.includes('/target/') || file.includes('/dist/') || file.includes('/node_modules/')) return false;
  const ext = path.extname(file);
  if (!SCANNED_EXTENSIONS.has(ext)) return false;
  return true;
}

function wildcardToRegExp(pattern: string): RegExp {
  const escaped = pattern
    .replace(/[.+^${}()|[\]\\]/g, '\\$&')
    .replace(/\*\*/g, '\u0000')
    .replace(/\*/g, '[^/]*')
    .replace(/\u0000/g, '.*');
  return new RegExp(`^${escaped}$`);
}

function allowlistMatches(entry: AllowlistEntry, violation: Violation): boolean {
  const pathMatches = wildcardToRegExp(normalizePath(entry.path)).test(violation.path);
  if (!pathMatches) return false;
  if (entry.kind !== violation.kind && entry.kind !== '*') return false;
  const haystack = `${violation.excerpt || ''}\n${violation.detail}`;
  return haystack.includes(entry.match);
}

function isAllowed(allowlist: Allowlist, violation: Violation): boolean {
  return (allowlist.entries || []).some((entry) => allowlistMatches(entry, violation));
}

function validateAllowlist(allowlist: Allowlist, violations: Violation[]): void {
  const required = allowlist.entry_contract?.required_fields || [];
  const allowedBoundaries = new Set(allowlist.entry_contract?.allowed_boundaries || []);
  const seen = new Set<string>();
  for (const entry of allowlist.entries || []) {
    for (const field of required) {
      const value = (entry as any)[field];
      if (typeof value !== 'string' || value.trim() === '') {
        violations.push({
          kind: 'primitive_first_allowlist_entry_invalid',
          path: DEFAULT_ALLOWLIST,
          detail: `Allowlist entry ${entry.id || '<missing id>'} is missing required field ${field}.`,
        });
      }
    }
    if (entry.id && seen.has(entry.id)) {
      violations.push({
        kind: 'primitive_first_allowlist_entry_duplicate',
        path: DEFAULT_ALLOWLIST,
        detail: `Duplicate allowlist id: ${entry.id}`,
      });
    }
    if (entry.id) seen.add(entry.id);
    if (entry.allowed_boundary && allowedBoundaries.size > 0 && !allowedBoundaries.has(entry.allowed_boundary)) {
      violations.push({
        kind: 'primitive_first_allowlist_boundary_invalid',
        path: DEFAULT_ALLOWLIST,
        detail: `Allowlist entry ${entry.id} uses unknown boundary ${entry.allowed_boundary}.`,
      });
    }
  }
}

function validatePrimitiveRegistry(registry: PrimitiveRegistry, violations: Violation[]): void {
  const required = registry.entry_contract?.required_fields || [];
  const allowedKinds = new Set(registry.entry_contract?.allowed_kinds || []);
  const allowedStatuses = new Set(registry.entry_contract?.allowed_promotion_statuses || []);
  const seen = new Set<string>();
  for (const entry of registry.primitives || []) {
    for (const field of required) {
      const value = (entry as any)[field];
      if (typeof value !== 'string' || value.trim() === '') {
        violations.push({
          kind: 'primitive_registry_entry_invalid',
          path: DEFAULT_PRIMITIVE_REGISTRY,
          detail: `Registry entry ${entry.id || '<missing id>'} is missing required field ${field}.`,
        });
      }
    }
    if (entry.id && seen.has(entry.id)) {
      violations.push({
        kind: 'primitive_registry_entry_duplicate',
        path: DEFAULT_PRIMITIVE_REGISTRY,
        detail: `Duplicate primitive registry id: ${entry.id}`,
      });
    }
    if (entry.id) seen.add(entry.id);
    if (entry.kind && allowedKinds.size > 0 && !allowedKinds.has(entry.kind)) {
      violations.push({
        kind: 'primitive_registry_kind_invalid',
        path: DEFAULT_PRIMITIVE_REGISTRY,
        detail: `Registry entry ${entry.id} uses unknown kind ${entry.kind}.`,
      });
    }
    if (entry.promotion_status && allowedStatuses.size > 0 && !allowedStatuses.has(entry.promotion_status)) {
      violations.push({
        kind: 'primitive_registry_status_invalid',
        path: DEFAULT_PRIMITIVE_REGISTRY,
        detail: `Registry entry ${entry.id} uses unknown promotion_status ${entry.promotion_status}.`,
      });
    }
    validateRegisteredCapability(entry, violations);
  }
}

function validateRegressionGateContract(contract: RegressionGateContract, relPath: string, violations: Violation[]): void {
  const policy = contract.lower_level_regression_policy;
  if (!policy || typeof policy !== 'object') {
    violations.push({
      kind: 'lower_level_regression_policy_missing',
      path: relPath,
      detail: 'Coding regression gates must declare lower_level_regression_policy.',
    });
    return;
  }
  if (policy.required_for_higher_level_changes !== true) {
    violations.push({
      kind: 'lower_level_regression_policy_not_required',
      path: relPath,
      detail: 'lower_level_regression_policy.required_for_higher_level_changes must be true.',
    });
  }
  if (policy.failure_class !== 'primitive_composition_boundary_violation') {
    violations.push({
      kind: 'lower_level_regression_failure_class_invalid',
      path: relPath,
      detail: 'lower_level_regression_policy.failure_class must be primitive_composition_boundary_violation.',
    });
  }
  if (!Array.isArray(policy.stable_lower_level_floor_gate_ids) || policy.stable_lower_level_floor_gate_ids.length === 0) {
    violations.push({
      kind: 'lower_level_floor_gates_missing',
      path: relPath,
      detail: 'lower_level_regression_policy must name at least one stable lower-level floor gate.',
    });
  }
  if (!Array.isArray(policy.promotion_blockers) || policy.promotion_blockers.length === 0) {
    violations.push({
      kind: 'lower_level_regression_blockers_missing',
      path: relPath,
      detail: 'lower_level_regression_policy must declare promotion blockers.',
    });
  }
  const gateIds = new Set((contract.gates || []).map((gate) => String(gate.gate_id || '')).filter(Boolean));
  for (const gateId of policy.stable_lower_level_floor_gate_ids || []) {
    if (!gateIds.has(gateId)) {
      violations.push({
        kind: 'lower_level_floor_gate_unknown',
        path: relPath,
        detail: `lower_level_regression_policy references unknown gate_id ${gateId}.`,
      });
    }
  }
}

function validateRegisteredCapability(entry: PrimitiveRegistryEntry, violations: Violation[]): void {
  const entryPath = normalizePath(entry.path || '');
  if (!entryPath || !fs.existsSync(abs(entryPath))) {
    violations.push({
      kind: 'primitive_registry_path_missing',
      path: DEFAULT_PRIMITIVE_REGISTRY,
      detail: `Registry entry ${entry.id} points to missing path ${entry.path}.`,
    });
    return;
  }
  if (!(entryPath.endsWith('.workflow.json') || entryPath.endsWith('.tool.json'))) return;
  let cd: any;
  try {
    cd = readJson<any>(entryPath);
  } catch (error) {
    violations.push({
      kind: 'primitive_registry_cd_json_invalid',
      path: entryPath,
      detail: `Registered CD is not valid JSON: ${error instanceof Error ? error.message : String(error)}`,
    });
    return;
  }
  const primitiveFirst = cd.primitive_first_contract;
  if (!primitiveFirst || typeof primitiveFirst !== 'object') {
    violations.push({
      kind: 'registered_cd_missing_primitive_first_contract',
      path: entryPath,
      detail: `Registered capability ${entry.id} must declare primitive_first_contract.`,
    });
    return;
  }
  if (primitiveFirst.case_specific_hardcoding_allowed !== false) {
    violations.push({
      kind: 'registered_cd_allows_case_hardcoding',
      path: entryPath,
      detail: `Registered capability ${entry.id} must set primitive_first_contract.case_specific_hardcoding_allowed to false.`,
    });
  }
  if (primitiveFirst.registered_capability_id !== entry.id) {
    violations.push({
      kind: 'registered_cd_capability_id_mismatch',
      path: entryPath,
      detail: `primitive_first_contract.registered_capability_id must equal ${entry.id}.`,
    });
  }
  if (typeof primitiveFirst.specificity_owner !== 'string' || primitiveFirst.specificity_owner.trim() === '') {
    violations.push({
      kind: 'registered_cd_specificity_owner_missing',
      path: entryPath,
      detail: `Registered capability ${entry.id} must declare specificity_owner.`,
    });
  }
  if (typeof primitiveFirst.extension_surface !== 'string' || primitiveFirst.extension_surface.trim() === '') {
    violations.push({
      kind: 'registered_cd_extension_surface_missing',
      path: entryPath,
      detail: `Registered capability ${entry.id} must declare extension_surface.`,
    });
  }
  if (entryPath.endsWith('.workflow.json')) {
    const level = cd.workflow_composition?.primitive_level;
    if (entry.kind === 'primitive' && level !== 0) {
      violations.push({
        kind: 'registered_primitive_level_invalid',
        path: entryPath,
        detail: `Primitive registry entry ${entry.id} must have workflow_composition.primitive_level 0.`,
      });
    }
    if (entry.kind === 'composite' && !(typeof level === 'number' && level > 0)) {
      violations.push({
        kind: 'registered_composite_level_invalid',
        path: entryPath,
        detail: `Composite registry entry ${entry.id} must have workflow_composition.primitive_level greater than 0.`,
      });
    }
  }
}

function validateDocs(violations: Violation[]): void {
  for (const [file, tokens] of Object.entries(REQUIRED_DOC_TOKENS)) {
    if (!fs.existsSync(abs(file))) {
      violations.push({ kind: 'primitive_first_doc_missing', path: file, detail: 'Required doctrine document is missing.' });
      continue;
    }
    const text = readText(file);
    for (const token of tokens) {
      if (!text.includes(token)) {
        violations.push({
          kind: 'primitive_first_doc_token_missing',
          path: file,
          detail: `Missing required doctrine token: ${token}`,
        });
      }
    }
  }
}

function scanProductionFiles(
  allowlist: Allowlist,
  violations: Violation[],
  ruleFilter?: Set<string>,
): number {
  let scanned = 0;
  for (const file of trackedFiles()) {
    if (!shouldScanFile(file)) continue;
    scanned += 1;
    const fullPath = abs(file);
    const stat = fs.statSync(fullPath);
    if (stat.size > 1_000_000) continue;
    const lines = readText(file).split(/\r?\n/);
    const workflowOrToolContract = file.endsWith('.workflow.json') || file.endsWith('.tool.json');
    for (let index = 0; index < lines.length; index += 1) {
      const line = lines[index];
      for (const rule of RULES) {
        if (ruleFilter && !ruleFilter.has(rule.kind)) continue;
        if (rule.kind === 'case_specific_workflow_literal' && !workflowOrToolContract) continue;
        if (!rule.pattern.test(line)) continue;
        const violation: Violation = {
          kind: rule.kind,
          path: file,
          line: index + 1,
          detail: rule.detail,
          excerpt: line.trim().slice(0, 260),
        };
        if (!isAllowed(allowlist, violation)) violations.push(violation);
      }
    }
  }
  return scanned;
}

function markdown(payload: any): string {
  const lines: string[] = [];
  lines.push('# Primitive-First Doctrine Guard');
  lines.push('');
  lines.push(`- Generated at: ${payload.generated_at}`);
  lines.push(`- Pass: ${payload.ok}`);
  lines.push(`- Scan kind: ${payload.scan_kind}`);
  lines.push(`- Allowlist: ${payload.allowlist_path}`);
  lines.push(`- Files scanned: ${payload.summary.files_scanned}`);
  lines.push(`- Violations: ${payload.summary.violations}`);
  lines.push('');
  lines.push('## Violations');
  if (!payload.violations.length) {
    lines.push('- none');
  } else {
    for (const violation of payload.violations) {
      const line = violation.line ? `:${violation.line}` : '';
      lines.push(`- ${violation.kind}: ${violation.path}${line} - ${violation.detail}`);
    }
  }
  lines.push('');
  lines.push('## Doctrine');
  lines.push('- Production behavior must be primitive first.');
  lines.push('- Specific-case hardcoding is allowed only inside explicit test/eval boundaries.');
  lines.push('- Coding runtimes must not synthesize domain implementation bodies from fixture, test, probe, or benchmark evidence.');
  lines.push('- Legitimate production specificity must be represented as primitives, contracts, policies, schemas, adapters, profiles, config, or composition.');
  return `${lines.join('\n')}\n`;
}

function writeArtifact(relPath: string, text: string): void {
  fs.mkdirSync(path.dirname(abs(relPath)), { recursive: true });
  fs.writeFileSync(abs(relPath), text);
}

async function run(argv = process.argv.slice(2)) {
  const args = parseArgs(argv);
  const allowlist = readJson<Allowlist>(args.allowlistPath);
  const primitiveRegistry = readJson<PrimitiveRegistry>(args.registryPath);
  const regressionGates = readJson<RegressionGateContract>(args.regressionGatesPath);
  const violations: Violation[] = [];
  validateDocs(violations);
  if (args.scanKind === 'repo') {
    validateAllowlist(allowlist, violations);
    validatePrimitiveRegistry(primitiveRegistry, violations);
    validateRegressionGateContract(regressionGates, args.regressionGatesPath, violations);
  } else if (args.scanKind === 'coding-synthesis') {
    validateAllowlist(allowlist, violations);
  } else {
    violations.push({
      kind: 'primitive_first_scan_kind_invalid',
      path: 'tests/tooling/scripts/ci/primitive_first_doctrine_guard.ts',
      detail: `Unknown scan kind: ${args.scanKind}`,
    });
  }
  const ruleFilter = args.scanKind === 'coding-synthesis' ? CODING_SYNTHESIS_RULE_KINDS : undefined;
  const filesScanned = scanProductionFiles(allowlist, violations, ruleFilter);
  if (args.includeControlledViolation) {
    violations.push({
      kind: 'controlled_primitive_first_violation',
      path: 'tests/tooling/scripts/ci/primitive_first_doctrine_guard.ts',
      detail: 'Controlled violation requested; guard must fail closed in strict mode.',
    });
  }
  const payload = {
    generated_at: new Date().toISOString(),
    ok: violations.length === 0,
    scan_kind: args.scanKind,
    allowlist_path: args.allowlistPath,
    primitive_registry_path: args.registryPath,
    regression_gates_path: args.regressionGatesPath,
    doctrine_path: allowlist.policy_path,
    summary: {
      files_scanned: filesScanned,
      violations: violations.length,
      allowlist_entries: (allowlist.entries || []).length,
      primitive_registry_entries: (primitiveRegistry.primitives || []).length,
    },
    violations,
  };
  writeArtifact(args.outJson, `${JSON.stringify(payload, null, 2)}\n`);
  writeArtifact(args.outMarkdown, markdown(payload));
  console.log(JSON.stringify(payload, null, 2));
  if (args.strict && violations.length > 0) process.exit(1);
}

run().catch((error) => {
  console.error(error instanceof Error ? error.stack || error.message : String(error));
  process.exit(1);
});
