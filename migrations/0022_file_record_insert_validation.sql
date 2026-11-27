-- Let the records policy check ownership before validating private type data.
-- The validator only inspects NEW. An AFTER INSERT error rolls back the write
-- in the same statement, before the adapter can issue a mutation receipt.
CREATE OR REPLACE TRIGGER file_record_guard
    BEFORE UPDATE ON flint_files.records
    FOR EACH ROW EXECUTE FUNCTION flint_files.check_record();

CREATE TRIGGER file_record_insert_guard
    AFTER INSERT ON flint_files.records
    FOR EACH ROW EXECUTE FUNCTION flint_files.check_record();
