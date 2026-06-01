#!/usr/bin/env sh
# FILE_SIZE_EXCEPTION: reason=Single-file curl installer distribution requires contiguous standalone script; owner=jay; expires=2026-04-12
set -eu

DEFAULT_REPO_OWNER="protheuslabs"
REPO_OWNER="${INFRING_REPO_OWNER:-$DEFAULT_REPO_OWNER}"
REPO_NAME="${INFRING_REPO_NAME:-InfRing}"
DEFAULT_API="https://api.github.com/repos/${REPO_OWNER}/${REPO_NAME}/releases/latest"
DEFAULT_LATEST_URL="https://github.com/${REPO_OWNER}/${REPO_NAME}/releases/latest"
DEFAULT_BASE="https://github.com/${REPO_OWNER}/${REPO_NAME}/releases/download"
DEFAULT_SOURCE_ARCHIVE_BASE="https://github.com/${REPO_OWNER}/${REPO_NAME}/archive/refs/tags"
DEFAULT_RUSTUP_INIT_URL="https://sh.rustup.rs"
DEFAULT_BOOTSTRAP_BASE_URL="https://raw.githubusercontent.com/${REPO_OWNER}/${REPO_NAME}/main/dist/install-bootstrap"

INFRING_HOME="${INFRING_HOME:-$HOME/.infring}"
DEFAULT_INSTALL_DIR="${INFRING_HOME}/bin"
DEFAULT_WORKSPACE_DIR="${INFRING_HOME}"
INSTALL_DIR="${INFRING_INSTALL_DIR:-$DEFAULT_INSTALL_DIR}"
WORKSPACE_DIR="${INFRING_WORKSPACE_DIR:-$DEFAULT_WORKSPACE_DIR}"
NODE_RUNTIME_VERSION="${INFRING_NODE_RUNTIME_VERSION:-22.15.0}"
INSTALL_DIR_EXPLICIT=0
if [ -n "${INFRING_INSTALL_DIR:-}" ]; then
  INSTALL_DIR_EXPLICIT=1
fi
INSTALL_TMP_DIR="${INFRING_TMP_DIR:-${TMPDIR:-}}"
REQUESTED_VERSION="${INFRING_VERSION:-latest}"
API_URL="${INFRING_RELEASE_API_URL:-$DEFAULT_API}"
LATEST_URL="${INFRING_RELEASE_LATEST_URL:-$DEFAULT_LATEST_URL}"
BASE_URL="${INFRING_RELEASE_BASE_URL:-$DEFAULT_BASE}"
SOURCE_ARCHIVE_BASE="${INFRING_SOURCE_ARCHIVE_BASE_URL:-$DEFAULT_SOURCE_ARCHIVE_BASE}"
RUSTUP_INIT_URL="${INFRING_RUSTUP_INIT_URL:-$DEFAULT_RUSTUP_INIT_URL}"
BOOTSTRAP_BASE_URL="${INFRING_BOOTSTRAP_BASE_URL:-$DEFAULT_BOOTSTRAP_BASE_URL}"
INSTALL_FULL="${INFRING_INSTALL_FULL:-0}"
INSTALL_PURE="${INFRING_INSTALL_PURE:-0}"
INSTALL_TINY_MAX="${INFRING_INSTALL_TINY_MAX:-0}"
INSTALL_REPAIR="${INFRING_INSTALL_REPAIR:-0}"
INSTALL_DEBUG="${INFRING_INSTALL_DEBUG:-0}"
INSTALL_NODE="${INFRING_INSTALL_NODE:-0}"
INSTALL_NODE_AUTO="${INFRING_INSTALL_NODE_AUTO:-1}"
INSTALL_OLLAMA="${INFRING_INSTALL_OLLAMA:-0}"
INSTALL_OLLAMA_AUTO="${INFRING_INSTALL_OLLAMA_AUTO:-1}"
INSTALL_OLLAMA_PULL="${INFRING_INSTALL_OLLAMA_PULL:-1}"
INSTALL_REQUIRE_MODEL_READY="${INFRING_INSTALL_REQUIRE_MODEL_READY:-0}"
INSTALL_STRICT_SMOKE="${INFRING_INSTALL_STRICT_SMOKE:-0}"
INSTALL_TOOLCHAIN_POLICY_RAW="${INFRING_INSTALL_TOOLCHAIN_POLICY:-auto}"
OLLAMA_STARTER_MODEL="${INFRING_OLLAMA_STARTER_MODEL:-qwen2.5:3b-instruct}"
OLLAMA_PULL_TIMEOUT="${INFRING_OLLAMA_PULL_TIMEOUT:-900}"
OLLAMA_INSTALL_CONFIRMED=0
OLLAMA_LAST_MODEL_COUNT=0
SOURCE_FALLBACK_DIR=""
SOURCE_FALLBACK_TMP=""
PATH_SHIM_DIR=""
PATH_PERSISTED_FILE=""
PATH_PERSISTED_KIND=""
PATH_PERSISTED_MIRRORS=""
PATH_ACTIVATE_FILE=""
INSTALL_SUDO_SHIMS="${INFRING_INSTALL_SUDO_SHIMS:-auto}"
RUNTIME_MANIFEST_REL="client/runtime/config/install_runtime_manifest_v1.txt"
RUNTIME_NODE_MODULE_MANIFEST_REL="${INFRING_RUNTIME_NODE_MODULE_MANIFEST_REL:-client/runtime/config/install_runtime_node_modules_v1.txt}"
RUNTIME_NODE_REQUIRED_MODULES="${INFRING_RUNTIME_NODE_REQUIRED_MODULES:-typescript ws}"
RUNTIME_TIER1_REQUIRED_ENTRYPOINTS="${INFRING_RUNTIME_TIER1_REQUIRED_ENTRYPOINTS:-client/runtime/systems/ops/infringd.ts client/runtime/systems/ops/infring_status_dashboard.ts client/runtime/systems/ops/infring_unknown_guard.ts}"
INSTALL_VERIFY_ASSETS="${INFRING_INSTALL_VERIFY_ASSETS:-1}"
INSTALL_ALLOW_UNVERIFIED_ASSETS="${INFRING_INSTALL_ALLOW_UNVERIFIED_ASSETS:-${INFRING_INSTALL_ALLOW_UNVERIFIED_ASSETS:-0}}"
INSTALL_STRICT_PRERELEASE_CHECKSUM="${INFRING_INSTALL_STRICT_PRERELEASE_CHECKSUM:-0}"
INSTALL_ASSET_CACHE="${INFRING_INSTALL_ASSET_CACHE:-1}"
INSTALL_OFFLINE="${INFRING_INSTALL_OFFLINE:-0}"
INSTALL_SUMMARY_FILE="${INFRING_INSTALL_SUMMARY_FILE:-$INFRING_HOME/logs/last_install_summary.txt}"
INSTALL_ASSET_LOCKFILE="${INFRING_INSTALL_ASSET_LOCKFILE:-$INFRING_HOME/state/install_asset_lock_v1.tsv}"
CHECKSUM_MANIFEST_PATH=""
CHECKSUM_MANIFEST_VERSION=""
CHECKSUM_MANIFEST_TMP_DIR=""
CHECKSUM_MANIFEST_MISSING_WARNED_VERSION=""
INSTALL_SUMMARY_STATUS="failed"
INSTALL_SUMMARY_COMPLETED_AT=""
INSTALL_SUMMARY_FAILED_AT=""
INSTALL_SUMMARY_EXIT_CODE=""
INSTALL_SUMMARY_FAILURE_REASON=""
INSTALL_SUMMARY_LAST_NOTE=""
INSTALL_DASHBOARD_SMOKE_PASSED=0
INSTALL_JSON_OUTPUT="${INFRING_INSTALL_JSON:-0}"
INSTALL_SUMMARY_JSON_FILE="${INFRING_INSTALL_SUMMARY_JSON_FILE:-$INFRING_HOME/logs/last_install_summary.json}"
INSTALL_SMOKE_SUMMARY_JSON_FILE="${INFRING_INSTALL_SMOKE_SUMMARY_JSON_FILE:-$INFRING_HOME/logs/last_install_smoke_summary.json}"
INSTALL_RUNTIME_CONTRACT_MODE="unknown"
INSTALL_RUNTIME_CONTRACT_OK=0
INSTALL_CLIENT_RUNTIME_MODE="not_installed"
WORKSPACE_REFRESH_REQUIRED=0
WORKSPACE_REFRESH_APPLIED=0
WORKSPACE_REFRESH_REASON=""
WORKSPACE_REFRESH_TAG_STATE_MISSING=0
WORKSPACE_RELEASE_TAG_PREVIOUS=""
WORKSPACE_RELEASE_TAG_CURRENT=""
WORKSPACE_RELEASE_TAG_WRITTEN=0
WORKSPACE_RELEASE_TAG_WRITE_VERIFIED=0
INSTALL_SCRIPT_DIR="$(CDPATH= cd -- "$(dirname -- "$0")" 2>/dev/null && pwd -P || pwd)"
INSTALL_MODULE_DIR="${INFRING_INSTALL_MODULE_DIR:-}"
if [ -z "$INSTALL_MODULE_DIR" ]; then
  if [ -r "$INSTALL_SCRIPT_DIR/install/modules/bootstrap_common.sh" ]; then
    INSTALL_MODULE_DIR="$INSTALL_SCRIPT_DIR/install/modules"
  else
    INSTALL_MODULE_DIR="$INSTALL_SCRIPT_DIR/modules"
  fi
fi
if [ "${INFRING_INSTALL_USE_MODULES:-1}" != "0" ] && [ -r "$INSTALL_MODULE_DIR/bootstrap_common.sh" ]; then
  # shellcheck disable=SC1090
  . "$INSTALL_MODULE_DIR/bootstrap_common.sh"
fi

need_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "[infring install] missing required command: $1" >&2
    exit 1
  fi
}

finalize_installed_binary() {
  binary_path="$1"
  chmod 755 "$binary_path"
  if [ "$(uname -s)" = "Darwin" ] && command -v codesign >/dev/null 2>&1; then
    if ! codesign --force --sign - "$binary_path" >/dev/null 2>&1; then
      echo "[infring install] warning: failed to ad-hoc sign $(basename "$binary_path"); launchd may reject it" >&2
    fi
  fi
}

is_truthy() {
  case "$(printf '%s' "${1:-}" | tr '[:upper:]' '[:lower:]')" in
    1|true|yes|on) return 0 ;;
    *) return 1 ;;
  esac
}

normalize_install_toolchain_policy() {
  case "$(printf '%s' "${1:-auto}" | tr '[:upper:]' '[:lower:]')" in
    fail|fail_closed|strict) printf '%s\n' "fail_closed" ;;
    auto|*) printf '%s\n' "auto" ;;
  esac
}

INSTALL_TOOLCHAIN_POLICY="$(normalize_install_toolchain_policy "$INSTALL_TOOLCHAIN_POLICY_RAW")"

is_prerelease_version_tag() {
  version_tag="$1"
  case "$version_tag" in
    *-alpha*|*-beta*|*-rc*|*-preview*|*-pre*) return 0 ;;
    *) return 1 ;;
  esac
}

warn_checksum_manifest_missing_once() {
  version_tag="$1"
  reason="$2"
  if [ "$CHECKSUM_MANIFEST_MISSING_WARNED_VERSION" = "$version_tag" ]; then
    return 0
  fi
  CHECKSUM_MANIFEST_MISSING_WARNED_VERSION="$version_tag"
  case "$reason" in
    override)
      echo "[infring install] warning: checksum manifest missing for $version_tag; continuing due to override."
      ;;
    prerelease)
      echo "[infring install] warning: checksum manifest missing for prerelease $version_tag; continuing with unverified assets."
      echo "[infring install] note: publish SHA256SUMS for $version_tag to re-enable strict verification."
      echo "[infring install] note: set INFRING_INSTALL_STRICT_PRERELEASE_CHECKSUM=1 to fail closed for prereleases."
      ;;
    *)
      echo "[infring install] warning: checksum manifest missing for $version_tag; continuing with unverified assets."
      ;;
  esac
}

node_version_major() {
  node_bin_path="$(resolve_node_binary_path 2>/dev/null || true)"
  [ -n "$node_bin_path" ] || return 1
  version="$("$node_bin_path" --version 2>/dev/null || true)"
  version="${version#v}"
  major="$(printf '%s' "$version" | cut -d. -f1)"
  case "$major" in
    ''|*[!0-9]*) return 1 ;;
  esac
  printf '%s\n' "$major"
  return 0
}

node_runtime_meets_minimum() {
  major="$(node_version_major 2>/dev/null || true)"
  [ -n "$major" ] || return 1
  [ "$major" -ge 22 ]
}

ensure_cargo_command_ready() {
  if command -v cargo >/dev/null 2>&1; then
    if cargo --version >/dev/null 2>&1; then
      return 0
    fi
    if [ -x "$HOME/.cargo/bin/rustup" ] && [ -x "$HOME/.cargo/bin/cargo" ]; then
      export PATH="$HOME/.cargo/bin:$PATH"
      if cargo --version >/dev/null 2>&1; then
        return 0
      fi
    fi
  fi
  return 1
}

repair_rustup_default_toolchain() {
  if command -v rustup >/dev/null 2>&1; then
    echo "[infring install] rustup present but default cargo toolchain is missing; attempting recovery."
    if rustup default stable >/dev/null 2>&1; then
      return 0
    fi
    if rustup toolchain install stable >/dev/null 2>&1 && rustup default stable >/dev/null 2>&1; then
      return 0
    fi
  fi
  return 1
}

print_rust_toolchain_recovery_hint() {
  echo "[infring install] unable to provision a runnable cargo toolchain for source fallback." >&2
  if command -v rustup >/dev/null 2>&1; then
    echo "[infring install] fix: rustup default stable" >&2
    echo "[infring install] fallback: rustup toolchain install stable && rustup default stable" >&2
  else
    echo "[infring install] fix: curl --proto '=https' --tlsv1.2 -sSf $RUSTUP_INIT_URL | sh -s -- -y --profile minimal --default-toolchain stable" >&2
  fi
}

resolve_node_binary_path() {
  preferred="${INFRING_NODE_BINARY:-}"
  if [ -n "$preferred" ]; then
    if [ -x "$preferred" ]; then
      printf '%s\n' "$preferred"
      return 0
    fi
    if command -v "$preferred" >/dev/null 2>&1; then
      command -v "$preferred"
      return 0
    fi
  fi

  if [ -x "$INFRING_HOME/node-runtime/bin/node" ]; then
    printf '%s\n' "$INFRING_HOME/node-runtime/bin/node"
    return 0
  fi

  if [ -d "$INFRING_HOME/node-runtime" ]; then
    candidate="$(find "$INFRING_HOME/node-runtime" -maxdepth 4 -type f -name node 2>/dev/null | sort | head -n 1 || true)"
    if [ -n "$candidate" ] && [ -x "$candidate" ]; then
      printf '%s\n' "$candidate"
      return 0
    fi
  fi

  if command -v node >/dev/null 2>&1; then
    command -v node
    return 0
  fi
  return 1
}

resolve_npm_binary_path() {
  preferred="${INFRING_NPM_BINARY:-}"
  if [ -n "$preferred" ]; then
    if [ -x "$preferred" ]; then
      printf '%s\n' "$preferred"
      return 0
    fi
    if command -v "$preferred" >/dev/null 2>&1; then
      command -v "$preferred"
      return 0
    fi
  fi

  node_bin_path="$(resolve_node_binary_path 2>/dev/null || true)"
  if [ -n "$node_bin_path" ]; then
    npm_candidate="$(dirname "$node_bin_path")/npm"
    if [ -x "$npm_candidate" ]; then
      printf '%s\n' "$npm_candidate"
      return 0
    fi
  fi

  if command -v npm >/dev/null 2>&1; then
    command -v npm
    return 0
  fi
  return 1
}

runtime_module_resolvable() {
  workspace="$1"
  module_name="$2"
  node_bin_path="$(resolve_node_binary_path 2>/dev/null || true)"
  [ -n "$node_bin_path" ] || return 1
  (
    cd "$workspace" >/dev/null 2>&1 || exit 1
    "$node_bin_path" -e "try{require.resolve(process.argv[1]);process.exit(0);}catch(_e){process.exit(1);}" "$module_name" \
      >/dev/null 2>&1
  )
}

portable_node_archive_name() {
  os_name="$(norm_os)"
  arch_name="$(norm_arch)"
  case "$os_name" in
    darwin)
      case "$arch_name" in
        aarch64) printf '%s\n' "node-v${NODE_RUNTIME_VERSION}-darwin-arm64.tar.gz" ;;
        x86_64) printf '%s\n' "node-v${NODE_RUNTIME_VERSION}-darwin-x64.tar.gz" ;;
        *) return 1 ;;
      esac
      ;;
    linux)
      case "$arch_name" in
        aarch64) printf '%s\n' "node-v${NODE_RUNTIME_VERSION}-linux-arm64.tar.xz" ;;
        x86_64) printf '%s\n' "node-v${NODE_RUNTIME_VERSION}-linux-x64.tar.xz" ;;
        *) return 1 ;;
      esac
      ;;
    *)
      return 1
      ;;
  esac
  return 0
}

install_node_runtime_portable() {
  archive_name="$(portable_node_archive_name 2>/dev/null || true)"
  if [ -z "$archive_name" ]; then
    echo "[infring install] portable Node runtime is unavailable for this platform."
    return 1
  fi

  runtime_dir="$INFRING_HOME/node-runtime"
  tmpdir="$(mktemp -d)"
  archive_path="$tmpdir/$archive_name"
  node_url="https://nodejs.org/dist/v${NODE_RUNTIME_VERSION}/${archive_name}"

  echo "[infring install] attempting portable Node runtime install:"
  echo "[infring install]   $node_url"

  if ! curl_fetch "$node_url" -o "$archive_path"; then
    rm -rf "$tmpdir"
    echo "[infring install] portable Node download failed."
    return 1
  fi

  rm -rf "$runtime_dir"
  mkdir -p "$runtime_dir"
  case "$archive_name" in
    *.tar.gz)
      tar -xzf "$archive_path" -C "$runtime_dir" || {
        rm -rf "$tmpdir"
        echo "[infring install] portable Node extract failed (.tar.gz)."
        return 1
      }
      ;;
    *.tar.xz)
      tar -xJf "$archive_path" -C "$runtime_dir" || {
        rm -rf "$tmpdir"
        echo "[infring install] portable Node extract failed (.tar.xz)."
        return 1
      }
      ;;
    *)
      rm -rf "$tmpdir"
      echo "[infring install] portable Node archive format unsupported: $archive_name"
      return 1
      ;;
  esac

  node_bin_path="$(find "$runtime_dir" -maxdepth 4 -type f -path '*/bin/node' 2>/dev/null | head -n 1)"
  if [ -z "$node_bin_path" ] || [ ! -x "$node_bin_path" ]; then
    rm -rf "$tmpdir"
    echo "[infring install] portable Node install incomplete (node binary missing)."
    return 1
  fi

  export INFRING_NODE_BINARY="$node_bin_path"
  rm -rf "$tmpdir"
  echo "[infring install] portable Node installed: $node_bin_path"
  return 0
}

detect_node_install_command() {
  os_name="$(norm_os)"
  if [ "$os_name" = "darwin" ]; then
    if command -v brew >/dev/null 2>&1; then
      printf '%s\n' "brew install node@22 && brew link --overwrite --force node@22"
      return 0
    fi
    if command -v port >/dev/null 2>&1; then
      printf '%s\n' "sudo port selfupdate && sudo port install nodejs22"
      return 0
    fi
    printf '%s\n' "Install Homebrew from https://brew.sh then run: brew install node@22"
    return 0
  fi

  if command -v apt-get >/dev/null 2>&1; then
    printf '%s\n' "curl -fsSL https://deb.nodesource.com/setup_22.x | sudo -E bash - && sudo apt-get install -y nodejs"
    return 0
  fi
  if command -v dnf >/dev/null 2>&1; then
    printf '%s\n' "sudo dnf install -y nodejs npm"
    return 0
  fi
  if command -v yum >/dev/null 2>&1; then
    printf '%s\n' "sudo yum install -y nodejs npm"
    return 0
  fi
  if command -v pacman >/dev/null 2>&1; then
    printf '%s\n' "sudo pacman -S --noconfirm nodejs npm"
    return 0
  fi
  if command -v apk >/dev/null 2>&1; then
    printf '%s\n' "sudo apk add --no-cache nodejs npm"
    return 0
  fi
  if command -v zypper >/dev/null 2>&1; then
    printf '%s\n' "sudo zypper install -y nodejs22"
    return 0
  fi
  printf '%s\n' "Install Node.js 22+ from https://nodejs.org/en/download"
  return 0
}

ensure_node_runtime_notice() {
  if node_runtime_meets_minimum; then
    node_bin_path="$(resolve_node_binary_path 2>/dev/null || true)"
    node_ver="$("$node_bin_path" --version 2>/dev/null || true)"
    if [ -n "$node_ver" ]; then
      echo "[infring install] Node.js check: $node_ver (OK for full CLI surface)"
      [ -n "$node_bin_path" ] && echo "[infring install] Node.js binary: $node_bin_path"
    fi
    return 0
  fi

  install_cmd="$(detect_node_install_command)"
  if node_bin_path="$(resolve_node_binary_path 2>/dev/null || true)"; then
    node_ver="$("$node_bin_path" --version 2>/dev/null || true)"
    if [ -n "$node_ver" ]; then
      echo "[infring install] Node.js check: detected $node_ver (requires 22+ for full CLI surface)"
    else
      echo "[infring install] Node.js check: detected but version could not be read"
    fi
  else
    echo "[infring install] Node.js check: not detected (requires 22+ for full CLI surface)"
  fi

  auto_node_mode=""
  if is_truthy "$INSTALL_NODE"; then
    auto_node_mode="explicit"
  elif is_truthy "$INSTALL_FULL" && is_truthy "$INSTALL_NODE_AUTO"; then
    auto_node_mode="full_auto"
  fi

  if [ -n "$auto_node_mode" ]; then
    auto_installed=0
    if [ "$auto_node_mode" = "full_auto" ]; then
      echo "[infring install] full mode: attempting automatic portable Node.js runtime bootstrap"
    fi
    if install_node_runtime_portable >/dev/null 2>&1 && node_runtime_meets_minimum; then
      auto_installed=1
    fi
    if [ "$auto_installed" = "1" ]; then
      node_bin_path="$(resolve_node_binary_path 2>/dev/null || true)"
      node_ver="$("$node_bin_path" --version 2>/dev/null || true)"
      echo "[infring install] Node.js install complete: ${node_ver:-unknown version}"
      [ -n "$node_bin_path" ] && echo "[infring install] Node.js binary: $node_bin_path"
      return 0
    fi
    if [ "$auto_node_mode" = "full_auto" ]; then
      echo "[infring install] warning: portable Node auto-install failed in full mode."
      echo "[infring install] run manually: $install_cmd"
      echo "[infring install] fallback: set INFRING_NODE_BINARY to a valid node executable."
      echo "[infring install] verify setup: infring setup status --json"
      echo "[infring install] verify gateway: infring gateway status"
      return 1
    fi
    case "$install_cmd" in
      Install\ Homebrew*|Install\ Node.js*)
        echo "[infring install] package-manager Node install unavailable on this host."
        ;;
      *)
        echo "[infring install] attempting automatic Node.js install:"
        echo "[infring install]   $install_cmd"
        if sh -c "$install_cmd" && node_runtime_meets_minimum; then
          auto_installed=1
        fi
        ;;
    esac
    if [ "$auto_installed" = "1" ]; then
      node_bin_path="$(resolve_node_binary_path 2>/dev/null || true)"
      node_ver="$("$node_bin_path" --version 2>/dev/null || true)"
      echo "[infring install] Node.js install complete: ${node_ver:-unknown version}"
      [ -n "$node_bin_path" ] && echo "[infring install] Node.js binary: $node_bin_path"
      return 0
    fi
    echo "[infring install] warning: automatic Node.js install failed."
    echo "[infring install] run manually: $install_cmd"
    echo "[infring install] fallback: set INFRING_NODE_BINARY to a valid node executable."
    echo "[infring install] verify setup: infring setup status --json"
    echo "[infring install] verify gateway: infring gateway status"
    return 1
  fi

  echo "[infring install] install Node.js now:"
  echo "[infring install]   $install_cmd"
  echo "[infring install] tip: rerun installer with --install-node to attempt automatic install."
  return 1
}

ollama_runtime_online() {
  if ! command -v curl >/dev/null 2>&1; then
    return 1
  fi
  host="${OLLAMA_HOST:-http://127.0.0.1:11434}"
  case "$host" in
    http://*|https://*) ;;
    *) host="http://$host" ;;
  esac
  base="${host%/}"
  if curl -fsS --connect-timeout 2 --max-time 4 "$base/api/tags" >/dev/null 2>&1; then
    return 0
  fi
  if curl -fsS --connect-timeout 2 --max-time 4 "$base/api/version" >/dev/null 2>&1; then
    return 0
  fi
  return 1
}

start_ollama_runtime_best_effort() {
  if ! command -v ollama >/dev/null 2>&1; then
    return 1
  fi
  if ollama_runtime_online; then
    return 0
  fi
  (
    ollama serve >/dev/null 2>&1 &
  ) || true
  i=0
  while [ "$i" -lt 20 ]; do
    if ollama_runtime_online; then
      return 0
    fi
    sleep 1
    i=$((i + 1))
  done
  return 1
}

detect_ollama_install_command() {
  os_name="$(norm_os)"
  case "$os_name" in
    darwin)
      if command -v brew >/dev/null 2>&1; then
        printf '%s\n' "brew install ollama"
        return 0
      fi
      printf '%s\n' "curl -fsSL https://ollama.com/install.sh | sh"
      return 0
      ;;
    linux)
      printf '%s\n' "curl -fsSL https://ollama.com/install.sh | sh"
      return 0
      ;;
    *)
      printf '%s\n' "Install Ollama from https://ollama.com/download"
      return 0
      ;;
  esac
}

install_ollama_runtime() {
  if command -v ollama >/dev/null 2>&1; then
    return 0
  fi
  install_cmd="$(detect_ollama_install_command)"
  case "$install_cmd" in
    Install\ Ollama*)
      return 1
      ;;
  esac
  if ! sh -c "$install_cmd"; then
    return 1
  fi
  command -v ollama >/dev/null 2>&1
}

normalize_ollama_model_ref() {
  raw_ref="$1"
  trimmed="$(printf '%s' "$raw_ref" | sed 's/^[[:space:]]*//;s/[[:space:]]*$//')"
  trimmed="${trimmed#ollama/}"
  printf '%s\n' "$trimmed"
}

ollama_list_model_names() {
  if ! command -v ollama >/dev/null 2>&1; then
    return 1
  fi
  ollama list 2>/dev/null | awk 'NR>1 && $1 != "" {print $1}'
}

ollama_model_count() {
  if ! command -v ollama >/dev/null 2>&1; then
    printf '%s\n' "0"
    return 0
  fi
  count="$(ollama_list_model_names | awk 'END {print NR+0}')"
  case "$count" in
    ''|*[!0-9]*) count=0 ;;
  esac
  printf '%s\n' "$count"
  return 0
}

ollama_model_present() {
  target_raw="$1"
  target="$(normalize_ollama_model_ref "$target_raw")"
  [ -n "$target" ] || return 1
  if ! command -v ollama >/dev/null 2>&1; then
    return 1
  fi
  ollama_list_model_names | grep -Fqx "$target" >/dev/null 2>&1
}

ensure_ollama_starter_model() {
  if ! command -v ollama >/dev/null 2>&1; then
    return 1
  fi
  if ! is_truthy "$INSTALL_OLLAMA_PULL"; then
    OLLAMA_LAST_MODEL_COUNT="$(ollama_model_count)"
    echo "[infring install] starter model pull skipped (INFRING_INSTALL_OLLAMA_PULL=0)."
    return 0
  fi
  starter="$(normalize_ollama_model_ref "$OLLAMA_STARTER_MODEL")"
  if [ -z "$starter" ]; then
    OLLAMA_LAST_MODEL_COUNT="$(ollama_model_count)"
    return 0
  fi
  if ! start_ollama_runtime_best_effort; then
    echo "[infring install] starter model pull skipped: Ollama daemon is offline."
    return 1
  fi
  current_count="$(ollama_model_count)"
  OLLAMA_LAST_MODEL_COUNT="$current_count"
  if [ "$current_count" -gt 0 ]; then
    echo "[infring install] Ollama models: $current_count detected (starter pull not required)."
    return 0
  fi
  pull_timeout="$OLLAMA_PULL_TIMEOUT"
  case "$pull_timeout" in
    ''|*[!0-9]*) pull_timeout=900 ;;
  esac
  if [ "$pull_timeout" -lt 60 ]; then
    pull_timeout=60
  fi
  if [ "$pull_timeout" -gt 3600 ]; then
    pull_timeout=3600
  fi
  pull_log="$(mktemp)"
  echo "[infring install] pulling starter model for first-run readiness: $starter"
  if run_command_with_timeout "$pull_timeout" ollama pull "$starter" >"$pull_log" 2>&1; then
    current_count="$(ollama_model_count)"
    OLLAMA_LAST_MODEL_COUNT="$current_count"
    echo "[infring install] starter model pull complete: $starter"
    rm -f "$pull_log" >/dev/null 2>&1 || true
    return 0
  fi
  pull_status="$?"
  echo "[infring install] starter model pull failed (${pull_status}): $starter" >&2
  tail -n 20 "$pull_log" >&2 || true
  rm -f "$pull_log" >/dev/null 2>&1 || true
  return 1
}

prompt_yes_no_tty() {
  prompt="$1"
  default_answer="$2"
  if [ ! -t 0 ] || [ ! -r /dev/tty ] || [ ! -w /dev/tty ]; then
    return 2
  fi
  while true; do
    if ! printf '%s' "$prompt" > /dev/tty 2>/dev/null; then
      return 2
    fi
    reply=""
    if ! IFS= read -r reply < /dev/tty 2>/dev/null; then
      return 2
    fi
    reply="$(printf '%s' "$reply" | tr '[:upper:]' '[:lower:]')"
    if [ -z "$reply" ]; then
      reply="$default_answer"
    fi
    case "$reply" in
      y|yes) return 0 ;;
      n|no) return 1 ;;
    esac
    if ! printf '%s\n' "Please answer y or n." > /dev/tty 2>/dev/null; then
      return 2
    fi
  done
}

ensure_ollama_runtime_notice() {
  OLLAMA_LAST_MODEL_COUNT=0
  require_model_ready_now=0
  if is_truthy "$INSTALL_REQUIRE_MODEL_READY"; then
    require_model_ready_now=1
  fi
  if command -v ollama >/dev/null 2>&1; then
    ollama_bin="$(command -v ollama 2>/dev/null || true)"
    echo "[infring install] Ollama check: detected (${ollama_bin:-ollama})"
    if start_ollama_runtime_best_effort; then
      echo "[infring install] Ollama runtime: online"
      if is_truthy "$INSTALL_FULL"; then
        if ! ensure_ollama_starter_model; then
          if [ "$require_model_ready_now" = "1" ]; then
            echo "[infring install] model readiness: required but starter model bootstrap failed." >&2
            return 1
          fi
        fi
      else
        OLLAMA_LAST_MODEL_COUNT="$(ollama_model_count)"
      fi
      if [ "$OLLAMA_LAST_MODEL_COUNT" -gt 0 ]; then
        echo "[infring install] model readiness: ${OLLAMA_LAST_MODEL_COUNT} local model(s) detected"
      else
        echo "[infring install] model readiness: no local models detected yet"
      fi
      return 0
    fi
    echo "[infring install] Ollama runtime: offline (install succeeded but daemon not reachable)"
    echo "[infring install] start it now: ollama serve"
    if [ "$require_model_ready_now" = "1" ]; then
      return 1
    fi
    return 0
  fi

  install_cmd="$(detect_ollama_install_command)"
  echo "[infring install] Ollama check: not detected (recommended for local models out of the box)"
  install_mode=""
  if is_truthy "$INSTALL_OLLAMA"; then
    install_mode="explicit"
  elif is_truthy "$INSTALL_FULL" && is_truthy "$INSTALL_OLLAMA_AUTO"; then
    install_mode="prompt"
  fi

  should_install=0
  if [ "$install_mode" = "explicit" ]; then
    should_install=1
    OLLAMA_INSTALL_CONFIRMED=1
    require_model_ready_now=1
  elif [ "$install_mode" = "prompt" ]; then
    if prompt_yes_no_tty "[infring install] Install Ollama now to enable local models? [y/N] " "n"; then
      should_install=1
      OLLAMA_INSTALL_CONFIRMED=1
      require_model_ready_now=1
    elif [ "$?" = "2" ]; then
      echo "[infring install] Ollama prompt skipped (no interactive TTY detected)."
    else
      echo "[infring install] Ollama install skipped by user."
    fi
  fi

  if [ "$should_install" = "1" ]; then
    if install_ollama_runtime; then
      echo "[infring install] Ollama install complete."
      if start_ollama_runtime_best_effort; then
        echo "[infring install] Ollama runtime: online"
        if ensure_ollama_starter_model; then
          if [ "$OLLAMA_LAST_MODEL_COUNT" -gt 0 ]; then
            echo "[infring install] model readiness: ${OLLAMA_LAST_MODEL_COUNT} local model(s) detected"
          fi
          return 0
        fi
        echo "[infring install] model readiness: starter model bootstrap failed." >&2
        if [ "$require_model_ready_now" = "1" ]; then
          return 1
        fi
      else
        echo "[infring install] Ollama runtime: install complete, start manually with 'ollama serve'"
        if [ "$require_model_ready_now" = "1" ]; then
          return 1
        fi
      fi
      return 0
    fi
    echo "[infring install] Ollama install failed."
    if [ "$require_model_ready_now" = "1" ]; then
      return 1
    fi
  fi

  echo "[infring install] install Ollama now:"
  echo "[infring install]   $install_cmd"
  echo "[infring install] then pull a model: ollama pull $(normalize_ollama_model_ref "$OLLAMA_STARTER_MODEL")"
  echo "[infring install] and verify: ollama list"
  if [ "$require_model_ready_now" = "1" ]; then
    return 1
  fi
  return 1
}

curl_fetch() {
  if is_truthy "$INSTALL_DEBUG"; then
    curl -fsSL "$@"
  else
    curl -fsSL "$@" 2>/dev/null
  fi
}

parse_install_args() {
  while [ "$#" -gt 0 ]; do
    arg="$1"
    case "$arg" in
      --json|--json=1)
        INSTALL_JSON_OUTPUT=1
        ;;
      --full)
        INSTALL_FULL=1
        INSTALL_PURE=0
        ;;
      --minimal)
        INSTALL_FULL=0
        ;;
      --pure)
        INSTALL_PURE=1
        INSTALL_FULL=0
        ;;
      --tiny-max)
        INSTALL_TINY_MAX=1
        INSTALL_PURE=1
        INSTALL_FULL=0
        ;;
      --repair)
        INSTALL_REPAIR=1
        ;;
      --install-node)
        INSTALL_NODE=1
        ;;
      --install-ollama)
        INSTALL_OLLAMA=1
        ;;
      --offline)
        INSTALL_OFFLINE=1
        ;;
      --install-dir)
        shift
        if [ "$#" -eq 0 ]; then
          echo "[infring install] --install-dir requires a value" >&2
          exit 1
        fi
        INSTALL_DIR="$1"
        INSTALL_DIR_EXPLICIT=1
        ;;
      --install-dir=*)
        INSTALL_DIR="${arg#--install-dir=}"
        INSTALL_DIR_EXPLICIT=1
        ;;
      --tmp-dir)
        shift
        if [ "$#" -eq 0 ]; then
          echo "[infring install] --tmp-dir requires a value" >&2
          exit 1
        fi
        INSTALL_TMP_DIR="$1"
        ;;
      --tmp-dir=*)
        INSTALL_TMP_DIR="${arg#--tmp-dir=}"
        ;;
      --help|-h)
        echo "Usage: install.sh [--full|--minimal|--pure|--tiny-max|--repair|--install-node|--install-ollama|--offline|--json] [--install-dir PATH] [--tmp-dir PATH]"
        echo "  --full            install optional client runtime bundle when available"
        echo "  --minimal         install daemon + CLI only (default)"
        echo "  --pure            install pure Rust client + daemon only (no Node/TS surfaces)"
        echo "  --tiny-max        install tiny-max pure profile for old/embedded hardware targets"
        echo "  --repair          clear stale install wrappers + workspace runtime state before install"
        echo "  --install-node    attempt automatic Node.js 22+ install for full CLI command surface"
        echo "  --install-ollama  attempt automatic Ollama install and starter local model bootstrap"
        echo "  --offline         disable network fetch; require cached verified release artifacts"
        echo "  --json            emit machine-readable install success summary JSON"
        echo "  --install-dir     install wrappers/binaries into this directory"
        echo "  --tmp-dir         use this temp directory for download/build staging"
        echo "  --verify-install-summary-contract  verify INFRING_INSTALL_SUMMARY_FILE status/completed_at contract"
        exit 0
        ;;
      *)
        echo "[infring install] unknown argument: $arg" >&2
        exit 1
        ;;
    esac
    shift
  done
}

install_summary_init() {
  summary_file="$INSTALL_SUMMARY_FILE"
  summary_dir="$(dirname "$summary_file")"
  mkdir -p "$summary_dir"
  {
    echo "infring_install_summary_v1"
    echo "timestamp: $(date -u '+%Y-%m-%dT%H:%M:%SZ' 2>/dev/null || true)"
    echo "repo: ${REPO_OWNER}/${REPO_NAME}"
    echo "requested_version: ${REQUESTED_VERSION}"
    echo "install_mode_full: ${INSTALL_FULL}"
    echo "install_mode_pure: ${INSTALL_PURE}"
    echo "install_mode_tiny_max: ${INSTALL_TINY_MAX}"
    echo "install_mode_repair: ${INSTALL_REPAIR}"
    echo "install_mode_install_node: ${INSTALL_NODE}"
    echo "install_mode_install_ollama: ${INSTALL_OLLAMA}"
    echo "install_mode_install_ollama_auto: ${INSTALL_OLLAMA_AUTO}"
    echo "install_mode_require_model_ready: ${INSTALL_REQUIRE_MODEL_READY}"
    echo "install_mode_offline: ${INSTALL_OFFLINE}"
    echo "ollama_starter_model: ${OLLAMA_STARTER_MODEL}"
    echo "install_dir: ${INSTALL_DIR}"
    echo "workspace_dir: ${WORKSPACE_DIR}"
    echo "status: failed"
  } > "$summary_file"
}

install_summary_note() {
  note="$1"
  [ -n "$note" ] || return 0
  INSTALL_SUMMARY_LAST_NOTE="$(printf '%s' "$note" | tr '\r\n' ' ' | tr -s ' ')"
  {
    printf '%s\n' "$note"
  } >> "$INSTALL_SUMMARY_FILE" 2>/dev/null || true
}

install_summary_sync() {
  summary_file="$INSTALL_SUMMARY_FILE"
  summary_dir="$(dirname "$summary_file")"
  mkdir -p "$summary_dir" >/dev/null 2>&1 || true
  if [ ! -f "$summary_file" ]; then
    {
      echo "infring_install_summary_v1"
      echo "timestamp: $(date -u '+%Y-%m-%dT%H:%M:%SZ' 2>/dev/null || true)"
      echo "repo: ${REPO_OWNER}/${REPO_NAME}"
      echo "requested_version: ${REQUESTED_VERSION}"
      echo "install_dir: ${INSTALL_DIR}"
      echo "workspace_dir: ${WORKSPACE_DIR}"
    } > "$summary_file" 2>/dev/null || return 1
  fi
  tmp_file="${summary_file}.tmp"
  (
    while IFS= read -r line || [ -n "$line" ]; do
      case "$line" in
        status:*|completed_at:*|failed_at:*|exit_code:*|failure_reason:*|last_note:*)
          ;;
        *)
          echo "$line"
          ;;
      esac
    done < "$summary_file"
    if [ -n "$INSTALL_SUMMARY_COMPLETED_AT" ]; then
      echo "completed_at: ${INSTALL_SUMMARY_COMPLETED_AT}"
    fi
    if [ "$INSTALL_SUMMARY_STATUS" != "success" ]; then
      if [ -n "$INSTALL_SUMMARY_FAILED_AT" ]; then
        echo "failed_at: ${INSTALL_SUMMARY_FAILED_AT}"
      fi
      if [ -n "$INSTALL_SUMMARY_EXIT_CODE" ]; then
        echo "exit_code: ${INSTALL_SUMMARY_EXIT_CODE}"
      fi
      if [ -n "$INSTALL_SUMMARY_FAILURE_REASON" ]; then
        echo "failure_reason: ${INSTALL_SUMMARY_FAILURE_REASON}"
      fi
      if [ -n "$INSTALL_SUMMARY_LAST_NOTE" ]; then
        echo "last_note: ${INSTALL_SUMMARY_LAST_NOTE}"
      fi
    fi
    echo "status: ${INSTALL_SUMMARY_STATUS}"
  ) > "$tmp_file" 2>/dev/null || return 1
  mv "$tmp_file" "$summary_file" >/dev/null 2>&1 || return 1
  return 0
}

install_summary_mark_success() {
  INSTALL_SUMMARY_STATUS="success"
  INSTALL_SUMMARY_COMPLETED_AT="$(date -u '+%Y-%m-%dT%H:%M:%SZ' 2>/dev/null || true)"
  INSTALL_SUMMARY_FAILED_AT=""
  INSTALL_SUMMARY_EXIT_CODE=""
  INSTALL_SUMMARY_FAILURE_REASON=""
  install_summary_sync
}

verify_install_summary_success_contract() {
  summary_file="$INSTALL_SUMMARY_FILE"
  if [ ! -f "$summary_file" ]; then
    echo "[infring install] summary contract failed: missing $summary_file" >&2
    return 1
  fi
  status_line="$(grep -E '^status:' "$summary_file" 2>/dev/null | tail -n 1 | tr -d '\r' || true)"
  if [ "$status_line" != "status: success" ]; then
    echo "[infring install] summary contract failed: expected status: success, got '${status_line:-missing}'" >&2
    return 1
  fi
  last_line="$(awk 'NF{line=$0} END{print line}' "$summary_file" 2>/dev/null | tr -d '\r' || true)"
  if [ "$last_line" != "status: success" ]; then
    echo "[infring install] summary contract failed: status is not final line" >&2
    return 1
  fi
  if ! grep -q '^completed_at:' "$summary_file" 2>/dev/null; then
    echo "[infring install] summary contract failed: completed_at missing" >&2
    return 1
  fi
  if ! grep -q '^workspace_runtime_refresh_required:' "$summary_file" 2>/dev/null; then
    echo "[infring install] summary contract failed: workspace_runtime_refresh_required missing" >&2
    return 1
  fi
  if ! grep -q '^workspace_runtime_refresh_applied:' "$summary_file" 2>/dev/null; then
    echo "[infring install] summary contract failed: workspace_runtime_refresh_applied missing" >&2
    return 1
  fi
  if ! grep -q '^workspace_release_tag_written:' "$summary_file" 2>/dev/null; then
    echo "[infring install] summary contract failed: workspace_release_tag_written missing" >&2
    return 1
  fi
  if ! grep -q '^workspace_release_tag_write_verified:' "$summary_file" 2>/dev/null; then
    echo "[infring install] summary contract failed: workspace_release_tag_write_verified missing" >&2
    return 1
  fi
  echo "[infring install] summary contract: ok"
  return 0
}

install_summary_finalize() {
  exit_code="${1:-1}"
  if [ "$exit_code" = "0" ]; then
    INSTALL_SUMMARY_STATUS="success"
    if [ -z "$INSTALL_SUMMARY_COMPLETED_AT" ]; then
      INSTALL_SUMMARY_COMPLETED_AT="$(date -u '+%Y-%m-%dT%H:%M:%SZ' 2>/dev/null || true)"
    fi
    INSTALL_SUMMARY_FAILED_AT=""
    INSTALL_SUMMARY_EXIT_CODE=""
    INSTALL_SUMMARY_FAILURE_REASON=""
  else
    INSTALL_SUMMARY_STATUS="failed"
    INSTALL_SUMMARY_COMPLETED_AT=""
    if [ -z "$INSTALL_SUMMARY_FAILED_AT" ]; then
      INSTALL_SUMMARY_FAILED_AT="$(date -u '+%Y-%m-%dT%H:%M:%SZ' 2>/dev/null || true)"
    fi
    INSTALL_SUMMARY_EXIT_CODE="$exit_code"
    if [ -z "$INSTALL_SUMMARY_FAILURE_REASON" ]; then
      INSTALL_SUMMARY_FAILURE_REASON="installer_exit_nonzero"
    fi
  fi
  install_summary_sync || true
}

install_json_escape() {
  printf '%s' "${1:-}" | sed 's/\\/\\\\/g; s/"/\\"/g; s/\r//g; :a;N;$!ba;s/\n/\\n/g'
}

install_color_enabled() {
  [ -t 1 ] && [ -z "${NO_COLOR:-}" ]
}

install_print_colored() {
  color_code="$1"
  text="$2"
  no_newline="${3:-0}"
  if install_color_enabled; then
    if [ "$no_newline" = "1" ]; then
      printf '\033[%sm%s\033[0m' "$color_code" "$text"
    else
      printf '\033[%sm%s\033[0m\n' "$color_code" "$text"
    fi
  elif [ "$no_newline" = "1" ]; then
    printf '%s' "$text"
  else
    printf '%s\n' "$text"
  fi
}

emit_install_completion_card() {
  version_tag="$1"
  install_location="$2"
  if [ "${INFRING_INSTALL_USE_MODULES:-1}" != "0" ] && command -v infring_install_completion_card >/dev/null 2>&1; then
    infring_install_completion_card "$version_tag." "$install_location" "infring --help"
    return
  fi
  printf '\nSetting up InfRing...\n\n'
  install_print_colored "32" "✔ InfRing successfully installed!"
  printf '\n'
  printf '  Version: '
  install_print_colored "38;5;208" "${version_tag}."
  printf '  Location: %s\n\n' "$install_location"
  printf '  Next: Run '
  install_print_colored "38;5;208" "infring --help" "1"
  printf ' to get started.\n\n'
  printf '✅ Installation complete!\n'
}

emit_install_success_summary() {
  version_tag="$1"
  triple_id="$2"
  quick_prefix="$3"
  wrappers_status="ok"
  gateway_smoke_status="passed"
  if is_truthy "$INSTALL_PURE"; then
    gateway_smoke_status="skipped_pure_profile"
  fi
  dashboard_smoke_status="skipped"
  if is_truthy "$INSTALL_FULL"; then
    if [ "$INSTALL_DASHBOARD_SMOKE_PASSED" = "1" ]; then
      dashboard_smoke_status="passed"
    else
      dashboard_smoke_status="failed"
    fi
  fi
  verification_confidence="high"
  if [ "$INSTALL_RUNTIME_CONTRACT_OK" != "1" ]; then
    verification_confidence="medium"
  fi
  if [ "$gateway_smoke_status" != "passed" ] && ! is_truthy "$INSTALL_PURE"; then
    verification_confidence="medium"
  fi
  if is_truthy "$INSTALL_FULL" && [ "$dashboard_smoke_status" != "passed" ]; then
    verification_confidence="medium"
  fi
  launcher_command="${quick_prefix}infring gateway"
  restart_command="${quick_prefix}infring gateway restart"
  recovery_command="${quick_prefix}infring recover"
  node_summary_bin="$(resolve_node_binary_path 2>/dev/null || true)"
  node_detected=0
  if [ -n "$node_summary_bin" ]; then
    node_detected=1
  fi
  runtime_contract_mode="${INSTALL_RUNTIME_CONTRACT_MODE:-unknown}"
  client_runtime_mode="${INSTALL_CLIENT_RUNTIME_MODE:-not_installed}"
  workspace_refresh_required_value="${WORKSPACE_REFRESH_REQUIRED:-0}"
  workspace_refresh_applied_value="${WORKSPACE_REFRESH_APPLIED:-0}"
  workspace_refresh_reason_value="${WORKSPACE_REFRESH_REASON:-}"
  workspace_refresh_tag_missing_value="${WORKSPACE_REFRESH_TAG_STATE_MISSING:-0}"
  workspace_release_tag_previous_value="${WORKSPACE_RELEASE_TAG_PREVIOUS:-}"
  workspace_release_tag_current_value="${WORKSPACE_RELEASE_TAG_CURRENT:-}"
  workspace_release_tag_written_value="${WORKSPACE_RELEASE_TAG_WRITTEN:-0}"
  workspace_release_tag_verified_value="${WORKSPACE_RELEASE_TAG_WRITE_VERIFIED:-0}"
  shell_default_value="${INSTALL_SHELL_DEFAULT:-ui}"
  shell_fallback_value="${INSTALL_SHELL_FALLBACK:-terminal}"
  echo "[infring install] success summary: binaries=${wrappers_status} runtime=${runtime_contract_mode} launcher=infring gateway restart=infring gateway restart verification_confidence=${verification_confidence}"
  echo "[infring install] success summary: gateway_smoke=${gateway_smoke_status} dashboard_smoke=${dashboard_smoke_status} recovery=infring recover"
  echo "[infring install] success summary: shell_default=${shell_default_value} shell_fallback=${shell_fallback_value}"

  summary_json_path="$INSTALL_SUMMARY_JSON_FILE"
  mkdir -p "$(dirname "$summary_json_path")" >/dev/null 2>&1 || true
  payload="$(cat <<EOF
{"ok":true,"type":"infring_install_success_summary","version":"$(install_json_escape "$version_tag")","triple":"$(install_json_escape "$triple_id")","install_mode":{"full":$( [ "$INSTALL_FULL" = "1" ] && printf 'true' || printf 'false' ),"pure":$( [ "$INSTALL_PURE" = "1" ] && printf 'true' || printf 'false' ),"tiny_max":$( [ "$INSTALL_TINY_MAX" = "1" ] && printf 'true' || printf 'false' ),"repair":$( [ "$INSTALL_REPAIR" = "1" ] && printf 'true' || printf 'false' ),"offline":$( [ "$INSTALL_OFFLINE" = "1" ] && printf 'true' || printf 'false' )},"shell":{"default":"$(install_json_escape "$shell_default_value")","fallback":"$(install_json_escape "$shell_fallback_value")","override_flag":"--shell=terminal"},"verification":{"confidence":"$(install_json_escape "$verification_confidence")","runtime_contract_ok":$( [ "$INSTALL_RUNTIME_CONTRACT_OK" = "1" ] && printf 'true' || printf 'false' ),"runtime_contract_mode":"$(install_json_escape "$runtime_contract_mode")","client_runtime_mode":"$(install_json_escape "$client_runtime_mode")","gateway_smoke":"$(install_json_escape "$gateway_smoke_status")","dashboard_smoke":"$(install_json_escape "$dashboard_smoke_status")","node_runtime_detected":$( [ "$node_detected" = "1" ] && printf 'true' || printf 'false' )},"workspace_runtime_refresh":{"required":$( [ "$workspace_refresh_required_value" = "1" ] && printf 'true' || printf 'false' ),"applied":$( [ "$workspace_refresh_applied_value" = "1" ] && printf 'true' || printf 'false' ),"reason":"$(install_json_escape "$workspace_refresh_reason_value")","tag_state_missing":$( [ "$workspace_refresh_tag_missing_value" = "1" ] && printf 'true' || printf 'false' ),"previous_release_tag":"$(install_json_escape "$workspace_release_tag_previous_value")","current_release_tag":"$(install_json_escape "$workspace_release_tag_current_value")","release_tag_write_applied":$( [ "$workspace_release_tag_written_value" = "1" ] && printf 'true' || printf 'false' ),"release_tag_write_verified":$( [ "$workspace_release_tag_verified_value" = "1" ] && printf 'true' || printf 'false' )},"commands":{"launcher":"$(install_json_escape "$launcher_command")","restart":"$(install_json_escape "$restart_command")","recovery":"$(install_json_escape "$recovery_command")"},"summary_files":{"text":"$(install_json_escape "$INSTALL_SUMMARY_FILE")","json":"$(install_json_escape "$summary_json_path")"}}
EOF
)"
  printf '%s\n' "$payload" > "$summary_json_path" 2>/dev/null || true
  echo "[infring install] summary json: $summary_json_path"
  if is_truthy "$INSTALL_JSON_OUTPUT"; then
    printf '%s\n' "$payload"
  fi
}

tool_install_hint() {
  tool="$1"
  host_os="unknown"
  if command -v uname >/dev/null 2>&1; then
    host_os="$(uname -s 2>/dev/null | tr '[:upper:]' '[:lower:]')"
  fi
  case "$host_os" in
    darwin)
      case "$tool" in
        curl|git|tar|unzip) echo "brew install $tool" ;;
        *) echo "install missing system command: $tool" ;;
      esac
      ;;
    linux)
      if command -v apt-get >/dev/null 2>&1; then
        case "$tool" in
          curl|git|tar|unzip) echo "sudo apt-get update && sudo apt-get install -y $tool" ;;
          *) echo "install missing system command: $tool" ;;
        esac
      elif command -v dnf >/dev/null 2>&1; then
        case "$tool" in
          curl|git|tar|unzip) echo "sudo dnf install -y $tool" ;;
          *) echo "install missing system command: $tool" ;;
        esac
      else
        echo "install missing system command: $tool"
      fi
      ;;
    *)
      echo "install missing system command: $tool"
      ;;
  esac
}

run_install_preflight() {
  echo "[infring install] preflight: checking host prerequisites"
  missing_required=0
  for cmd in sh curl chmod mkdir uname tar; do
    if command -v "$cmd" >/dev/null 2>&1; then
      echo "[infring install] preflight $cmd: ok"
    else
      missing_required=1
      hint="$(tool_install_hint "$cmd")"
      echo "[infring install] preflight $cmd: missing"
      echo "[infring install] fix: $hint"
    fi
  done

  if command -v git >/dev/null 2>&1; then
    echo "[infring install] preflight git: ok"
  else
    echo "[infring install] preflight git: missing (source fallback may be slower or unavailable)"
    echo "[infring install] fix: $(tool_install_hint git)"
  fi

  if command -v unzip >/dev/null 2>&1; then
    echo "[infring install] preflight unzip: ok"
  else
    echo "[infring install] preflight unzip: missing (non-blocking)"
    echo "[infring install] fix: $(tool_install_hint unzip)"
  fi

  if node_runtime_meets_minimum; then
    node_bin_path="$(resolve_node_binary_path 2>/dev/null || true)"
    node_ver="$("$node_bin_path" --version 2>/dev/null || true)"
    echo "[infring install] preflight node: ${node_ver:-detected} (ok)"
  else
    echo "[infring install] preflight node: missing or <22 (optional for full surfaces)"
    if is_truthy "$INSTALL_FULL" && is_truthy "$INSTALL_NODE_AUTO"; then
      echo "[infring install] note: full mode will attempt automatic portable Node bootstrap later in install flow."
    fi
    if command -v uname >/dev/null 2>&1; then
      echo "[infring install] fix: $(detect_node_install_command)"
    else
      echo "[infring install] fix: install Node.js 22+ from https://nodejs.org/en/download"
    fi
  fi

  if ensure_cargo_command_ready; then
    echo "[infring install] preflight cargo: ok"
  elif command -v rustup >/dev/null 2>&1; then
    echo "[infring install] preflight cargo: rustup detected but default toolchain missing"
    echo "[infring install] fix: rustup default stable"
  else
    echo "[infring install] preflight cargo: missing (only needed for source fallback build path)"
    echo "[infring install] note: installer can bootstrap rustup automatically if source fallback is required."
    echo "[infring install] fix: curl --proto '=https' --tlsv1.2 -sSf $RUSTUP_INIT_URL | sh -s -- -y --profile minimal --default-toolchain stable"
  fi

  if [ "$missing_required" -ne 0 ]; then
    echo "[infring install] preflight failed: required host tools are missing." >&2
    return 1
  fi
  echo "[infring install] preflight: passed"
  return 0
}

resolve_sha256_tool() {
  if command -v shasum >/dev/null 2>&1; then
    echo "shasum"
    return 0
  fi
  if command -v sha256sum >/dev/null 2>&1; then
    echo "sha256sum"
    return 0
  fi
  if command -v openssl >/dev/null 2>&1; then
    echo "openssl"
    return 0
  fi
  return 1
}

sha256_file() {
  target_file="$1"
  tool="$(resolve_sha256_tool 2>/dev/null || true)"
  case "$tool" in
    shasum) shasum -a 256 "$target_file" | awk '{print $1}' ;;
    sha256sum) sha256sum "$target_file" | awk '{print $1}' ;;
    openssl) openssl dgst -sha256 "$target_file" | awk '{print $NF}' ;;
    *) return 1 ;;
  esac
}

load_release_checksum_manifest() {
  version_tag="$1"
  cache_dir="$INFRING_HOME/cache/install-assets/$version_tag"
  if [ "$CHECKSUM_MANIFEST_VERSION" = "$version_tag" ] && [ -n "$CHECKSUM_MANIFEST_PATH" ] && [ -f "$CHECKSUM_MANIFEST_PATH" ]; then
    return 0
  fi
  for checksum_asset in SHA256SUMS SHA256SUMS.txt checksums.txt checksums.sha256; do
    cache_manifest="$cache_dir/$checksum_asset"
    if [ -f "$cache_manifest" ]; then
      CHECKSUM_MANIFEST_PATH="$cache_manifest"
      CHECKSUM_MANIFEST_VERSION="$version_tag"
      install_summary_note "checksum_manifest: ${checksum_asset} (cache)"
      return 0
    fi
  done
  if is_truthy "$INSTALL_OFFLINE"; then
    return 1
  fi
  [ -n "$CHECKSUM_MANIFEST_TMP_DIR" ] && rm -rf "$CHECKSUM_MANIFEST_TMP_DIR" >/dev/null 2>&1 || true
  CHECKSUM_MANIFEST_TMP_DIR="$(mktemp -d)"
  CHECKSUM_MANIFEST_PATH=""
  for checksum_asset in SHA256SUMS SHA256SUMS.txt checksums.txt checksums.sha256; do
    if curl_fetch "$BASE_URL/$version_tag/$checksum_asset" -o "$CHECKSUM_MANIFEST_TMP_DIR/$checksum_asset"; then
      CHECKSUM_MANIFEST_PATH="$CHECKSUM_MANIFEST_TMP_DIR/$checksum_asset"
      CHECKSUM_MANIFEST_VERSION="$version_tag"
      if is_truthy "$INSTALL_ASSET_CACHE"; then
        mkdir -p "$cache_dir" >/dev/null 2>&1 || true
        cp "$CHECKSUM_MANIFEST_PATH" "$cache_dir/$checksum_asset" >/dev/null 2>&1 || true
      fi
      install_summary_note "checksum_manifest: $checksum_asset"
      return 0
    fi
  done
  return 1
}

expected_asset_sha256() {
  manifest_path="$1"
  asset_name="$2"
  [ -f "$manifest_path" ] || return 1
  awk -v asset="$asset_name" '
    BEGIN { IGNORECASE=1 }
    {
      line=$0
      gsub("\r", "", line)
      if (match(line, /^SHA256 \(([^)]+)\) = ([a-fA-F0-9]{64})$/, m)) {
        if (m[1] == asset) {
          print tolower(m[2]); exit
        }
      }
      n=split(line, parts, /[[:space:]]+/)
      if (n >= 2) {
        digest=tolower(parts[1])
        file=parts[n]
        sub(/^\*+/, "", file)
        sub(/^\.\/+/, "", file)
        if (length(digest) == 64 && file == asset) {
          print digest; exit
        }
      }
    }
  ' "$manifest_path"
}

verify_downloaded_asset() {
  version_tag="$1"
  asset_name="$2"
  asset_path="$3"
  if ! is_truthy "$INSTALL_VERIFY_ASSETS"; then
    return 0
  fi
  digest_tool="$(resolve_sha256_tool 2>/dev/null || true)"
  if [ -z "$digest_tool" ]; then
    echo "[infring install] asset verification failed: no sha256 tool found" >&2
    echo "[infring install] fix: install 'shasum' or 'sha256sum' (or openssl)" >&2
    return 1
  fi
  if ! load_release_checksum_manifest "$version_tag"; then
    allow_unverified=0
    allow_reason=""
    if is_truthy "$INSTALL_ALLOW_UNVERIFIED_ASSETS"; then
      allow_unverified=1
      allow_reason="override"
    elif is_prerelease_version_tag "$version_tag" && ! is_truthy "$INSTALL_STRICT_PRERELEASE_CHECKSUM"; then
      allow_unverified=1
      allow_reason="prerelease"
    fi
    if [ "$allow_unverified" = "1" ]; then
      warn_checksum_manifest_missing_once "$version_tag" "$allow_reason"
      install_summary_note "asset_unverified: ${asset_name} (manifest missing:${allow_reason})"
      return 0
    fi
    echo "[infring install] asset verification failed: checksum manifest unavailable for $version_tag" >&2
    echo "[infring install] fix: publish release checksum manifest (SHA256SUMS) or set INFRING_INSTALL_ALLOW_UNVERIFIED_ASSETS=1" >&2
    return 1
  fi
  expected_digest="$(expected_asset_sha256 "$CHECKSUM_MANIFEST_PATH" "$asset_name" || true)"
  if [ -z "$expected_digest" ]; then
    if is_truthy "$INSTALL_ALLOW_UNVERIFIED_ASSETS"; then
      echo "[infring install] warning: no checksum entry for $asset_name; continuing due to override."
      install_summary_note "asset_unverified: ${asset_name} (entry missing)"
      return 0
    fi
    if is_prerelease_version_tag "$version_tag" && ! is_truthy "$INSTALL_STRICT_PRERELEASE_CHECKSUM"; then
      echo "[infring install] warning: no checksum entry for $asset_name in prerelease $version_tag; continuing with unverified asset."
      install_summary_note "asset_unverified: ${asset_name} (entry missing:prerelease)"
      return 0
    fi
    echo "[infring install] asset verification failed: missing checksum entry for $asset_name" >&2
    return 1
  fi
  actual_digest="$(sha256_file "$asset_path" 2>/dev/null || true)"
  if [ -z "$actual_digest" ]; then
    echo "[infring install] asset verification failed: unable to hash $asset_name" >&2
    return 1
  fi
  if [ "$actual_digest" != "$expected_digest" ]; then
    echo "[infring install] asset verification failed: checksum mismatch for $asset_name" >&2
    echo "[infring install] expected: $expected_digest" >&2
    echo "[infring install] actual:   $actual_digest" >&2
    return 1
  fi
  install_summary_note "asset_verified: ${asset_name} sha256:${actual_digest}"
  record_verified_asset_digest "$version_tag" "$asset_name" "$actual_digest" "$asset_path"
  return 0
}

record_verified_asset_digest() {
  version_tag="$1"
  asset_name="$2"
  digest="$3"
  asset_path="$4"
  [ -n "$version_tag" ] || return 0
  [ -n "$asset_name" ] || return 0
  [ -n "$digest" ] || return 0
  lockfile="$INSTALL_ASSET_LOCKFILE"
  lockdir="$(dirname "$lockfile")"
  mkdir -p "$lockdir" >/dev/null 2>&1 || return 0
  ts="$(date -u '+%Y-%m-%dT%H:%M:%SZ' 2>/dev/null || true)"
  [ -n "$ts" ] || ts="unknown"
  tmp_file="${lockfile}.tmp"
  {
    printf '%s\n' "infring_install_asset_lock_v1"
    printf '%s\t%s\t%s\t%s\t%s\n' "$version_tag" "$asset_name" "$digest" "$ts" "$asset_path"
    if [ -f "$lockfile" ]; then
      awk -F '\t' -v v="$version_tag" -v a="$asset_name" '
        NR == 1 { next }
        NF < 3 { next }
        !($1 == v && $2 == a) { print }
      ' "$lockfile" 2>/dev/null || true
    fi
  } > "$tmp_file"
  mv "$tmp_file" "$lockfile" >/dev/null 2>&1 || return 0
  install_summary_note "asset_lockfile: ${lockfile}"
}

path_dir_writable_or_creatable() {
  candidate="$1"
  [ -n "$candidate" ] || return 1
  case "$candidate" in
    .) return 1 ;;
  esac
  if [ -d "$candidate" ]; then
    [ -w "$candidate" ] || return 1
    return 0
  fi
  mkdir -p "$candidate" 2>/dev/null || return 1
  [ -d "$candidate" ] || return 1
  [ -w "$candidate" ] || return 1
  return 0
}

first_writable_path_dir() {
  old_ifs="$IFS"
  IFS=':'
  for candidate in $PATH; do
    [ -n "$candidate" ] || continue
    if [ ! -d "$candidate" ] || [ ! -w "$candidate" ]; then
      continue
    fi
    printf '%s\n' "$candidate"
    IFS="$old_ifs"
    return 0
  done
  IFS="$old_ifs"
  return 1
}

path_contains_dir() {
  candidate="$1"
  [ -n "$candidate" ] || return 1
  case ":$PATH:" in
    *":$candidate:"*) return 0 ;;
    *) return 1 ;;
  esac
}

should_attempt_sudo_path_shims() {
  mode="$(printf '%s' "${INSTALL_SUDO_SHIMS:-}" | tr '[:upper:]' '[:lower:]')"
  case "$mode" in
    1|true|yes|on|auto) ;;
    *) return 1 ;;
  esac
  command -v sudo >/dev/null 2>&1 || return 1
  [ -t 0 ] || return 1
  [ -t 1 ] || return 1
  return 0
}

ensure_path_shims_via_sudo() {
  should_attempt_sudo_path_shims || return 1

  echo "[infring install] PATH fallback: attempting privileged shim install so commands work immediately from any terminal path"
  if ! sudo -v >/dev/null 2>&1; then
    echo "[infring install] sudo path shim skipped (authorization failed)"
    return 1
  fi

  created_any=0
  for shim_dir in /usr/local/bin /opt/homebrew/bin /usr/local/sbin; do
    path_contains_dir "$shim_dir" || continue
    [ "$shim_dir" = "$INSTALL_DIR" ] && continue
    sudo mkdir -p "$shim_dir" >/dev/null 2>&1 || continue

    for name in infring infringctl infringd; do
      target="$INSTALL_DIR/$name"
      shim="$shim_dir/$name"
      [ -e "$target" ] || continue
      if sudo test -e "$shim"; then
        if sudo test -L "$shim"; then
          link_target="$(sudo readlink "$shim" 2>/dev/null || true)"
          if [ "$link_target" = "$target" ]; then
            continue
          fi
        fi
        continue
      fi
      if sudo ln -s "$target" "$shim" >/dev/null 2>&1; then
        created_any=1
        continue
      fi
      if sudo cp "$target" "$shim" >/dev/null 2>&1; then
        sudo chmod 755 "$shim" >/dev/null 2>&1 || true
        created_any=1
      fi
    done

    if [ "$created_any" = "1" ]; then
      PATH_SHIM_DIR="$shim_dir"
      echo "[infring install] linked commands into PATH dir via sudo: $PATH_SHIM_DIR"
      return 0
    fi
  done

  return 1
}

resolve_install_dir_default() {
  if [ "$INSTALL_DIR_EXPLICIT" = "1" ]; then
    return 0
  fi
  preferred="$DEFAULT_INSTALL_DIR"
  if path_dir_writable_or_creatable "$preferred"; then
    INSTALL_DIR="$preferred"
    return 0
  fi
  fallback="$HOME/.local/bin"
  if path_dir_writable_or_creatable "$fallback"; then
    INSTALL_DIR="$fallback"
    return 0
  fi
  INSTALL_DIR="$preferred"
  return 0
}

ensure_path_shims() {
  path_contains_dir "$INSTALL_DIR" && return 0
  shim_dir="$(first_writable_path_dir 2>/dev/null || true)"
  if [ -z "$shim_dir" ]; then
    ensure_path_shims_via_sudo || true
    return 0
  fi
  [ "$shim_dir" = "$INSTALL_DIR" ] && return 0

  created_any=0
  for name in infring infringctl infringd; do
    target="$INSTALL_DIR/$name"
    shim="$shim_dir/$name"
    [ -e "$target" ] || continue
    if [ -e "$shim" ]; then
      if [ -L "$shim" ]; then
        link_target="$(readlink "$shim" || true)"
        if [ "$link_target" = "$target" ]; then
          continue
        fi
      fi
      continue
    fi
    if ln -s "$target" "$shim" 2>/dev/null; then
      created_any=1
      continue
    fi
    if cp "$target" "$shim" 2>/dev/null; then
      chmod 755 "$shim" 2>/dev/null || true
      created_any=1
    fi
  done
  if [ "$created_any" = "1" ]; then
    PATH_SHIM_DIR="$shim_dir"
    echo "[infring install] linked commands into PATH dir: $PATH_SHIM_DIR"
  else
    ensure_path_shims_via_sudo || true
  fi
}

shell_name_guess() {
  shell_path="${SHELL:-}"
  if [ -n "$shell_path" ]; then
    shell_name="$(basename "$shell_path" 2>/dev/null || true)"
    if [ -n "$shell_name" ]; then
      printf '%s\n' "$shell_name"
      return 0
    fi
  fi
  printf '%s\n' "sh"
}

path_persist_candidates() {
  shell_name="$1"
  case "$shell_name" in
    zsh)
      printf '%s\n' "$HOME/.zshenv"
      printf '%s\n' "$HOME/.zshrc"
      printf '%s\n' "$HOME/.zprofile"
      printf '%s\n' "$HOME/.profile"
      ;;
    bash)
      printf '%s\n' "$HOME/.bashrc"
      printf '%s\n' "$HOME/.bash_profile"
      printf '%s\n' "$HOME/.profile"
      ;;
    fish)
      printf '%s\n' "$HOME/.config/fish/config.fish"
      ;;
    *)
      printf '%s\n' "$HOME/.profile"
      ;;
  esac
}

select_path_persist_file() {
  shell_name="$1"
  candidates="$(path_persist_candidates "$shell_name")"
  first=""
  old_ifs="$IFS"
  IFS='
'
  for candidate in $candidates; do
    [ -n "$candidate" ] || continue
    if [ -z "$first" ]; then
      first="$candidate"
    fi
    if [ -f "$candidate" ]; then
      printf '%s\n' "$candidate"
      IFS="$old_ifs"
      return 0
    fi
  done
  IFS="$old_ifs"
  if [ -n "$first" ]; then
    printf '%s\n' "$first"
    return 0
  fi
  return 1
}

path_persist_kind_for_file() {
  file="$1"
  case "$file" in
    */config.fish) printf '%s\n' "fish" ;;
    *) printf '%s\n' "posix" ;;
  esac
}

strip_marker_block_from_file() {
  file="$1"
  marker_begin="$2"
  marker_end="$3"
  [ -f "$file" ] || return 0
  tmp="$(mktemp 2>/dev/null || true)"
  [ -n "$tmp" ] || return 1
  if ! awk -v begin="$marker_begin" -v end="$marker_end" '
    index($0, begin) { skip = 1; next }
    skip && index($0, end) { skip = 0; next }
    !skip { print }
  ' "$file" > "$tmp"; then
    rm -f "$tmp" >/dev/null 2>&1 || true
    return 1
  fi
  if ! cat "$tmp" > "$file"; then
    rm -f "$tmp" >/dev/null 2>&1 || true
    return 1
  fi
  rm -f "$tmp" >/dev/null 2>&1 || true
  return 0
}

append_path_block_posix() {
  file="$1"
  marker_begin="# >>> infring PATH >>>"
  marker_end="# <<< infring PATH <<<"
  if [ -f "$file" ] && grep -F "$marker_begin" "$file" >/dev/null 2>&1; then
    strip_marker_block_from_file "$file" "$marker_begin" "$marker_end" || return 1
  fi
  dir_name="$(dirname "$file")"
  mkdir -p "$dir_name"
  {
    printf '\n%s\n' "$marker_begin"
    printf '%s\n' "if [ -d \"$INSTALL_DIR\" ]; then"
    printf '%s\n' "  case \":\$PATH:\" in"
    printf '%s\n' "    *\":$INSTALL_DIR:\"*) ;;"
    printf '%s\n' "    *) export PATH=\"$INSTALL_DIR:\$PATH\" ;;"
    printf '%s\n' "  esac"
    printf '%s\n' "fi"
    printf '%s\n' "$marker_end"
  } >> "$file"
}

append_path_block_fish() {
  file="$1"
  marker_begin="# >>> infring PATH >>>"
  marker_end="# <<< infring PATH <<<"
  if [ -f "$file" ] && grep -F "$marker_begin" "$file" >/dev/null 2>&1; then
    strip_marker_block_from_file "$file" "$marker_begin" "$marker_end" || return 1
  fi
  dir_name="$(dirname "$file")"
  mkdir -p "$dir_name"
  {
    printf '\n%s\n' "$marker_begin"
    printf '%s\n' "if test -d \"$INSTALL_DIR\""
    printf '%s\n' "  if not contains -- \"$INSTALL_DIR\" \$PATH"
    printf '%s\n' "    set -gx PATH \"$INSTALL_DIR\" \$PATH"
    printf '%s\n' "  end"
    printf '%s\n' "end"
    printf '%s\n' "$marker_end"
  } >> "$file"
}

persist_path_block_to_file() {
  target_file="$1"
  [ -n "$target_file" ] || return 0
  persist_kind="$(path_persist_kind_for_file "$target_file")"
  if [ "$persist_kind" = "fish" ]; then
    append_path_block_fish "$target_file"
  else
    append_path_block_posix "$target_file"
  fi
}

append_path_persist_mirror() {
  entry="$1"
  [ -n "$entry" ] || return 0
  if [ -z "$PATH_PERSISTED_MIRRORS" ]; then
    PATH_PERSISTED_MIRRORS="$entry"
  else
    PATH_PERSISTED_MIRRORS="$PATH_PERSISTED_MIRRORS, $entry"
  fi
}

persist_path_to_additional_shell_files() {
  shell_name="$1"
  primary="$2"
  candidates="$(path_persist_candidates "$shell_name")"
  old_ifs="$IFS"
  IFS='
'
  for candidate in $candidates; do
    [ -n "$candidate" ] || continue
    [ "$candidate" = "$primary" ] && continue
    if [ -f "$candidate" ]; then
      persist_path_block_to_file "$candidate"
      append_path_persist_mirror "$candidate"
    fi
  done
  IFS="$old_ifs"
}

persist_path_for_shell() {
  path_contains_dir "$INSTALL_DIR" && return 0
  if [ -n "$PATH_SHIM_DIR" ]; then
    return 0
  fi
  shell_name="$(shell_name_guess)"
  target_file="$(select_path_persist_file "$shell_name" 2>/dev/null || true)"
  [ -n "$target_file" ] || return 0
  persist_path_block_to_file "$target_file"
  persist_kind="$(path_persist_kind_for_file "$target_file")"
  PATH_PERSISTED_FILE="$target_file"
  PATH_PERSISTED_KIND="$persist_kind"
  persist_path_to_additional_shell_files "$shell_name" "$target_file"
  echo "[infring install] PATH persisted in $PATH_PERSISTED_FILE"
  if [ -n "$PATH_PERSISTED_MIRRORS" ]; then
    echo "[infring install] PATH mirrored in: $PATH_PERSISTED_MIRRORS"
  fi
}

print_shell_activation_snippets() {
  activate_script="$INFRING_HOME/env.sh"
  if [ -n "$PATH_ACTIVATE_FILE" ]; then
    activate_script="$PATH_ACTIVATE_FILE"
  fi
  echo "[infring install] shell activation snippets:"
  echo "[infring install]   zsh:  . \"$activate_script\" && hash -r 2>/dev/null || true && infring --help"
  echo "[infring install]   bash: . \"$activate_script\" && hash -r 2>/dev/null || true && infring --help"
  echo "[infring install]   fish: set -gx PATH \"$INSTALL_DIR\" \$PATH; and command -q rehash; and rehash; and infring --help"
  echo "[infring install]   pwsh: \$env:Path = \"$INSTALL_DIR;\$env:Path\"; infring --help"
  echo "[infring install] shell troubleshooting snippets:"
  echo "[infring install]   zsh/bash: command -v infring || echo \$PATH"
  echo "[infring install]   fish: type -a infring; echo \$PATH"
  echo "[infring install]   pwsh: Get-Command infring -ErrorAction SilentlyContinue; \$env:Path"
}

write_path_activate_script() {
  activate_root="${INFRING_ACTIVATE_DIR:-$INFRING_HOME}"
  [ -n "$activate_root" ] || return 0
  mkdir -p "$activate_root"
  activate_file="$activate_root/env.sh"
  {
    printf '%s\n' "#!/usr/bin/env sh"
    printf '%s\n' "# Generated by Infring installer."
    printf '%s\n' "# Recovery hints:"
    printf '%s\n' "#   command -v infring || echo \$PATH"
    printf '%s\n' "#   . \"$INFRING_HOME/env.sh\" && hash -r 2>/dev/null || true"
    printf '%s\n' "#   cargo --version && rustc --version"
    printf '%s\n' "#   curl -fsSL https://raw.githubusercontent.com/protheuslabs/InfRing/main/install.sh | sh -s -- --full --install-node"
    printf '%s\n' "#   infring setup --yes --defaults"
    printf '%s\n' "#   infring setup status --json"
    printf '%s\n' "#   infring gateway status"
    printf '%s\n' "#   infring doctor --json"
    printf '%s\n' "export INFRING_HOME=\"$INFRING_HOME\""
    printf '%s\n' "export INFRING_WORKSPACE_ROOT=\"$WORKSPACE_DIR\""
    if node_bin_path="$(resolve_node_binary_path 2>/dev/null || true)"; then
      case "$node_bin_path" in
        "$INFRING_HOME"/*)
          printf '%s\n' "export INFRING_NODE_BINARY=\"$node_bin_path\""
          ;;
      esac
    fi
    printf '%s\n' "if [ -z \"\${INFRING_NODE_BINARY:-}\" ]; then"
    printf '%s\n' "  if [ -x \"$INFRING_HOME/node-runtime/bin/node\" ]; then"
    printf '%s\n' "    export INFRING_NODE_BINARY=\"$INFRING_HOME/node-runtime/bin/node\""
    printf '%s\n' "  elif [ -d \"$INFRING_HOME/node-runtime\" ]; then"
    printf '%s\n' "    node_candidate=\"\$(find \"$INFRING_HOME/node-runtime\" -maxdepth 4 -type f -name node 2>/dev/null | sort | head -n 1 || true)\""
    printf '%s\n' "    if [ -n \"\$node_candidate\" ] && [ -x \"\$node_candidate\" ]; then"
    printf '%s\n' "      export INFRING_NODE_BINARY=\"\$node_candidate\""
    printf '%s\n' "    fi"
    printf '%s\n' "  fi"
    printf '%s\n' "fi"
    printf '%s\n' "if [ -d \"$INSTALL_DIR\" ]; then"
    printf '%s\n' "  case \":\$PATH:\" in"
    printf '%s\n' "    *\":$INSTALL_DIR:\"*) ;;"
    printf '%s\n' "    *) export PATH=\"$INSTALL_DIR:\$PATH\" ;;"
    printf '%s\n' "  esac"
    printf '%s\n' "fi"
    printf '%s\n' "hash -r 2>/dev/null || true"
  } > "$activate_file"
  chmod 644 "$activate_file" 2>/dev/null || true
  PATH_ACTIVATE_FILE="$activate_file"
}

repair_artifact_healthy() {
  target="$1"
  [ -e "$target" ] || return 1
  if [ -d "$target" ]; then
    case "$(find "$target" -mindepth 1 -maxdepth 1 2>/dev/null | head -n 1 || true)" in
      '') return 1 ;;
      *) return 0 ;;
    esac
  fi
  [ -s "$target" ] || return 1
  case "$target" in
    */infring|*/infringctl|*/infringd|*/infring-ops|*/infringd-bin|*/conduit_daemon|*/infring-pure-workspace|*/infring-pure-workspace-tiny-max)
      [ -x "$target" ] || return 1
      ;;
  esac
  return 0
}

repair_install_dir() {
  ts="$(date -u +%Y%m%dT%H%M%SZ)"
  archive_root="$INSTALL_DIR/_repair_archive"
  archive_run="$archive_root/$ts"
  mkdir -p "$archive_run" >/dev/null 2>&1 || true
  repair_removed=0
  repair_preserved=0
  for name in \
    infring infringctl infringd \
    infring-ops infringd-bin conduit_daemon \
    infring-pure-workspace infring-pure-workspace-tiny-max \
    infring-client \
    infring.cmd infringctl.cmd infringd.cmd \
    infring.ps1 infringctl.ps1 infringd.ps1
  do
    target="$INSTALL_DIR/$name"
    [ -e "$target" ] || continue
    if repair_artifact_healthy "$target"; then
      if cp -R "$target" "$archive_run/$name" >/dev/null 2>&1; then
        echo "[infring install] repair archived healthy install artifact: $target"
      else
        echo "[infring install] repair warning: failed to archive healthy install artifact: $target"
      fi
      repair_preserved=$((repair_preserved + 1))
      echo "[infring install] repair preserved healthy install artifact: $target"
    else
      rm -rf "$target"
      repair_removed=$((repair_removed + 1))
      echo "[infring install] repair removed broken install artifact: $target"
    fi
  done
  echo "[infring install] repair summary: removed=$repair_removed preserved=$repair_preserved archive=$archive_run"
  install_summary_note "repair_summary: removed=${repair_removed} preserved=${repair_preserved} archive=${archive_run}"
}

resolve_workspace_root_for_repair() {
  for candidate in \
    "${WORKSPACE_DIR:-}" \
    "${INFRING_WORKSPACE_ROOT:-}" \
    "$(pwd)" \
    "$HOME/.infring/workspace" \
    "$HOME/.infring"
  do
    [ -n "$candidate" ] || continue
    if [ -d "$candidate/client/runtime" ] || [ -d "$candidate/infring-client/client/runtime" ]; then
      echo "$candidate"
      return 0
    fi
  done
  return 1
}

repair_workspace_state() {
  if ! workspace_root="$(resolve_workspace_root_for_repair)"; then
    echo "[infring install] repair skipped workspace cleanup (workspace root not detected)"
    return 0
  fi
  ts="$(date -u +%Y%m%dT%H%M%SZ)"
  archive_dir="$workspace_root/local/workspace/archive/install-repair"
  mkdir -p "$archive_dir"

  if [ -d "$workspace_root/local/workspace/memory" ]; then
    tar -czf "$archive_dir/memory-$ts.tgz" -C "$workspace_root/local/workspace" memory >/dev/null 2>&1 || true
    echo "[infring install] repair archived local/workspace/memory to $archive_dir/memory-$ts.tgz"
  fi
  if [ -d "$workspace_root/local/state" ]; then
    tar -czf "$archive_dir/state-$ts.tgz" -C "$workspace_root/local" state >/dev/null 2>&1 || true
    echo "[infring install] repair archived local/state to $archive_dir/state-$ts.tgz"
  fi

  for rel in client/runtime/local client/tmp core/local/tmp local/state; do
    abs="$workspace_root/$rel"
    if [ -e "$abs" ]; then
      rm -rf "$abs"
      echo "[infring install] repair removed stale runtime path: $rel"
    fi
  done
  mkdir -p "$workspace_root/local/state"
}

norm_os() {
  os="$(uname -s | tr '[:upper:]' '[:lower:]')"
  case "$os" in
    linux) echo "linux" ;;
    darwin) echo "darwin" ;;
    *)
      echo "[infring install] unsupported OS: $os" >&2
      exit 1
      ;;
  esac
}

norm_arch() {
  arch="$(uname -m)"
  case "$arch" in
    x86_64|amd64) echo "x86_64" ;;
    arm64|aarch64) echo "aarch64" ;;
    *)
      echo "[infring install] unsupported architecture: $arch" >&2
      exit 1
      ;;
  esac
}

platform_triple() {
  os="$(norm_os)"
  arch="$(norm_arch)"
  case "$os" in
    linux) echo "${arch}-unknown-linux-gnu" ;;
    darwin) echo "${arch}-apple-darwin" ;;
  esac
}

latest_version() {
  releases_url="https://api.github.com/repos/${REPO_OWNER}/${REPO_NAME}/releases?per_page=100"
  curl_fetch "$releases_url" \
    | sed -n 's/.*"tag_name"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' \
    | highest_semver_tag
}

latest_version_from_redirect() {
  final_url="$(curl_fetch -I -o /dev/null -w '%{url_effective}' "$LATEST_URL" || true)"
  case "$final_url" in
    */releases/tag/v*)
      printf '%s\n' "$final_url" | sed -n 's#.*\/releases\/tag\/\(v[^/?#]*\).*#\1#p' | head -n 1
      ;;
    *)
      ;;
  esac
}

latest_version_from_git_tags() {
  if ! command -v git >/dev/null 2>&1; then
    return 1
  fi
  repo_url="https://github.com/${REPO_OWNER}/${REPO_NAME}.git"
  git ls-remote --tags --refs "$repo_url" 2>/dev/null \
    | awk '{print $2}' \
    | highest_semver_tag
}

highest_semver_tag() {
  awk '
    function semver_key(tag,   body, core, pre, split_dash, n, parts, major, minor, patch, stable) {
      body = tag
      sub(/^v/, "", body)
      split(body, split_dash, "-")
      core = split_dash[1]
      pre = ""
      if (length(body) > length(core)) {
        pre = substr(body, length(core) + 2)
      }
      n = split(core, parts, ".")
      major = (n >= 1 && parts[1] ~ /^[0-9]+$/) ? parts[1] + 0 : 0
      minor = (n >= 2 && parts[2] ~ /^[0-9]+$/) ? parts[2] + 0 : 0
      patch = (n >= 3 && parts[3] ~ /^[0-9]+$/) ? parts[3] + 0 : 0
      stable = (pre == "") ? 1 : 0
      return sprintf("%09d.%09d.%09d.%01d.%s", major, minor, patch, stable, pre)
    }
    {
      tag = $0
      gsub(/^[[:space:]]+|[[:space:]]+$/, "", tag)
      sub(/^refs\/tags\//, "", tag)
      if (tag ~ /^v[0-9]+(\.[0-9]+){0,2}([-.][0-9A-Za-z][0-9A-Za-z._-]*)?$/) {
        printf "%s\t%s\n", semver_key(tag), tag
      }
    }
  ' | sort | tail -n 1 | awk -F '\t' '{print $2}'
}

normalize_version() {
  raw="$1"
  case "$raw" in
    v*) printf '%s\n' "$raw" ;;
    *) printf 'v%s\n' "$raw" ;;
  esac
}

resolve_version() {
  if [ "$REQUESTED_VERSION" != "latest" ]; then
    normalize_version "$REQUESTED_VERSION"
    return
  fi
  if is_truthy "$INSTALL_OFFLINE"; then
    echo "[infring install] offline mode requires an explicit release tag." >&2
    echo "[infring install] fix: rerun with INFRING_VERSION=vX.Y.Z (or --version via 'infring update')." >&2
    exit 1
  fi

  version="$(latest_version || true)"
  if [ -z "$version" ]; then
    version="$(latest_version_from_redirect || true)"
    if [ -n "$version" ]; then
      echo "[infring install] GitHub API unavailable; resolved latest tag via releases/latest redirect: $version" >&2
    fi
  fi
  if [ -z "$version" ]; then
    version="$(latest_version_from_git_tags || true)"
    if [ -n "$version" ]; then
      echo "[infring install] release API/redirect unavailable; resolved latest tag via git tags: $version" >&2
    fi
  fi
  if [ -z "$version" ]; then
    fallback="${INFRING_FALLBACK_VERSION:-}"
    if [ -n "$fallback" ]; then
      version="$(normalize_version "$fallback")"
      echo "[infring install] using fallback version: $version" >&2
    fi
  fi
  if [ -z "$version" ]; then
    echo "[infring install] failed to resolve latest release tag (GitHub API + releases/latest redirect + git tags)." >&2
    echo "[infring install] set INFRING_VERSION=vX.Y.Z and rerun installer." >&2
    exit 1
  fi
  echo "$version"
}

download_asset() {
  version_tag="$1"
  asset_name="$2"
  asset_out="$3"
  cache_dir="$INFRING_HOME/cache/install-assets/$version_tag"
  cache_file="$cache_dir/$asset_name"
  url="$BASE_URL/$version_tag/$asset_name"
  if is_truthy "$INSTALL_ASSET_CACHE" && [ -f "$cache_file" ]; then
    cp "$cache_file" "$asset_out"
    if verify_downloaded_asset "$version_tag" "$asset_name" "$asset_out"; then
      echo "[infring install] downloaded $asset_name (cache hit)"
      return 0
    fi
    rm -f "$asset_out" >/dev/null 2>&1 || true
    rm -f "$cache_file" >/dev/null 2>&1 || true
    if is_truthy "$INSTALL_OFFLINE"; then
      echo "[infring install] offline cache invalid for $asset_name; cannot refetch in offline mode." >&2
      echo "[infring install] fix: rerun once without --offline to refresh cache for $version_tag." >&2
      return 1
    fi
    echo "[infring install] cache invalid for $asset_name; refetching"
  fi
  if is_truthy "$INSTALL_OFFLINE"; then
    echo "[infring install] offline cache miss for $asset_name under $cache_dir" >&2
    echo "[infring install] fix: rerun once without --offline to hydrate cache for $version_tag." >&2
    return 1
  fi
  # TODO(rk): Consider adding retry logic with exponential backoff for transient network failures.
  # This would improve install reliability in CI environments and regions with intermittent connectivity.
  if curl_fetch "$url" -o "$asset_out"; then
    if ! verify_downloaded_asset "$version_tag" "$asset_name" "$asset_out"; then
      rm -f "$asset_out" >/dev/null 2>&1 || true
      return 1
    fi
    if is_truthy "$INSTALL_ASSET_CACHE"; then
      mkdir -p "$cache_dir" >/dev/null 2>&1 || true
      cp "$asset_out" "$cache_file" >/dev/null 2>&1 || true
    fi
    echo "[infring install] downloaded $asset_name"
    return 0
  fi
  return 1
}

download_bootstrap_asset() {
  asset_name="$1"
  asset_out="$2"
  if [ -z "${BOOTSTRAP_BASE_URL:-}" ]; then
    return 1
  fi
  if is_truthy "$INSTALL_OFFLINE"; then
    return 1
  fi
  if curl_fetch "${BOOTSTRAP_BASE_URL}/${asset_name}" -o "$asset_out"; then
    echo "[infring install] downloaded bootstrap fallback $asset_name"
    return 0
  fi
  return 1
}

source_fallback_bin_candidates() {
  stem_name="$1"
  case "$stem_name" in
    infring-ops)
      printf '%s\n' "infring-ops"
      # 'infring-ops' is deprecated and kept as a compatibility alias for legacy release bundles.
      printf '%s\n' "infring-ops"
      ;;
    infringd|infringd-tiny-max)
      printf '%s\n' "infringd"
      # 'infringd' is deprecated and kept as a compatibility alias for legacy release bundles.
      printf '%s\n' "infringd"
      ;;
    conduit_daemon)
      printf '%s\n' "conduit_daemon"
      ;;
    infring-pure-workspace|infring-pure-workspace-tiny-max)
      printf '%s\n' "infring-pure-workspace"
      printf '%s\n' "infring-pure-workspace"
      ;;
    *) return 1 ;;
  esac
}

fallback_triple_alias() {
  triple_id="$1"
  case "$triple_id" in
    x86_64-unknown-linux-gnu) echo "x86_64-unknown-linux-musl" ;;
    aarch64-unknown-linux-gnu) echo "aarch64-unknown-linux-musl" ;;
    *) return 1 ;;
  esac
}

ensure_source_build_prereqs() {
  if ensure_cargo_command_ready; then
    return 0
  fi

  echo "[infring install] cargo missing; bootstrapping rustup toolchain for source fallback"
  rustup_tmp="$(mktemp -d)"
  rustup_script="$rustup_tmp/rustup-init.sh"
  if ! curl -fsSL "$RUSTUP_INIT_URL" -o "$rustup_script"; then
    rm -rf "$rustup_tmp"
    print_rust_toolchain_recovery_hint
    return 1
  fi
  if ! sh "$rustup_script" -y --profile minimal --default-toolchain stable >/dev/null 2>&1; then
    rm -rf "$rustup_tmp"
    print_rust_toolchain_recovery_hint
    return 1
  fi
  rm -rf "$rustup_tmp"
  export PATH="$HOME/.cargo/bin:$PATH"
  if ensure_cargo_command_ready; then
    return 0
  fi
  if repair_rustup_default_toolchain; then
    export PATH="$HOME/.cargo/bin:$PATH"
    if ensure_cargo_command_ready; then
      return 0
    fi
  fi
  print_rust_toolchain_recovery_hint
  return 1
}

ensure_source_repo_fetch_prereqs() {
  if ! command -v curl >/dev/null 2>&1; then
    echo "[infring install] source fallback unavailable: missing required command 'curl'" >&2
    return 1
  fi
  if ! command -v tar >/dev/null 2>&1; then
    echo "[infring install] source fallback unavailable: missing required command 'tar'" >&2
    return 1
  fi
  return 0
}

prepare_source_fallback_repo() {
  version_tag="$1"
  if [ -n "$SOURCE_FALLBACK_DIR" ] && [ -d "$SOURCE_FALLBACK_DIR" ]; then
    return 0
  fi
  if ! ensure_source_repo_fetch_prereqs; then
    return 1
  fi

  SOURCE_FALLBACK_TMP="$(mktemp -d)"
  SOURCE_FALLBACK_DIR="$SOURCE_FALLBACK_TMP/repo"
  repo_url="https://github.com/${REPO_OWNER}/${REPO_NAME}.git"

  if command -v git >/dev/null 2>&1; then
    if git clone --depth 1 --branch "$version_tag" "$repo_url" "$SOURCE_FALLBACK_DIR" >/dev/null 2>&1; then
      return 0
    fi
    if git clone --depth 1 "$repo_url" "$SOURCE_FALLBACK_DIR" >/dev/null 2>&1; then
      return 0
    fi
  fi

  archive="$SOURCE_FALLBACK_TMP/source.tar.gz"
  archive_url="${SOURCE_ARCHIVE_BASE}/${version_tag}.tar.gz"
  if curl -fsSL "$archive_url" -o "$archive"; then
    if tar -xzf "$archive" -C "$SOURCE_FALLBACK_TMP"; then
      extracted_dir="$(find "$SOURCE_FALLBACK_TMP" -maxdepth 1 -type d -name "${REPO_NAME}-*" | head -n 1)"
      if [ -n "$extracted_dir" ] && [ -f "$extracted_dir/core/layer0/ops/Cargo.toml" ]; then
        SOURCE_FALLBACK_DIR="$extracted_dir"
        return 0
      fi
    fi
  fi

  archive_url="${SOURCE_ARCHIVE_BASE}/main.tar.gz"
  if curl -fsSL "$archive_url" -o "$archive"; then
    if tar -xzf "$archive" -C "$SOURCE_FALLBACK_TMP"; then
      extracted_dir="$(find "$SOURCE_FALLBACK_TMP" -maxdepth 1 -type d -name "${REPO_NAME}-*" | head -n 1)"
      if [ -n "$extracted_dir" ] && [ -f "$extracted_dir/core/layer0/ops/Cargo.toml" ]; then
        SOURCE_FALLBACK_DIR="$extracted_dir"
        return 0
      fi
    fi
  fi

  rm -rf "$SOURCE_FALLBACK_TMP"
  SOURCE_FALLBACK_TMP=""
  SOURCE_FALLBACK_DIR=""
  return 1
}

install_binary_from_source_fallback() {
  version_tag="$1"
  stem_name="$2"
  binary_out="$3"

  bin_candidates="$(source_fallback_bin_candidates "$stem_name" || true)"
  [ -n "$bin_candidates" ] || return 1

  prepare_source_fallback_repo "$version_tag" || return 1
  repo_dir="$SOURCE_FALLBACK_DIR"
  [ -n "$repo_dir" ] || return 1
  ensure_source_build_prereqs || return 1

  manifest="$repo_dir/core/layer0/ops/Cargo.toml"
  build_log="$(mktemp)"
  selected_bin=""
  compile_failure_seen=0
  compile_failure_candidate=""
  old_ifs="$IFS"
  IFS='
'
  for candidate_bin in $bin_candidates; do
    [ -n "$candidate_bin" ] || continue
    if cargo build --release --manifest-path "$manifest" --bin "$candidate_bin" >"$build_log" 2>&1; then
      selected_bin="$candidate_bin"
      break
    fi
    if grep -Eqi "no bin target named|is not a binary target" "$build_log"; then
      continue
    fi
    compile_failure_seen=1
    compile_failure_candidate="$candidate_bin"
    break
  done
  IFS="$old_ifs"

  if [ -z "$selected_bin" ]; then
    if [ "$compile_failure_seen" = "1" ]; then
      echo "[infring install] source fallback build failed while compiling '${compile_failure_candidate}' for ${stem_name}" >&2
      tail -n 20 "$build_log" >&2 || true
      if grep -Eqi "xcode-select: note: no developer tools|xcrun: error|clang: error|command line tools|linker .* not found|cc: command not found" "$build_log"; then
        echo "[infring install] detected missing C toolchain for Rust source fallback." >&2
        if [ "$(norm_os)" = "darwin" ]; then
          echo "[infring install] fix: run 'xcode-select --install', then rerun installer." >&2
        else
          echo "[infring install] fix: install a system C toolchain (gcc/clang + linker), then rerun installer." >&2
        fi
      fi
    else
      echo "[infring install] source fallback build failed: no compatible bin target for ${stem_name}" >&2
      echo "[infring install] tried source bin targets: $(printf '%s' "$bin_candidates" | tr '\n' ' ')" >&2
    fi
    rm -f "$build_log" >/dev/null 2>&1 || true
    return 1
  fi
  rm -f "$build_log" >/dev/null 2>&1 || true
  built="$repo_dir/target/release/$selected_bin"
  [ -f "$built" ] || return 1

  cp "$built" "$binary_out"
  finalize_installed_binary "$binary_out"
  echo "[infring install] built $selected_bin from source fallback"
  return 0
}

ops_binary_supports_gateway() {
  binary_path="$1"
  [ -x "$binary_path" ] || return 1
  return 1
}

ensure_ops_gateway_contract() {
  version_tag="$1" # reserved for forward-compatible policy checks
  binary_path="$2"
  if ops_binary_supports_gateway "$binary_path"; then
    return 0
  fi
  echo "[infring install] notice: core ops runtime does not expose 'gateway' directly; gateway is provided by control-surface wrappers when available"
  return 0
}

install_binary() {
  version_tag="$1"
  triple_id="$2"
  stem_name="$3"
  binary_out="$4"

  triples_to_try="$triple_id"
  if alias_triple="$(fallback_triple_alias "$triple_id" 2>/dev/null || true)"; then
    if [ -n "$alias_triple" ] && [ "$alias_triple" != "$triple_id" ]; then
      triples_to_try="$triples_to_try $alias_triple"
    fi
  fi

  for candidate_triple in $triples_to_try; do
    tmpdir="$(mktemp -d)"
    if download_asset "$version_tag" "${stem_name}-${candidate_triple}" "$tmpdir/$stem_name"; then
      mv "$tmpdir/$stem_name" "$binary_out"
      finalize_installed_binary "$binary_out"
      rm -rf "$tmpdir"
      return 0
    fi

    if download_asset "$version_tag" "${stem_name}-${candidate_triple}.bin" "$tmpdir/$stem_name"; then
      mv "$tmpdir/$stem_name" "$binary_out"
      finalize_installed_binary "$binary_out"
      rm -rf "$tmpdir"
      return 0
    fi

    if download_asset "$version_tag" "${stem_name}" "$tmpdir/$stem_name"; then
      mv "$tmpdir/$stem_name" "$binary_out"
      finalize_installed_binary "$binary_out"
      rm -rf "$tmpdir"
      return 0
    fi

    if download_asset "$version_tag" "${stem_name}.bin" "$tmpdir/$stem_name"; then
      mv "$tmpdir/$stem_name" "$binary_out"
      finalize_installed_binary "$binary_out"
      rm -rf "$tmpdir"
      return 0
    fi

    if download_asset "$version_tag" "${stem_name}-${candidate_triple}.tar.gz" "$tmpdir/${stem_name}.tar.gz"; then
      tar -xzf "$tmpdir/${stem_name}.tar.gz" -C "$tmpdir"
      if [ -f "$tmpdir/$stem_name" ]; then
        mv "$tmpdir/$stem_name" "$binary_out"
        finalize_installed_binary "$binary_out"
        rm -rf "$tmpdir"
        return 0
      fi
    fi

    if download_bootstrap_asset "${stem_name}-${candidate_triple}" "$tmpdir/$stem_name"; then
      mv "$tmpdir/$stem_name" "$binary_out"
      finalize_installed_binary "$binary_out"
      rm -rf "$tmpdir"
      return 0
    fi

    if download_bootstrap_asset "${stem_name}-${candidate_triple}.bin" "$tmpdir/$stem_name"; then
      mv "$tmpdir/$stem_name" "$binary_out"
      finalize_installed_binary "$binary_out"
      rm -rf "$tmpdir"
      return 0
    fi

    rm -rf "$tmpdir"
  done

  if install_binary_from_source_fallback "$version_tag" "$stem_name" "$binary_out"; then
    return 0
  fi
  return 1
}

install_client_bundle() {
  version_tag="$1"
  triple_id="$2"
  output_dir="$3"

  tmpdir="$(mktemp -d)"
  archive="$tmpdir/client-runtime.bundle"
  extract_dir="$tmpdir/extract"
  mkdir -p "$extract_dir"

  extract_bundle() {
    archive_path="$1"
    asset_name="${2:-$archive_path}"
    case "$asset_name" in
      *.tar.zst)
        if command -v unzstd >/dev/null 2>&1; then
          unzstd -c "$archive_path" | tar -xf - -C "$extract_dir"
          return $?
        fi
        if command -v zstd >/dev/null 2>&1; then
          zstd -dc "$archive_path" | tar -xf - -C "$extract_dir"
          return $?
        fi
        echo "[infring install] skipping .tar.zst bundle (zstd not installed); falling back to .tar.gz assets"
        return 1
        ;;
      *.tar.gz)
        tar -xzf "$archive_path" -C "$extract_dir"
        return $?
        ;;
      *)
        if tar -xzf "$archive_path" -C "$extract_dir" >/dev/null 2>&1; then
          return 0
        fi
        if command -v unzstd >/dev/null 2>&1; then
          unzstd -c "$archive_path" | tar -xf - -C "$extract_dir" >/dev/null 2>&1
          return $?
        fi
        if command -v zstd >/dev/null 2>&1; then
          zstd -dc "$archive_path" | tar -xf - -C "$extract_dir" >/dev/null 2>&1
          return $?
        fi
        return 1
        ;;
    esac
  }

  for asset in \
    "infring-client-runtime-${triple_id}.tar.zst" \
    "infring-client-runtime.tar.zst" \
    "infring-client-${triple_id}.tar.zst" \
    "infring-client.tar.zst" \
    "infring-client-runtime-${triple_id}.tar.gz" \
    "infring-client-runtime.tar.gz" \
    "infring-client-${triple_id}.tar.gz" \
    "infring-client.tar.gz"
  do
    if download_asset "$version_tag" "$asset" "$archive"; then
      rm -rf "$extract_dir"
      mkdir -p "$extract_dir"
      if extract_bundle "$archive" "$asset"; then
        runtime_root="$(workspace_runtime_root "$extract_dir" 2>/dev/null || true)"
        if [ -n "$runtime_root" ]; then
          install_workspace_tree_from_dir "$runtime_root" "$output_dir" || {
            rm -rf "$tmpdir"
            return 1
          }
          if workspace_has_runtime "$output_dir"; then
            rm -rf "$tmpdir"
            echo "[infring install] installed optional client runtime bundle"
            return 0
          fi
        fi
        echo "[infring install] ignored invalid client runtime bundle asset: $asset"
      fi
    fi
  done

  rm -rf "$tmpdir"
  return 1
}

workspace_has_runtime_dirs() {
  workspace="$1"
  [ -d "$workspace/client/runtime" ] && [ -d "$workspace/client/runtime/config" ]
}

workspace_has_tier1_runtime() {
  workspace="$1"
  workspace_has_runtime_dirs "$workspace" || return 1
  manifest_path="$workspace/$RUNTIME_MANIFEST_REL"
  [ -f "$manifest_path" ] || return 1

  while IFS= read -r row || [ -n "$row" ]; do
    rel="$(printf '%s' "$row" | sed 's/^[[:space:]]*//;s/[[:space:]]*$//')"
    [ -n "$rel" ] || continue
    case "$rel" in
      \#*) continue ;;
    esac
    if ! workspace_runtime_entrypoint_exists "$workspace" "$rel"; then
      return 1
    fi
  done < "$manifest_path"
  return 0
}

workspace_has_runtime() {
  workspace="$1"
  workspace_has_tier1_runtime "$workspace"
}

workspace_has_xtask_member() {
  workspace="$1"
  [ -f "$workspace/Cargo.toml" ] || return 0
  if ! grep -Eq "\"xtask\"" "$workspace/Cargo.toml"; then
    return 0
  fi
  [ -f "$workspace/xtask/Cargo.toml" ]
}

ensure_workspace_source_member_closure() {
  version_tag="$1"
  workspace="$2"
  if workspace_has_xtask_member "$workspace"; then
    return 0
  fi
  echo "[infring install] workspace source closure missing (xtask); attempting source fallback refresh"
  if install_workspace_from_source_fallback "$version_tag" "$workspace" && workspace_has_xtask_member "$workspace"; then
    echo "[infring install] workspace source closure repaired (xtask restored)"
    return 0
  fi
  echo "[infring install] workspace source closure repair failed (xtask missing)" >&2
  return 1
}

workspace_runtime_root() {
  base="$1"
  for candidate in \
    "$base" \
    "$base/workspace" \
    "$base/infring-client" \
    "$base/infring-workspace"
  do
    [ -n "$candidate" ] || continue
    if workspace_has_runtime "$candidate"; then
      printf '%s\n' "$candidate"
      return 0
    fi
  done
  return 1
}

workspace_release_tag_path() {
  workspace="$1"
  printf '%s\n' "$workspace/local/state/ops/install_release_tag.txt"
}

workspace_release_meta_path() {
  workspace="$1"
  printf '%s\n' "$workspace/local/state/ops/install_release_meta.json"
}

read_workspace_release_tag() {
  workspace="$1"
  marker_path="$(workspace_release_tag_path "$workspace")"
  [ -f "$marker_path" ] || return 1
  installed_tag="$(head -n 1 "$marker_path" 2>/dev/null | tr -d '\r' | sed 's/[[:space:]]*$//')"
  [ -n "$installed_tag" ] || return 1
  printf '%s\n' "$installed_tag"
  return 0
}

workspace_release_tag_matches() {
  workspace="$1"
  expected_tag="$2"
  installed_tag="$(read_workspace_release_tag "$workspace" 2>/dev/null || true)"
  [ -n "$installed_tag" ] || return 1
  [ "$installed_tag" = "$expected_tag" ]
}

write_workspace_release_tag() {
  workspace="$1"
  release_tag="$2"
  marker_path="$(workspace_release_tag_path "$workspace")"
  meta_path="$(workspace_release_meta_path "$workspace")"
  normalized_release="$(printf '%s' "$release_tag" | sed 's/^[Vv]//')"
  installed_at="$(date -u '+%Y-%m-%dT%H:%M:%SZ' 2>/dev/null || true)"
  mkdir -p "$(dirname "$marker_path")" || return 1
  printf '%s\n' "$release_tag" > "$marker_path" || return 1
  cat > "$meta_path" <<EOF || return 1
{
  "release_tag": "$release_tag",
  "release_version_normalized": "$normalized_release",
  "install_source": "install.sh",
  "installed_at": "$installed_at"
}
EOF
  verified_tag="$(read_workspace_release_tag "$workspace" 2>/dev/null || true)"
  [ "$verified_tag" = "$release_tag" ] || return 1
  return 0
}

clear_managed_workspace_paths() {
  workspace="$1"
  for rel in \
    client \
    core \
    adapters \
    docs \
    tests \
    xtask \
    Cargo.toml \
    Cargo.lock \
    package.json \
    package-lock.json \
    tsconfig.json \
    tsconfig.base.json \
    tsconfig.build.json \
    tsconfig.runtime.json \
    AGENTS.md \
    README.md \
    SECURITY.md \
    verify.sh
  do
    abs="$workspace/$rel"
    [ -e "$abs" ] || continue
    rm -rf "$abs" || return 1
  done
  return 0
}

install_workspace_tree_from_dir() {
  source_dir="$1"
  output_dir="$2"
  mkdir -p "$output_dir" || return 1
  clear_managed_workspace_paths "$output_dir" || return 1
  (cd "$source_dir" && tar -cf - .) | (cd "$output_dir" && tar -xf -) || return 1
  return 0
}

install_workspace_from_source_fallback() {
  version_tag="$1"
  output_dir="$2"
  prepare_source_fallback_repo "$version_tag" || return 1
  repo_dir="$SOURCE_FALLBACK_DIR"
  [ -n "$repo_dir" ] || return 1
  [ -d "$repo_dir/client/runtime" ] || return 1

  tmpdir="$(mktemp -d)"
  staged="$tmpdir/staged"
  mkdir -p "$staged"
  for rel in \
    client \
    core \
    adapters \
    docs \
    tests \
    xtask \
    Cargo.toml \
    Cargo.lock \
    package.json \
    package-lock.json \
    tsconfig.json \
    tsconfig.base.json \
    tsconfig.build.json \
    tsconfig.runtime.json \
    AGENTS.md \
    README.md \
    SECURITY.md \
    verify.sh
  do
    [ -e "$repo_dir/$rel" ] || continue
    (cd "$repo_dir" && tar -cf - "$rel") | (cd "$staged" && tar -xf -) || {
      rm -rf "$tmpdir"
      return 1
    }
  done

  install_workspace_tree_from_dir "$staged" "$output_dir" || {
    rm -rf "$tmpdir"
    return 1
  }
  mkdir -p "$output_dir/local/state" "$output_dir/local/workspace/memory" "$output_dir/local/workspace/assistant"
  if ! workspace_has_runtime "$output_dir"; then
    rm -rf "$tmpdir"
    echo "[infring install] source fallback runtime contract check failed; workspace is incomplete" >&2
    return 1
  fi
  rm -rf "$tmpdir"
  echo "[infring install] installed workspace runtime from source fallback"
  return 0
}

ensure_workspace_setup_wizard_compat() {
  workspace="$1"
  [ -n "$workspace" ] || return 0
  ops_dir="$workspace/client/runtime/systems/ops"
  shim_path="$ops_dir/infring_setup_wizard.js"
  [ -d "$ops_dir" ] || mkdir -p "$ops_dir"
  if [ -f "$shim_path" ]; then
    return 0
  fi
  cat > "$shim_path" <<'__INFRING_SETUP_SHIM__'
#!/usr/bin/env node
'use strict';
const fs = require('fs');
const path = require('path');
const { spawnSync } = require('child_process');
const ROOT = path.resolve(__dirname, '..', '..', '..', '..');
const TS_ENTRY = path.join(ROOT, 'client', 'runtime', 'lib', 'ts_entrypoint.ts');
const TS_TARGET = path.join(__dirname, 'infring_setup_wizard.ts');
const STATE_PATH = path.join(ROOT, 'local', 'state', 'ops', 'infring_setup_wizard', 'latest.json');

if (fs.existsSync(TS_ENTRY) && fs.existsSync(TS_TARGET)) {
  const out = spawnSync(process.execPath, [TS_ENTRY, TS_TARGET, ...process.argv.slice(2)], { stdio: 'inherit', cwd: ROOT });
  process.exit(Number.isFinite(out.status) ? out.status : 1);
}

const payload = {
  type: 'infring_setup_wizard_state',
  completed: true,
  completed_at: new Date().toISOString(),
  completion_mode: 'install_shim_fallback',
  node_runtime_detected: true,
  interaction_style: 'silent',
  notifications: 'none',
  covenant_acknowledged: false,
  version: 1
};
fs.mkdirSync(path.dirname(STATE_PATH), { recursive: true });
fs.writeFileSync(STATE_PATH, `${JSON.stringify(payload, null, 2)}\n`, 'utf8');
const jsonMode = process.argv.slice(2).some((arg) => {
  const token = String(arg || '').trim().toLowerCase();
  return token === '--json' || token === '--json=1';
});
if (jsonMode) {
  process.stdout.write(`${JSON.stringify({ ok: true, type: 'infring_setup_wizard_fallback', mode: 'install_shim_fallback', state_path: STATE_PATH, state: payload })}\n`);
} else {
  process.stdout.write('[infring setup] compatibility fallback completed\n');
}
process.exit(0);
__INFRING_SETUP_SHIM__
  chmod 755 "$shim_path" 2>/dev/null || true
  echo "[infring install] installed setup wizard compatibility shim"
  return 0
}

runtime_entrypoint_exists_for_mode() {
  runtime_root="$1"
  rel="$2"
  runtime_mode="${3:-source}"
  [ -f "$runtime_root/$rel" ] && return 0
  if [ "$runtime_mode" != "source" ]; then
    return 1
  fi
  case "$rel" in
    *.js)
      ts_rel="${rel%.js}.ts"
      [ -f "$runtime_root/$ts_rel" ] && return 0
      ;;
    *.ts)
      js_rel="${rel%.ts}.js"
      [ -f "$runtime_root/$js_rel" ] && return 0
      ;;
  esac
  return 1
}

verify_runtime_contract_for_mode() {
  runtime_root="$1"
  runtime_mode="$2"
  context_label="$3"
  [ -n "$runtime_root" ] || return 1
  [ -n "$runtime_mode" ] || runtime_mode="source"
  [ -n "$context_label" ] || context_label="runtime"
  manifest_rel="${RUNTIME_MANIFEST_REL:-client/runtime/config/install_runtime_manifest_v1.txt}"
  manifest_path="$runtime_root/$manifest_rel"
  if [ ! -f "$manifest_path" ]; then
    echo "[infring install] runtime integrity check failed (${context_label}): manifest missing" >&2
    echo "[infring install] missing: $manifest_rel" >&2
    echo "[infring install] fix: publish runtime bundle/source fallback with manifest and rerun install." >&2
    return 1
  fi
  missing_manifest_entries=""
  for required_rel in $RUNTIME_TIER1_REQUIRED_ENTRYPOINTS; do
    if ! awk -v target="$required_rel" '
      {
        gsub(/^[[:space:]]+|[[:space:]]+$/, "", $0);
        if ($0 == "" || substr($0,1,1) == "#") next;
        if ($0 == target) { found = 1; exit 0; }
      }
      END { exit(found ? 0 : 1); }
    ' "$manifest_path"; then
      missing_manifest_entries="${missing_manifest_entries}${required_rel}\n"
    fi
  done
  if [ -n "$missing_manifest_entries" ]; then
    echo "[infring install] runtime integrity check failed (${context_label} mode=${runtime_mode}): manifest missing required Tier-1 runtime entries" >&2
    printf '%b' "$missing_manifest_entries" | while IFS= read -r row; do
      [ -n "$row" ] || continue
      echo "[infring install] manifest-missing: $row" >&2
    done
    echo "[infring install] manifest: $manifest_rel" >&2
    echo "[infring install] fix: refresh install runtime bundle/source fallback so Tier-1 runtime entries are declared." >&2
    return 1
  fi
  missing_list=""
  while IFS= read -r row || [ -n "$row" ]; do
    rel="$(printf '%s' "$row" | sed 's/^[[:space:]]*//;s/[[:space:]]*$//')"
    [ -n "$rel" ] || continue
    case "$rel" in
      \#*) continue ;;
    esac
    if ! runtime_entrypoint_exists_for_mode "$runtime_root" "$rel" "$runtime_mode"; then
      missing_list="${missing_list}${rel}\n"
    fi
  done < "$manifest_path"
  if [ -n "$missing_list" ]; then
    echo "[infring install] runtime integrity check failed (${context_label} mode=${runtime_mode}): required command entrypoints are missing" >&2
    printf '%b' "$missing_list" | while IFS= read -r row; do
      [ -n "$row" ] || continue
      echo "[infring install] missing: $row" >&2
    done
    echo "[infring install] manifest: $manifest_rel" >&2
    echo "[infring install] fix: rerun with --full after publishing a complete runtime bundle, or set INFRING_VERSION to a release with complete runtime assets." >&2
    return 1
  fi
  echo "[infring install] runtime integrity check: manifest verified ($manifest_rel) [${context_label} mode=${runtime_mode}]"
  return 0
}

runtime_required_node_modules() {
  runtime_root="$1"
  [ -n "$runtime_root" ] || return 1
  manifest_rel="${RUNTIME_NODE_MODULE_MANIFEST_REL:-client/runtime/config/install_runtime_node_modules_v1.txt}"
  manifest_path="$runtime_root/$manifest_rel"
  if [ ! -f "$manifest_path" ]; then
    echo "[infring install] node module closure failed: dependency manifest missing ($manifest_rel)" >&2
    return 1
  fi
  modules="$(
    awk '
      {
        gsub(/^[[:space:]]+|[[:space:]]+$/, "", $0);
        if ($0 == "" || substr($0,1,1) == "#") next;
        print $0;
      }
    ' "$manifest_path" | tr '\n' ' '
  )"
  if [ -z "$modules" ]; then
    modules="${RUNTIME_NODE_REQUIRED_MODULES:-typescript ws}"
  fi
  printf '%s\n' "$modules"
  return 0
}

verify_workspace_runtime_contract() {
  workspace="$1"
  [ -n "$workspace" ] || return 1
  if verify_runtime_contract_for_mode "$workspace" "source" "workspace_runtime"; then
    INSTALL_RUNTIME_CONTRACT_MODE="source"
    INSTALL_RUNTIME_CONTRACT_OK=1
    return 0
  fi
  return 1
}

repair_workspace_runtime_contract() {
  version_tag="$1"
  triple_id="$2"
  workspace="$3"
  [ -n "$workspace" ] || return 1
  echo "[infring install] attempting runtime self-heal for missing manifest entrypoints"
  if install_client_bundle "$version_tag" "$triple_id" "$workspace"; then
    ensure_workspace_setup_wizard_compat "$workspace" || true
    if verify_workspace_runtime_contract "$workspace"; then
      echo "[infring install] runtime self-heal succeeded via runtime bundle refresh"
      return 0
    fi
  fi
  if install_workspace_from_source_fallback "$version_tag" "$workspace"; then
    ensure_workspace_setup_wizard_compat "$workspace" || true
    if verify_workspace_runtime_contract "$workspace"; then
      echo "[infring install] runtime self-heal succeeded via source fallback refresh"
      return 0
    fi
  fi
  echo "[infring install] runtime self-heal failed; runtime contract is still incomplete" >&2
  return 1
}

force_workspace_runtime_mode_source() {
  workspace="$1"
  [ -n "$workspace" ] || return 1
  mode_state_path="$workspace/local/state/ops/runtime_mode.json"
  mkdir -p "$(dirname "$mode_state_path")" >/dev/null 2>&1 || return 1
  cat > "$mode_state_path" <<'__INFRING_RUNTIME_MODE__'
{
  "mode": "source",
  "set_by": "install.sh"
}
__INFRING_RUNTIME_MODE__
  echo "[infring install] runtime mode pinned: source"
  return 0
}

normalize_install_shell_default() {
  raw="$(printf '%s' "${1:-}" | tr 'A-Z' 'a-z' | tr -d "\"'" | sed 's/^[[:space:]]*//;s/[[:space:]]*$//')"
  case "$raw" in
    ui|browser|dashboard)
      printf '%s\n' "ui"
      ;;
    ui-v2|browser-v2|dashboard-v2|shell-v2)
      printf '%s\n' "ui-v2"
      ;;
    legacy-ui|legacy|legacy-browser)
      printf '%s\n' "legacy-ui"
      ;;
    terminal|cli|tty)
      printf '%s\n' "terminal"
      ;;
    none|off|disabled|0)
      printf '%s\n' "none"
      ;;
    *)
      return 1
      ;;
  esac
}

infer_install_shell_default() {
  if [ -n "${INFRING_GATEWAY_DEFAULT_SHELL:-}" ]; then
    normalize_install_shell_default "$INFRING_GATEWAY_DEFAULT_SHELL" && return 0
  fi
  if is_truthy "$INSTALL_PURE" || is_truthy "$INSTALL_TINY_MAX"; then
    printf '%s\n' "terminal"
    return 0
  fi
  case "$(uname -s 2>/dev/null || printf '')" in
    Darwin)
      printf '%s\n' "ui"
      return 0
      ;;
  esac
  if [ -n "${DISPLAY:-}" ] || [ -n "${WAYLAND_DISPLAY:-}" ]; then
    printf '%s\n' "ui"
  else
    printf '%s\n' "terminal"
  fi
}

write_shell_launch_config() {
  workspace="$1"
  default_shell="$(normalize_install_shell_default "${2:-ui}" 2>/dev/null || printf '%s\n' "ui")"
  fallback_shell="$(normalize_install_shell_default "${3:-terminal}" 2>/dev/null || printf '%s\n' "terminal")"
  home_config_dir="$INFRING_HOME/config"
  workspace_config_dir="$workspace/local/state/ops"
  mkdir -p "$home_config_dir" "$workspace_config_dir" >/dev/null 2>&1 || return 1
  for config_path in "$home_config_dir/shell.env" "$workspace_config_dir/shell_launch.env"; do
    cat > "$config_path" <<__INFRING_SHELL_CONFIG__
INFRING_GATEWAY_DEFAULT_SHELL=${default_shell}
INFRING_GATEWAY_FALLBACK_SHELL=${fallback_shell}
INFRING_GATEWAY_LAUNCH_ON_START=1
__INFRING_SHELL_CONFIG__
  done
  echo "[infring install] shell default: ${default_shell} (override with: infring gateway --shell=terminal)"
  return 0
}

ensure_runtime_node_module_closure() {
  workspace="$1"
  [ -n "$workspace" ] || return 1
  required_modules="$(runtime_required_node_modules "$workspace" 2>/dev/null || true)"
  if [ -z "$required_modules" ]; then
    echo "[infring install] node module closure failed: required-module list is empty" >&2
    return 1
  fi
  node_bin_path="$(resolve_node_binary_path 2>/dev/null || true)"
  if ! node_runtime_meets_minimum; then
    echo "[infring install] node module closure skipped (node runtime unavailable)"
    return 0
  fi

  missing_modules=""
  for module_name in $required_modules; do
    if ! runtime_module_resolvable "$workspace" "$module_name"; then
      missing_modules="${missing_modules} ${module_name}"
    fi
  done

  if [ -z "$missing_modules" ]; then
    echo "[infring install] node module closure: satisfied"
    return 0
  fi

  npm_bin_path="$(resolve_npm_binary_path 2>/dev/null || true)"
  if [ -z "$node_bin_path" ] || [ -z "$npm_bin_path" ]; then
    echo "[infring install] node module closure failed: npm unavailable" >&2
    echo "[infring install] missing modules:${missing_modules}" >&2
    return 1
  fi
  if [ ! -f "$workspace/package.json" ]; then
    echo "[infring install] node module closure failed: package.json missing in workspace" >&2
    echo "[infring install] missing modules:${missing_modules}" >&2
    return 1
  fi

  echo "[infring install] installing runtime node module closure:${missing_modules}"
  npm_cmd_dir="$(dirname "$npm_bin_path")"
  node_cmd_dir="$(dirname "$node_bin_path")"
  # npm entrypoint scripts are node-shebang wrappers; force PATH so `env node` resolves
  # the resolved installer node binary.
  if ! (
    cd "$workspace" >/dev/null 2>&1 && \
    INFRING_NODE_BINARY="$node_bin_path" \
    PATH="$node_cmd_dir:$npm_cmd_dir:$PATH" "$npm_bin_path" install --silent --no-audit --no-fund --no-save $missing_modules
  ); then
    echo "[infring install] node module closure install failed" >&2
    return 1
  fi

  still_missing=""
  for module_name in $required_modules; do
    if ! runtime_module_resolvable "$workspace" "$module_name"; then
      still_missing="${still_missing} ${module_name}"
    fi
  done
  if [ -n "$still_missing" ]; then
    echo "[infring install] node module closure verification failed:${still_missing}" >&2
    return 1
  fi

  echo "[infring install] node module closure: installed and verified"
  return 0
}

run_post_install_smoke_command() {
  smoke_dir="$1"
  label="$2"
  shift 2 || true
  log="$smoke_dir/$label.log"
  if "$@" >"$log" 2>&1; then
    echo "[infring install] smoke $label: ok"
    return 0
  fi
  case "$label" in
    infringctl_help)
      if grep -Eq "could not choose a version of cargo to run|no default is configured|run 'rustup default stable'|spawnSync cargo ENOENT|command not found: cargo|No such file or directory.*cargo" "$log"; then
        echo "[infring install] smoke $label: skipped (missing cargo toolchain/runtime)"
        return 0
      fi
      ;;
  esac
  echo "[infring install] smoke $label: failed" >&2
  cat "$log" >&2 || true
  return 1
}

append_install_smoke_record() {
  records_file="$1"
  check_name="$2"
  command_desc="$3"
  required_flag="$4"
  ok_flag="$5"
  status_label="$6"
  log_path="$7"
  error_code="${8:-}"
  printf '{"name":"%s","command":"%s","required":%s,"ok":%s,"status":"%s","error_code":"%s","log_path":"%s"}\n' \
    "$(install_json_escape "$check_name")" \
    "$(install_json_escape "$command_desc")" \
    "$required_flag" \
    "$ok_flag" \
    "$(install_json_escape "$status_label")" \
    "$(install_json_escape "$error_code")" \
    "$(install_json_escape "$log_path")" >> "$records_file"
}

write_install_smoke_summary_json() {
  records_file="$1"
  output_path="$2"
  required_failed_count="$3"
  install_dir="$4"
  workspace_root="$5"
  checks_json=""
  while IFS= read -r row || [ -n "$row" ]; do
    [ -n "$row" ] || continue
    if [ -z "$checks_json" ]; then
      checks_json="$row"
    else
      checks_json="${checks_json},${row}"
    fi
  done < "$records_file"
  [ -n "$checks_json" ] || checks_json=""
  output_dir="$(dirname "$output_path")"
  mkdir -p "$output_dir" >/dev/null 2>&1 || true
  tmp_output="$(mktemp)"
  if [ "$required_failed_count" -eq 0 ]; then
    summary_ok="true"
  else
    summary_ok="false"
  fi
  cat > "$tmp_output" <<EOF
{"ok":${summary_ok},"type":"infring_install_smoke_summary","required_failed_count":${required_failed_count},"toolchain_policy":"$(install_json_escape "$INSTALL_TOOLCHAIN_POLICY")","install_dir":"$(install_json_escape "$install_dir")","workspace_root":"$(install_json_escape "$workspace_root")","checks":[${checks_json}]}
EOF
  mv "$tmp_output" "$output_path"
  echo "[infring install] smoke summary json: $output_path"
}

rustup_default_toolchain_missing() {
  if ! command -v rustup >/dev/null 2>&1; then
    return 1
  fi
  if command -v cargo >/dev/null 2>&1; then
    if cargo --version >/dev/null 2>&1; then
      return 1
    fi
    return 0
  fi
  if rustup default >/dev/null 2>&1; then
    return 1
  fi
  return 0
}

run_dashboard_health_smoke() {
  smoke_dir="$1"
  install_dir="$2"
  host="${3:-127.0.0.1}"
  port="${4:-4173}"
  log="$smoke_dir/dashboard_health.log"
  [ -x "$install_dir/infring" ] || return 1

  dashboard_smoke_root="${INFRING_WORKSPACE_ROOT:-}"
  if [ -z "$dashboard_smoke_root" ] || [ ! -d "$dashboard_smoke_root" ]; then
    for candidate in "$HOME/.infring/workspace" "$HOME/.infring/workspace"; do
      if [ -d "$candidate" ]; then
        dashboard_smoke_root="$candidate"
        break
      fi
    done
  fi

  print_dashboard_smoke_failure_logs() {
    state_dir=""
    if [ -n "$dashboard_smoke_root" ] && [ -d "$dashboard_smoke_root" ]; then
      state_dir="$dashboard_smoke_root/local/state/ops/daemon_control"
    fi
    for log_name in dashboard_ui.log dashboard_watchdog.log; do
      log_path="$state_dir/$log_name"
      if [ -f "$log_path" ]; then
        echo "[infring install] tail $log_path" >&2
        tail -n 80 "$log_path" >&2 || true
      fi
    done
  }

  run_command_with_timeout 30 env INFRING_DASHBOARD_LAUNCHD=0 INFRING_NO_BROWSER=1 \
    "$install_dir/infring" gateway stop \
    "--dashboard-host=${host}" "--dashboard-port=${port}" >/dev/null 2>&1 || true

  if run_command_with_timeout 90 env INFRING_DASHBOARD_LAUNCHD=0 INFRING_DASHBOARD_WAIT_MAX=30 INFRING_NO_BROWSER=1 \
    "$install_dir/infring" gateway start \
    "--dashboard-host=${host}" "--dashboard-port=${port}" \
    "--dashboard-open=0" "--gateway-persist=0" >"$log" 2>&1; then
    :
  else
    start_status=$?
    run_command_with_timeout 30 env INFRING_DASHBOARD_LAUNCHD=0 \
      "$install_dir/infring" gateway stop \
      "--dashboard-host=${host}" "--dashboard-port=${port}" >/dev/null 2>&1 || true
    if [ "$start_status" = "124" ]; then
      echo "[infring install] smoke dashboard_health: failed (gateway start timeout)" >&2
    else
      echo "[infring install] smoke dashboard_health: failed (gateway start)" >&2
    fi
    cat "$log" >&2 || true
    print_dashboard_smoke_failure_logs
    return 1
  fi

  ready=0
  i=0
  while [ "$i" -lt 45 ]; do
    if curl -fsS --max-time 2 "http://${host}:${port}/healthz" >/dev/null 2>&1; then
      ready=1
      break
    fi
    sleep 1
    i=$((i + 1))
  done

  run_command_with_timeout 30 env INFRING_DASHBOARD_LAUNCHD=0 \
    "$install_dir/infring" gateway stop \
    "--dashboard-host=${host}" "--dashboard-port=${port}" >/dev/null 2>&1 || true

  if [ "$ready" != "1" ]; then
    echo "[infring install] smoke dashboard_health: failed (healthz timeout)" >&2
    cat "$log" >&2 || true
    print_dashboard_smoke_failure_logs
    return 1
  fi
  echo "[infring install] smoke dashboard_health: ok"
  return 0
}

run_model_readiness_smoke() {
  smoke_dir="$1"
  require_ready="${2:-0}"
  log="$smoke_dir/model_readiness.log"
  status=0
  (
    if ! command -v ollama >/dev/null 2>&1; then
      echo "ollama_missing"
      exit 20
    fi
    if ! start_ollama_runtime_best_effort; then
      echo "ollama_offline"
      exit 21
    fi
    before_count="$(ollama_model_count)"
    echo "model_count_before:${before_count}"
    if [ "$before_count" -lt 1 ]; then
      if ! ensure_ollama_starter_model; then
        echo "starter_pull_failed"
        exit 22
      fi
    fi
    after_count="$(ollama_model_count)"
    echo "model_count_after:${after_count}"
    if [ "$after_count" -lt 1 ]; then
      echo "no_models_detected"
      exit 23
    fi
    exit 0
  ) >"$log" 2>&1 || status="$?"
  if [ "$status" = "0" ]; then
    detected_count="$(ollama_model_count)"
    echo "[infring install] smoke model_readiness: ok (${detected_count} local model(s))"
    OLLAMA_LAST_MODEL_COUNT="$detected_count"
    return 0
  fi
  case "$status" in
    20)
      if [ "$require_ready" = "1" ]; then
        echo "[infring install] smoke model_readiness: failed (ollama missing)" >&2
        cat "$log" >&2 || true
        return 1
      fi
      echo "[infring install] smoke model_readiness: skipped (ollama missing)"
      return 0
      ;;
    21)
      if [ "$require_ready" = "1" ]; then
        echo "[infring install] smoke model_readiness: failed (ollama offline)" >&2
        cat "$log" >&2 || true
        return 1
      fi
      echo "[infring install] smoke model_readiness: skipped (ollama offline)"
      return 0
      ;;
    22|23)
      if [ "$require_ready" = "1" ]; then
        echo "[infring install] smoke model_readiness: failed (no local runnable models)" >&2
        cat "$log" >&2 || true
        return 1
      fi
      echo "[infring install] smoke model_readiness: warning (no local runnable models)"
      cat "$log" >&2 || true
      return 0
      ;;
  esac
  echo "[infring install] smoke model_readiness: failed" >&2
  cat "$log" >&2 || true
  return 1
}

run_command_with_timeout() {
  timeout_s="$1"
  shift || true
  [ $# -gt 0 ] || return 2
  case "$timeout_s" in
    ''|*[!0-9]*)
      timeout_s=60
      ;;
  esac
  if [ "$timeout_s" -lt 1 ]; then
    timeout_s=1
  fi
  "$@" &
  cmd_pid=$!
  elapsed=0
  while kill -0 "$cmd_pid" >/dev/null 2>&1; do
    if [ "$elapsed" -ge "$timeout_s" ]; then
      kill "$cmd_pid" >/dev/null 2>&1 || true
      sleep 1
      kill -9 "$cmd_pid" >/dev/null 2>&1 || true
      wait "$cmd_pid" >/dev/null 2>&1 || true
      return 124
    fi
    sleep 1
    elapsed=$((elapsed + 1))
  done
  wait_status=0
  if wait "$cmd_pid" >/dev/null 2>&1; then
    wait_status=0
  else
    wait_status=$?
  fi
  return "$wait_status"
}

run_post_install_smoke_tests() {
  install_dir="$1"
  workspace="$2"
  [ -x "$install_dir/infring" ] || return 1
  [ -x "$install_dir/infringctl" ] || return 1
  smoke_dir="$(mktemp -d)"
  export INFRING_WORKSPACE_ROOT="$workspace"
  export INFRING_WRAPPER_CD_WORKSPACE=1
  INSTALL_DASHBOARD_SMOKE_PASSED=0
  failures=0
  required_failures=0
  smoke_records_file="$smoke_dir/install_smoke_records.jsonl"
  : > "$smoke_records_file"
  if run_post_install_smoke_command "$smoke_dir" "infring_help" "$install_dir/infring" --help; then
    append_install_smoke_record "$smoke_records_file" "infring_help" "infring --help" "true" "true" "passed" "$smoke_dir/infring_help.log"
  else
    failures=$((failures + 1))
    required_failures=$((required_failures + 1))
    append_install_smoke_record "$smoke_records_file" "infring_help" "infring --help" "true" "false" "failed" "$smoke_dir/infring_help.log" "command_failed"
  fi
  if rustup_default_toolchain_missing; then
    if [ "$INSTALL_TOOLCHAIN_POLICY" = "fail_closed" ]; then
      cat <<EOF > "$smoke_dir/infringctl_help.log"
failed (toolchain policy fail_closed): missing rustup default toolchain
fix: run 'rustup default stable'
EOF
      echo "[infring install] smoke infringctl_help: failed (toolchain policy fail_closed; missing rustup default toolchain)" >&2
      echo "[infring install] smoke infringctl_help: run 'rustup default stable' and rerun install." >&2
      failures=$((failures + 1))
      required_failures=$((required_failures + 1))
      append_install_smoke_record "$smoke_records_file" "infringctl_help" "infringctl --help" "true" "false" "failed_policy_toolchain" "$smoke_dir/infringctl_help.log" "rustup_default_toolchain_missing"
    else
      cat <<EOF > "$smoke_dir/infringctl_help.log"
skipped (missing rustup default toolchain)
help: run 'rustup default stable' to download the latest stable release of Rust and set it as your default toolchain.
EOF
      echo "[infring install] smoke infringctl_help: skipped (missing rustup default toolchain; policy=auto)"
      echo "[infring install] smoke infringctl_help: run 'rustup default stable' to enable this check."
      append_install_smoke_record "$smoke_records_file" "infringctl_help" "infringctl --help" "false" "true" "skipped_toolchain" "$smoke_dir/infringctl_help.log"
    fi
  else
    if run_post_install_smoke_command "$smoke_dir" "infringctl_help" "$install_dir/infringctl" --help; then
      append_install_smoke_record "$smoke_records_file" "infringctl_help" "infringctl --help" "true" "true" "passed" "$smoke_dir/infringctl_help.log"
    else
      failures=$((failures + 1))
      required_failures=$((required_failures + 1))
      append_install_smoke_record "$smoke_records_file" "infringctl_help" "infringctl --help" "true" "false" "failed" "$smoke_dir/infringctl_help.log" "command_failed"
    fi
  fi
  if run_post_install_smoke_command "$smoke_dir" "infring_status" "$install_dir/infring" status; then
    append_install_smoke_record "$smoke_records_file" "infring_status" "infring status" "true" "true" "passed" "$smoke_dir/infring_status.log"
  else
    failures=$((failures + 1))
    required_failures=$((required_failures + 1))
    append_install_smoke_record "$smoke_records_file" "infring_status" "infring status" "true" "false" "failed" "$smoke_dir/infring_status.log" "command_failed"
  fi
  if run_post_install_smoke_command "$smoke_dir" "gateway_status" "$install_dir/infring" gateway status --auto-heal=0 --dashboard-open=0; then
    append_install_smoke_record "$smoke_records_file" "gateway_status" "infring gateway status --auto-heal=0 --dashboard-open=0" "true" "true" "passed" "$smoke_dir/gateway_status.log"
  else
    failures=$((failures + 1))
    required_failures=$((required_failures + 1))
    append_install_smoke_record "$smoke_records_file" "gateway_status" "infring gateway status --auto-heal=0 --dashboard-open=0" "true" "false" "failed" "$smoke_dir/gateway_status.log" "command_failed"
  fi
  dashboard_smoke_required=0
  if is_truthy "$INSTALL_FULL" || is_truthy "$INSTALL_STRICT_SMOKE"; then
    dashboard_smoke_required=1
  fi
  if node_runtime_meets_minimum; then
    if run_post_install_smoke_command "$smoke_dir" "verify_install" "$install_dir/infringctl" verify-install --json; then
      append_install_smoke_record "$smoke_records_file" "verify_install" "infringctl verify-install --json" "true" "true" "passed" "$smoke_dir/verify_install.log"
    else
      failures=$((failures + 1))
      required_failures=$((required_failures + 1))
      append_install_smoke_record "$smoke_records_file" "verify_install" "infringctl verify-install --json" "true" "false" "failed" "$smoke_dir/verify_install.log" "command_failed"
    fi
  else
    echo "[infring install] smoke verify_install: skipped (node runtime unavailable)"
    append_install_smoke_record "$smoke_records_file" "verify_install" "infringctl verify-install --json" "false" "true" "skipped_node_runtime_unavailable" "$smoke_dir/verify_install.log"
  fi
  if [ "$dashboard_smoke_required" = "1" ]; then
    smoke_port="$((4400 + ($$ % 1000)))"
    if run_dashboard_health_smoke "$smoke_dir" "$install_dir" "127.0.0.1" "$smoke_port"; then
      INSTALL_DASHBOARD_SMOKE_PASSED=1
      append_install_smoke_record "$smoke_records_file" "dashboard_healthz" "GET http://127.0.0.1:${smoke_port}/healthz" "true" "true" "passed" "$smoke_dir/dashboard_health.log"
    else
      failures=$((failures + 1))
      required_failures=$((required_failures + 1))
      append_install_smoke_record "$smoke_records_file" "dashboard_healthz" "GET http://127.0.0.1:${smoke_port}/healthz" "true" "false" "failed" "$smoke_dir/dashboard_health.log" "healthz_unreachable"
    fi
  else
    echo "[infring install] smoke dashboard_health: skipped (set INFRING_INSTALL_STRICT_SMOKE=1 or use --full to enforce)"
    append_install_smoke_record "$smoke_records_file" "dashboard_healthz" "GET http://127.0.0.1:4173/healthz" "false" "true" "skipped_not_required" "$smoke_dir/dashboard_health.log"
  fi
  model_ready_required=0
  if [ "$OLLAMA_INSTALL_CONFIRMED" = "1" ] || is_truthy "$INSTALL_REQUIRE_MODEL_READY"; then
    model_ready_required=1
  fi
  run_model_readiness_smoke "$smoke_dir" "$model_ready_required" || failures=$((failures + 1))
  write_install_smoke_summary_json "$smoke_records_file" "$INSTALL_SMOKE_SUMMARY_JSON_FILE" "$required_failures" "$install_dir" "$workspace"
  if [ "$failures" -ne 0 ]; then
    echo "[infring install] post-install smoke test failed ($failures checks)" >&2
    echo "[infring install] smoke logs: $smoke_dir" >&2
    return 1
  fi
  rm -rf "$smoke_dir" >/dev/null 2>&1 || true
  echo "[infring install] post-install smoke: passed"
  return 0
}

write_wrapper() {
  wrapper_name="$1"
  wrapper_body="$2"
  wrapper_path="$INSTALL_DIR/$wrapper_name"
  printf '%s\n' "#!/usr/bin/env sh" > "$wrapper_path"
  printf '%s\n' "infring_workspace_entry_exists() {" >> "$wrapper_path"
  printf '%s\n' "  root=\"\$1\"" >> "$wrapper_path"
  printf '%s\n' "  rel=\"\$2\"" >> "$wrapper_path"
  printf '%s\n' "  [ -f \"\$root/\$rel\" ] && return 0" >> "$wrapper_path"
  printf '%s\n' "  case \"\$rel\" in" >> "$wrapper_path"
  printf '%s\n' "    *.js)" >> "$wrapper_path"
  printf '%s\n' "      ts_rel=\"\${rel%.js}.ts\"" >> "$wrapper_path"
  printf '%s\n' "      [ -f \"\$root/\$ts_rel\" ] && return 0" >> "$wrapper_path"
  printf '%s\n' "      ;;" >> "$wrapper_path"
  printf '%s\n' "    *.ts)" >> "$wrapper_path"
  printf '%s\n' "      js_rel=\"\${rel%.ts}.js\"" >> "$wrapper_path"
  printf '%s\n' "      [ -f \"\$root/\$js_rel\" ] && return 0" >> "$wrapper_path"
  printf '%s\n' "      ;;" >> "$wrapper_path"
  printf '%s\n' "  esac" >> "$wrapper_path"
  printf '%s\n' "  return 1" >> "$wrapper_path"
  printf '%s\n' "}" >> "$wrapper_path"
  printf '%s\n' "infring_workspace_valid() {" >> "$wrapper_path"
  printf '%s\n' "  candidate=\"\$1\"" >> "$wrapper_path"
  printf '%s\n' "  [ -n \"\$candidate\" ] || return 1" >> "$wrapper_path"
  printf '%s\n' "  [ -d \"\$candidate/client/runtime\" ] || return 1" >> "$wrapper_path"
  printf '%s\n' "  manifest=\"\$candidate/client/runtime/config/install_runtime_manifest_v1.txt\"" >> "$wrapper_path"
  printf '%s\n' "  [ -f \"\$manifest\" ] || return 1" >> "$wrapper_path"
  printf '%s\n' "  while IFS= read -r row || [ -n \"\$row\" ]; do" >> "$wrapper_path"
  printf '%s\n' "    rel=\"\$(printf '%s' \"\$row\" | sed 's/^[[:space:]]*//;s/[[:space:]]*$//')\"" >> "$wrapper_path"
  printf '%s\n' "    [ -n \"\$rel\" ] || continue" >> "$wrapper_path"
  printf '%s\n' "    case \"\$rel\" in" >> "$wrapper_path"
  printf '%s\n' "      \#*) continue ;;" >> "$wrapper_path"
  printf '%s\n' "    esac" >> "$wrapper_path"
  printf '%s\n' "    infring_workspace_entry_exists \"\$candidate\" \"\$rel\" || return 1" >> "$wrapper_path"
  printf '%s\n' "  done < \"\$manifest\"" >> "$wrapper_path"
  printf '%s\n' "  return 0" >> "$wrapper_path"
  printf '%s\n' "}" >> "$wrapper_path"
  printf '%s\n' "resolve_workspace_root() {" >> "$wrapper_path"
  printf '%s\n' "  if [ -n \"\${INFRING_WORKSPACE_ROOT:-}\" ] && infring_workspace_valid \"\${INFRING_WORKSPACE_ROOT}\"; then" >> "$wrapper_path"
  printf '%s\n' "    printf '%s\n' \"\${INFRING_WORKSPACE_ROOT}\"" >> "$wrapper_path"
  printf '%s\n' "    return 0" >> "$wrapper_path"
  printf '%s\n' "  fi" >> "$wrapper_path"
  printf '%s\n' "  probe=\"\${PWD:-.}\"" >> "$wrapper_path"
  printf '%s\n' "  while [ -n \"\$probe\" ]; do" >> "$wrapper_path"
  printf '%s\n' "    if infring_workspace_valid \"\$probe\"; then" >> "$wrapper_path"
  printf '%s\n' "      printf '%s\n' \"\$probe\"" >> "$wrapper_path"
  printf '%s\n' "      return 0" >> "$wrapper_path"
  printf '%s\n' "    fi" >> "$wrapper_path"
  printf '%s\n' "    if [ \"\$probe\" = \"/\" ] || [ \"\$probe\" = \".\" ]; then" >> "$wrapper_path"
  printf '%s\n' "      break" >> "$wrapper_path"
  printf '%s\n' "    fi" >> "$wrapper_path"
  printf '%s\n' "    parent=\"\$(dirname \"\$probe\")\"" >> "$wrapper_path"
  printf '%s\n' "    if [ \"\$parent\" = \"\$probe\" ]; then" >> "$wrapper_path"
  printf '%s\n' "      break" >> "$wrapper_path"
  printf '%s\n' "    fi" >> "$wrapper_path"
  printf '%s\n' "    probe=\"\$parent\"" >> "$wrapper_path"
  printf '%s\n' "  done" >> "$wrapper_path"
  printf '%s\n' "  for candidate in \"__WORKSPACE_DIR__\" \"__INFRING_HOME__/workspace\" \"__INFRING_HOME__\" \"__INSTALL_DIR__/infring-client\"; do" >> "$wrapper_path"
  printf '%s\n' "    [ -n \"\$candidate\" ] || continue" >> "$wrapper_path"
  printf '%s\n' "    if infring_workspace_valid \"\$candidate\"; then" >> "$wrapper_path"
  printf '%s\n' "      printf '%s\n' \"\$candidate\"" >> "$wrapper_path"
  printf '%s\n' "      return 0" >> "$wrapper_path"
  printf '%s\n' "    fi" >> "$wrapper_path"
  printf '%s\n' "  done" >> "$wrapper_path"
  printf '%s\n' "  return 1" >> "$wrapper_path"
  printf '%s\n' "}" >> "$wrapper_path"
  printf '%s\n' "if workspace_root=\"\$(resolve_workspace_root 2>/dev/null)\"; then" >> "$wrapper_path"
  printf '%s\n' "  export INFRING_HOME=\"__INFRING_HOME__\"" >> "$wrapper_path"
  printf '%s\n' "  export INFRING_WORKSPACE_ROOT=\"\$workspace_root\"" >> "$wrapper_path"
  printf '%s\n' "  if [ -z \"\${INFRING_NODE_BINARY:-}\" ]; then" >> "$wrapper_path"
  printf '%s\n' "    if [ -x \"__INFRING_HOME__/node-runtime/bin/node\" ]; then" >> "$wrapper_path"
  printf '%s\n' "      export INFRING_NODE_BINARY=\"__INFRING_HOME__/node-runtime/bin/node\"" >> "$wrapper_path"
  printf '%s\n' "    elif [ -d \"__INFRING_HOME__/node-runtime\" ]; then" >> "$wrapper_path"
  printf '%s\n' "      node_candidate=\"\$(find \"__INFRING_HOME__/node-runtime\" -maxdepth 4 -type f -name node 2>/dev/null | sort | head -n 1 || true)\"" >> "$wrapper_path"
  printf '%s\n' "      if [ -n \"\$node_candidate\" ] && [ -x \"\$node_candidate\" ]; then" >> "$wrapper_path"
  printf '%s\n' "        export INFRING_NODE_BINARY=\"\$node_candidate\"" >> "$wrapper_path"
  printf '%s\n' "      fi" >> "$wrapper_path"
  printf '%s\n' "    fi" >> "$wrapper_path"
  printf '%s\n' "  fi" >> "$wrapper_path"
  printf '%s\n' "  cd_workspace_mode=\"\${INFRING_WRAPPER_CD_WORKSPACE:-1}\"" >> "$wrapper_path"
  printf '%s\n' "  if [ \"\$cd_workspace_mode\" != \"0\" ]; then" >> "$wrapper_path"
  printf '%s\n' "    cd \"\$workspace_root\" 2>/dev/null || true" >> "$wrapper_path"
  printf '%s\n' "  fi" >> "$wrapper_path"
  printf '%s\n' "fi" >> "$wrapper_path"
  printf '%s\n' "$wrapper_body" >> "$wrapper_path"
  rendered_path="$(mktemp)"
  sed \
    -e "s|__INSTALL_DIR__|${INSTALL_DIR}|g" \
    -e "s|__WORKSPACE_DIR__|${WORKSPACE_DIR}|g" \
    -e "s|__INFRING_HOME__|${INFRING_HOME}|g" \
    "$wrapper_path" > "$rendered_path"
  mv "$rendered_path" "$wrapper_path"
  chmod 755 "$wrapper_path"
}

daemon_binary_wrapper_body() {
  cat <<'EOF'
infring_codesign_runtime_if_needed() {
  binary_path="$1"
  [ -n "$binary_path" ] || return 0
  [ -x "$binary_path" ] || return 0
  [ "${INFRING_DAEMON_CODESIGN_REFRESH:-1}" != "0" ] || return 0
  case "$(uname -s 2>/dev/null || printf '')" in
    Darwin) ;;
    *) return 0 ;;
  esac
  command -v codesign >/dev/null 2>&1 || return 0
  codesign --force --sign - "$binary_path" >/dev/null 2>&1 || true
}

daemon_cmd="${1:-status}"
case "$daemon_cmd" in
    start|stop|restart|status|heal|attach|subscribe|tick|diagnostics|efficiency-status|embedded-core-status|watchdog)
    ops_bin="${INFRING_DAEMON_FALLBACK_OPS_BIN:-__INSTALL_DIR__/infring-ops}"
    if [ -x "$ops_bin" ]; then
      infring_codesign_runtime_if_needed "$ops_bin"
      daemon_action="$daemon_cmd"
      shift || true
      needs_node_hint=0
      case "$daemon_action" in
        start|restart|heal|watchdog)
          needs_node_hint=1
          ;;
      esac
      if [ "$needs_node_hint" = "1" ]; then
        has_node_flag=0
        for token in "$@"; do
          case "$token" in
            --node-binary=*)
              has_node_flag=1
              ;;
          esac
        done
        if [ "$has_node_flag" = "0" ]; then
          node_bin="${INFRING_NODE_BINARY:-}"
          if [ -z "$node_bin" ]; then
            node_bin="$(command -v node 2>/dev/null || true)"
          fi
          if [ -n "$node_bin" ]; then
            infring_codesign_runtime_if_needed "$ops_bin"
            exec "$ops_bin" daemon-control "$daemon_action" "$@" "--node-binary=${node_bin}"
          fi
        fi
      fi
      infring_codesign_runtime_if_needed "$ops_bin"
      exec "$ops_bin" daemon-control "$daemon_action" "$@"
    fi
    ;;
  daemon-control|dashboard-ui)
    ops_bin="${INFRING_DAEMON_FALLBACK_OPS_BIN:-__INSTALL_DIR__/infring-ops}"
    if [ -x "$ops_bin" ]; then
      infring_codesign_runtime_if_needed "$ops_bin"
      ops_domain="${INFRING_OPS_DOMAIN:-}"
      if [ -z "$ops_domain" ]; then
        if "$ops_bin" infringctl --help >/dev/null 2>&1; then
          ops_domain="infringctl"
        else
          ops_domain="infringctl"
        fi
      fi
      exec "$ops_bin" "$ops_domain" "$@"
    fi
    ;;
esac
exec "__INSTALL_DIR__/infringd-bin" "$@"
EOF
}

gateway_wrapper_body() {
  cat <<'EOF'
infring_gateway_pidfile() {
  host_safe="$(printf '%s' "${1:-127.0.0.1}" | tr -c 'A-Za-z0-9._-' '_')"
  port_safe="$(printf '%s' "${2:-4173}" | tr -c '0-9' '_')"
  printf '%s\n' "${TMPDIR:-/tmp}/infring-dashboard-${host_safe}-${port_safe}.pid"
}

infring_gateway_pid_read() {
  pid_file="$(infring_gateway_pidfile "$1" "$2")"
  [ -f "$pid_file" ] || return 1
  pid="$(sed -n '1p' "$pid_file" | tr -cd '0-9')"
  [ -n "$pid" ] || return 1
  printf '%s\n' "$pid"
}

infring_gateway_pid_running() {
  pid="$(infring_gateway_pid_read "$1" "$2" 2>/dev/null || true)"
  [ -n "$pid" ] || return 1
  kill -0 "$pid" >/dev/null 2>&1
}

infring_gateway_pid_clear() {
  pid_file="$(infring_gateway_pidfile "$1" "$2")"
  rm -f "$pid_file" >/dev/null 2>&1 || true
}

infring_gateway_watchdog_pidfile() {
  host_safe="$(printf '%s' "${1:-127.0.0.1}" | tr -c 'A-Za-z0-9._-' '_')"
  port_safe="$(printf '%s' "${2:-4173}" | tr -c '0-9' '_')"
  printf '%s\n' "${TMPDIR:-/tmp}/infring-dashboard-watchdog-${host_safe}-${port_safe}.pid"
}

infring_gateway_watchdog_pid_read() {
  pid_file="$(infring_gateway_watchdog_pidfile "$1" "$2")"
  [ -f "$pid_file" ] || return 1
  pid="$(sed -n '1p' "$pid_file" | tr -cd '0-9')"
  [ -n "$pid" ] || return 1
  printf '%s\n' "$pid"
}

infring_gateway_watchdog_pid_running() {
  pid="$(infring_gateway_watchdog_pid_read "$1" "$2" 2>/dev/null || true)"
  [ -n "$pid" ] || return 1
  kill -0 "$pid" >/dev/null 2>&1
}

infring_gateway_watchdog_pid_clear() {
  pid_file="$(infring_gateway_watchdog_pidfile "$1" "$2")"
  rm -f "$pid_file" >/dev/null 2>&1 || true
}

infring_gateway_launchd_mode_enabled() {
  case "$(uname -s 2>/dev/null || printf '')" in
    Darwin) ;;
    *) return 1 ;;
  esac
  [ "${INFRING_DASHBOARD_LAUNCHD:-1}" != "0" ] || return 1
  command -v launchctl >/dev/null 2>&1 || return 1
  return 0
}

infring_gateway_launchd_domain() {
  uid="$(id -u 2>/dev/null || true)"
  [ -n "$uid" ] || return 1
  for candidate in "gui/${uid}" "user/${uid}"; do
    if launchctl print "$candidate" >/dev/null 2>&1; then
      printf '%s\n' "$candidate"
      return 0
    fi
  done
  printf '%s\n' "user/${uid}"
}

infring_gateway_launchd_label() {
  printf '%s\n' "com.infring.infring.dashboard.shelltest2"
}

infring_gateway_launchd_plist() {
  label="$(infring_gateway_launchd_label "$1" "$2")"
  printf '%s\n' "$HOME/Library/LaunchAgents/${label}.plist"
}

infring_gateway_launchd_loaded() {
  domain="$(infring_gateway_launchd_domain 2>/dev/null || true)"
  label="$(infring_gateway_launchd_label "$1" "$2")"
  [ -n "$domain" ] || return 1
  launchctl print "${domain}/${label}" >/dev/null 2>&1
}

infring_gateway_xml_escape() {
  printf '%s' "${1:-}" | sed -e 's/&/\&amp;/g' -e 's/</\&lt;/g' -e 's/>/\&gt;/g'
}

infring_gateway_launchd_write_plist() {
  host="${1:-127.0.0.1}"
  port="${2:-4173}"
  root="${3:-}"
  [ -n "$root" ] || return 1
  label="$(infring_gateway_launchd_label "$host" "$port")"
  plist="$(infring_gateway_launchd_plist "$host" "$port")"
  launch_dir="$(dirname "$plist")"
  mkdir -p "$launch_dir" >/dev/null 2>&1 || return 1
  dashboard_bin="${INFRING_DASHBOARD_BIN:-__INSTALL_DIR__/infring-ops}"
  [ -x "$dashboard_bin" ] || dashboard_bin="__INSTALL_DIR__/infringctl"
  [ -x "$dashboard_bin" ] || dashboard_bin="__INSTALL_DIR__/infringctl"
  [ -x "$dashboard_bin" ] || return 1
  label_xml="$(infring_gateway_xml_escape "$label")"
  launch_cmd="cd $root && exec $dashboard_bin gateway start --dashboard-host=$host --dashboard-port=$port --dashboard-open=0"
  launch_cmd_xml="$(infring_gateway_xml_escape "$launch_cmd")"
  cat > "$plist" <<__INFRING_PLIST__
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key>
  <string>${label_xml}</string>
  <key>ProgramArguments</key>
  <array>
    <string>/bin/sh</string>
    <string>-lc</string>
    <string>${launch_cmd_xml}</string>
  </array>
  <key>RunAtLoad</key>
  <true/>
  <key>KeepAlive</key>
  <true/>
  <key>StandardOutPath</key>
  <string>/tmp/infring-dashboard-serve.log</string>
  <key>StandardErrorPath</key>
  <string>/tmp/infring-dashboard-serve.log</string>
</dict>
</plist>
__INFRING_PLIST__
  return 0
}

infring_gateway_launchd_start() {
  host="${1:-127.0.0.1}"
  port="${2:-4173}"
  root="${3:-}"
  infring_gateway_launchd_mode_enabled || return 1
  [ -n "$root" ] || return 1
  infring_gateway_launchd_write_plist "$host" "$port" "$root" || return 1
  domain="$(infring_gateway_launchd_domain 2>/dev/null || true)"
  label="$(infring_gateway_launchd_label "$host" "$port")"
  plist="$(infring_gateway_launchd_plist "$host" "$port")"
  [ -n "$domain" ] || return 1
  launchctl bootout "${domain}/${label}" >/dev/null 2>&1 || launchctl bootout "$domain" "$plist" >/dev/null 2>&1 || true
  launchctl bootstrap "$domain" "$plist" >/dev/null 2>&1 || return 1
  launchctl enable "${domain}/${label}" >/dev/null 2>&1 || true
  launchctl kickstart -k "${domain}/${label}" >/dev/null 2>&1 || true
  return 0
}

infring_gateway_launchd_stop() {
  host="${1:-127.0.0.1}"
  port="${2:-4173}"
  infring_gateway_launchd_mode_enabled || return 0
  domain="$(infring_gateway_launchd_domain 2>/dev/null || true)"
  label="$(infring_gateway_launchd_label "$host" "$port")"
  plist="$(infring_gateway_launchd_plist "$host" "$port")"
  if [ -n "$domain" ]; then
    launchctl disable "${domain}/${label}" >/dev/null 2>&1 || true
    launchctl bootout "${domain}/${label}" >/dev/null 2>&1 || launchctl bootout "$domain" "$plist" >/dev/null 2>&1 || true
  fi
  rm -f "$plist" >/dev/null 2>&1 || true
  return 0
}

infring_gateway_watchdog_stop() {
  host="${1:-127.0.0.1}"
  port="${2:-4173}"
  pid="$(infring_gateway_watchdog_pid_read "$host" "$port" 2>/dev/null || true)"
  if [ -n "$pid" ] && kill -0 "$pid" >/dev/null 2>&1; then
    kill "$pid" >/dev/null 2>&1 || true
    i=0
    while [ "$i" -lt 6 ]; do
      if ! kill -0 "$pid" >/dev/null 2>&1; then
        break
      fi
      i=$((i + 1))
      sleep 1
    done
    if kill -0 "$pid" >/dev/null 2>&1; then
      kill -9 "$pid" >/dev/null 2>&1 || true
    fi
  fi
  infring_gateway_watchdog_pid_clear "$host" "$port"
}

infring_gateway_dashboard_match() {
  host="${1:-127.0.0.1}"
  port="${2:-4173}"
  printf '%s\n' "gateway start --dashboard-host=${host} --dashboard-port=${port}"
}

infring_gateway_dashboard_match_legacy() {
  host="${1:-127.0.0.1}"
  port="${2:-4173}"
  printf '%s\n' "dashboard-ui serve --host=${host} --port=${port}"
}

infring_gateway_dashboard_match_legacy_ts() {
  host="${1:-127.0.0.1}"
  port="${2:-4173}"
  printf '%s\n' "client/runtime/systems/ui/infring_dashboard.ts serve --host=${host} --port=${port}"
}

infring_gateway_dashboard_process_running() {
  match="$(infring_gateway_dashboard_match "$1" "$2")"
  legacy_match="$(infring_gateway_dashboard_match_legacy "$1" "$2")"
  legacy_match_ts="$(infring_gateway_dashboard_match_legacy_ts "$1" "$2")"
  if command -v pgrep >/dev/null 2>&1; then
    pgrep -f "$match" >/dev/null 2>&1 && return 0
    pgrep -f "$legacy_match" >/dev/null 2>&1 && return 0
    pgrep -f "$legacy_match_ts" >/dev/null 2>&1 && return 0
  else
    ps ax -o command= 2>/dev/null | awk -v m="$match" -v l="$legacy_match" -v t="$legacy_match_ts" 'index($0,m)>0 || index($0,l)>0 || index($0,t)>0 {found=1} END {exit(found?0:1)}'
    return $?
  fi
  return 1
}

infring_gateway_pid_matches_dashboard() {
  host="${1:-127.0.0.1}"
  port="${2:-4173}"
  pid="$(infring_gateway_pid_read "$host" "$port" 2>/dev/null || true)"
  [ -n "$pid" ] || return 1
  cmd="$(ps -p "$pid" -o command= 2>/dev/null || true)"
  [ -n "$cmd" ] || return 1
  match="$(infring_gateway_dashboard_match "$host" "$port")"
  legacy_match="$(infring_gateway_dashboard_match_legacy "$host" "$port")"
  legacy_match_ts="$(infring_gateway_dashboard_match_legacy_ts "$host" "$port")"
  case "$cmd" in
    *"$match"*) return 0 ;;
    *"$legacy_match"*) return 0 ;;
    *"$legacy_match_ts"*) return 0 ;;
  esac
  return 1
}

infring_gateway_pid_sanitize() {
  host="${1:-127.0.0.1}"
  port="${2:-4173}"
  if ! infring_gateway_pid_running "$host" "$port"; then
    infring_gateway_pid_clear "$host" "$port"
    return 0
  fi
  if ! infring_gateway_pid_matches_dashboard "$host" "$port"; then
    infring_gateway_pid_clear "$host" "$port"
    return 0
  fi
  return 0
}

infring_gateway_stop_dashboard_managed() {
  host="${1:-127.0.0.1}"
  port="${2:-4173}"
  if infring_gateway_launchd_mode_enabled; then
    infring_gateway_launchd_stop "$host" "$port" >/dev/null 2>&1 || true
  fi
  pid="$(infring_gateway_pid_read "$host" "$port" 2>/dev/null || true)"
  if [ -n "$pid" ]; then
    if kill -0 "$pid" >/dev/null 2>&1; then
      kill "$pid" >/dev/null 2>&1 || true
      i=0
      while [ "$i" -lt 8 ]; do
        if ! kill -0 "$pid" >/dev/null 2>&1; then
          break
        fi
        i=$((i + 1))
        sleep 1
      done
      if kill -0 "$pid" >/dev/null 2>&1; then
        kill -9 "$pid" >/dev/null 2>&1 || true
      fi
    fi
  fi
  match="$(infring_gateway_dashboard_match "$host" "$port")"
  legacy_match="$(infring_gateway_dashboard_match_legacy "$host" "$port")"
  legacy_match_ts="$(infring_gateway_dashboard_match_legacy_ts "$host" "$port")"
  if command -v pkill >/dev/null 2>&1; then
    pkill -f "$match" >/dev/null 2>&1 || true
    pkill -f "$legacy_match" >/dev/null 2>&1 || true
    pkill -f "$legacy_match_ts" >/dev/null 2>&1 || true
    sleep 1
    pkill -9 -f "$match" >/dev/null 2>&1 || true
    pkill -9 -f "$legacy_match" >/dev/null 2>&1 || true
    pkill -9 -f "$legacy_match_ts" >/dev/null 2>&1 || true
  else
    pids="$(ps ax -o pid= -o command= 2>/dev/null | awk -v m="$match" -v l="$legacy_match" -v t="$legacy_match_ts" 'index($0,m)>0 || index($0,l)>0 || index($0,t)>0 {print $1}')"
    for row in $pids; do
      kill "$row" >/dev/null 2>&1 || true
    done
    sleep 1
    for row in $pids; do
      kill -9 "$row" >/dev/null 2>&1 || true
    done
  fi
  if command -v lsof >/dev/null 2>&1; then
    case "$port" in
      ''|*[!0-9]*) ;;
      *)
        stale_listeners="$(lsof -nP -iTCP:"$port" -sTCP:LISTEN -t 2>/dev/null | awk '!seen[$0]++')"
        for row in $stale_listeners; do
          cmd="$(ps -p "$row" -o command= 2>/dev/null || true)"
          case "$cmd" in
            *infring*|*dashboard-ui*|*infring_dashboard*)
              kill "$row" >/dev/null 2>&1 || true
              sleep 1
              kill -9 "$row" >/dev/null 2>&1 || true
              ;;
          esac
        done
        ;;
    esac
  fi
  infring_gateway_pid_clear "$host" "$port"
  return 0
}

infring_gateway_health_ok() {
  host="$1"
  port="$2"
  connect_timeout="${INFRING_GATEWAY_HEALTH_CONNECT_TIMEOUT:-1}"
  max_time="${INFRING_GATEWAY_HEALTH_MAX_TIME:-3}"
  case "$connect_timeout" in
    ''|*[!0-9]*)
      connect_timeout=1
      ;;
  esac
  case "$max_time" in
    ''|*[!0-9]*)
      max_time=3
      ;;
  esac
  if command -v curl >/dev/null 2>&1; then
    curl --connect-timeout "$connect_timeout" --max-time "$max_time" -fsS "http://${host}:${port}/healthz" >/dev/null 2>&1 && return 0
  elif command -v wget >/dev/null 2>&1; then
    wget --timeout="$max_time" -q -O - "http://${host}:${port}/healthz" >/dev/null 2>&1 && return 0
  fi
  return 1
}

infring_gateway_matching_pids() {
  host="${1:-127.0.0.1}"
  port="${2:-4173}"
  match="$(infring_gateway_dashboard_match "$host" "$port")"
  if command -v pgrep >/dev/null 2>&1; then
    pgrep -f "$match" 2>/dev/null | awk '!seen[$0]++'
  else
    ps ax -o pid= -o command= 2>/dev/null | awk -v m="$match" 'index($0,m)>0 {print $1}' | awk '!seen[$0]++'
  fi
}

infring_gateway_listener_pids() {
  port="${1:-4173}"
  if command -v lsof >/dev/null 2>&1; then
    lsof -nP -iTCP:"$port" -sTCP:LISTEN -t 2>/dev/null | awk '!seen[$0]++'
  fi
}

infring_gateway_clear_unhealthy_dashboard_processes() {
  host="${1:-127.0.0.1}"
  port="${2:-4173}"
  if infring_gateway_health_ok "$host" "$port"; then
    return 0
  fi
  for pid in $(infring_gateway_matching_pids "$host" "$port"); do
    [ -n "$pid" ] || continue
    kill "$pid" >/dev/null 2>&1 || true
  done
  sleep 1
  for pid in $(infring_gateway_matching_pids "$host" "$port"); do
    [ -n "$pid" ] || continue
    kill -9 "$pid" >/dev/null 2>&1 || true
  done
  infring_gateway_pid_clear "$host" "$port"
  return 0
}

infring_gateway_print_runtime_diagnostics() {
  host="${1:-127.0.0.1}"
  port="${2:-4173}"
  echo "[infring gateway] diagnostic: configured_healthz=http://${host}:${port}/healthz" >&2
  if infring_gateway_health_ok "$host" "$port"; then
    echo "[infring gateway] diagnostic: configured_healthz_ready=true" >&2
  else
    echo "[infring gateway] diagnostic: configured_healthz_ready=false" >&2
  fi
  listener_pids="$(infring_gateway_listener_pids "$port" | tr '\n' ' ' | sed 's/[[:space:]]*$//')"
  matching_pids="$(infring_gateway_matching_pids "$host" "$port" | tr '\n' ' ' | sed 's/[[:space:]]*$//')"
  [ -n "$listener_pids" ] && echo "[infring gateway] diagnostic: listener_pids=${listener_pids}" >&2
  [ -n "$matching_pids" ] && echo "[infring gateway] diagnostic: matching_dashboard_pids=${matching_pids}" >&2
  pid_file="$(infring_gateway_pidfile "$host" "$port")"
  watchdog_file="$(infring_gateway_watchdog_pidfile "$host" "$port")"
  [ -f "$pid_file" ] && echo "[infring gateway] diagnostic: pid_file=${pid_file}:$(sed -n '1p' "$pid_file" 2>/dev/null)" >&2
  [ -f "$watchdog_file" ] && echo "[infring gateway] diagnostic: watchdog_pid_file=${watchdog_file}:$(sed -n '1p' "$watchdog_file" 2>/dev/null)" >&2
  if [ "$port" != "5173" ] && infring_gateway_health_ok "$host" "5173"; then
    echo "[infring gateway] diagnostic: alternate_healthz_ready=http://${host}:5173/healthz" >&2
  fi
}

infring_gateway_find_workspace_root() {
  for candidate in "${INFRING_WORKSPACE_ROOT:-}" "${PWD:-.}" "__WORKSPACE_DIR__" "__INSTALL_DIR__/infring-client" "__INFRING_HOME__/workspace" "__INFRING_HOME__"; do
    [ -n "$candidate" ] || continue
    if [ -d "$candidate/client/runtime" ] || [ -d "$candidate/shell/socket" ]; then
      printf '%s\n' "$candidate"
      return 0
    fi
    if [ -d "$candidate/infring-client/client/runtime" ] || [ -d "$candidate/infring-client/shell/socket" ]; then
      printf '%s\n' "$candidate/infring-client"
      return 0
    fi
  done
  return 1
}

infring_gateway_normalize_shell() {
  raw="$(printf '%s' "${1:-}" | tr 'A-Z' 'a-z' | tr -d "\"'" | sed 's/^[[:space:]]*//;s/[[:space:]]*$//')"
  case "$raw" in
    ui|browser|dashboard)
      printf '%s\n' "ui"
      ;;
    ui-v2|browser-v2|dashboard-v2|shell-v2)
      printf '%s\n' "ui-v2"
      ;;
    legacy-ui|legacy|legacy-browser)
      printf '%s\n' "legacy-ui"
      ;;
    terminal|cli|tty)
      printf '%s\n' "terminal"
      ;;
    none|off|disabled|0)
      printf '%s\n' "none"
      ;;
    default|"")
      return 1
      ;;
    *)
      return 1
      ;;
  esac
}

infring_gateway_configured_shell() {
  root="${1:-}"
  for config_path in "${INFRING_GATEWAY_SHELL_CONFIG:-}" "${root}/local/state/ops/shell_launch.env" "__INFRING_HOME__/config/shell.env"; do
    [ -n "$config_path" ] || continue
    [ -f "$config_path" ] || continue
    value="$(sed -n \
      -e 's/^[[:space:]]*INFRING_GATEWAY_DEFAULT_SHELL[[:space:]]*=[[:space:]]*//p' \
      -e 's/^[[:space:]]*default[[:space:]]*=[[:space:]]*//p' \
      -e 's/^[[:space:]]*shell.default[[:space:]]*=[[:space:]]*//p' \
      "$config_path" | head -n 1 | tr -d "\"'")"
    [ -n "$value" ] || continue
    infring_gateway_normalize_shell "$value" && return 0
  done
  return 1
}

infring_gateway_select_shell() {
  override="${1:-}"
  root="${2:-}"
  if [ -n "$override" ]; then
    if selected="$(infring_gateway_normalize_shell "$override" 2>/dev/null)"; then
      printf '%s\n' "$selected"
      return 0
    fi
    echo "[infring gateway] unsupported shell override '${override}', falling back to ui" >&2
    printf '%s\n' "ui"
    return 0
  fi
  if [ -n "${INFRING_GATEWAY_SHELL:-}" ]; then
    if selected="$(infring_gateway_normalize_shell "$INFRING_GATEWAY_SHELL" 2>/dev/null)"; then
      printf '%s\n' "$selected"
      return 0
    fi
  fi
  if selected="$(infring_gateway_configured_shell "$root" 2>/dev/null)"; then
    printf '%s\n' "$selected"
    return 0
  fi
  if selected="$(infring_gateway_normalize_shell "${INFRING_GATEWAY_DEFAULT_SHELL:-ui}" 2>/dev/null)"; then
    printf '%s\n' "$selected"
    return 0
  fi
  printf '%s\n' "ui"
}

infring_gateway_shell_uses_browser() {
  case "${1:-ui}" in
    ui|ui-v2|legacy-ui)
      return 0
      ;;
  esac
  return 1
}

infring_gateway_node_binary() {
  if [ -n "${INFRING_NODE_BINARY:-}" ] && [ -x "$INFRING_NODE_BINARY" ]; then
    printf '%s\n' "$INFRING_NODE_BINARY"
    return 0
  fi
  if [ -x "__INFRING_HOME__/node-runtime/bin/node" ]; then
    printf '%s\n' "__INFRING_HOME__/node-runtime/bin/node"
    return 0
  fi
  if command -v node >/dev/null 2>&1; then
    command -v node
    return 0
  fi
  return 1
}

infring_gateway_launch_terminal_shell() {
  root="${1:-}"
  [ -n "$root" ] || root="$(infring_gateway_find_workspace_root 2>/dev/null || true)"
  [ -n "$root" ] || {
    echo "[infring gateway] terminal shell unavailable: workspace root not found" >&2
    return 1
  }
  node_bin="$(infring_gateway_node_binary 2>/dev/null || true)"
  [ -n "$node_bin" ] || {
    echo "[infring gateway] terminal shell unavailable: node runtime not found" >&2
    return 1
  }
  entrypoint="$root/client/runtime/lib/ts_entrypoint.ts"
  terminal_shell="$root/shell/terminal/terminal_shell.ts"
  [ -f "$entrypoint" ] || {
    echo "[infring gateway] terminal shell unavailable: missing $entrypoint" >&2
    return 1
  }
  [ -f "$terminal_shell" ] || {
    echo "[infring gateway] terminal shell unavailable: missing $terminal_shell" >&2
    return 1
  }
  base_url="${INFRING_SHELL_SOCKET_URL:-http://127.0.0.1:5173}"
  echo "[infring gateway] shell: terminal"
  TERMINAL_SHELL_REQUIRE_LIVE=0 "$node_bin" "$entrypoint" "$terminal_shell" \
    "--live=1" "--base-url=${base_url}" "--require-live=0" "--interactive=1" \
    "--out-json=${root}/core/local/artifacts/terminal_shell_live_response_test_current.json" \
    "--out-markdown=${root}/local/workspace/reports/TERMINAL_SHELL_LIVE_RESPONSE_TEST_CURRENT.md" || true
}

infring_gateway_prepare_browser_v2_shell() {
  root="${1:-}"
  [ -n "$root" ] || root="$(infring_gateway_find_workspace_root 2>/dev/null || true)"
  [ -n "$root" ] || {
    echo "[infring gateway] ui-v2 shell unavailable: workspace root not found" >&2
    return 1
  }
  node_bin="$(infring_gateway_node_binary 2>/dev/null || true)"
  [ -n "$node_bin" ] || {
    echo "[infring gateway] ui-v2 shell unavailable: node runtime not found" >&2
    return 1
  }
  entrypoint="$root/client/runtime/lib/ts_entrypoint.ts"
  browser_v2_server="$root/shell/browser-v2/browser_shell_v2_server.ts"
  [ -f "$entrypoint" ] || {
    echo "[infring gateway] ui-v2 shell unavailable: missing $entrypoint" >&2
    return 1
  }
  [ -f "$browser_v2_server" ] || {
    echo "[infring gateway] ui-v2 shell unavailable: missing $browser_v2_server" >&2
    return 1
  }
  host="${INFRING_BROWSER_SHELL_V2_HOST:-127.0.0.1}"
  port="${INFRING_BROWSER_SHELL_V2_PORT:-5273}"
  gateway_url="${INFRING_SHELL_SOCKET_URL:-http://127.0.0.1:5173}"
  shell_url="http://${host}:${port}/?gateway=${gateway_url}"
  if command -v curl >/dev/null 2>&1 && curl -fsS "http://${host}:${port}/" >/dev/null 2>&1; then
    printf '%s\n' "$shell_url"
    return 0
  fi
  mkdir -p "$root/core/local/state/shell" "$root/core/local/artifacts" >/dev/null 2>&1 || true
  log_path="$root/core/local/artifacts/browser_shell_v2_server_current.log"
  "$node_bin" "$entrypoint" "$browser_v2_server" --serve=1 "--host=${host}" "--port=${port}" >"$log_path" 2>&1 &
  printf '%s\n' "$!" > "$root/core/local/state/shell/browser_shell_v2_server.pid" 2>/dev/null || true
  i=0
  while [ "$i" -lt 20 ]; do
    if command -v curl >/dev/null 2>&1 && curl -fsS "http://${host}:${port}/" >/dev/null 2>&1; then
      printf '%s\n' "$shell_url"
      return 0
    fi
    i=$((i + 1))
    sleep 0.2
  done
  echo "[infring gateway] ui-v2 shell server did not become ready at http://${host}:${port}/" >&2
  return 1
}

infring_gateway_open_browser_shell() {
  shell_mode="${1:-ui}"
  dashboard_open="${2:-1}"
  dashboard_url="${3:-}"
  [ "$dashboard_open" = "1" ] || return 0
  [ -n "$dashboard_url" ] || return 0
  if [ "$shell_mode" = "ui-v2" ]; then
    root_for_v2="$(infring_gateway_find_workspace_root 2>/dev/null || true)"
    if v2_url="$(infring_gateway_prepare_browser_v2_shell "$root_for_v2" 2>/dev/null)"; then
      dashboard_url="$v2_url"
    else
      echo "[infring gateway] ui-v2 shell unavailable; falling back to legacy dashboard URL" >&2
    fi
  fi
  echo "[infring gateway] shell: ${shell_mode}"
  if [ "$shell_mode" = "ui-v2" ]; then
    echo "[infring gateway] browser-shell-v2: ${dashboard_url}"
  fi
  if command -v open >/dev/null 2>&1; then
    open "$dashboard_url" >/dev/null 2>&1 || true
  elif command -v xdg-open >/dev/null 2>&1; then
    xdg-open "$dashboard_url" >/dev/null 2>&1 || true
  fi
}

infring_gateway_wait_dashboard() {
  host="$1"
  port="$2"
  attempts="${3:-1}"
  i=0
  while [ "$i" -lt "$attempts" ]; do
    if infring_gateway_health_ok "$host" "$port"; then
      return 0
    fi
    i=$((i + 1))
    sleep 1
  done
  return 1
}

infring_gateway_wait_dashboard_adaptive() {
  host="$1"
  port="$2"
  timeout_s="${3:-90}"
  enforce_fallback="${4:-1}"
  i=0
  while [ "$i" -lt "$timeout_s" ]; do
    if infring_gateway_health_ok "$host" "$port"; then
      return 0
    fi
    if [ "$enforce_fallback" = "1" ]; then
      infring_gateway_pid_sanitize "$host" "$port"
      infring_gateway_start_dashboard_fallback "$host" "$port" >/dev/null 2>&1 || true
    fi
    if [ "$i" -gt 0 ] && [ $((i % 10)) -eq 0 ]; then
      echo "[infring gateway] waiting for dashboard healthz: http://${host}:${port}/healthz (${i}/${timeout_s}s)" >&2
    fi
    i=$((i + 1))
    sleep 1
  done
  return 1
}

infring_gateway_watchdog_loop() {
  host="${1:-127.0.0.1}"
  port="${2:-4173}"
  interval_s="${3:-5}"
  case "$interval_s" in
    ''|*[!0-9]*)
      interval_s=5
      ;;
  esac
  if [ "$interval_s" -lt 2 ]; then
    interval_s=2
  fi
  if [ "$interval_s" -gt 60 ]; then
    interval_s=60
  fi
  trap 'exit 0' INT TERM
  while true; do
    infring_gateway_pid_sanitize "$host" "$port"
    if ! infring_gateway_health_ok "$host" "$port"; then
      if ! infring_gateway_dashboard_process_running "$host" "$port"; then
        infring_gateway_start_dashboard_fallback "$host" "$port" >/dev/null 2>&1 || true
      fi
      infring_gateway_wait_dashboard "$host" "$port" 8 >/dev/null 2>&1 || true
    fi
    sleep "$interval_s"
  done
}

infring_gateway_spawn_detached_logged() {
  log_file="$1"
  shift || true
  [ -n "$log_file" ] || return 1
  [ "$#" -gt 0 ] || return 1
  if command -v setsid >/dev/null 2>&1; then
    setsid "$@" </dev/null >>"$log_file" 2>&1 &
  elif command -v nohup >/dev/null 2>&1; then
    nohup "$@" </dev/null >>"$log_file" 2>&1 &
  else
    "$@" </dev/null >>"$log_file" 2>&1 &
  fi
  printf '%s\n' "$!"
  return 0
}

infring_gateway_watchdog_start() {
  host="${1:-127.0.0.1}"
  port="${2:-4173}"
  interval_s="${3:-5}"
  case "$interval_s" in
    ''|*[!0-9]*)
      interval_s=5
      ;;
  esac
  if [ "$interval_s" -lt 2 ]; then
    interval_s=2
  fi
  if [ "$interval_s" -gt 60 ]; then
    interval_s=60
  fi
  if infring_gateway_launchd_mode_enabled && infring_gateway_launchd_loaded "$host" "$port"; then
    return 0
  fi
  if infring_gateway_watchdog_pid_running "$host" "$port"; then
    return 0
  fi
  infring_gateway_watchdog_pid_clear "$host" "$port"
  watchdog_pid="$(infring_gateway_spawn_detached_logged /tmp/infring-dashboard-watchdog.log "$0" "__dashboard-watchdog" "--host=${host}" "--port=${port}" "--interval=${interval_s}" 2>/dev/null || true)"
  if [ -n "$watchdog_pid" ]; then
    printf '%s\n' "$watchdog_pid" > "$(infring_gateway_watchdog_pidfile "$host" "$port")"
  fi
  return 0
}

infring_gateway_start_dashboard_fallback() {
  host="$1"
  port="$2"
  infring_gateway_pid_sanitize "$host" "$port"
  if infring_gateway_health_ok "$host" "$port"; then
    return 0
  fi
  if infring_gateway_dashboard_process_running "$host" "$port"; then
    infring_gateway_clear_unhealthy_dashboard_processes "$host" "$port" >/dev/null 2>&1 || true
  fi
  root=""
  for candidate in "${INFRING_WORKSPACE_ROOT:-}" "${PWD:-.}" "__WORKSPACE_DIR__" "__INSTALL_DIR__/infring-client"; do
    [ -n "$candidate" ] || continue
    if [ -d "$candidate/client/runtime" ] || [ -d "$candidate/infring-client/client/runtime" ]; then
      root="$candidate"
      break
    fi
  done
  [ -n "$root" ] || return 1
  dashboard_bin="${INFRING_DASHBOARD_BIN:-__INSTALL_DIR__/infring-ops}"
  [ -x "$dashboard_bin" ] || dashboard_bin="__INSTALL_DIR__/infringctl"
  [ -x "$dashboard_bin" ] || dashboard_bin="__INSTALL_DIR__/infringctl"
  [ -x "$dashboard_bin" ] || return 1
  if infring_gateway_launchd_mode_enabled; then
    if infring_gateway_launchd_start "$host" "$port" "$root" >/dev/null 2>&1; then
      match="$(infring_gateway_dashboard_match "$host" "$port")"
      if command -v pgrep >/dev/null 2>&1; then
        child_pid="$(pgrep -f "$match" | head -n 1 || true)"
        if [ -n "$child_pid" ]; then
          printf '%s\n' "$child_pid" > "$(infring_gateway_pidfile "$host" "$port")"
        fi
      fi
      return 0
    fi
  fi
  (
    cd "$root" 2>/dev/null || exit 1
    child_pid="$(infring_gateway_spawn_detached_logged /tmp/infring-dashboard-serve.log "$dashboard_bin" \
      gateway start "--dashboard-host=${host}" "--dashboard-port=${port}" "--dashboard-open=0" \
      2>/dev/null || true)"
    if [ -n "$child_pid" ]; then
      printf '%s\n' "$child_pid" > "$(infring_gateway_pidfile "$host" "$port")"
    fi
  ) >/dev/null 2>&1 || return 1
  return 0
}

infring_verify_gateway() {
  host="${1:-127.0.0.1}"
  port="${2:-4173}"
  wait_max="${3:-45}"
  case "$wait_max" in
    ''|*[!0-9]*)
      wait_max=45
      ;;
  esac
  if [ "$wait_max" -lt 5 ]; then
    wait_max=5
  fi
  if [ "$wait_max" -gt 180 ]; then
    wait_max=180
  fi
  if ! infring_gateway_wait_dashboard "$host" "$port" 2; then
    "$0" gateway start "--dashboard-open=0" "--dashboard-host=${host}" "--dashboard-port=${port}" >/dev/null 2>&1 || true
  fi
  if infring_gateway_wait_dashboard_adaptive "$host" "$port" "$wait_max" 1; then
    echo "[infring verify-gateway] ok: dashboard healthy at http://${host}:${port}/healthz"
    if infring_gateway_pid_running "$host" "$port"; then
      echo "[infring verify-gateway] dashboard pid: $(infring_gateway_pid_read "$host" "$port" 2>/dev/null || true)"
    fi
    return 0
  fi
  echo "[infring verify-gateway] failed: dashboard healthz not ready at http://${host}:${port}/healthz" >&2
  echo "[infring verify-gateway] tip: run 'infring gateway status --dashboard-host=${host} --dashboard-port=${port}'" >&2
  return 1
}

infring_recover() {
  recover_host="127.0.0.1"
  recover_port="4173"
  recover_wait_max="${INFRING_VERIFY_GATEWAY_WAIT_MAX:-90}"
  for token in "$@"; do
    case "$token" in
      --dashboard-host=*)
        recover_host="${token#*=}"
        ;;
      --dashboard-port=*)
        recover_port="${token#*=}"
        ;;
      --wait-max=*)
        recover_wait_max="${token#*=}"
        ;;
      --help|-h|help)
        echo "Usage: infring recover [--dashboard-host=127.0.0.1] [--dashboard-port=4173] [--wait-max=90]"
        return 0
        ;;
    esac
  done

  echo "[infring recover] stopping runtime"
  "$0" gateway stop "--dashboard-host=${recover_host}" "--dashboard-port=${recover_port}" "--dashboard-open=0" >/dev/null 2>&1 || true

  echo "[infring recover] starting runtime"
  if ! "$0" gateway start "--dashboard-host=${recover_host}" "--dashboard-port=${recover_port}" "--dashboard-open=0"; then
    echo "[infring recover] failed: gateway start failed" >&2
    return 1
  fi

  echo "[infring recover] verifying dashboard health"
  if ! "$0" verify-gateway "--dashboard-host=${recover_host}" "--dashboard-port=${recover_port}" "--wait-max=${recover_wait_max}"; then
    echo "[infring recover] failed: dashboard health verification failed" >&2
    return 1
  fi

  if [ -x "__INSTALL_DIR__/infringctl" ]; then
    echo "[infring recover] running verify-install"
    doctor_output="$("__INSTALL_DIR__/infringctl" verify-install --json 2>&1)"
    doctor_status=$?
    if [ "$doctor_status" -ne 0 ] || ! printf '%s\n' "$doctor_output" | grep -Eq '"ok"[[:space:]]*:[[:space:]]*true'; then
      if [ -n "$doctor_output" ]; then
        printf '%s\n' "$doctor_output" >&2
      fi
      echo "[infring recover] failed: verify-install did not return ok=true" >&2
      return 1
    fi
  fi

  echo "[infring recover] complete"
  return 0
}

infring_update_usage() {
  cat <<'__INFRING_UPDATE_HELP__'
Usage: infring update [options]
Options:
  --repair                 clear stale wrappers/runtime artifacts before reinstall
  --full|--minimal|--pure|--tiny-max
                           choose install profile (default: --full)
  --version <tag>          install a specific release tag (for example: v0.3.1-alpha)
  --install-node           request portable Node runtime installation
  --install-ollama         request Ollama install + starter local model bootstrap
  --offline                disable network fetch; require cached release artifacts
  --help                   show this help
__INFRING_UPDATE_HELP__
}

infring_update_run() {
  update_url="${INFRING_INSTALLER_URL:-https://raw.githubusercontent.com/protheuslabs/InfRing/main/install.sh}"
  update_mode="full"
  update_repair=0
  update_version=""
  update_install_node=0
  update_install_ollama=0
  update_offline=0

  while [ "$#" -gt 0 ]; do
    case "$1" in
      --repair)
        update_repair=1
        ;;
      --full|--minimal|--pure|--tiny-max)
        update_mode="${1#--}"
        ;;
      --version=*)
        update_version="${1#*=}"
        ;;
      --version)
        shift || true
        update_version="${1:-}"
        if [ -z "$update_version" ]; then
          echo "[infring update] missing value for --version" >&2
          return 2
        fi
        ;;
      --install-node)
        update_install_node=1
        ;;
      --install-ollama)
        update_install_ollama=1
        ;;
      --offline)
        update_offline=1
        ;;
      --help|-h|help)
        infring_update_usage
        return 0
        ;;
      *)
        echo "[infring update] unsupported option: $1" >&2
        infring_update_usage >&2
        return 2
        ;;
    esac
    shift || true
  done

  if [ "$update_offline" = "1" ] && [ -z "$update_version" ]; then
    echo "[infring update] --offline requires --version vX.Y.Z (latest lookup is network-backed)." >&2
    return 2
  fi

  installer_tmp="$(mktemp "${TMPDIR:-/tmp}/infring-install.XXXXXX")" || {
    echo "[infring update] failed to allocate installer temp file" >&2
    return 1
  }

  if command -v curl >/dev/null 2>&1; then
    if ! curl -fsSL "$update_url" -o "$installer_tmp"; then
      rm -f "$installer_tmp" >/dev/null 2>&1 || true
      echo "[infring update] failed to download installer from $update_url" >&2
      return 1
    fi
  elif command -v wget >/dev/null 2>&1; then
    if ! wget -qO "$installer_tmp" "$update_url"; then
      rm -f "$installer_tmp" >/dev/null 2>&1 || true
      echo "[infring update] failed to download installer from $update_url" >&2
      return 1
    fi
  else
    rm -f "$installer_tmp" >/dev/null 2>&1 || true
    echo "[infring update] neither curl nor wget is available; cannot fetch installer" >&2
    return 1
  fi

  update_flags="--${update_mode} --install-dir __INSTALL_DIR__"
  if [ "$update_repair" = "1" ]; then
    update_flags="$update_flags --repair"
  fi
  if [ "$update_install_node" = "1" ]; then
    update_flags="$update_flags --install-node"
  fi
  if [ "$update_install_ollama" = "1" ]; then
    update_flags="$update_flags --install-ollama"
  fi
  if [ "$update_offline" = "1" ]; then
    update_flags="$update_flags --offline"
  fi

  echo "[infring update] downloading installer: $update_url"
  echo "[infring update] mode: --${update_mode}"
  if [ "$update_offline" = "1" ]; then
    echo "[infring update] offline: enabled"
  fi
  if [ "$update_repair" = "1" ]; then
    echo "[infring update] repair: enabled"
  fi
  if [ -n "$update_version" ]; then
    echo "[infring update] target version: $update_version"
    INFRING_HOME="__INFRING_HOME__" \
      INFRING_VERSION="$update_version" \
      sh "$installer_tmp" $update_flags
  else
    INFRING_HOME="__INFRING_HOME__" \
      sh "$installer_tmp" $update_flags
  fi
  update_status=$?
  rm -f "$installer_tmp" >/dev/null 2>&1 || true
  if [ "$update_status" -ne 0 ]; then
    echo "[infring update] failed" >&2
    return "$update_status"
  fi
  echo "[infring update] complete"
  return 0
}

if [ "${1:-}" = "__dashboard-watchdog" ]; then
  shift || true
  watchdog_host="127.0.0.1"
  watchdog_port="4173"
  watchdog_interval="5"
  for token in "$@"; do
    case "$token" in
      --host=*)
        watchdog_host="${token#*=}"
        ;;
      --port=*)
        watchdog_port="${token#*=}"
        ;;
      --interval=*)
        watchdog_interval="${token#*=}"
        ;;
    esac
  done
  infring_gateway_watchdog_loop "$watchdog_host" "$watchdog_port" "$watchdog_interval"
  exit 0
fi

if [ "${1:-}" = "update" ] || [ "${1:-}" = "upgrade" ]; then
  shift || true
  infring_update_run "$@"
  update_status=$?
  exit "$update_status"
fi

if [ "${1:-}" = "verify-gateway" ]; then
  shift || true
  verify_host="127.0.0.1"
  verify_port="4173"
  verify_wait_max="${INFRING_VERIFY_GATEWAY_WAIT_MAX:-45}"
  for token in "$@"; do
    case "$token" in
      --dashboard-host=*)
        verify_host="${token#*=}"
        ;;
      --dashboard-port=*)
        verify_port="${token#*=}"
        ;;
      --wait-max=*)
        verify_wait_max="${token#*=}"
        ;;
    esac
  done
  if infring_verify_gateway "$verify_host" "$verify_port" "$verify_wait_max"; then
    exit 0
  fi
  exit 1
fi

if [ "${1:-}" = "recover" ]; then
  shift || true
  if infring_recover "$@"; then
    exit 0
  fi
  exit 1
fi

if [ "${1:-}" = "gateway" ]; then
  shift || true
  gateway_action=""
  case "${1:-}" in
    "" )
      gateway_action="start"
      ;;
    start|boot|stop|restart|status|heal|attach|subscribe|tick|diagnostics|efficiency-status|embedded-core-status)
      gateway_action="${1:-start}"
      shift || true
      ;;
    --help|-h|help)
      echo "Usage: infring gateway [start|stop|restart|status|heal|attach|subscribe|tick|diagnostics] [flags]"
      echo "  default action is 'start'"
      echo "  --shell=ui|ui-v2|terminal|legacy-ui|none selects the shell plug for start/restart"
      echo "  add --dashboard-open=0 to skip browser auto-open on start"
      exit 0
      ;;
    *)
      gateway_action="start"
      ;;
  esac

  if [ "$gateway_action" = "boot" ]; then
    gateway_action="start"
  fi

  dashboard_preflight_host="127.0.0.1"
  dashboard_preflight_port="4173"
  dashboard_preflight_open="1"
  gateway_shell_override=""
  gateway_shell_next=0
  if [ "${INFRING_NO_BROWSER:-0}" = "1" ]; then
    dashboard_preflight_open="0"
  fi
  for token in "$@"; do
    if [ "$gateway_shell_next" = "1" ]; then
      gateway_shell_override="$token"
      gateway_shell_next=0
      continue
    fi
    case "$token" in
      --shell=*)
        gateway_shell_override="${token#*=}"
        ;;
      --shell)
        gateway_shell_next=1
        ;;
      --terminal-shell|--terminal)
        gateway_shell_override="terminal"
        ;;
      --ui-shell|--ui)
        gateway_shell_override="ui"
        ;;
      --ui-v2-shell|--ui-v2)
        gateway_shell_override="ui-v2"
        ;;
      --no-shell)
        gateway_shell_override="none"
        ;;
      --dashboard-open=0|--no-browser)
        dashboard_preflight_open="0"
        ;;
      --dashboard-open=1)
        dashboard_preflight_open="1"
        ;;
      --dashboard-host=*)
        dashboard_preflight_host="${token#*=}"
        ;;
      --dashboard-port=*)
        dashboard_preflight_port="${token#*=}"
      ;;
    esac
  done
  gateway_preflight_root="$(infring_gateway_find_workspace_root 2>/dev/null || true)"
  gateway_selected_shell="$(infring_gateway_select_shell "$gateway_shell_override" "$gateway_preflight_root")"
  if ! infring_gateway_shell_uses_browser "$gateway_selected_shell"; then
    dashboard_preflight_open="0"
  fi
  dashboard_preflight_url="${INFRING_DASHBOARD_URL:-http://${dashboard_preflight_host}:${dashboard_preflight_port}/dashboard#chat}"
  if [ "$gateway_action" = "start" ] && infring_gateway_health_ok "$dashboard_preflight_host" "$dashboard_preflight_port"; then
    echo "P o w e r  T o  T h e  U s e r s"
    echo "[infring gateway] already active"
    echo "[infring gateway] dashboard: $dashboard_preflight_url"
    if [ "$gateway_selected_shell" = "terminal" ]; then
      infring_gateway_launch_terminal_shell "$gateway_preflight_root" || true
    elif infring_gateway_shell_uses_browser "$gateway_selected_shell"; then
      infring_gateway_open_browser_shell "$gateway_selected_shell" "$dashboard_preflight_open" "$dashboard_preflight_url"
    else
      echo "[infring gateway] shell: none"
    fi
    [ -n "${INFRING_WORKSPACE_ROOT:-}" ] && echo "[infring gateway] workspace: $INFRING_WORKSPACE_ROOT"
    exit 0
  fi

  if ! infring_gateway_shell_uses_browser "$gateway_selected_shell"; then
    gateway_output="$(INFRING_DASHBOARD_OPEN_ON_START=0 "__INSTALL_DIR__/infringd" "$gateway_action" "$@" 2>&1)"
  else
    gateway_output="$("__INSTALL_DIR__/infringd" "$gateway_action" "$@" 2>&1)"
  fi
  gateway_status=$?
  if [ "$gateway_status" -ne 0 ]; then
    if [ "$gateway_action" = "start" ] || [ "$gateway_action" = "restart" ]; then
      echo "[infring gateway] core ${gateway_action} returned failure; attempting one bounded heal" >&2
      heal_ready_timeout="${INFRING_GATEWAY_HEAL_READY_TIMEOUT_MS:-6000}"
      if ! infring_gateway_shell_uses_browser "$gateway_selected_shell"; then
        heal_output="$(INFRING_DASHBOARD_OPEN_ON_START=0 INFRING_DASHBOARD_READY_TIMEOUT_MS="$heal_ready_timeout" "__INSTALL_DIR__/infringd" heal "$@" 2>&1)"
      else
        heal_output="$(INFRING_DASHBOARD_READY_TIMEOUT_MS="$heal_ready_timeout" "__INSTALL_DIR__/infringd" heal "$@" 2>&1)"
      fi
      if infring_gateway_wait_dashboard "$dashboard_preflight_host" "$dashboard_preflight_port" 4; then
        echo "P o w e r  T o  T h e  U s e r s"
        echo "[infring gateway] runtime healed"
        echo "[infring gateway] dashboard: $dashboard_preflight_url"
        if [ "$gateway_selected_shell" = "terminal" ]; then
          infring_gateway_launch_terminal_shell "$gateway_preflight_root" || true
        elif infring_gateway_shell_uses_browser "$gateway_selected_shell"; then
          infring_gateway_open_browser_shell "$gateway_selected_shell" "$dashboard_preflight_open" "$dashboard_preflight_url"
        else
          echo "[infring gateway] shell: none"
        fi
        [ -n "${INFRING_WORKSPACE_ROOT:-}" ] && echo "[infring gateway] workspace: $INFRING_WORKSPACE_ROOT"
        exit 0
      fi
      [ -n "$heal_output" ] && printf '%s\n' "$heal_output" >&2
    fi
    if [ -n "$gateway_output" ]; then
      printf '%s\n' "$gateway_output" >&2
    fi
    infring_gateway_print_runtime_diagnostics "$dashboard_preflight_host" "$dashboard_preflight_port"
    echo "[infring gateway] ${gateway_action} failed" >&2
    exit "$gateway_status"
  fi
  if [ "${INFRING_GATEWAY_RAW:-0}" = "1" ]; then
    if [ -n "$gateway_output" ]; then
      printf '%s\n' "$gateway_output"
    fi
  fi

  receipt_hash="$(printf '%s\n' "$gateway_output" | sed -n 's/.*"receipt_hash":"\([^"]*\)".*/\1/p' | head -n 1)"
  root_path="$(printf '%s\n' "$gateway_output" | sed -n 's/.*"root":"\([^"]*\)".*/\1/p' | head -n 1)"
  if [ -z "$root_path" ] && [ -n "${INFRING_WORKSPACE_ROOT:-}" ]; then
    root_path="$INFRING_WORKSPACE_ROOT"
  fi

  dashboard_open="1"
  dashboard_host="127.0.0.1"
  dashboard_port="4173"
  dashboard_watchdog_enabled="${INFRING_DASHBOARD_WATCHDOG:-0}"
  dashboard_watchdog_interval="${INFRING_DASHBOARD_WATCHDOG_INTERVAL:-5}"
  legacy_supervisor_mode="${INFRING_GATEWAY_LEGACY_SUPERVISOR:-0}"
  case "$dashboard_watchdog_interval" in
    ''|*[!0-9]*)
      dashboard_watchdog_interval=5
      ;;
  esac
  if [ "$dashboard_watchdog_interval" -lt 2 ]; then
    dashboard_watchdog_interval=2
  fi
  if [ "$dashboard_watchdog_interval" -gt 60 ]; then
    dashboard_watchdog_interval=60
  fi
  if [ "${INFRING_NO_BROWSER:-0}" = "1" ]; then
    dashboard_open="0"
  fi
  for token in "$@"; do
    case "$token" in
      --dashboard-open=0|--no-browser)
        dashboard_open="0"
        ;;
      --dashboard-open=1)
        dashboard_open="1"
        ;;
      --dashboard-host=*)
        dashboard_host="${token#*=}"
        ;;
      --dashboard-port=*)
        dashboard_port="${token#*=}"
        ;;
    esac
  done
  if ! infring_gateway_shell_uses_browser "$gateway_selected_shell"; then
    dashboard_open="0"
  fi
  dashboard_url="${INFRING_DASHBOARD_URL:-http://${dashboard_host}:${dashboard_port}/dashboard#chat}"
  core_opened_browser="0"
  if printf '%s\n' "$gateway_output" | grep -Eq '"opened_browser"[[:space:]]*:[[:space:]]*true'; then
    core_opened_browser="1"
  fi

  if [ "$gateway_action" = "start" ] || [ "$gateway_action" = "restart" ]; then
    dashboard_already_active="0"
    if [ "$gateway_action" = "start" ] \
      && printf '%s\n' "$gateway_output" | grep -Eq '"running"[[:space:]]*:[[:space:]]*true' \
      && printf '%s\n' "$gateway_output" | grep -Eq '"launched"[[:space:]]*:[[:space:]]*false'; then
      dashboard_already_active="1"
    fi
    if [ "$legacy_supervisor_mode" != "1" ]; then
      # Default mode: Rust core is sole authority for dashboard lifecycle/watchdog.
      infring_gateway_watchdog_stop "$dashboard_host" "$dashboard_port" >/dev/null 2>&1 || true
      infring_gateway_watchdog_pid_clear "$dashboard_host" "$dashboard_port" >/dev/null 2>&1 || true
      infring_gateway_launchd_stop "$dashboard_host" "$dashboard_port" >/dev/null 2>&1 || true
      infring_gateway_pid_sanitize "$dashboard_host" "$dashboard_port"
    elif [ "$gateway_action" = "restart" ]; then
      infring_gateway_watchdog_stop "$dashboard_host" "$dashboard_port" >/dev/null 2>&1 || true
      infring_gateway_stop_dashboard_managed "$dashboard_host" "$dashboard_port" >/dev/null 2>&1 || true
    fi
    dashboard_ready="0"
    if infring_gateway_wait_dashboard "$dashboard_host" "$dashboard_port" 20; then
      dashboard_ready="1"
    else
      if [ "$legacy_supervisor_mode" = "1" ] && [ "${INFRING_DASHBOARD_FALLBACK:-1}" != "0" ]; then
        infring_gateway_start_dashboard_fallback "$dashboard_host" "$dashboard_port" >/dev/null 2>&1 || true
      fi
      if [ "$dashboard_ready" != "1" ]; then
        wait_max="${INFRING_DASHBOARD_WAIT_MAX:-90}"
        case "$wait_max" in
          ''|*[!0-9]*)
            wait_max=90
            ;;
        esac
        adaptive_fallback=0
        if [ "$legacy_supervisor_mode" = "1" ]; then
          adaptive_fallback=1
        fi
        if infring_gateway_wait_dashboard_adaptive "$dashboard_host" "$dashboard_port" "$wait_max" "$adaptive_fallback"; then
          dashboard_ready="1"
        fi
      fi
    fi
    # Core daemon-control is authoritative for browser launch.
    # Only fallback-open here when core explicitly did not open a tab.
    if infring_gateway_shell_uses_browser "$gateway_selected_shell" && [ "$dashboard_open" = "1" ] && [ "$core_opened_browser" != "1" ]; then
      infring_gateway_open_browser_shell "$gateway_selected_shell" "$dashboard_open" "$dashboard_url"
    fi
    if [ "$dashboard_ready" != "1" ]; then
      echo "[infring gateway] ${gateway_action} failed: dashboard healthz not ready at http://${dashboard_host}:${dashboard_port}/healthz" >&2
      infring_gateway_print_runtime_diagnostics "$dashboard_host" "$dashboard_port"
      dashboard_error="$(printf '%s\n' "$gateway_output" | sed -n 's/.*"error":"\([^"]*\)".*/\1/p' | head -n 1)"
      dashboard_issue_code="$(printf '%s\n' "$gateway_output" | sed -n 's/.*"code":"\([^"]*\)".*/\1/p' | head -n 1)"
      dashboard_current_executable="$(printf '%s\n' "$gateway_output" | sed -n 's/.*"current_executable":"\([^"]*\)".*/\1/p' | head -n 1)"
      dashboard_expected_executable="$(printf '%s\n' "$gateway_output" | sed -n 's/.*"expected_executable":"\([^"]*\)".*/\1/p' | head -n 1)"
      dashboard_expected_launcher="$(printf '%s\n' "$gateway_output" | sed -n 's/.*"expected_launcher":"\([^"]*\)".*/\1/p' | head -n 1)"
      dashboard_reasons="$(printf '%s\n' "$gateway_output" | sed -n 's/.*"reasons":\[\([^]]*\)\].*/\1/p' | head -n 1)"
      if [ -n "$dashboard_error" ]; then
        echo "[infring gateway] startup-cause: ${dashboard_error}" >&2
      fi
      if [ -n "$dashboard_issue_code" ]; then
        echo "[infring gateway] startup-issue: ${dashboard_issue_code}" >&2
      fi
      if [ -n "$dashboard_current_executable" ]; then
        echo "[infring gateway] current-executable: ${dashboard_current_executable}" >&2
      fi
      if [ -n "$dashboard_expected_executable" ]; then
        echo "[infring gateway] expected-executable: ${dashboard_expected_executable}" >&2
      fi
      if [ -n "$dashboard_expected_launcher" ]; then
        echo "[infring gateway] expected-launcher: ${dashboard_expected_launcher}" >&2
      fi
      if [ -n "$dashboard_reasons" ]; then
        echo "[infring gateway] startup-reasons: ${dashboard_reasons}" >&2
      fi
      if [ "$dashboard_error" = "dashboard_duplicate_runtime_detected" ] || [ "$dashboard_issue_code" = "dashboard_runtime_binary_authority_mismatch" ]; then
        echo "[infring gateway] authority-recovery: repair/reinstall the canonical launcher, or temporarily set INFRING_DAEMON_EXPECTED_BINARY to the resolved runtime binary." >&2
      fi
      echo "[infring gateway] next-action: infring gateway status --dashboard-host=${dashboard_host} --dashboard-port=${dashboard_port}" >&2
      echo "[infring gateway] recovery: infring gateway restart --dashboard-host=${dashboard_host} --dashboard-port=${dashboard_port}" >&2
      echo "[infring gateway] diagnostics: infring doctor --json" >&2
      exit 1
    fi
    if [ "$legacy_supervisor_mode" = "1" ] && [ "$dashboard_watchdog_enabled" != "0" ]; then
      infring_gateway_watchdog_start "$dashboard_host" "$dashboard_port" "$dashboard_watchdog_interval" >/dev/null 2>&1 || true
    fi
    echo "P o w e r  T o  T h e  U s e r s"
    if [ "$gateway_action" = "restart" ]; then
      echo "[infring gateway] runtime restarted"
    elif [ "$dashboard_already_active" = "1" ]; then
      echo "[infring gateway] already active"
    else
      echo "[infring gateway] runtime started"
    fi
    echo "[infring gateway] dashboard: $dashboard_url"
    if [ "$gateway_selected_shell" = "terminal" ]; then
      infring_gateway_launch_terminal_shell "${root_path:-$gateway_preflight_root}" || true
    elif ! infring_gateway_shell_uses_browser "$gateway_selected_shell"; then
      echo "[infring gateway] shell: none"
    elif [ "$core_opened_browser" = "1" ]; then
      echo "[infring gateway] shell: $gateway_selected_shell"
    fi
    [ -n "$root_path" ] && echo "[infring gateway] workspace: $root_path"
    [ -n "$receipt_hash" ] && echo "[infring gateway] receipt: $receipt_hash"
  elif [ "$gateway_action" = "stop" ]; then
    infring_gateway_watchdog_stop "$dashboard_host" "$dashboard_port" >/dev/null 2>&1 || true
    infring_gateway_stop_dashboard_managed "$dashboard_host" "$dashboard_port" >/dev/null 2>&1 || true
    echo "[infring gateway] runtime stopped"
    [ -n "$receipt_hash" ] && echo "[infring gateway] receipt: $receipt_hash"
  elif [ "$gateway_action" = "status" ] || [ "$gateway_action" = "heal" ]; then
    echo "[infring gateway] runtime status received"
    dashboard_status_healthy="0"
    if infring_gateway_wait_dashboard "$dashboard_host" "$dashboard_port" 3; then
      dashboard_status_healthy="1"
    fi
    if [ "$dashboard_status_healthy" = "1" ]; then
      echo "[infring gateway] dashboard healthy: http://${dashboard_host}:${dashboard_port}/healthz"
    else
      echo "[infring gateway] dashboard down: http://${dashboard_host}:${dashboard_port}/healthz"
    fi
    if infring_gateway_pid_running "$dashboard_host" "$dashboard_port"; then
      echo "[infring gateway] dashboard pid: $(infring_gateway_pid_read "$dashboard_host" "$dashboard_port" 2>/dev/null || true)"
    fi
    core_watchdog_pid=""
    if [ -n "$root_path" ] && [ -f "$root_path/local/state/ops/daemon_control/dashboard_watchdog.pid" ]; then
      core_watchdog_pid="$(sed -n '1p' "$root_path/local/state/ops/daemon_control/dashboard_watchdog.pid" | tr -cd '0-9')"
    fi
    if [ -n "$core_watchdog_pid" ] && kill -0 "$core_watchdog_pid" >/dev/null 2>&1; then
      echo "[infring gateway] dashboard watchdog pid: $core_watchdog_pid (core)"
    elif infring_gateway_watchdog_pid_running "$dashboard_host" "$dashboard_port"; then
      echo "[infring gateway] dashboard watchdog pid: $(infring_gateway_watchdog_pid_read "$dashboard_host" "$dashboard_port" 2>/dev/null || true) (legacy)"
    fi
    if [ "$legacy_supervisor_mode" = "1" ] && infring_gateway_launchd_mode_enabled; then
      if infring_gateway_launchd_loaded "$dashboard_host" "$dashboard_port"; then
        echo "[infring gateway] dashboard supervisor: launchd ($(infring_gateway_launchd_label "$dashboard_host" "$dashboard_port"))"
      else
        echo "[infring gateway] dashboard supervisor: launchd (not loaded)"
      fi
    fi
    echo "[infring gateway] dashboard: $dashboard_url"
    [ -n "$root_path" ] && echo "[infring gateway] workspace: $root_path"
    [ -n "$receipt_hash" ] && echo "[infring gateway] receipt: $receipt_hash"
    if [ "$gateway_action" = "heal" ]; then
      echo "[infring gateway] heal routine executed"
    fi
  else
    echo "[infring gateway] action complete: $gateway_action"
    [ -n "$receipt_hash" ] && echo "[infring gateway] receipt: $receipt_hash"
  fi
  exit 0
fi
EOF
}

main() {
  if [ "${1:-}" = "--verify-install-summary-contract" ]; then
    if verify_install_summary_success_contract; then
      exit 0
    fi
    exit 1
  fi
  parse_install_args "$@"
  if is_truthy "$INSTALL_OFFLINE"; then
    INSTALL_ASSET_CACHE=1
  fi
  install_summary_init
  trap 'install_summary_finalize "$?"' EXIT
  resolve_install_dir_default
  run_install_preflight || exit 1

  if [ -n "${INSTALL_TMP_DIR:-}" ]; then
    mkdir -p "$INSTALL_TMP_DIR"
    export TMPDIR="$INSTALL_TMP_DIR"
  fi
  mkdir -p "$INSTALL_DIR"
  if is_truthy "$INSTALL_REPAIR"; then
    echo "[infring install] repair mode enabled"
    repair_install_dir
    repair_workspace_state
  fi
  triple="$(platform_triple)"
  version="$(resolve_version)"
  install_summary_note "resolved_version: ${version}"
  install_summary_note "platform_triple: ${triple}"

  echo "[infring install] version: $version"
  echo "[infring install] platform: $triple"
  echo "[infring install] install dir: $INSTALL_DIR"
  echo "[infring install] workspace dir: $WORKSPACE_DIR"
  if is_truthy "$INSTALL_OFFLINE"; then
    echo "[infring install] mode: offline (network disabled; using cached artifacts only)"
  fi

  ops_bin="$INSTALL_DIR/infring-ops"
  pure_bin="$INSTALL_DIR/infring-pure-workspace"
  infringd_bin="$INSTALL_DIR/infringd-bin"
  daemon_bin="$INSTALL_DIR/conduit_daemon"
  daemon_wrapper_body=""
  prefer_musl_infringd=0

  if [ "$(norm_os)" = "linux" ] && [ "$(norm_arch)" = "x86_64" ]; then
    prefer_musl_infringd=1
  fi

  if is_truthy "$INSTALL_PURE"; then
    if is_truthy "$INSTALL_TINY_MAX"; then
      if ! install_binary "$version" "$triple" "infring-pure-workspace-tiny-max" "$pure_bin"; then
        if ! install_binary "$version" "$triple" "infring-pure-workspace" "$pure_bin"; then
          echo "[infring install] failed to fetch pure workspace runtime for $triple ($version)" >&2
          exit 1
        fi
      fi
    elif ! install_binary "$version" "$triple" "infring-pure-workspace" "$pure_bin"; then
      echo "[infring install] failed to fetch pure workspace runtime for $triple ($version)" >&2
      exit 1
    fi
    if is_truthy "$INSTALL_TINY_MAX"; then
      echo "[infring install] tiny-max pure mode selected: Rust-only tiny profile installed"
    else
      echo "[infring install] pure mode selected: Rust-only client installed"
    fi
  else
    if ! install_binary "$version" "$triple" "infring-ops" "$ops_bin"; then
      echo "[infring install] failed to fetch core ops runtime for $triple ($version)" >&2
      exit 1
    fi
    if ! ensure_ops_gateway_contract "$version" "$ops_bin"; then
      exit 1
    fi
  fi

  if [ "$prefer_musl_infringd" = "1" ]; then
    if is_truthy "$INSTALL_TINY_MAX"; then
      if install_binary "$version" "x86_64-unknown-linux-musl" "infringd-tiny-max" "$infringd_bin"; then
        daemon_wrapper_body="$(daemon_binary_wrapper_body)"
        echo "[infring install] using static musl tiny-max daemon runtime"
      fi
    fi
    if [ -z "$daemon_wrapper_body" ] && install_binary "$version" "x86_64-unknown-linux-musl" "infringd" "$infringd_bin"; then
      daemon_wrapper_body="$(daemon_binary_wrapper_body)"
      echo "[infring install] using static musl daemon runtime (embedded-minimal-core)"
    fi
  fi

  if [ -z "$daemon_wrapper_body" ] && is_truthy "$INSTALL_TINY_MAX"; then
    if install_binary "$version" "$triple" "infringd-tiny-max" "$infringd_bin"; then
      daemon_wrapper_body="$(daemon_binary_wrapper_body)"
      echo "[infring install] using native tiny-max daemon runtime"
    fi
  fi

  if [ -z "$daemon_wrapper_body" ] && install_binary "$version" "$triple" "infringd" "$infringd_bin"; then
    daemon_wrapper_body="$(daemon_binary_wrapper_body)"
    echo "[infring install] using native daemon runtime"
  fi

  if [ -z "$daemon_wrapper_body" ] && install_binary "$version" "$triple" "conduit_daemon" "$daemon_bin"; then
    daemon_wrapper_body="exec \"$daemon_bin\" \"\$@\""
    echo "[infring install] using conduit_daemon compatibility fallback"
  else
    if [ -z "$daemon_wrapper_body" ]; then
      echo "[infring install] no dedicated daemon binary found; falling back to spine mode via core ops runtime"
    fi
  fi

  gateway_shim="$(gateway_wrapper_body)"
  infring_help_shim='if [ "${1:-}" = "--help" ] || [ "${1:-}" = "-h" ] || [ "${1:-}" = "help" ]; then
  cat <<'"'"'__INFRING_HELP__'"'"'
Usage: infring <command> [args]
Primary commands:
  gateway [start|stop|restart|status|heal|attach|subscribe|tick|diagnostics]
  recover [--dashboard-host=127.0.0.1] [--dashboard-port=4173] [--wait-max=90]
  update [--repair] [--full|--minimal|--pure|--tiny-max] [--version vX.Y.Z] [--offline]
  verify-gateway [--dashboard-host=127.0.0.1] [--dashboard-port=4173]
  list
  status
  version
  --help
Use "infring gateway --help" for gateway controls.
Use "infring recover --help" for deterministic recovery controls.
__INFRING_HELP__
  exit 0
fi'
  if is_truthy "$INSTALL_PURE"; then
    if is_truthy "$INSTALL_TINY_MAX"; then
      write_wrapper "infring" "${infring_help_shim}
${gateway_shim}
exec \"$pure_bin\" --tiny-max=1 \"\$@\""
    else
      write_wrapper "infring" "${infring_help_shim}
${gateway_shim}
exec \"$pure_bin\" \"\$@\""
    fi
    write_wrapper "infringctl" "${gateway_shim}
exec \"$pure_bin\" conduit \"\$@\""
  else
    ops_domain_dispatch="ops_bin_candidate=\"$ops_bin\"
if [ \"\${1:-}\" = \"setup\" ] && [ -n \"\${INFRING_WORKSPACE_ROOT:-}\" ]; then
  for dev_ops_bin in \"\$INFRING_WORKSPACE_ROOT/target/release/infring-ops\" \"\$INFRING_WORKSPACE_ROOT/target/debug/infring-ops\"; do
    if [ -x \"\$dev_ops_bin\" ]; then
      ops_bin_candidate=\"\$dev_ops_bin\"
      break
    fi
  done
fi
ops_domain=\"\${INFRING_OPS_DOMAIN:-}\"
if [ -z \"\$ops_domain\" ]; then
  if \"\$ops_bin_candidate\" infringctl --help >/dev/null 2>&1; then
    ops_domain=\"infringctl\"
  else
    ops_domain=\"infringctl\"
  fi
fi
exec \"\$ops_bin_candidate\" \"\$ops_domain\" \"\$@\""
    write_wrapper "infring" "${infring_help_shim}
${gateway_shim}
${ops_domain_dispatch}"
    write_wrapper "infringctl" "${gateway_shim}
${ops_domain_dispatch}"
  fi

  if [ -n "$daemon_wrapper_body" ]; then
    write_wrapper "infringd" "$daemon_wrapper_body"
  else
    if is_truthy "$INSTALL_PURE"; then
      echo "[infring install] no daemon binary available for pure mode" >&2
      exit 1
    fi
    write_wrapper "infringd" "exec \"$ops_bin\" spine \"\$@\""
  fi

  workspace_ready=0
  workspace_refresh_required=0
  workspace_refresh_applied=0
  workspace_refresh_reason=""
  workspace_refresh_tag_state_missing=0
  previous_workspace_release_tag=""
  workspace_release_tag_written=0
  workspace_release_tag_write_verified=0
  export INFRING_WORKSPACE_ROOT="$WORKSPACE_DIR"
  if is_truthy "$INSTALL_PURE"; then
    echo "[infring install] pure mode: skipping workspace runtime bootstrap"
    INSTALL_RUNTIME_CONTRACT_MODE="pure_profile"
    INSTALL_RUNTIME_CONTRACT_OK=1
    INSTALL_CLIENT_RUNTIME_MODE="pure_profile"
  else
    previous_workspace_release_tag="$(read_workspace_release_tag "$WORKSPACE_DIR" 2>/dev/null || true)"
    if is_truthy "$INSTALL_REPAIR"; then
      workspace_refresh_reason="repair_mode"
    elif ! workspace_has_runtime "$WORKSPACE_DIR"; then
      workspace_refresh_reason="runtime_missing"
    elif [ -z "$previous_workspace_release_tag" ]; then
      workspace_refresh_reason="tag_state_missing"
      workspace_refresh_tag_state_missing=1
    elif [ "$previous_workspace_release_tag" != "$version" ]; then
      workspace_refresh_reason="release_tag_changed"
    fi

    if [ -z "$workspace_refresh_reason" ]; then
      workspace_ready=1
      workspace_refresh_required=0
      workspace_refresh_applied=0
      echo "[infring install] workspace runtime already present at $WORKSPACE_DIR"
    else
      workspace_refresh_required=1
      echo "[infring install] refreshing workspace runtime at $WORKSPACE_DIR ($workspace_refresh_reason)"
      if install_client_bundle "$version" "$triple" "$WORKSPACE_DIR"; then
        workspace_ready=1
        workspace_refresh_applied=1
        if is_truthy "$INSTALL_FULL"; then
          echo "[infring install] full mode enabled: workspace runtime installed at $WORKSPACE_DIR"
        else
          echo "[infring install] workspace runtime installed at $WORKSPACE_DIR"
        fi
      elif install_workspace_from_source_fallback "$version" "$WORKSPACE_DIR"; then
        workspace_ready=1
        workspace_refresh_applied=1
        echo "[infring install] workspace runtime bootstrapped from source fallback at $WORKSPACE_DIR"
      fi
    fi
    if [ "$workspace_ready" != "1" ]; then
      echo "[infring install] failed to provision workspace runtime for this release/platform" >&2
      echo "[infring install] expected workspace runtime root: $WORKSPACE_DIR" >&2
      exit 1
    fi
    if [ "$workspace_refresh_required" = "1" ] && [ "$workspace_refresh_applied" != "1" ]; then
      echo "[infring install] workspace runtime refresh required but not applied ($workspace_refresh_reason); refusing release-tag update" >&2
      exit 1
    fi
    ensure_workspace_source_member_closure "$version" "$WORKSPACE_DIR" || exit 1
    ensure_workspace_setup_wizard_compat "$WORKSPACE_DIR" || true
    if ! verify_workspace_runtime_contract "$WORKSPACE_DIR"; then
      repair_workspace_runtime_contract "$version" "$triple" "$WORKSPACE_DIR" || exit 1
      INSTALL_RUNTIME_CONTRACT_MODE="source_repaired"
      INSTALL_RUNTIME_CONTRACT_OK=1
    fi
    INSTALL_CLIENT_RUNTIME_MODE="source_workspace"
    write_workspace_release_tag "$WORKSPACE_DIR" "$version" || exit 1
    workspace_release_tag_written=1
    if workspace_release_tag_matches "$WORKSPACE_DIR" "$version"; then
      workspace_release_tag_write_verified=1
    else
      echo "[infring install] workspace release-tag state write verification failed for $WORKSPACE_DIR" >&2
      exit 1
    fi
    force_workspace_runtime_mode_source "$WORKSPACE_DIR" || exit 1
    ensure_node_runtime_notice || true
    if ! ensure_ollama_runtime_notice; then
      if is_truthy "$INSTALL_OLLAMA" || [ "$OLLAMA_INSTALL_CONFIRMED" = "1" ] || is_truthy "$INSTALL_REQUIRE_MODEL_READY"; then
        echo "[infring install] requested local model bootstrap did not complete; aborting install." >&2
        exit 1
      fi
    fi
    install_summary_note "workspace_runtime_refresh_required: ${workspace_refresh_required}"
    install_summary_note "workspace_runtime_refresh_applied: ${workspace_refresh_applied}"
    install_summary_note "workspace_runtime_refresh_reason: ${workspace_refresh_reason:-none}"
    install_summary_note "workspace_runtime_tag_state_missing: ${workspace_refresh_tag_state_missing}"
    install_summary_note "workspace_release_tag_previous: ${previous_workspace_release_tag:-}"
    install_summary_note "workspace_release_tag_current: ${version}"
    install_summary_note "workspace_release_tag_written: ${workspace_release_tag_written}"
    install_summary_note "workspace_release_tag_write_verified: ${workspace_release_tag_write_verified}"
    install_summary_note "ollama_install_confirmed: ${OLLAMA_INSTALL_CONFIRMED}"
    install_summary_note "ollama_last_model_count: ${OLLAMA_LAST_MODEL_COUNT}"
    ensure_runtime_node_module_closure "$WORKSPACE_DIR" || exit 1
    # Write activation script before smoke tests so users always have a recovery path
    # even if a smoke check hangs or fails after artifacts are already installed.
    write_path_activate_script
    run_post_install_smoke_tests "$INSTALL_DIR" "$WORKSPACE_DIR" || exit 1
  fi

  INSTALL_SHELL_DEFAULT="$(infer_install_shell_default 2>/dev/null || printf '%s\n' "ui")"
  INSTALL_SHELL_FALLBACK="${INFRING_GATEWAY_FALLBACK_SHELL:-terminal}"
  write_shell_launch_config "$WORKSPACE_DIR" "$INSTALL_SHELL_DEFAULT" "$INSTALL_SHELL_FALLBACK" || true

  WORKSPACE_REFRESH_REQUIRED="${workspace_refresh_required}"
  WORKSPACE_REFRESH_APPLIED="${workspace_refresh_applied}"
  WORKSPACE_REFRESH_REASON="${workspace_refresh_reason}"
  WORKSPACE_REFRESH_TAG_STATE_MISSING="${workspace_refresh_tag_state_missing}"
  WORKSPACE_RELEASE_TAG_PREVIOUS="${previous_workspace_release_tag}"
  WORKSPACE_RELEASE_TAG_CURRENT="${version}"
  WORKSPACE_RELEASE_TAG_WRITTEN="${workspace_release_tag_written}"
  WORKSPACE_RELEASE_TAG_WRITE_VERIFIED="${workspace_release_tag_write_verified}"
  install_summary_note "workspace_runtime_refresh_required: ${WORKSPACE_REFRESH_REQUIRED}"
  install_summary_note "workspace_runtime_refresh_applied: ${WORKSPACE_REFRESH_APPLIED}"
  install_summary_note "workspace_runtime_refresh_reason: ${WORKSPACE_REFRESH_REASON:-none}"
  install_summary_note "workspace_runtime_tag_state_missing: ${WORKSPACE_REFRESH_TAG_STATE_MISSING}"
  install_summary_note "workspace_release_tag_previous: ${WORKSPACE_RELEASE_TAG_PREVIOUS:-}"
  install_summary_note "workspace_release_tag_current: ${WORKSPACE_RELEASE_TAG_CURRENT:-}"
  install_summary_note "workspace_release_tag_written: ${WORKSPACE_RELEASE_TAG_WRITTEN}"
  install_summary_note "workspace_release_tag_write_verified: ${WORKSPACE_RELEASE_TAG_WRITE_VERIFIED}"

  emit_install_completion_card "$version" "$INSTALL_DIR/infring"
  echo "[infring install] installed commands: infring, infringctl, infringd"
  if [ -x "$INSTALL_DIR/infring" ] && [ -x "$INSTALL_DIR/infringctl" ] && [ -x "$INSTALL_DIR/infringd" ]; then
    echo "[infring install] sanity check: wrapper binaries installed"
  else
    echo "[infring install] sanity check failed: wrapper binaries missing under $INSTALL_DIR" >&2
    exit 1
  fi

  ensure_path_shims
  persist_path_for_shell
  write_path_activate_script
  quickstart_prefix=""
  if [ -n "$PATH_ACTIVATE_FILE" ]; then
    quickstart_prefix=". \"$PATH_ACTIVATE_FILE\" && "
  fi
  if command -v infring >/dev/null 2>&1; then
    echo "[infring install] PATH check: infring command available in installer shell"
    if [ -n "$PATH_ACTIVATE_FILE" ]; then
      echo "[infring install] note: activate in your current shell with . \"$PATH_ACTIVATE_FILE\""
    elif [ -n "$PATH_PERSISTED_FILE" ]; then
      echo "[infring install] note: activate in your current shell with . \"$PATH_PERSISTED_FILE\""
    fi
  else
    case ":$PATH:" in
      *":$INSTALL_DIR:"*)
        echo "[infring install] PATH check: install dir is on PATH but shell may require command hash refresh"
        echo "[infring install] activate now: hash -r 2>/dev/null || true"
        if [ -z "$quickstart_prefix" ]; then
          quickstart_prefix="hash -r 2>/dev/null || true && "
        fi
        ;;
      *)
        if [ -n "$PATH_SHIM_DIR" ]; then
          echo "[infring install] PATH notice: wrappers installed in $INSTALL_DIR and linked from $PATH_SHIM_DIR"
          if [ -n "$PATH_ACTIVATE_FILE" ]; then
            echo "[infring install] activate now: . \"$PATH_ACTIVATE_FILE\""
            quickstart_prefix=". \"$PATH_ACTIVATE_FILE\" && "
          fi
        elif [ -n "$PATH_PERSISTED_FILE" ]; then
          echo "[infring install] activate now: . \"$PATH_PERSISTED_FILE\""
          echo "[infring install] fallback: export PATH=\"$INSTALL_DIR:\$PATH\""
          quickstart_prefix=". \"$PATH_PERSISTED_FILE\" && "
          if [ -n "$PATH_ACTIVATE_FILE" ]; then
            echo "[infring install] portable activate script: . \"$PATH_ACTIVATE_FILE\""
          fi
        else
          echo "[infring install] add to PATH: export PATH=\"$INSTALL_DIR:\$PATH\""
          if [ -n "$PATH_ACTIVATE_FILE" ]; then
            echo "[infring install] portable activate script: . \"$PATH_ACTIVATE_FILE\""
            quickstart_prefix=". \"$PATH_ACTIVATE_FILE\" && "
          else
            quickstart_prefix="export PATH=\"$INSTALL_DIR:\$PATH\" && "
          fi
        fi
        ;;
    esac
  fi
  if [ -n "$PATH_ACTIVATE_FILE" ]; then
    echo "[infring install] activation script: $PATH_ACTIVATE_FILE"
  fi
  echo "[infring install] run now (direct path): \"$INSTALL_DIR/infring\" --help"
  echo "[infring install] quickstart now (direct path): \"$INSTALL_DIR/infring\" gateway"
  echo "[infring install] run: ${quickstart_prefix}infring --help"
  echo "[infring install] quickstart: ${quickstart_prefix}infring gateway"
  echo "[infring install] stop: ${quickstart_prefix}infring gateway stop"
  if [ "$INSTALL_DASHBOARD_SMOKE_PASSED" = "1" ]; then
    echo "[infring install] dashboard smoke passed"
  else
    echo "[infring install] dashboard smoke skipped or not run in this install mode"
  fi
  echo "[infring install] note: installer validates dashboard startup but does not keep it running"
  echo "[infring install] start dashboard now: ${quickstart_prefix}infring gateway"
  echo "[infring install] stop dashboard: ${quickstart_prefix}infring gateway stop"
  print_shell_activation_snippets
  node_summary_bin="$(resolve_node_binary_path 2>/dev/null || true)"
  if [ -n "$node_summary_bin" ]; then
    node_summary_ver="$("$node_summary_bin" --version 2>/dev/null || true)"
    install_summary_note "node_binary: ${node_summary_bin}"
    install_summary_note "node_version: ${node_summary_ver}"
  else
    install_summary_note "node_binary: missing"
  fi
  if command -v ollama >/dev/null 2>&1; then
    ollama_summary_bin="$(command -v ollama 2>/dev/null || true)"
    OLLAMA_LAST_MODEL_COUNT="$(ollama_model_count)"
    install_summary_note "ollama_binary: ${ollama_summary_bin:-ollama}"
    install_summary_note "ollama_model_count: ${OLLAMA_LAST_MODEL_COUNT}"
    echo "[infring install] local models: ollama list"
    if [ "$OLLAMA_LAST_MODEL_COUNT" -gt 0 ]; then
      echo "[infring install] local model readiness: ${OLLAMA_LAST_MODEL_COUNT} model(s) available"
    else
      echo "[infring install] local model readiness: 0 models detected (run 'ollama pull $(normalize_ollama_model_ref "$OLLAMA_STARTER_MODEL")')"
    fi
  else
    install_summary_note "ollama_binary: missing"
    install_summary_note "ollama_model_count: 0"
    echo "[infring install] local models setup: install Ollama (https://ollama.com/download), then run 'ollama serve' and 'ollama pull $(normalize_ollama_model_ref "$OLLAMA_STARTER_MODEL")'"
  fi
  if ! install_summary_mark_success; then
    INSTALL_SUMMARY_FAILURE_REASON="summary_sync_failed_after_success"
    echo "[infring install] summary contract failed: could not write success summary" >&2
    exit 1
  fi
  if ! verify_install_summary_success_contract; then
    INSTALL_SUMMARY_FAILURE_REASON="summary_contract_verification_failed"
    exit 1
  fi
  echo "[infring install] summary log: $INSTALL_SUMMARY_FILE"
  emit_install_success_summary "$version" "$triple" "$quickstart_prefix"

  if [ -n "$SOURCE_FALLBACK_TMP" ] && [ -d "$SOURCE_FALLBACK_TMP" ]; then
    rm -rf "$SOURCE_FALLBACK_TMP"
  fi
  if [ -n "$CHECKSUM_MANIFEST_TMP_DIR" ] && [ -d "$CHECKSUM_MANIFEST_TMP_DIR" ]; then
    rm -rf "$CHECKSUM_MANIFEST_TMP_DIR"
  fi
}

main "$@"
