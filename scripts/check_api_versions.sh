#!/usr/bin/env bash
# scripts/check_api_versions.sh — Verify API doc versions match .env.example.
#
# Enforces that the version integer embedded in docs/api/a2ui.md and
# docs/api/kiln-abi.md matches the corresponding variable in .env.example.
# Run on every CI push (see .github/workflows/ci.yml).
#
# Fails with a clear error if any pair is out of sync, guiding the author
# to update BOTH the doc and .env.example in the same commit.
#
# Parsing contracts:
#   docs/api/a2ui.md      — line:  **Current version:** `N`
#   docs/api/kiln-abi.md  — line:  **Current ABI version:** `N`
#   .env.example          — lines: FLINT_A2UI_API_VERSION=N
#                                  FLINT_KILN_ABI_VERSION=N
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && cd .. && pwd)"
FAIL=0

# ── helpers ───────────────────────────────────────────────────────────────────

# Extract the first backtick-quoted integer from a line matching a pattern.
# Usage: extract_version <file> <grep_pattern>
extract_version() {
    local file="$1"
    local pattern="$2"
    local line
    line=$(grep -F "${pattern}" "${file}" | head -1 || true)
    if [ -z "${line}" ]; then
        echo ""
        return
    fi
    # Extract the first backtick-quoted integer.
    echo "${line}" | grep -oE '[0-9]+' | head -1 || true
}

# Extract a variable value from .env.example.
# Usage: extract_env_var <varname>
extract_env_var() {
    local var="$1"
    local line
    line=$(grep "^${var}=" "${ROOT}/.env.example" | head -1 || true)
    if [ -z "${line}" ]; then
        echo ""
        return
    fi
    echo "${line}" | cut -d'=' -f2- | tr -d ' \\r' | head -1
}

check() {
    local name="$1"
    local doc_version="$2"
    local env_version="$3"
    local doc_file="$4"
    local env_var="$5"

    if [ -z "$doc_version" ]; then
        echo "ERROR: Could not parse version from ${doc_file}" >&2
        echo "       Expected a line matching: **Current*version:** \`N\`" >&2
        FAIL=1
        return
    fi

    if [ -z "$env_version" ]; then
        echo "ERROR: ${env_var} not found in .env.example" >&2
        FAIL=1
        return
    fi

    if [ "$doc_version" = "$env_version" ]; then
        printf "  %-35s doc=%-4s env=%-4s  OK\n" "$name" "$doc_version" "$env_version"
    else
        printf "  %-35s doc=%-4s env=%-4s  MISMATCH\n" "$name" "$doc_version" "$env_version" >&2
        echo "" >&2
        echo "  To fix: update BOTH in the same commit:" >&2
        echo "    1. Change the version integer in ${doc_file}" >&2
        echo "       Line format: **Current*version:** \`N\`" >&2
        echo "    2. Change ${env_var}=N in .env.example" >&2
        echo "    3. Update MIGRATION.md with the breaking-change description" >&2
        echo "    4. See docs/api/versioning.md for the full policy" >&2
        echo "" >&2
        FAIL=1
    fi
}

# ── checks ────────────────────────────────────────────────────────────────────

echo "Checking API version consistency..."
echo ""

# A2UI HTTP API
A2UI_DOC_VER=$(extract_version "${ROOT}/docs/api/a2ui.md" "Current version")
A2UI_ENV_VER=$(extract_env_var "FLINT_A2UI_API_VERSION")
check "A2UI HTTP API" "$A2UI_DOC_VER" "$A2UI_ENV_VER" "docs/api/a2ui.md" "FLINT_A2UI_API_VERSION"

# Kiln WIT ABI
KILN_DOC_VER=$(extract_version "${ROOT}/docs/api/kiln-abi.md" "Current ABI version")
KILN_ENV_VER=$(extract_env_var "FLINT_KILN_ABI_VERSION")
check "Kiln WIT ABI" "$KILN_DOC_VER" "$KILN_ENV_VER" "docs/api/kiln-abi.md" "FLINT_KILN_ABI_VERSION"

# ── result ────────────────────────────────────────────────────────────────────

echo ""
if [ "$FAIL" -eq 0 ]; then
    echo "All API version checks passed."
    exit 0
else
    echo "API version check FAILED — see errors above." >&2
    exit 1
fi
