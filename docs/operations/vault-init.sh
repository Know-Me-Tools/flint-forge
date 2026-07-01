#!/usr/bin/env bash
# vault-init.sh — Generate and wrap the Flint Vault DEK for initial deployment.
#
# Run this ONCE per environment during initial setup. The script:
#   1. Generates a 32-byte random DEK using the OS CSPRNG
#   2. Wraps (encrypts) it with the Azure Key Vault KEK via RSA-OAEP-256
#   3. Shreds the plaintext DEK from disk immediately after wrapping
#   4. Prints the base64-encoded wrapped DEK for storage in a Kubernetes secret
#
# Store the output in: kubectl secret FLINT_VAULT_DEK_WRAPPED
# Do NOT commit, log, or email the wrapped DEK value.
#
# Usage:
#   VAULT_NAME=my-vault KEY_NAME=flint-vault-kek ./vault-init.sh
#
# Prerequisites:
#   - az CLI authenticated with Key Wrap permission on the vault
#   - openssl on PATH
#   - shred available (GNU coreutils; on macOS install via: brew install coreutils)

set -euo pipefail

VAULT_NAME="${VAULT_NAME:?VAULT_NAME env var required (Azure Key Vault name)}"
KEY_NAME="${KEY_NAME:-flint-vault-kek}"
DEK_FILE=""

cleanup() {
    if [[ -n "$DEK_FILE" && -f "$DEK_FILE" ]]; then
        shred -u "$DEK_FILE" 2>/dev/null || rm -f "$DEK_FILE"
    fi
}
trap cleanup EXIT INT TERM

# Verify required tools are present before touching any key material.
for cmd in openssl az shred base64; do
    if ! command -v "$cmd" &>/dev/null; then
        echo "ERROR: required command not found: $cmd" >&2
        exit 1
    fi
done

echo "Step 1: Generating 32-byte DEK via CSPRNG..." >&2
DEK_FILE=$(mktemp)
openssl rand -out "$DEK_FILE" 32

# Azure Key Vault wrap requires the plaintext as base64 (not raw bytes).
DEK_B64=$(base64 < "$DEK_FILE")

echo "Step 2: Wrapping DEK with Azure Key Vault (${VAULT_NAME}/${KEY_NAME})..." >&2
WRAPPED=$(az keyvault key wrap \
    --vault-name "$VAULT_NAME" \
    --name "$KEY_NAME" \
    --algorithm RSA-OAEP-256 \
    --value "$DEK_B64" \
    --query result \
    --output tsv)

echo "Step 3: Shredding plaintext DEK from disk..." >&2
shred -u "$DEK_FILE"
DEK_FILE=""  # Prevent double-shred in trap.

echo "" >&2
echo "=== Vault Init Complete ===" >&2
echo "" >&2
echo "Store the following value in Kubernetes secret FLINT_VAULT_DEK_WRAPPED:" >&2
echo "" >&2

# The wrapped DEK is the only output to stdout — redirect to a file or pipe
# directly into kubectl. Everything else goes to stderr.
echo "$WRAPPED"

echo "" >&2
echo "Example:" >&2
echo "  kubectl create secret generic flint-vault-secrets \\" >&2
echo "    --namespace flint \\" >&2
echo "    --from-literal=FLINT_VAULT_DEK_WRAPPED=\"\$(./vault-init.sh)\" \\" >&2
echo "    --from-literal=FLINT_VAULT_UNWRAP_CMD=/opt/flint/unwrap-dek.sh \\" >&2
echo "    --from-literal=AZURE_VAULT_NAME=${VAULT_NAME}" >&2
echo "" >&2
echo "WARNING: The wrapped DEK above is sensitive key material." >&2
echo "WARNING: Do NOT log, commit, or email this value." >&2
echo "WARNING: Treat the Kubernetes secret as equivalent to the plaintext DEK." >&2
