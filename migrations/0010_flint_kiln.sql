-- Migration: 0010_flint_kiln.sql
-- Kiln function registry + artifact store (Phase 6 p6-c001/c002).
-- Stores WASM component metadata, bytes, and execution records.

CREATE SCHEMA IF NOT EXISTS flint_kiln;

-- ── functions ────────────────────────────────────────────────────────────────
-- Each row is a versioned, signed function registration.

CREATE TABLE IF NOT EXISTS flint_kiln.functions (
    id              uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    name            text NOT NULL,
    version         text NOT NULL,
    content_digest  text NOT NULL,
    manifest        jsonb NOT NULL,
    active          boolean NOT NULL DEFAULT true,
    registered_at   timestamptz NOT NULL DEFAULT now(),
    UNIQUE (name, version)
);

CREATE INDEX IF NOT EXISTS kiln_functions_name_idx
    ON flint_kiln.functions (name, active);

-- ── artifacts ────────────────────────────────────────────────────────────────
-- Raw WASM component bytes, keyed by content_digest (SHA-256 hex).

CREATE TABLE IF NOT EXISTS flint_kiln.artifacts (
    content_digest  text PRIMARY KEY,
    bytes           bytea NOT NULL,
    created_at      timestamptz NOT NULL DEFAULT now()
);

-- ── invocations ──────────────────────────────────────────────────────────────
-- Audit log of function executions (non-durable; truncated by pg_cron).

CREATE TABLE IF NOT EXISTS flint_kiln.invocations (
    id              bigserial PRIMARY KEY,
    function_id     uuid NOT NULL REFERENCES flint_kiln.functions(id),
    run_id          text,
    status          text NOT NULL DEFAULT 'pending'
        CHECK (status IN ('pending', 'running', 'completed', 'failed')),
    fuel_used       bigint,
    duration_ms     int,
    error           text,
    invoked_at      timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS kiln_invocations_function_idx
    ON flint_kiln.invocations (function_id, invoked_at DESC);

-- ── hook queue (kiln target) ─────────────────────────────────────────────────
-- Entries from flint.webhook_outbox WHERE target_type='kiln' are routed here
-- by the Kiln BGW when Phase 6 is live.
-- The outbox row is marked 'delivered' after the function invocation succeeds.
