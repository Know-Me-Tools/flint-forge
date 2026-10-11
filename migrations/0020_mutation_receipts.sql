-- Retry receipts for offline-first REST mutations.
-- The receipt and target mutation are committed by PgRest in one transaction.

CREATE SCHEMA IF NOT EXISTS flint;

CREATE TABLE IF NOT EXISTS flint.mutation_receipts (
    subject             text        NOT NULL,
    mutation_id         text        NOT NULL,
    request_fingerprint text        NOT NULL,
    response            jsonb       NOT NULL,
    created_at          timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (subject, mutation_id),
    CHECK (length(mutation_id) BETWEEN 1 AND 128),
    CHECK (length(request_fingerprint) = 64)
);

ALTER TABLE flint.mutation_receipts ENABLE ROW LEVEL SECURITY;
ALTER TABLE flint.mutation_receipts FORCE ROW LEVEL SECURITY;

DROP POLICY IF EXISTS mutation_receipts_read_own ON flint.mutation_receipts;
CREATE POLICY mutation_receipts_read_own
    ON flint.mutation_receipts FOR SELECT TO authenticated
    USING (subject = auth.uid());

DROP POLICY IF EXISTS mutation_receipts_insert_own ON flint.mutation_receipts;
CREATE POLICY mutation_receipts_insert_own
    ON flint.mutation_receipts FOR INSERT TO authenticated
    WITH CHECK (subject = auth.uid());

GRANT USAGE ON SCHEMA flint TO authenticated, service_role;
GRANT SELECT, INSERT ON flint.mutation_receipts TO authenticated, service_role;

COMMENT ON TABLE flint.mutation_receipts IS
    'Per-subject REST mutation receipts used to replay retries without duplicate writes.';
