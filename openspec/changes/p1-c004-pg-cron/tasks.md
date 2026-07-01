# p1-c004 — Tasks

- [ ] Read `images/postgres18/Dockerfile` — understand current base image and package installs
- [ ] Determine pg_cron PG18 package availability (check `apt-cache search pg-cron` output for the target Debian/Ubuntu base)
- [ ] Add pg_cron installation to Dockerfile: either APT package or build from source
- [ ] Add `pg_cron` to `shared_preload_libraries` in postgresql.conf layer of Dockerfile
- [ ] Add `cron.database_name = 'postgres'` (or target DB name) to postgresql.conf
- [ ] Add `CREATE EXTENSION IF NOT EXISTS pg_cron` to init SQL or Docker ENTRYPOINT script
- [ ] Add `CREATE EXTENSION IF NOT EXISTS pgcrypto` (required by flint_hooks HMAC) if not already present
- [ ] Register webhook outbox GC cron job in init SQL
- [ ] Register meta-full-refresh cron job stub (function reference — will be resolved when p1-c009 ships)
- [ ] Build and run the Postgres 18 container locally: verify startup, `\dx` shows pg_cron, `\df cron.*` shows scheduler functions
- [ ] GATE: `SELECT * FROM cron.job` returns webhook-gc and meta-full-refresh rows after init
