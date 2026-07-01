# Platform Comparison: AI Agent Development & Deployment Infrastructure

> **Research Date:** 2026-06-29  
> **Focus:** AI agent development, deployment flexibility, sovereign inference, edge compute, and resource-constrained environments  
> **Source:** Web research + Flint Forge RFC-FORGE-001 specification

---

## 1. Supabase for AI Agents

### 1.1 What Supabase Provides

Supabase has evolved into a full-stack backend platform that many teams now use as their default for AI-native applications. Its core capabilities for AI agents include:

| Capability | Implementation | Agent Relevance |
|---|---|---|
| **Vector Search** | `pgvector` extension natively bundled; HNSW/IVFFlat indexes | Embeddings, RAG, semantic memory |
| **Edge Functions** | Deno runtime, globally deployed, TypeScript-first | AI preprocessing, lightweight inference, API gateway for LLMs |
| **Real-time** | Postgres logical replication → WebSocket/SSE | Streaming agent state, tool-call progress, collaborative AI |
| **RLS** | Row-level security policies via JWT claims | Multi-tenant agent isolation |
| **Auth** | Kratos-based JWT, OAuth, SAML | Agent identity and user attribution |
| **Storage** | S3-compatible buckets | Document ingestion, generated artifacts |
| **MCP Support** | Supabase MCP server for agent IDE integration | Schema introspection, query execution |

### 1.2 AI/LLM Integrations

- **Supabase.ai.Session API**: In-Edge-Function inference using models like `gte-small` for embeddings, with `llama2`/`mistral` support rolling out. This removes cold-start issues by using Ort instead of WASM bootstrap.
- **Agent Plugin**: Bundles MCP server + curated skills for AI coding agents (Claude Code, Codex, Cursor, etc.).
- **LangChain integration**: Supabase vector store class; pgvector as a LangChain retriever.
- **Supabase Vault**: pgsodium-based encrypted secret store for API keys (e.g., OpenAI keys) — though it is not as hardened as a full KMS envelope.

### 1.3 Limitations for Agent Use Cases

| Limitation | Impact on Agents | Mitigation |
|---|---|---|
| **No per-agent sandboxing** | Projects are full Postgres instances; you cannot spin up an isolated DB per agent session in milliseconds | RLS provides tenant isolation, but not sandbox-level isolation |
| **Always-on pricing** | Projects don't scale to zero; bursty agent fleets (500 agents for 10 min, idle for 1 hour) incur full cost | No native scale-to-zero for compute |
| **CPU-bound vector search** | `pgvector` is CPU-only; millions of embeddings + concurrent agent queries hit latency walls | Add Pinecone/Weaviate for GPU-accelerated search at scale |
| **No native token metering** | No built-in cost attribution per agent, per user, or per model call | Must build custom logging/aggregation |
| **No in-DB LLM execution** | Supabase Edge Functions call out to providers; no sovereign inference inside the DB | Rely on external providers or self-host vLLM |
| **Memory gaps** | Agents often "forget" tool call results across turns unless explicitly persisted by the application | Application must store intermediate steps in tables |
| **WAL bypasses RLS** | Realtime subscriptions use WAL which doesn't enforce RLS; requires re-query per event (same technique as Supabase Realtime, but adds latency) | Acceptable for many apps; predicate-pushdown is risky |
| **Slow provisioning** | New projects take minutes; agents need environments in milliseconds | Use existing project with schema-per-tenant |

### 1.4 Where Supabase Shines

- **Single-stack simplicity**: One platform covers DB, vectors, auth, storage, edge functions, and real-time — eliminating the need for a separate vector DB, auth service, or real-time layer.
- **Web-app-first design**: Ideal for human-in-the-loop agents, chat UIs, and collaborative AI tools where user auth and real-time UI updates are primary.
- **Cost efficiency at small scale**: Free tier (500 MB DB, 500K edge function invocations) is generous for prototyping.
- **MCP ecosystem**: Supabase's MCP server and agent skills dramatically improve AI editor success rates (Codex: 88% with MCP + Skill vs. 71% with MCP only).

---

## 2. Flint Ecosystem Advantages for AI Agents

Flint Forge (RFC-FORGE-001) is a sovereign data and edge-compute plane designed from the ground up for agentic workloads. Its architecture explicitly addresses the gaps left by general-purpose platforms like Supabase.

### 2.1 Architecture Overview

```
                         ┌──────────── flint-gate (ingress / auth boundary) ────────────┐
  browser / app ───────▶ │ Kratos session → RLS JWT                                    │
  webhook callbacks ───▶ │ StreamProcessor: WS / SSE / NDJSON proxy + backpressure      │
                         └───────┬───────────────────────────────┬─────────────────────┘
                                 ▼                                 ▼
                     ┌──────── Flint Quarry ────────┐    ┌──────── Flint Kiln ────────┐
                     │ REST (PostgREST-compatible)  │    │ Admin REST (control plane)  │
                     │ GraphQL Q/M → pg_graphql     │    │ /functions/v1/<name>        │
                     │ GraphQL Sub → async-graphql    │    │   (data plane, WASM)        │
                     └───────┬──────────────┬───────┘    └────────┬───────────┬────────┘
                             ▼              ▼                      ▼           ▼
                ┌─ Postgres 18 ────────────────┐         ComponentStore   flint-gate
                │ pg_graphql · pgvector · pg_net│         (OCI/IPFS/S3)  (governed
                │ pgcrypto · Flint Anvil:       │◀── flint:db / flint:llm callbacks
                │   flint_auth · flint_hooks ·  │                          
                │   flint_llm · flint_vault     │     
                └───────┬───────────────────────┘
                        ▼  WatchEntityType (gRPC stream)
              flint-realtime-fabric  (CDC → Iggy → Keto gate)
```

### 2.2 Agent-Specific Advantages

| Flint Component | What It Does | Why It Matters for Agents |
|---|---|---|
| **Flint Quarry** (`fdb-*`) | REST/GraphQL DB API gateway over Postgres | Agents query structured data via REST or GraphQL with RLS-enforced every row |
| **Flint Ember** (`flint_llm`) | **In-DB LLM** via liter-llm gateway | Sovereign inference: agents can call models **inside the database** without leaving the trust boundary. Sync (`llm.complete`) and async (`llm.jobs` queue) surfaces. No external API keys in application code. |
| **Flint Kiln** (`fke-*`) | WASM Component Model edge-function gateway | Agents can deploy **polyglot tool endpoints** (Rust, JS, Python, Go, C#) as signed, sandboxed WASM components. Fuel/epoch limits prevent runaway agents. |
| **Flint Vault** (`flint_vault`) | KMS-wrapped secret store (XChaCha20-Poly1305) | Agent API keys, tokens, and credentials are encrypted at rest with **envelope encryption** (KEK in Azure Key Vault / AWS KMS / GCP KMS). Zero plaintext secrets in DB. |
| **Flint Anvil** (`ext-flint-*`) | pgrx extensions: auth, hooks, LLM, vault | All agent logic runs **inside Postgres** as native extensions — zero network hops for auth context, webhook dispatch, and LLM inference. |
| **Realtime Fabric** | CDC + Iggy spine + Keto gate | Agent state changes stream to subscribers with **per-event RLS re-query** — same technique as Supabase but with a dedicated fabric spine. |
| **Cedar Policy** (`forge-policy`) | Action/capability policy evaluation | Fine-grained authorization: which agents can call which tools, which models, which tables. |

### 2.3 Key Differentiators vs. Supabase

| Dimension | Supabase | Flint Forge |
|---|---|---|
| **Vector search** | `pgvector` (CPU) | `pgvector` + potential GPU integration via sovereign inference plane |
| **LLM execution** | External calls only (Edge Functions → OpenAI/Anthropic) | **In-DB LLM** (Flint Ember) via liter-llm; routes to candle-vllm/RunPod or UAR |
| **Token metering** | None native | Rate-limit governor in `flint_llm` BGW; cost attribution per tenant/model |
| **Edge functions** | Deno (JS/TS only) | **Polyglot WASM components** (Rust, JS, Python, Go, C#, C/C++) via Wasmtime Component Model |
| **Secret storage** | Vault (pgsodium, file-based DEK) | **KMS-wrapped DEK** with zeroization; brokered secret injection into WASM (never enters linear memory) |
| **Auth layers** | 2 (Kratos + RLS) | **4 layers** (Kratos + Keto + RLS + Cedar) |
| **Streaming** | WS/SSE via Realtime | WS/SSE/NDJSON via flint-gate StreamProcessor + backpressure + AG-UI/A2UI support |
| **WASM sandbox** | N/A | Per-request isolation, fuel/epoch limits, memory ceilings, signed components |
| **Sovereign inference** | No | Yes — inference runs on infrastructure you control (UAR Tier-2, candle-vllm, local GPU) |
| **Offline capability** | No | WASM components can run on edge devices; in-DB LLM for local inference |
| **Signing / provenance** | N/A | Ed25519 + DID-VC + Cosign/Sigstore for every component |
| **Deployment target** | Cloud-only | Cloud + edge + desktop + embedded (WASM is portable) |

### 2.4 Streaming (AG-UI / A2UI)

Flint's `flint-gate` StreamProcessor supports WS, SSE, and NDJSON with **backpressure** — critical for agent UIs that need to stream token-by-token responses without overwhelming the client. This is designed for the **AG-UI (Agent Graphical User Interface)** and **A2UI (Agent-to-Agent UI)** patterns where agents collaborate in real time and stream intermediate reasoning steps to human observers.

### 2.5 Token Metering & Cost Governance

Flint Ember includes a **rate-limit governor** in the pgrx background worker. This is a first-class concern:
- Per-tenant model routing
- Cedar policy checks on which models an agent can use
- Cost attribution tied to the origin JWT
- Async worker batches LLM calls to optimize throughput

Supabase has no equivalent; teams must build their own middleware.

### 2.6 Edge Functions with WASM

Flint Kiln is not a traditional "serverless function" platform. It is a **WASM Component Model host**:
- **Polyglot**: Any language targeting `wasi:http/proxy` works.
- **Control/data-plane split**: Admin server compiles (Cranelift AOT); invocation server only deserializes `.cwasm` — no compiler in the request path.
- **Capability-gated**: The host `Linker` adds only interfaces the component's signed manifest requests ∩ Cedar allows.
- **Secrets brokered**: High-value secrets are injected at the host boundary; they never enter WASM linear memory.
- **Cross-platform**: `.cwasm` targets x86_64, aarch64, s390x, riscv64 — suitable for mobile, desktop, and embedded.

---

## 3. Other Platforms for AI Agents

### 3.1 LangChain / LangGraph

| Aspect | Details |
|---|---|
| **Deployment pattern** | LangGraph Cloud (managed, $0.01/min) or self-hosted Docker Compose |
| **State management** | Postgres checkpointer (`AsyncPostgresSaver`) — explicit, durable state |
| **Streaming** | Native token streaming via `astream`/`astream_events` |
| **Observability** | LangSmith tracing, LangGraph Studio for debugging |
| **Agent patterns** | Graph-based: cycles, conditional branching, parallel branches, human-in-the-loop interrupts |
| **Limitations** | Medium vendor lock-in; max runtime 10 min on Cloud; US-only multi-region; storage 10GB/app |

**LangGraph** is the stateful orchestration layer. It makes agent state explicit — every node receives state, transforms it, and passes it along. This is superior to implicit context-window management for multi-step agents. However, it requires you to bring your own database, auth, and vector store. LangGraph Cloud is essentially "agent infrastructure as a service" but with LangChain-ecosystem lock-in.

### 3.2 CrewAI

| Aspect | Details |
|---|---|
| **Deployment pattern** | Kubernetes Helm chart (CrewAI Platform) or Docker Compose; AMD64 only (no ARM64) |
| **Resource requirements** | 14 Gi RAM / 4 CPU minimum cluster; 20 Gi+ recommended for production |
| **State / memory** | Default: ChromaDB (local, ephemeral). Production: Mem0, Zep, or Qdrant for cross-session persistence |
| **Observability** | OpenTelemetry, AgentOps, Langfuse integrations |
| **Agent patterns** | Hierarchical, sequential, concurrent crews; role-based multi-agent teams |
| **Limitations** | Heavy K8s footprint; no ARM64 support; memory not scoped by user by default (leaks between users); container cold starts are slow; manager agent can loop |

CrewAI's strength is **multi-agent role-based orchestration**. Its weakness is infrastructure: you must manage K8s, PostgreSQL, Redis, and external memory backends. The default memory is file-based and wipes on restart. Production requires significant DevOps investment.

### 3.3 Microsoft AutoGen / Semantic Kernel → Microsoft Agent Framework

| Aspect | Details |
|---|---|
| **Consolidation** | AutoGen + Semantic Kernel merged into **Microsoft Agent Framework** (public preview Oct 2025) |
| **Deployment** | Azure AI Foundry Agent Service (primary); no self-hosted enterprise support without third parties |
| **State** | Thread-based state management via Redis, Cosmos DB, or custom backends |
| **Agent patterns** | Sequential, concurrent, group chat, handoff, Magentic-One, graph-based workflows |
| **Governance** | Native Entra integration, OpenTelemetry, responsible AI features, prompt injection protection |
| **Protocols** | MCP Steering Committee member; A2A (Agent-to-Agent) protocol support |
| **Limitations** | Deepest Azure integration; ecosystem still developing vs. LangGraph; .NET/Python bias; no true self-hosted option |

Microsoft's consolidation is significant: **AutoGen's research-grade multi-agent patterns** + **Semantic Kernel's enterprise-grade state/telemetry** = a production-ready framework. However, the primary deployment path is **Azure AI Foundry**, creating a strong cloud lock-in. The Agent Framework supports MCP and A2A, making it interoperable, but the runtime is Azure-centric.

### 3.4 AWS Bedrock / AgentCore

| Aspect | Details |
|---|---|
| **Foundation** | Amazon Bedrock (multi-model access: Claude, Llama, Mistral, Nova) |
| **Agent runtime** | **AgentCore** — serverless runtime with per-session microVM isolation |
| **Memory** | AgentCore Memory (short-term + long-term episodic memory) |
| **Identity** | AgentCore Identity (Amazon Cognito integration) |
| **Observability** | CloudWatch GenAI Observability, OpenTelemetry, distributed tracing |
| **Governance** | Policy guardrails (predefined + custom), rate limits, cost monitoring |
| **Protocols** | AgentCore Gateway transforms APIs/Lambdas into MCP endpoints; A2A support added |
| **Cost** | ~$1,456/mo for 100K conversations (5 hrs active/day, 1K input + 500 output tokens avg) |
| **Limitations** | High AWS lock-in; ARM64-only runtime (Docker builds slow on x86); complexity of CDK deployment; cost can escalate with multi-agent orchestration |

AWS Bedrock AgentCore is the most **enterprise-hardened** agent infrastructure. It provides session isolation via microVMs, integrated identity, and comprehensive observability. The trade-off is complexity: deploying a multi-agent system requires AWS CDK, Cognito, CloudFront, OpenSearch, and multiple AgentCore runtimes. The "Agent Blueprints" help, but this is fundamentally a cloud-native, AWS-locked solution.

### 3.5 Google Vertex AI / ADK

| Aspect | Details |
|---|---|
| **Framework** | Agent Development Kit (ADK) — Python, Java, Go |
| **Runtime** | Vertex AI Agent Engine (managed) or Cloud Run (containerized, more flexible) |
| **Agent patterns** | Sequential, parallel, loop orchestration; A2A protocol; Code Execution Sandbox |
| **Observability** | Agent Engine dashboard: token usage, latency, error rates; evaluation layer simulates user interactions |
| **Governance** | Cloud IAM agent identities, Model Armor (prompt injection blocking) |
| **Deployment** | `adk deploy agent_engine` — single command; or Cloud Run with IAP |
| **Limitations** | Agent Engine is API-only (no custom Web UI/A2A endpoint directly); pricing per-hardware is less flexible than request-based; deepest GCP integration |

Google's ADK has been downloaded 7M+ times and now supports Go in addition to Python/Java. The **Agent Engine** is opinionated but streamlined: one command deploys. The **Cloud Run** path offers more flexibility (custom UI, A2A endpoints) but requires more setup. Model Armor is a notable security feature for prompt injection defense. Like Microsoft and AWS, the deepest integration is with the parent cloud.

### 3.6 Platform Comparison Matrix

| Capability | Supabase | LangGraph Cloud | CrewAI | MS Agent Framework | AWS Bedrock | Google Vertex AI | Flint Forge |
|---|---|---|---|---|---|---|---|
| **Managed DB** | ✅ Postgres | ✅ Postgres | ❌ Bring your own | ❌ Bring your own | ❌ Bring your own | ❌ Bring your own | ✅ Postgres 18 |
| **Vector Search** | ✅ pgvector | ✅ pgvector | ✅ Chroma/Qdrant | ✅ Azure Search | ✅ S3 Vectors | ✅ Vertex AI | ✅ pgvector |
| **In-DB LLM** | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ✅ **Flint Ember** |
| **Polyglot Edge** | ❌ Deno only | ❌ Python only | ❌ Python only | ✅ .NET/Python/Go | ❌ Python primary | ✅ Python/Java/Go | ✅ **WASM (any)** |
| **WASM Sandbox** | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ✅ **Wasmtime** |
| **Token Metering** | ❌ | ❌ | ❌ | ❌ | ✅ CloudWatch | ✅ Dashboard | ✅ **Governor** |
| **Sovereign Inference** | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ✅ **UAR / candle** |
| **4-Layer Auth** | ❌ (2-layer) | ❌ | ❌ | ✅ Entra+ | ✅ Cognito+IAM | ✅ Cloud IAM | ✅ **Kratos+Keto+RLS+Cedar** |
| **MCP Support** | ✅ Native | ✅ | ✅ | ✅ Steering Committee | ✅ Gateway | ✅ | ✅ |
| **A2A Protocol** | ❌ | ❌ | ❌ | ✅ | ✅ | ✅ | Planned |
| **Offline Capable** | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ✅ **WASM components** |
| **Mobile/Edge Deploy** | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ✅ **Cross-compile WASM** |
| **Signing/Provenance** | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ✅ **Ed25519+DID+Cosign** |
| **Scale-to-Zero** | ❌ | ❌ | ❌ | ✅ Serverless | ✅ Serverless | ✅ Serverless | ❌ (planned) |
| **Cost (entry)** | Free tier | $0.01/min | Free + K8s cost | Azure cost | ~$1.5K/mo @ scale | GCP cost | Infrastructure cost |
| **Vendor Lock-in** | Low | Medium | Low | High (Azure) | High (AWS) | High (GCP) | **None** (open source) |

---

## 4. Mobile and Desktop Deployment

### 4.1 The Challenge

Most AI agent platforms are **cloud-native** and assume persistent internet connectivity. This is a major gap for:
- **Mobile apps** (iOS/Android) that need low-latency inference and offline functionality
- **Desktop apps** (Windows, macOS, Linux) that handle sensitive data locally
- **Resource-constrained environments** (embedded, IoT, edge gateways) with limited RAM/CPU
- **Sovereign / air-gapped deployments** where data cannot leave the device

### 4.2 Platform Support for Edge/Mobile/Desktop

| Platform | Mobile | Desktop | Embedded/IoT | Offline |
|---|---|---|---|---|
| **Supabase** | Client SDKs only (JS, Flutter, Swift); no on-device inference | ❌ | ❌ | ❌ |
| **LangGraph** | ❌ | ❌ | ❌ | ❌ |
| **CrewAI** | ❌ | ❌ | ❌ | ❌ |
| **MS Agent Framework** | ❌ | ❌ | ❌ | ❌ |
| **AWS Bedrock** | ❌ | ❌ | ❌ | ❌ |
| **Google Vertex AI** | ❌ | ❌ | ❌ | ❌ |
| **Flint Forge** | ✅ WASM components cross-compile to aarch64 | ✅ x86_64/aarch64 `.cwasm` | ✅ RISC-V, s390x via Wasmtime | ✅ In-DB LLM + local WASM runtime |

### 4.3 How Flint Addresses Edge Deployment

Flint's WASM-based architecture is uniquely suited for edge deployment:

**1. Polyglot WASM Components**
- Rust, JS, Python, Go, C#, C/C++ all compile to `wasi:http/proxy` components.
- These are **content-addressed** (sha256 digest) and **signed** (Ed25519 + DID-VC + Cosign).
- Components are **immutable** and **auditable** — critical for agents running on untrusted edge hardware.

**2. AOT Compilation for Native Performance**
- Cranelift AOT compiles WASM to native machine code (x86_64, aarch64, RISC-V, s390x).
- `.cwasm` artifacts are cached and deserialized at near-zero latency.
- This is **not interpreted** — it runs at native speed with sandbox guarantees.

**3. Resource Governance for Constrained Devices**
- **Fuel/epoch limits**: Prevent runaway agent loops (similar to Ethereum's gas but for AI).
- **Memory ceilings**: `StoreLimits` cap per-invocation RAM.
- **Per-request isolation**: Fresh `Store` + linear memory per invocation → no cross-request state leakage.

**4. Sovereign Inference (No Cloud Required)**
- Flint Ember routes to **UAR Tier-2** (sovereign inference plane) or **candle-vllm** / **RunPod**.
- On-device: Small models (Llama 3B, Phi-3, Gemma 2B) can run via candle or ONNX Runtime inside a WASM sandbox.
- **No API keys on device**: LLM calls route through the local Flint stack; secrets stay in `flint_vault` (KMS-wrapped).

**5. WASI Compatibility**
- The `wasi:http/proxy` world is a standard target supported by:
  - **Wasmer**: Sub-ms cold starts, iOS/Android support, IoT deployments.
  - **Wasmtime**: Production-grade, used by Flint Kiln.
  - **Browser**: `jco` / `componentize-js` can run the same component in a web page.
- This means **one component**, multiple targets: cloud edge function, mobile app plugin, desktop tool, browser extension, IoT firmware.

### 4.4 Emerging Patterns for Edge AI Agents

| Pattern | Technology | Use Case |
|---|---|---|
| **Mobile WASM** | `componentize-js` + `jco` + Wasmer/Wasmtime | Run agent tools inside iOS/Android apps without native code rewrites |
| **Desktop Tauri** | Tauri + Rust + WASM | Flint Forge backend + Tauri frontend = sovereign desktop agent app |
| **Embedded IoT** | Wasmtime on RISC-V/ARM | Sensor data preprocessing, local decision-making, intermittent cloud sync |
| **Browser-Local AI** | Transformers.js + ONNX Runtime + WASM | Privacy-first RAG in the browser; no data leaves the device |
| **Over-the-Air Updates** | Content-addressed WASM components | Update agent tools on edge fleets without downtime; rollback to previous digest if failure |

### 4.5 Limitations of Current Edge AI

- **Model size**: LLMs > 7B parameters are too large for most mobile/embedded devices. Solutions: model distillation, quantization (INT4/INT8), SLMs (Small Language Models).
- **Battery / thermal**: Continuous inference drains battery. Solutions: event-triggered inference, NPUs (Neural Processing Units), offloading to local edge server.
- **Memory**: WASM linear memory is typically 4GB max (32-bit). Large models need 64-bit memory or native execution. Solutions: `memory64` proposal, chunked inference, streaming KV cache.
- **Tool ecosystem**: Many AI tools (web search, code execution, database access) require network connectivity. Offline agents need local tool replacements (local SQLite, local file system, pre-cached knowledge).

---

## 5. Strategic Recommendations

### 5.1 Choose Supabase If...

- You are building a **web-first AI application** with human users (chat, RAG, collaborative editing).
- You want the **fastest time-to-market** with a single integrated platform.
- Your agent workloads are **small-to-medium scale** (< 1M vectors, < 100 concurrent agents).
- You are OK with **external LLM providers** (OpenAI, Anthropic) and don't need sovereign inference.
- You value **low vendor lock-in** (standard Postgres, can self-host or migrate).

### 5.2 Choose Flint Forge If...

- You need **sovereign inference** — models running on infrastructure you fully control.
- You are building **agent tool endpoints** in multiple languages (Rust, Go, Python, JS) and need sandboxed execution.
- You require **fine-grained cost governance** (token metering, per-tenant model routing, Cedar policy).
- You need to deploy agents to **mobile, desktop, or embedded** targets (WASM portability).
- You operate in **air-gapped or compliance-heavy environments** (KMS-wrapped secrets, 4-layer auth, audit logging).
- You want **in-DB LLM execution** (Flint Ember) to eliminate network hops and data egress.

### 5.3 Choose LangGraph If...

- Your primary challenge is **complex agent state management** (multi-step, cyclic, human-in-the-loop).
- You are already in the **LangChain ecosystem** and want managed infrastructure.
- You need **visual debugging** (LangGraph Studio) for agent trajectories.
- You can tolerate **LangChain-ecosystem lock-in**.

### 5.4 Choose CrewAI If...

- You are building **multi-agent teams with explicit roles** (researcher, writer, reviewer, manager).
- You have **Kubernetes expertise** and can manage the infrastructure overhead.
- You need **hierarchical agent delegation** (manager agent delegates to workers).
- You are OK with **Python-only** and primarily cloud deployment.

### 5.5 Choose AWS / Microsoft / Google If...

- You are **already deep in the cloud ecosystem** (AWS, Azure, GCP) and want tight integration.
- You need **enterprise-grade compliance** (SOC 2, SSO, VPC peering, audit trails) out of the box.
- You want **serverless scaling** without managing infrastructure (AgentCore, Azure AI Foundry, Agent Engine).
- You can accept **high vendor lock-in** and **cloud-only deployment**.

---

## 6. Summary: The AI Agent Infrastructure Landscape (2026)

The AI agent infrastructure market has bifurcated into two paths:

1. **Cloud-Native Managed Platforms** (Supabase, LangGraph Cloud, AWS Bedrock, Azure AI Foundry, Vertex AI Agent Engine): Optimized for rapid deployment, auto-scaling, and enterprise compliance. Best for teams that want to ship fast and don't need edge or sovereign control. Trade-off: vendor lock-in, no offline capability, limited cost governance.

2. **Sovereign Edge-Native Platforms** (Flint Forge, self-hosted vLLM + WASM): Optimized for data sovereignty, offline operation, polyglot tool deployment, and fine-grained governance. Best for teams building agents that must run anywhere — cloud, desktop, mobile, IoT — with full control over inference, secrets, and policy. Trade-off: higher initial setup, requires Rust/WASM expertise.

**Flint Forge occupies a unique position**: it is the only open-source platform that combines a Postgres-native data plane (like Supabase) with in-DB LLM inference (like no other), polyglot WASM edge functions (unlike Deno-only Supabase), and sovereign deployment to any target architecture. For teams building the next generation of AI agents — especially those that need to operate offline, across devices, or in compliance-heavy environments — Flint represents a fundamentally different architectural choice.

---

*Sources: Supabase documentation and blog (2024–2026), LangChain/LangGraph docs (2024–2026), CrewAI enterprise docs (2026), Microsoft Agent Framework preview docs (2025–2026), AWS Bedrock AgentCore guidance (2025–2026), Google Vertex AI ADK docs (2025–2026), Flint Forge RFC-FORGE-001 specification.*
