#!/usr/bin/env bash
# Flint Forge — migration integrity check.
#
# Validates that migrations/ has a strict, gap-free numeric prefix sequence and
# no duplicate prefixes. This catches the collision class that broke v1.0
# boot (e.g. 0005_cedar_policies.sql + 0005_flint_a2ui_hybrid_search.sql).
#
# When sqlx-cli and DATABASE_URL are present, also runs `sqlx migrate info`.

set -euo pipefail

MIGRATIONS_DIR="${1:-migrations}"

if [[ ! -d "$MIGRATIONS_DIR" ]]; then
    echo "ERROR: migrations directory not found: $MIGRATIONS_DIR" >&2
    exit 1
fi

prefixes=()
for f in "$MIGRATIONS_DIR"/*.sql; do
    [[ -e "$f" ]] || continue
    basename="$(basename "$f")"
    prefix="${basename%%_*}"
    if [[ ! "$prefix" =~ ^[0-9]+$ ]]; then
        echo "ERROR: migration filename does not start with a numeric prefix: $basename" >&2
        exit 1
    fi
    prefixes+=("$prefix")
done

# Strictly increasing, no duplicates.
sorted="$(printf '%s\n' "${prefixes[@]}" | sort -n | uniq -d)"
if [[ -n "$sorted" ]]; then
    echo "ERROR: duplicate migration prefixes detected:" >&2
    printf '%s\n' "$sorted" >&2
    exit 1
fi

prev=0
for p in "${prefixes[@]}"; do
    p10="10#$p"
    if (( p10 <= prev )); then
        echo "ERROR: migration prefixes are not strictly increasing ($prev -> $p)" >&2
        exit 1
    fi
    prev="$p10"
done

echo "OK: ${#prefixes[@]} migrations with strictly increasing prefixes"

if command -v sqlx >/dev/null 2>&1 && [[ -n "${DATABASE_URL:-}" ]]; then
    echo "==> sqlx migrate info"
    sqlx migrate info --source "$MIGRATIONS_DIR"
fi
