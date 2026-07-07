# p7b-c001 Tasks — Epoch Interruption

## Tasks

- [ ] Add `cfg.epoch_interruption(true)` to `EdgeRuntime::new()` in `fke-runtime/src/lib.rs`
- [ ] Read `KILN_EPOCH_INTERVAL_MS` env var in `EdgeRuntime::new()`; skip ticker when `0`
- [ ] Spawn background ticker task: `tokio::time::interval` + `engine.increment_epoch()` loop
- [ ] Add `_epoch_ticker: tokio::task::JoinHandle<()>` field to `EdgeRuntime` struct
- [ ] Add `store.set_epoch_deadline(1)` in `EdgeRuntime::handle()` after `store.set_fuel()`
- [ ] Add `#[allow(dead_code)]` to `_epoch_ticker` field (or `let _ =` if not stored)
- [ ] Test: fast component still returns HTTP 200 with 1 ms tick interval
- [ ] `cargo clippy -p fke-runtime -- -D warnings` clean
- [ ] `cargo test -p fke-runtime` passes (all 11 existing tests + new epoch test)
