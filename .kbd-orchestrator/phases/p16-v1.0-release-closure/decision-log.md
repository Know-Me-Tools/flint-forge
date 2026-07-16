# Decision Log — p16-v1.0-release-closure

D-001 · Delivery model is self-hosted OSS [spec · 2026-07-09]
D-002 · Spec backend is native-kbd, not OpenSpec [spec · 2026-07-09]
D-003 · Realtime fails closed; default inverted to `listen` [spec · 2026-07-09]
D-004 · `change_backend` pinned in project.json; wait budget capped at 3 [plan · 2026-07-09]

---

### D-004 — Pin `change_backend`; cap the wait budget at 3

**TL;DR:** Wrote `"change_backend": "native-kbd"` into `project.json`, and
allocated exactly 3 test-waits for the phase with an explicit halt rule.

**Why (backend pin):** `/kbd-plan`'s own detection rule says *"`openspec/` exists
at project root → emit `/opsx:new`."* That directly contradicts D-002. Following
it would have created a **second, competing task surface** for the same six
changes — one native under `.kbd-orchestrator/changes/`, one under
`openspec/changes/` — which is the exact divergence that broke p15, where the
phase was planned as OpenSpec, executed as native-tool, and `/kbd-apply` found no
tasks to walk. Pinning the field makes detection deterministic for every
downstream tool. Verified `kbd-apply list p16-c001-…` reads the native
`tasks.json` correctly, so the surface is walkable despite `detect` reporting
`openspec`.

**Why (wait budget):** the Integration-First policy allocates 3 test-waits per
epoch. p15 spent **6** — recorded as debt in `p15/handoffs/reflect.json` — partly
by validating pieces as they were written rather than at integration
checkpoints. c003 is the only genuine unknown in this phase (0 successes in 8
runs; nobody has seen the job past its build step), so the budget is steered
toward it and away from premature per-function validation.

**Alternatives rejected:** Follow the skill's detection rule literally (creates
the competing surface); leave `change_backend` unset (leaves the same trap armed
for the next tool); allocate an open-ended budget (this is how p15 reached 6
without anyone noticing).

**Consequence:** If the budget reaches 3 with c003 still red, the phase **halts
and reports** rather than overrunning. Recorded in `plan.md` → "Wait budget" and
`handoffs/plan.json` → `wait_budget.escalation`.

**Provenance:** research (skill rule vs D-002 conflict; p15 wait-count evidence)

---

### D-001 — Delivery model is self-hosted OSS

**TL;DR:** Customers run Flint Forge themselves from the MIT source and Helm
chart. This scopes out the entire managed-service operational surface.

**Why:** The readiness bar for "ready for real customers" is not one bar — it is
three disjoint ones. Assess and Analyze both stalled on this. Speccing against a
guess would have baked the wrong bar into every downstream task: a managed SaaS
needs a status page, on-call rotation, DPA, and SLA; self-hosted OSS needs none
of them and needs a far better upgrade guide, because the operator self-serves.

Repository evidence pointed toward self-hosted (MIT `LICENSE`, Helm chart,
"sovereign data plane" positioning) but did not settle it — MIT source and a
managed offering are entirely compatible (Supabase is exactly that). So the
question went to the user rather than being inferred.

**Alternatives rejected:** Managed SaaS (would add ~6 business artifacts with no
in-tree owner); licensed on-prem (conflicts with the current MIT license, and
would make the amd64 fix even more urgent).

**Consequence:** Out of scope — status page, on-call, DPA/ToS, SLA, incident
comms. In scope and *elevated* — the upgrade/migration guide, because nobody
else will run the migration for the operator.

**Provenance:** user · AskUserQuestion · 2026-07-09

**Learn more:** `analysis.md` → "What 'ready for real customers' actually
requires"; `handoffs/spec.json` → `delivery_model`.

---

### D-002 — Spec backend is native-kbd, not OpenSpec

**TL;DR:** Wrote `spec.md` / `tasks.json` / `verification.md` per change under
`.kbd-orchestrator/changes/`, rather than emitting `openspec new`.

**Why:** All six changes are defect-driven with identified root causes, exact
file:line sites, and testable acceptance criteria. OpenSpec's value is
proposal/design iteration before implementation is understood. Here it is
already understood. Native specs are the lighter, honest fit.

This is *not* a repeat of p15's failure. p15 was **planned** as OpenSpec, then
executed as native-tool because nobody ever created the change directories — a
silent drift that broke `/kbd-apply`. Here the choice is explicit, recorded, and
the directories exist. The OpenSpec CLI (v1.4.1) is present and functional; p14
used it correctly with 5 change dirs.

**Alternatives rejected:** OpenSpec (adds a proposal round-trip that would
restate known root causes); hybrid (no benefit).

**Consequence:** If `/kbd-apply` needs an OpenSpec task surface, mirror each
change with `openspec new <change-id>`. Recorded in `handoffs/spec.json`
→ `backend_rationale`.

**Provenance:** research (backend detection + p14/p15 precedent)

---

### D-003 — Realtime fails closed; default inverted to `listen`

**TL;DR:** `FabricChangeSource::watch()` will return
`Err(StreamError::Unavailable)` instead of `Ok(empty_stream)`, and the gateway
default becomes `ListenChangeSource`.

**Why:** The current code returns success with no data. A subscriber connects,
Keto passes, the subscription establishes — and no event ever arrives. It fails
*open, into silence*. Every deployment path except Helm is affected
(`.env.example` leaves `FLINT_CHANGE_SOURCE` commented out; docker-compose does
not set it), while `README.md:40` advertises subscriptions unconditionally.

For self-hosted OSS this is the worst class of defect: the operator has no
vendor to ask, and the system reports success while doing nothing. A missing
feature is a bug. A feature that pretends to work is an incident.

`ListenChangeSource` is a complete LISTEN/NOTIFY implementation whose Keto check
already fails closed. Defaulting to the stub over the working adapter is
indefensible.

**Alternatives rejected:** Wait for OQ-FRF-1 (external dependency on the
`flint-realtime-fabric` team, open since p3, no resolution date); document the
caveat instead of fixing it (papers over a defect with prose); remove
subscriptions from the README (viable only if the beta excludes them — see open
questions).

**Consequence:** BREAKING per Base Rule #16 — an operator explicitly using
`fabric` now receives an error where they previously received silence. But
nothing that works today can break, because nothing works today. Requires a
`CHANGELOG.md` BREAKING entry.

**Provenance:** research (`main.rs:601`, `fdb-realtime/src/lib.rs:116`,
`listen.rs:215`, `values.yaml:55`, `.env.example:59`)
