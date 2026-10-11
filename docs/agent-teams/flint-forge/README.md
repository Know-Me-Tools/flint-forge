# Flint Forge agent team

Eleven coding specialists for PostgreSQL 18, Quarry APIs, Anvil extensions, Kiln,
clients, security and operations. Definitions are installed locally; no agents
have been started. Most work needs only a lead, implementer and independent reviewer.

- [Research, role rationale and reference links](RESEARCH.md)
- [Task routing, ownership, KBD and handoffs](WORKFLOW.md)
- [Portable manifest and local state location](team-request.json)
- [Installed files and skill hashes](evidence/project-installation.json)
- [Validation results](evidence/validation.json)

## Installed project definitions

| Harness | Local version | Definition location | Verification level |
|---|---|---|---|
| Codex | 0.154.0 | `.codex/agents/forge-*.toml` | Export structure checked; native execution not invoked |
| Claude Code | 2.1.282 | `.claude/agents/forge-*.md` | Source/structure checked; native execution not invoked |
| OpenCode | 1.18.25-fork | `.opencode/agents/forge-*.md` | Installed native loader lists all 11 agents with `opencode agent list --pure` |
| Kimi Code | 0.42.0 | `.kimi-code/agents/forge-*.md` | Current CLI agent-file contract and export structure checked; native execution not invoked |
| Copilot | 0.0.389-1 | `.github/agents/forge-*.agent.md` | Source/structure checked; native execution not invoked |
| MiniMax CLI | 0.5.4 | `.agent-team/minimax/agents/forge-*/agent.md` | Installed package confirms data-directory/agent-file pattern; runtime acceptance unverified |

Eight skills live under `.agents/skills/`, with relative links in the five native
project skill directories. Every role prompt also instructs explicit skill-file
reading when native preloading is unavailable. Existing OpenSpec skills and native
settings were preserved. MiniMax's isolated data directory links the project skills;
no global MiniMax credentials or configuration were copied.

Open this project in the selected harness and reload its project definitions.
Ask the parent agent to delegate a bounded task to the relevant `forge-*` role.
For example: “Have forge-postgres18 assess this migration and forge-reviewer review
the evidence; do not apply it.” A model-backed invocation was intentionally not used
as a configuration test.

Kimi's installed CLI also supports selecting a main agent explicitly:

```sh
kimi --agent forge-lead
```

To use the isolated MiniMax profile, launch from the project directory:

```sh
MINIMAX_DATA_DIR="$PWD/.agent-team/minimax" mcode
```

That command starts the CLI when the user runs it; it was **not run during setup**.
Configure authentication/providers through MiniMax's native interface for this
profile. Its CLI has no verified custom-agent selector for `mcode exec`; select or
route to the installed roles through supported native agent controls. Do not assume
that merely launching `mcode` starts this roster. The exporter inspected 0.4.12,
while this machine has 0.5.4, so native runtime acceptance remains a stated check.

## Exports and alternatives

`exports/<target>/team-export.json` and `export-receipt.json` describe generated
files, hashes, native source versions and limitations. The export requests sit
alongside the portable manifest. Export refuses an existing destination; use a new
proposal directory for future updates, then compare/merge deliberately.

| Target | Available artifacts | Deployment status |
|---|---|---|
| Codex, Claude, OpenCode, Kimi, Copilot | Native project definitions | Copied into project locations above |
| Claude | Alternative local plugin and marketplace | Staged only; use instead of project agents, not in addition |
| Kimi | Alternative plugin and v2 marketplace | Staged only; use instead of project agents |
| MiniMax | Native user-data agent definitions | Copied to isolated local profile; no live sessions |
| DeepSeek | `.dsh` persona profiles and separate experimental team composition | Staged only; no installed CLI found, no plugin activation or static roster promise |
| UAR | AgentArtifact registration payloads | Staged only; requires selected endpoint, credentials, tool permissions and skill deployment |
| BossFang | Agent manifests, registration/workflow payloads; alternative Hand | Staged only; selected future path is standalone agents/workflow, never also activate the Hand |

Claude/Kimi alternative plugin agents still need the project skill payloads in their
target project. They are not claimed to be standalone globally deployable skill packs.
UAR/BossFang skill names are preferences/references, not a skill installation or
authorization policy. Their default tool permissions and model/provider selection
must be reviewed against a concrete deployment before registration. Registration,
activation and execution are distinct actions; no service calls were made here.
The generated BossFang workflow is a roster scaffold, not an approved work order.
Before running it, construct the task-specific subset and dependencies described in
WORKFLOW.md; do not run all eleven roles merely because they appear in the export.

## Models, memory and maintenance

Codex hard roles use `gpt-6-astra`; SDK/release use `gpt-5.6-sol`. Their availability
is based on local configuration/host declarations, with unknown prices and no inference
probe. Claude inherits; OpenCode uses configured `kimi-for-coding/k3`; Kimi defaults
to configured `kimi-code/k3` through native invocation controls. Other targets inherit
their configured defaults. See `models.json` and per-role selection receipts for
policy, provenance and limitations. No cross-harness provider ID is guessed.

The local team ledger is `.agent-team/flint-forge/state.json`, initially without
tasks. It and the MiniMax profile are Git-ignored. Use agent-team-manage for assigned
tasks and agent-team-handoff for owner/harness transitions. KBD remains authoritative.
Memory services are optional and unconfigured; setup does not start or publish to them.

Canonical portable content is `team-request.json` plus `.agents/skills/`.
Regenerate new proposals with the installed agent-team-creator CLI, validate, then
merge only these team's files. Preserve any later user edits. Symlinked skill
entrypoints follow canonical skill updates without maintaining five independent copies.

No application code, service configuration, global tool setup, existing KBD state,
other repository, or convergence worktree was modified. Nothing was committed or pushed.
