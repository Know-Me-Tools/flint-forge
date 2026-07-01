# Project Exploration Summary

## 1. Flint Platform Agent (`/Users/gqadonis/Projects/prometheus/flint-platform-agent`)

### Project Purpose and Scope

The **Flint Platform Agent (FPA)** is the administrative agent for the entire Prometheus Flint fabric. It is a single Rust/Axum server that exposes the platform's administrative and operational capabilities through four open agent protocols, enabling operators to manage the fabric from any compatible harness (Claude Desktop, Claude Code, OpenCode, Codex, Kimi, custom Tauri/CLI harnesses, etc.).

It serves as the central **administrative interface** over three sibling planes:
- **flint-forge** — sovereign data & edge-compute plane (source of fabric metadata and data)
- **flint-realtime-fabric** — realtime spine (CDC, CRDT sync, media/SFU signaling, federation bridges)
- **flint-gate** — AI-native auth proxy / API gateway

### Key Files and Their Purposes

| File/Directory | Purpose |
|----------------|---------|
| `README.md` | Project overview, protocol surfaces, architecture, build commands |
| `CLAUDE.md` | Authoritative architecture, quality gates, Prometheus Base Rules for all agents |
| `AGENTS.md` | Points all non-Claude harnesses to `CLAUDE.md` as canonical guidance |
| `Cargo.toml` | Workspace manifest with 9 crates (hexagonal layering enforced at Cargo level) |
| `rust-toolchain.toml` | Toolchain pinned to channel 1.90, MSRV 1.85, edition 2024 |
| `.kbd-orchestrator/project.json` | KBD orchestrator config referencing sibling repos as read-only references |
| `.kbd-orchestrator/constraints.md` | Machine-actionable blocking/warning constraints (clippy, no unwrap, hexagonal rule, etc.) |
| `crates/fpa-domain/` | **Layer 0** — pure domain types (Task, TaskState, IDs) with zero infra dependencies |
| `crates/fpa-ports/` | **Layer 1** — trait seams (ports): `ForgeMetadata`, `FabricClient`, `GateAdmin`, `McpClient` |
| `crates/fpa-app/` | **Layer 1** — use-cases orchestrating tasks against ports only |
| `crates/fpa-protocol/` | Wire-level payloads for AG-UI, A2A, A2UI, and MCP surfaces |
| `crates/fpa-forge/` | Adapter → `ForgeMetadata` port (Quarry metadata/data) |
| `crates/fpa-fabric/` | Adapter → `FabricClient` port (realtime spine) |
| `crates/fpa-gate/` | Adapter → `GateAdmin` port (gate admin API) |
| `crates/fpa-mcp/` | Adapter → `McpClient` port (downstream MCP client) |
| `crates/fpa-gateway/` | **Composition root** (bin) — Axum server, the ONLY crate importing concrete adapters |
| `crates/fpa-cli/` | CLI binary (`fpa`) for ops/dev commands |
| `scripts/ci-check.sh` | Full local CI gate script |
| `.github/workflows/ci.yml` | GitHub Actions CI workflow |
| `openspec/` | OpenSpec configuration for spec-driven changes (used by Codex, Claude, Kimi, OpenCode) |

### Architecture and Design Patterns

**Strict Hexagonal Architecture (Ports & Adapters)**

```
Layer 0: fpa-domain      — pure types (serde only, zero infra deps)
Layer 0: fpa-protocol    — AG-UI / A2A / A2UI / MCP wire types
Layer 1: fpa-ports       — trait seams (one async port per external plane)
Layer 1: fpa-app         — use-cases against ports only
Adapters: fpa-forge, fpa-fabric, fpa-gate, fpa-mcp
Interface: fpa-gateway  — Axum composition root (bin)
          fpa-cli       — ops / dev CLI (bin)
```

**Key Design Rules:**
- **Absolute dependency rule**: `fpa-domain` and `fpa-app` must NEVER depend on any adapter crate or interface crate. Composition happens only in `fpa-gateway`/`fpa-cli`.
- **Forward-compatible protocol payloads**: `#[non_exhaustive]` enums, `#[serde(tag = "type", rename_all = "snake_case")]`, with `#[serde(other)] Unknown` catch-all variant
- **Error handling**: `thiserror` in library crates; `anyhow` only at binary entry points (`fpa-gateway`, `fpa-cli`)
- **Quality gates**: Clippy pedantic with `-D warnings`, `rustfmt` clean, no file over 500 lines, no `unwrap()`/`expect()` in libraries

### Protocol Surfaces

| Surface | Role | Endpoint |
|---------|------|----------|
| **AG-UI** | Agent → UI event stream (run lifecycle, text deltas, tool calls, state) | `GET /agui/stream` |
| **A2A** | Administrative task lifecycle (submit / status / cancel) | `POST /a2a/tasks`, `GET /a2a/tasks/{id}` |
| **A2UI** | Dynamic UI primitives that render in any host and bind to fabric functions | Emitted as events |
| **MCP** | App (UI surfacing), client (downstream servers), server (HTTP-Streaming only) | `POST /mcp` |

### Existing Component / Registry / Metadata Models

- **A2UI Component Primitives** (canonical platform-wide UI vocabulary):
  - `Stack` (vertical/horizontal grouping)
  - `Text` (static text/heading)
  - `Button` (binds to `Action` → A2A task)
  - `TextField` (single-line input bound to action field)
  - `Table` (tabular display of fabric data)
  - All primitives are `#[non_exhaustive]` with `Unknown` catch-all

- **AdminTask Domain Model**:
  - `TaskState`: `Submitted`, `Working`, `InputRequired`, `Completed`, `Failed`, `Canceled`
  - `AdminTask`: `id`, `operator`, `kind` (catalog key like `"forge.table.inspect"`), `input` (JSON), `state`

- **Port Traits** (trait seams for each external plane):
  - `ForgeMetadata`: `list_tables()`, `describe_table(name)`
  - `FabricClient`: `health()` (realtime spine health)
  - `GateAdmin`: `list_routes()` (auth proxy admin)
  - `McpClient`: `list_tools()`, `call_tool(name, arguments)`

### Relevant Technologies and Frameworks

| Technology | Role |
|------------|------|
| Rust (Edition 2024) | Primary language |
| Tokio | Async runtime |
| Axum | HTTP server / routing |
| async-trait | Async trait definitions for ports |
| serde/serde_json | Serialization |
| thiserror/anyhow | Error handling |
| tracing/tracing-subscriber | Observability |
| UUID | ID generation |
| arc-swap | Lock-free configuration updates |
| futures | Async utilities |

### Integration with Larger Platform Ecosystem

The FPA is designed as the **central administrative hub** for the Prometheus Flint fabric:
- References `flint-forge` for metadata/data (Quarry, Anvil, Kiln)
- References `flint-realtime-fabric` for canonical protocol types (`frf-agentproto::ContentBlock`)
- References `flint-gate` for auth proxy administration
- MCP client role composes downstream servers' tools into its administrative toolset
- MCP server role exposes fabric tools to external agents via HTTP-Streaming
- The KBD orchestrator configuration defines the project boundary with sibling repos as read-only references

---

## 2. Open Design (`/Users/gqadonis/Projects/references/open-design`)

### Project Purpose and Scope

**Open Design** is an open-source, local-first alternative to Claude Design. It is a comprehensive agentic design workspace that enables users to go from a vague idea to fully rendered design artifacts (prototypes, dashboards, decks, images, videos, motion graphics) without leaving the app. It runs on 22+ coding agent CLIs (Claude Code, Codex, Cursor, Kimi, etc.) and supports BYOK (Bring Your Own Key) to any OpenAI-compatible endpoint.

**Core Concept**: The design engine is not a cloud service — it is the coding agent already installed on your laptop (Claude, Codex, Cursor, etc.). Open Design provides the orchestration layer, design system management, skill registry, and artifact rendering pipeline.

### Key Files and Their Purposes

| File/Directory | Purpose |
|----------------|---------|
| `README.md` | Comprehensive product overview, quick start, feature tour, comparison matrix |
| `AGENTS.md` | Authoritative directory guide for all agents entering the repo (data paths, boundaries, commands) |
| `CLAUDE.md` | Redirects to `AGENTS.md` |
| `QUICKSTART.md` | Quick start guide for developers |
| `docs/architecture.md` | System topology, runtime modes, data flow, component diagram |
| `docs/skills-protocol.md` | Skill protocol specification |
| `docs/agent-adapters.md` | Agent adapter interface (detect, spawn, stream, report capabilities) |
| `apps/web/` | Next.js 16 App Router + React 18 web runtime (UI, chat, preview, settings) |
| `apps/daemon/` | Node.js daemon (Express, SQLite, SSE streaming, agent spawning, MCP server) |
| `apps/desktop/` | Electron shell (macOS, Windows) |
| `apps/daemon/src/server.ts` | Daemon HTTP server composition root |
| `apps/daemon/src/agents.ts` | Agent adapter pool (22+ CLI adapters) |
| `apps/daemon/src/skills.ts` | Skill registry (scans `SKILL.md` files, parses frontmatter) |
| `apps/daemon/src/design-systems/index.ts` | Design system registry (150+ brand-grade `DESIGN.md` systems) |
| `apps/daemon/src/genui/` | GenUI surface registry (SQLite-backed, cache invalidation via schema digest) |
| `apps/daemon/src/mcp.ts` | stdio MCP server (proxies tool calls to daemon HTTP API) |
| `apps/daemon/src/library.ts` | Content-addressed asset library with source tracking |
| `apps/daemon/src/cli.ts` | `od` CLI subcommand registration |
| `packages/contracts/` | Pure TypeScript shared contracts (web/daemon DTOs, SSE unions, error shapes) |
| `skills/` | 100+ built-in functional skills |
| `design-systems/` | 150 brand-grade `DESIGN.md` systems (Linear, Stripe, Apple, etc.) |
| `design-templates/` | Rendering catalog (prototypes, decks, images, videos) |
| `plugins/_official/` | 261 official plugins (scenarios, image templates, video templates, atoms, examples) |
| `plugins/spec/SPEC.md` | Plugin manifest specification |
| `mocks/` | Replay-based mock CLIs for testing (anonymized Langfuse traces) |
| `tools/dev/` | Local development lifecycle control plane |
| `tools/pack/` | Packaged build/start/stop/logs, updater harness, installer identity |

### Architecture and Design Patterns

**Three Deployment Topologies:**

```
A. Fully local (default):    browser → Next.js dev server → od daemon → spawns CLI
B. Web + daemon:             browser → Vercel → WebSocket tunnel → daemon on laptop
C. Web-only (no daemon):     browser → Vercel → direct API (BYOK in browser)
```

**Layered Architecture:**

```
┌────────────────── Web App (Next.js 16, App Router) ──────────────────┐
│  chat · artifact tree · iframe preview · comment/slider · settings   │
│                     session bus (in-memory)                           │
│              transport layer (daemon SSE | api-direct | browser)      │
└──────────────────────────┬──────────────────────────────────────────┘
                           │
┌──────────────────────────┴──────────────────────────────────────────┐
│                          Daemon (Node, Express, SQLite)               │
│  session manager · skill registry · design-system resolver          │
│  agent adapter pool · artifact store · preview compile pipeline     │
│  export pipeline · MCP stdio server · detection service             │
└──────────────────────────┬──────────────────────────────────────────┘
                           │
              ┌────────────┴────────────┐
              ▼                       ▼
       ┌─ agent CLIs ─┐        ┌─ filesystem ─┐
       │ claude, codex, │        │ daemon data  │
       │ cursor, kimi,  │        │ skills/      │
       │ gemini, etc.   │        │ DESIGN.md    │
       └───────────────┘        └─────────────┘
```

**Key Design Principles:**
- **Local-first**: All computation runs on the user's machine; no cloud round-trip for core operations
- **Agent-native**: The agent is the design engine; OD is the orchestration layer
- **Filesystem-based**: Skills, design systems, and plugins are plain files anyone can author, version, and publish
- **Dual-track**: Every capability must be reachable via both web UI and `od` CLI
- **Plain-file artifacts**: Artifacts stored on disk, not proprietary binary format; `artifact.json` metadata + `history.jsonl` append-only log

### Design System Management

**The `DESIGN.md` Contract:**
- 9-section schema covering: color, typography, spacing, layout, components, motion, voice, brand, anti-patterns
- 150+ brand-grade systems shipped (Linear, Stripe, Vercel, Apple, Tesla, Notion, etc.)
- Injected as a prepended system message on every agent run
- Hot-reloads on file change in dev mode
- Supports `manifest.json` for machine-readable metadata (tokens, components, fonts, preview pages)

**Design System Registry (`apps/daemon/src/design-systems/index.ts`):**
- Scans `design-systems/*` for `DESIGN.md` files
- Parses frontmatter (YAML) and Markdown body
- Extracts: title, category, summary, color swatches, surface type, provenance
- Supports `manifest.json` for structured assets (tokens.css, components.html, design-tokens.json, tailwind-v4.css)
- Content-addressed caching with SHA-256 fingerprinting
- File-level access control via allowlist (manifest-declared files only)

**Component Registry (via `components.html` / `components.manifest.json`):**
- `components.html` provides worked UI fixtures
- `components.manifest.json` provides concise prompt-ready summaries
- Extracted via `extractComponentsManifest()` for agent consumption
- Fallback to HTML parsing when manifest absent

### GenUI (Agent-Generated UI) Surface Model

**GenUI Registry (`apps/daemon/src/genui/`):**
- SQLite-backed surface state table (`genui_surfaces`)
- Surfaces have: `kind`, `persist` tier (`run`/`conversation`/`project`), `schemaDigest`, `status`
- **F8 Cache**: Cross-conversation cache lookup — if a matching schema digest exists at the right tier, emits `respondedBy: 'cache'` without broadcasting a new request
- Schema digest computed via SHA-256 of canonicalized JSON (sorted keys for stability)
- Response sources: `user`, `agent`, `auto`, `cache`
- Status values: `pending`, `resolved`, `timeout`, `invalidated`

### Skill and Plugin Models

**Skill Registry (`apps/daemon/src/skills.ts`):**
- Scans three locations in priority order: `./.claude/skills/` → `./skills/` → `~/.claude/skills/`
- Parses `SKILL.md` with YAML frontmatter + Markdown body
- Frontmatter fields: `name`, `description`, `triggers`, `od.mode`, `od.scenario`, `od.design_system.requires`, `od.critique.policy`, `od.preview.type`
- Modes: `prototype`, `deck`, `image`, `video`, `audio`, `design-system`, `template`, `utility`
- Scenarios: `design`, `marketing`, `operation`, `engineering`, `product`, `finance`, `hr`, `sale`, `personal`
- Skill ID aliases for backward compatibility after renames

**Plugin Model (`plugins/spec/SPEC.md`):**
- Minimum: `SKILL.md` with YAML frontmatter
- Optional: `open-design.json` manifest for marketplace listing
- Manifest fields: `specVersion`, `name`, `version`, `od.kind`, `od.taskKind`, `od.mode`, `od.capabilities`, `od.inputs`
- Capabilities: `prompt:inject`, `file:read`, `file:write`, `artifact:create`, etc. (restricted by default)
- Plugin categories: `scenarios`, `image-templates`, `video-templates`, `design-systems`, `atoms`, `examples`

### Artifact and Asset Management

**Artifact Store:**
- Plain files on disk under daemon-managed storage
- `artifact.json` metadata per artifact
- `history.jsonl` append-only log (git-friendly, greppable)
- Preview: sandboxed `srcdoc` iframe with React 18 + Babel standalone for JSX
- Export: HTML (inlined), PDF (puppeteer), PPTX (pptxgenjs), ZIP, Markdown, MP4 (HyperFrames)

**Library (`apps/daemon/src/library.ts`):**
- Content-addressed asset storage (SHA-256 hash as path)
- Idempotent registration: same bytes → one asset row with multiple source records
- Source kinds: `owned`, `external`, `imported`
- Asset kinds: `image`, `video`, `audio`, `document`, `data`, `html`, `figma`
- Sidecars: Figma IR capture (`.od-figma.json`), element picker (`.element.html`)
- Enrichment pipeline: caption, OCR, embedding (AI enrichment stages marked `skipped` initially)

### MCP Integration

**Open Design as MCP Server:**
- stdio transport (contrasts with FPA's HTTP-Streaming constraint)
- Proxies all tool calls to daemon HTTP API (`OD_DAEMON_URL`)
- Tools: `od search-files`, `od get-file`, `od get-artifact`, `od plugin run`, `od skill list`
- Read-only by default; daemon binds to `127.0.0.1`
- SSRF blocked at proxy edge
- One-line install: `od mcp install <agent>` (supports 16+ CLIs)

### Relevant Technologies and Frameworks

| Technology | Role |
|------------|------|
| Node.js ~24 | Daemon runtime |
| TypeScript 5.9 | Primary language |
| Next.js 16 (App Router) | Web frontend |
| React 18 | UI framework |
| Express 5 | Daemon HTTP server |
| better-sqlite3 | Local database (sessions, projects, library, GenUI surfaces) |
| Electron | Desktop shell (macOS, Windows) |
| pnpm 10.33 | Package manager / workspace |
| MCP SDK | Model Context Protocol server/client |
| HyperFrames (HeyGen) | HTML → MP4 motion graphics |
| Seedance 2.0, Veo 3, Sora 2 | Video generation models |
| Puppeteer | PDF export |
| pptxgenjs | PPTX export |
| Babel standalone | JSX preview compilation |
| jszip | ZIP export |
| cheerio | HTML parsing |
| chokidar | File watching |
| tar | Archive handling |
| undici | HTTP client |

### Integration with Larger Platform Ecosystem

Open Design is designed as a **composable, open design workspace** that integrates with the broader agent ecosystem:
- **Agent-native**: 22+ coding agent CLIs are the design engine; OD is the orchestration layer
- **MCP-native**: Exposes all capabilities as MCP tools so any MCP-compatible agent can consume them
- **Design-system portable**: 150+ `DESIGN.md` brand contracts can be imported/exported and shared
- **Plugin ecosystem**: 261 official plugins + community plugins; publishable to registries
- **BYOK proxy**: Supports any OpenAI-compatible endpoint (Anthropic, OpenAI, Azure, Google, Ollama, LM Studio, vLLM)
- **Figma integration**: Figma capture IR, element picking, import/export workflows
- **GitHub integration**: PR creation, repo import, branch management
- **Vercel deployment**: Native support for Topology B (web on Vercel + daemon on laptop)
- **Nix flake**: Reproducible development environment

### Summary of Relevant Concepts for Component Registries / UI/UX Abstractions

1. **A2UI Component Primitives** (FPA): A canonical, platform-wide vocabulary of UI components (Stack, Text, Button, TextField, Table) that render across all hosts and bind to fabric functions via `Action` objects. This is a **declarative, cross-platform UI abstraction**.

2. **GenUI Surface Model** (OD): A SQLite-backed registry of dynamically generated UI surfaces with schema-based cache invalidation, tiered persistence (`run`/`conversation`/`project`), and multi-source responses (`user`, `agent`, `auto`, `cache`). This is a **stateful, cacheable UI surface abstraction**.

3. **Design System Registry** (OD): A file-based registry of `DESIGN.md` brand contracts with machine-readable manifests (`tokens.css`, `components.html`, `design-tokens.json`), content-addressed caching, and structured file access. This is a **brand-grade design token + component abstraction**.

4. **Skill Registry** (OD): A file-based registry of agent skills with YAML frontmatter, mode/scenario classification, and capability declarations. This is an **agent capability abstraction**.

5. **Plugin Manifest** (OD): A structured manifest (`open-design.json`) defining plugin capabilities, inputs, pipeline, and marketplace metadata. This is a **packaged, distributable capability abstraction**.

6. **Content-Addressed Asset Library** (OD): A deduplicated, source-tracked asset store with sidecars for derived data (Figma IR, element HTML). This is a **media + component asset abstraction**.

7. **MCP Server** (OD/FPA): A standardized protocol for exposing tools/resources to AI agents. OD uses stdio; FPA uses HTTP-Streaming. This is a **cross-agent capability exposure abstraction**.

---

*Generated: 2026-01-15*
