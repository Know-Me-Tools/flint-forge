#!/usr/bin/env bash
# p16-c009: coverage gate — enforces >=90% line coverage on crates touched by
# this change, without punishing legacy low-coverage crates the PR didn't
# touch. Workspace-wide coverage is always printed as a visible metric
# (task: "track workspace-wide coverage even if not gated at 90% yet").
#
# Usage: ./scripts/check_coverage.sh [base-ref]
#   base-ref defaults to $GITHUB_BASE_REF (set by GitHub Actions on pull_request
#   events) or "HEAD~1" on a plain push (best-effort single-commit diff).
#
# Requires: cargo-llvm-cov (`cargo install cargo-llvm-cov --locked`) and the
# llvm-tools-preview rustup component (`rustup component add llvm-tools-preview`).
set -euo pipefail

THRESHOLD=90
BASE_REF="${1:-${GITHUB_BASE_REF:-HEAD~1}}"
REPORT="target/llvm-cov-report.json"

# Resolve the base ref to a real commit so `git diff` works whether we were
# handed a branch name (PR) or already a commit-ish (push).
if git rev-parse --verify --quiet "origin/${BASE_REF}" >/dev/null; then
    BASE_COMMIT="origin/${BASE_REF}"
elif git rev-parse --verify --quiet "${BASE_REF}" >/dev/null; then
    BASE_COMMIT="${BASE_REF}"
else
    echo "warning: could not resolve base ref '${BASE_REF}'; skipping the changed-crate gate (workspace total still reported)." >&2
    BASE_COMMIT=""
fi

echo "==> Running cargo llvm-cov --workspace (this covers every crate once; changed-crate filtering happens after)"
mkdir -p target
cargo llvm-cov --workspace --json --output-path "$REPORT" --quiet

WORKSPACE_PCT="$(python3 -c "import json; print(round(json.load(open('$REPORT'))['data'][0]['totals']['lines']['percent'], 2))")"
echo ""
echo "==> Workspace-wide line coverage: ${WORKSPACE_PCT}%"
if [ -n "${GITHUB_STEP_SUMMARY:-}" ]; then
    echo "**Workspace-wide line coverage: ${WORKSPACE_PCT}%**" >> "$GITHUB_STEP_SUMMARY"
fi

if [ -z "$BASE_COMMIT" ]; then
    exit 0
fi

CHANGED_CRATES="$(git diff --name-only "${BASE_COMMIT}...HEAD" -- 'crates/*.rs' 'crates/**/*.rs' \
    | sed -n 's#^crates/\([^/]*\)/.*#\1#p' \
    | sort -u)"

if [ -z "$CHANGED_CRATES" ]; then
    echo "==> No crates/**/*.rs changes vs ${BASE_COMMIT}; nothing to gate."
    exit 0
fi

echo "==> Changed crates (gated at >=${THRESHOLD}% line coverage): ${CHANGED_CRATES}"
echo ""

FAILED=0
for crate in $CHANGED_CRATES; do
    # Skip pgrx crates — excluded from the default workspace, `cargo llvm-cov
    # --workspace` above never built or measured them.
    case "$crate" in
        ext-flint-*) echo "  - ${crate}: skipped (pgrx crate, not in default workspace)"; continue ;;
    esac

    PCT="$(python3 - "$REPORT" "$crate" <<'PYEOF'
import json, sys
report, crate = sys.argv[1], sys.argv[2]
data = json.load(open(report))["data"][0]["files"]
prefix = f"/crates/{crate}/src/"
covered = 0
total = 0
found = False
for f in data:
    if prefix in f["filename"]:
        found = True
        covered += f["summary"]["lines"]["covered"]
        total += f["summary"]["lines"]["count"]
if not found or total == 0:
    print("n/a")
else:
    print(round(100.0 * covered / total, 2))
PYEOF
)"

    if [ "$PCT" = "n/a" ]; then
        echo "  - ${crate}: n/a (no measured lines — binary-only crate or no src/ files changed)"
        continue
    fi

    PASS="$(python3 -c "print('yes' if float('$PCT') >= $THRESHOLD else 'no')")"
    echo "  - ${crate}: ${PCT}% (threshold ${THRESHOLD}%) — $([ "$PASS" = yes ] && echo PASS || echo FAIL)"
    if [ "$PASS" != yes ]; then
        FAILED=1
    fi
done

echo ""
if [ "$FAILED" -ne 0 ]; then
    echo "FAIL: at least one changed crate is below the ${THRESHOLD}% line-coverage threshold." >&2
    exit 1
fi
echo "PASS: all changed crates meet the ${THRESHOLD}% line-coverage threshold."
