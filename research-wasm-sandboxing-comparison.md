# Research: WASM Sandboxing vs Microsandbox & Edge Function Platforms

## Executive Summary

This research compares WebAssembly Component Model + Wasmtime (the stack used by Flint Kiln) against **microsandbox** (a microVM-based process sandbox) and other major edge function platforms. Key findings:

- **Microsandbox** provides hardware-level isolation via microVMs (libkrun/Firecracker-style) with ~200ms cold starts, targeting AI agent code execution and untrusted workloads. It is complementary to WASM, not a replacement, serving different security/performance trade-offs.
- **WASM + Wasmtime** offers microsecond-level startup, instruction-level isolation, and a capability-based security model ideal for high-frequency edge functions. It is the dominant architecture in production edge platforms (Fastly, Fermyon/Akamai, Shopify).
- **Process-based microVMs** (microsandbox, Firecracker) provide stronger isolation boundaries than WASM but at 100-1000x higher startup latency and memory overhead. They are best suited for long-running, high-risk workloads or full OS compatibility needs.
- **Recommendation**: For Flint Kiln, continue with WASM Component Model + Wasmtime for the primary edge function runtime. Consider microsandbox as an **optional, higher-isolation tier** for specific untrusted/AI-generated code scenarios, or for workloads requiring native binary execution that cannot compile to WASM.

---

## 1. Microsandbox Deep Dive

### 1.1 Overview

**Microsandbox** (github.com/microsandbox/microsandbox, formerly superradcompany/microsandbox) is an open-source Rust project that runs untrusted workloads inside fast, local microVMs. It is explicitly designed for AI agents, user code, plugins, CI jobs, and automation scenarios where hardware-level isolation is required.

Key tagline: *"Every agent deserves its own computer."*

### 1.2 Architecture

| Component | Description |
|-----------|-------------|
| **Hypervisor** | `libkrun` — a lightweight VMM library using KVM on Linux and HVF on macOS/Apple Silicon. Similar to AWS Firecracker but simpler. |
| **Guest Init** | Statically-linked Rust binary (`init.krun`) that runs as PID 1 inside the microVM, built for musl libc. |
| **Image Format** | OCI-compatible container images (Docker Hub, GHCR, ECR) automatically translated into microVM root filesystems. |
| **Networking** | virtio-net with slirp-style forwarding, virtio-vsock for host-guest communication (TSI: Transparent Socket Impersonation). |
| **Storage** | virtio-fs mounts, virtio-blk for disk volumes, guest-write quotas supported. |
| **Security Variants** | `libkrun-sev` (AMD SEV memory encryption), `libkrun-tdx` (Intel TDX) for confidential computing. |

### 1.3 Key Capabilities

- **Hardware-level isolation**: Each sandbox runs its own minimal Linux kernel — true virtualization, not container namespaces. Prevents kernel escape exploits.
- **Sub-200ms cold starts**: Through optimized kernel images, minimal device models, and snapshot caching. Slower than WASM but 50-100x faster than traditional VMs.
- **OCI image support**: Runs standard Docker images without conversion — any language, any runtime, any dependency.
- **Multi-language SDKs**: Rust, Python, TypeScript/JavaScript, Go.
- **Network policies**: Public-only (default), allow-all, or fully airgapped. DNS filtering, TLS interception, secret substitution at network layer.
- **Secrets management**: Credentials never enter the VM; placeholder substitution happens at the network layer.
- **Rootless execution**: No daemon required; runs as embedded child process.
- **MCP support**: Native Model Context Protocol integration for AI agent workflows.
- **Metrics**: CPU, memory, disk I/O, network I/O per sandbox.

### 1.4 Development Status

- **Current version**: ~v0.5.10 (as of June 2026)
- **Maturity**: Beta software — breaking changes expected, some missing features.
- **Platform support**: macOS (Apple Silicon only), Linux (x86_64, ARM64 with KVM).
- **No Windows support**: Windows listed as a target for CLI install but not a primary development platform.
- **Releases**: Automated CI for all platforms, publishes to npm, crates.io, PyPI.

---

## 2. WASM Component Model + Wasmtime vs Microsandbox

### 2.1 Comparison Matrix

| Dimension | WASM Component Model + Wasmtime | Microsandbox (microVM) |
|-----------|----------------------------------|------------------------|
| **Isolation boundary** | Instruction-level (virtual ISA) | Hardware-level (KVM/HVF) |
| **Security model** | Capability-based (WASI preopens, no syscalls by default) | Full VM boundary with own kernel |
| **Cold start** | ~0.5–1 ms (WASM instantiation) | ~100–200 ms (microVM boot) |
| **Memory overhead** | ~300 KB–1 MB per instance | ~5 MB+ per microVM (kernel + rootfs) |
| **Max instances per host** | Tens of thousands (pooling allocator) | Hundreds to thousands (limited by KVM capacity) |
| **Attack surface** | Small (VMM + runtime), but shared kernel | Very small (per-VM kernel), hardware-enforced |
| **Zero-day kernel exploit risk** | Potentially host-compromising | Contained within VM boundary |
| **Capability granularity** | Fine-grained (per-file, per-network, per-resource) | Coarse (network policy, volume mounts) |
| **Polyglot support** | Languages that compile to WASM (Rust, Go, C++, C#, JS, Python via componentize-py) | Any language runnable in Linux (via OCI images) |
| **Binary size** | 1–5 MB (WASM binaries) | 50–500 MB (OCI images with full OS) |
| **Startup overhead source** | JIT/AOT compilation (Cranelift) | Kernel boot, device init, filesystem mount |
| **Stateful execution** | Stateless by design (per-request instances) | Supports persistent volumes, long-running processes |
| **Production maturity** | High (Fastly, Fermyon, Shopify, wasmCloud in production) | Beta (newer project, evolving API) |
| **Confidential computing** | Not natively supported | AMD SEV, Intel TDX variants available |
| **Resource limiting** | wasm_fuel, max_memory, max_stack, timeout | CPU/memory quotas, disk I/O limits, network policies |
| **Host compatibility** | Cross-platform (no KVM needed) | Requires KVM (Linux) or HVF (macOS Apple Silicon) |

### 2.2 Security Boundaries in Detail

**WASM + Wasmtime:**
- The guest runs on a virtual instruction set with no direct access to host CPU, memory, or syscall interfaces.
- Security boundary is established *before* execution: all capabilities (file handles, network permissions, resource limits) are injected at instantiation time and cannot be extended at runtime.
- The runtime itself (Wasmtime) is a potential attack surface; RustSec advisories exist. A compiler bug or runtime vulnerability could theoretically allow guest escape, though no major escapes have been publicly documented.
- Spectre mitigations: Host-side, relies on runtime implementation (e.g., Cranelift's sandboxing). No built-in protection against Spectre-based side channels between co-located tenants unless explicitly implemented.

**Microsandbox:**
- Full hardware virtualization: each workload has its own kernel, memory space, and virtual devices.
- Even a zero-day kernel exploit inside the guest only compromises that single microVM.
- KVM/HVF provides hardware-enforced memory isolation (EPT/NPT page tables).
- TEE variants (SEV, TDX) provide memory encryption and remote attestation for confidential computing.
- Trade-off: larger attack surface in the VMM (`libkrun`) itself, but VMM is minimal by design.

### 2.3 Startup Latency Analysis

| Platform | Cold Start | Warm Start | Notes |
|----------|------------|------------|-------|
| WASM (Wasmtime, AOT) | ~0.5 ms | ~0.1 ms | Precompiled modules; pooling allocator |
| WASM (V8 isolate) | ~1–5 ms | ~0.5 ms | Cloudflare Workers scale |
| Microsandbox | ~100–200 ms | ~50 ms (cached) | Kernel boot + init; snapshot caching |
| Docker container | ~500 ms–5 s | ~100 ms | Image layers, overlayfs setup |
| Traditional VM | ~10–60 s | ~5–10 s | Full OS boot, device enumeration |

**Implication**: For high-frequency, short-lived edge functions (e.g., per-request auth, transformation, routing), WASM's microsecond startup is essential. Microsandbox's ~100ms is acceptable for agent tool execution, CI jobs, or long-running plugins but would be prohibitive for edge request handling at scale.

### 2.4 Memory Overhead Analysis

| Platform | Per-Instance Memory | Notes |
|----------|-------------------|-------|
| WASM (Wasmtime, pooled) | ~300 KB–1 MB | Linear memory + runtime structures; no guest OS |
| WASM (V8 isolate) | ~1–5 MB | Including JS heap overhead |
| Microsandbox | ~5 MB+ | Guest kernel + minimal rootfs + virtio overhead |
| Docker container | ~10–100 MB | Shared kernel, but userland + image layers |
| Traditional VM | ~1 GB+ | Full OS memory footprint |

**Implication**: WASM enables 10,000+ tenants per node; microsandbox enables hundreds. For multi-tenant edge hosting, WASM is the only economically viable model.

### 2.5 Capability Model

**WASM (WASI 0.2 / Component Model):**
- **Filesystem**: Per-directory preopened file descriptors (`preopen`). No global filesystem access.
- **Network**: Explicit allowlists (WASI sockets proposal). Host can implement custom filtering.
- **Time**: Monotonic clocks only; wall-clock access can be denied.
- **Randomness**: Explicit entropy source.
- **Environment**: Explicitly provided env vars; no host environment leakage.
- **No ambient authority**: By default, a WASM module has *no* capabilities. All must be explicitly granted.

**Microsandbox:**
- **Filesystem**: Virtio-fs mounts from host paths or named volumes. Read/write quotas configurable.
- **Network**: Three modes: public-only (default), allow-all, airgapped. DNS filtering (block domains/suffixes). TLS interception for secret substitution.
- **Secrets**: Host-side substitution; credentials never enter VM.
- **No fine-grained per-file or per-syscall capability model**: Access is broader than WASM's preopens but simpler to configure.

---

## 3. Other WASM Edge Function Platforms

### 3.1 Cloudflare Workers (V8 Isolates)

| Attribute | Details |
|-----------|---------|
| **Runtime** | V8 JavaScript engine with Isolates (not WASM runtime) |
| **Cold start** | ~1–5 ms |
| **Isolation** | V8 Isolate (single-process, memory-isolated JS contexts) |
| **WASM support** | Supported but slower startup than JS; WASM loaded inside V8 |
| **Languages** | JavaScript, TypeScript, Rust, C++, Python (via Pyodide/WASM) |
| **Scale** | 10M+ requests/second across 300+ edge locations |
| **Security** | Spectre mitigations: degraded local time measurement, no shared array buffers between isolates. V8 team continuously patches. |
| **Limitations** | 128 MB memory, 30–50 ms CPU limit (free tier), 30 s max (paid). No native syscalls — all via V8 APIs. |
| **Key insight** | Proves that non-OS sandboxing (V8) can scale to massive multi-tenancy, but with language limitations and Spectre concerns. |

### 3.2 Fastly Compute@Edge (Wasmtime)

| Attribute | Details |
|-----------|---------|
| **Runtime** | Wasmtime (Bytecode Alliance) directly — no V8 |
| **Cold start** | < 1 ms (sub-50 microsecond in some reports) |
| **Isolation** | Per-instance WASM sandbox |
| **WASM support** | Native; WASI 0.2 compliant |
| **Languages** | Rust, JavaScript/TypeScript (via JS engine compiled to WASM), Go, AssemblyScript |
| **Memory** | 128 MB per instance |
| **Security** | Stronger boundary than V8 isolates (per Wasmtime audit history). No shared JS heap. |
| **Key insight** | Fastest cold starts in the industry. Pure Wasmtime architecture proves WASM can handle production CDN edge traffic at scale. |

### 3.3 Fermyon Spin → Akamai (Wasmtime)

| Attribute | Details |
|-----------|---------|
| **Runtime** | Wasmtime (CNCF Sandbox project) |
| **Cold start** | < 1 ms (sub-0.5 ms claimed for Spin 3.0) |
| **Isolation** | Per-request WASM component instantiation, then discarded — no shared mutable state |
| **Languages** | Rust, TypeScript, Python, Go, .NET, Grain |
| **Deployment** | Self-hosted (Spin), Akamai Cloud (Fermyon Wasm Functions), Kubernetes (SpinKube) |
| **Throughput** | 10x improvement via pooling allocator (Wasmtime internals) |
| **Maturity** | Acquired by Akamai (Dec 2025). 3M+ downloads. Production at Akamai's 4,000+ edge locations. |
| **Key insight** | The clearest example of Component Model-first WASM infrastructure. SpinKube brings WASM to Kubernetes via CRI (Container Runtime Interface). |

### 3.4 Suborbital e2core (formerly Atmo)

| Attribute | Details |
|-----------|---------|
| **Runtime** | Custom WASM scheduler (Reactr) + application framework (Atmo/e2core) |
| **Isolation** | Per-function WASM sandbox |
| **Languages** | Rust, Go, AssemblyScript, Grain |
| **Architecture** | Declarative "Directive" file describes routes and function composition; no boilerplate server code needed. |
| **Use case focus** | Serverless extensibility for SaaS products (Shopify-style user plugins) |
| **Maturity** | Seed-funded ($1.6M), early-stage. Open source. |
| **Key insight** | Targets "serverless extensibility" — letting SaaS vendors run untrusted user code safely. Similar security model to WASM but with a higher-level framework. |

### 3.5 wasmCloud (CNCF, Cosmonic)

| Attribute | Details |
|-----------|---------|
| **Runtime** | Wasmtime |
| **Isolation** | Actor model — each actor is a WASM module |
| **Languages** | All Component Model languages |
| **Messaging** | NATS for inter-actor communication |
| **Differentiator** | Actor model with interface contracts; actors can be swapped without recompilation. |
| **Production users** | Adobe, BMW, MachineMetrics |
| **Key insight** | Distributed systems via WASM, not just edge functions. Strong for microservices replacement. |

### 3.6 Supabase Edge Functions (Deno Runtime)

| Attribute | Details |
|-----------|---------|
| **Runtime** | Deno (TypeScript-first, V8-based) |
| **Isolation** | Deno sandbox (no file/network access by default) — but NOT WASM-level isolation |
| **WASM support** | Can load WASM modules inside Deno, but primary runtime is JS/TS |
| **Cold start** | Similar to V8 isolates (~5–50 ms) |
| **Limitations** | Deno-specific; no native Node.js compatibility. Some developer friction reported (npm package support issues). |
| **Key insight** | Supabase chose Deno for TypeScript-native edge functions, NOT WASM. This is a different trade-off: developer experience (familiar JS syntax) over maximum isolation or polyglot support. |

### 3.7 Platform Comparison Summary

| Platform | Runtime | Cold Start | Isolation | WASI 0.2 | Languages | Best For |
|----------|---------|------------|-----------|----------|-----------|----------|
| **Cloudflare Workers** | V8 Isolates | ~1–5 ms | V8 Isolate | Partial | JS, TS, Rust, C++ | Massive scale, JS-centric |
| **Fastly Compute@Edge** | Wasmtime | < 1 ms | Per-instance WASM | Yes | Rust, JS, Go, AS | Ultra-low latency, security-critical |
| **Fermyon Spin** | Wasmtime | < 1 ms | Per-request WASM | Yes | Rust, TS, Python, Go, .NET | Self-hosted, K8s, edge AI |
| **Suborbital e2core** | Custom WASM | ~1–2 ms | Per-function WASM | Yes | Rust, Go, AS | SaaS extensibility, plugins |
| **wasmCloud** | Wasmtime | < 2 ms | Actor model | Yes | All Component langs | Distributed microservices |
| **Supabase Edge** | Deno/V8 | ~5–50 ms | Deno sandbox | Partial (via Deno) | TypeScript, JS | Developer DX, Supabase ecosystem |
| **AWS Lambda (WASM)** | Custom Wasmtime | ~50 ms | Process-level | Yes | Rust, AS | Heavy compute, longer duration |
| **Microsandbox** | libkrun | ~100–200 ms | Hardware VM | N/A | Any (OCI images) | Untrusted code, AI agents, long-running |

---

## 4. Analysis: Would Microsandbox Add Value to Flint Kiln?

### 4.1 Current Architecture Assumption

Flint Kiln uses **WASM Component Model + Wasmtime** for edge functions. This provides:
- Microsecond-level startup for per-request functions
- Fine-grained capability-based security
- Polyglot support (Rust, Go, JS, Python via componentize-py)
- Small memory footprint enabling high multi-tenancy density

### 4.2 Where Microsandbox Would Complement

| Scenario | Microsandbox Value | WASM Limitation |
|----------|-------------------|-----------------|
| **AI agent code execution** | Run untrusted AI-generated code with hardware isolation; 200ms is acceptable for agent tool calls | WASM requires compilation step; not all AI-generated code is WASM-compatible |
| **Native binary execution** | Run arbitrary Linux binaries (Python with C extensions, Node.js with native modules) without recompilation | WASM only runs code that compiles to WASM; native dependencies are excluded |
| **Long-running plugins** | Persistent microVMs with volumes, stateful execution, background processes | WASM is stateless by design; long-running stateful workloads require external services |
| **Maximum untrusted isolation** | Hardware-enforced boundary for user-submitted code from unknown sources | WASM runtime bugs (while rare) could theoretically allow escape; no hardware barrier |
| **CI/CD job execution** | Run build scripts, test suites in isolated environments with full OS toolchain | WASM lacks full OS toolchain access (no gcc, no Docker) |
| **Confidential computing** | AMD SEV/Intel TDX for encrypted memory and remote attestation | Not available in standard WASM runtimes |

### 4.3 Where Microsandbox Would NOT Add Value

| Scenario | Why Not | WASM Advantage |
|----------|---------|---------------|
| **Per-request edge functions** | 200ms cold start is 100-1000x slower than WASM; unacceptable for request routing | ~0.5 ms startup |
| **High-density multi-tenancy** | 5 MB+ per microVM vs. ~1 MB per WASM instance; cost-prohibitive at 10K+ tenants | 10x+ higher density |
| **Fine-grained capability control** | Microsandbox network policies are coarse (public/allow-all/airgapped); WASM preopens are per-file | Finer security granularity |
| **Languages that compile to WASM** | No benefit; adds overhead for no isolation gain | Direct execution with zero VM overhead |
| **Short-lived, high-frequency functions** | VM boot/shutdown overhead dominates for <10ms executions | Instantiation overhead is negligible |

### 4.4 Recommended Integration Strategy

| Tier | Runtime | Use Case |
|------|---------|----------|
| **Primary (default)** | WASM Component Model + Wasmtime | All standard edge functions: auth, routing, transformation, lightweight compute |
| **Secondary (opt-in)** | Microsandbox (microVM) | Untrusted/AI-generated code, native binary execution, long-running plugins, CI jobs, confidential workloads |
| **Migration path** | Provide both SDKs | Let developers choose isolation level: `#[flint::edge]` for WASM, `#[flint::sandboxed]` for microVM |

### 4.5 Technical Considerations for Integration

1. **Platform requirements**: Microsandbox requires KVM (Linux) or HVF (macOS Apple Silicon). Cannot run on generic x86_64 without virtualization support. This limits deployment targets compared to WASM's cross-platform portability.

2. **Beta maturity**: As of mid-2026, microsandbox is explicitly beta software with breaking changes expected. Production integration should be behind a feature flag with clear disclaimers.

3. **OCI image vs. WASM binary**: Microsandbox uses OCI images (50–500 MB); WASM binaries are 1–5 MB. Storage and bandwidth costs differ by 10-100x. A registry strategy must accommodate both artifact types.

4. **Network model mismatch**: Microsandbox uses virtio-net + port forwarding; WASM uses host-provided WASI sockets. Unifying the networking model in a single platform requires abstraction layers.

5. **No state sharing**: WASM components can share linear memory and host functions efficiently. MicroVMs communicate over virtio-vsock or network sockets — higher latency, lower bandwidth.

---

## 5. Key References & Sources

| Source | Date | URL |
|--------|------|-----|
| Microsandbox GitHub | 2026 | https://github.com/microsandbox/microsandbox |
| libkrun GitHub (VMM library) | 2026 | https://github.com/containers/libkrun |
| Wasmtime Documentation | 2026 | https://docs.wasmtime.dev/ |
| WASI Preview 2 Spec | 2024 | https://github.com/WebAssembly/wasi-cli |
| Cloudflare Workers Architecture | 2026 | https://developers.cloudflare.com/workers/ |
| Fastly Compute@Edge | 2026 | https://www.fastly.com/products/edge-compute |
| Fermyon Spin (now Akamai) | 2025 | https://github.com/fermyon/spin |
| Suborbital e2core (Atmo) | 2021 | https://github.com/suborbital/e2core |
| wasmCloud (CNCF) | 2026 | https://wasmcloud.com/ |
| Supabase Edge Runtime | 2026 | https://github.com/supabase/edge-runtime |
| Akamai acquires Fermyon | 2025-12 | https://devclass.com/2025/12/04/akamai-acquires-fermyon/ |
| WASM vs Container Isolation | 2026-05 | https://www.systemshardening.com/articles/wasm/wasm-isolation-vs-container-isolation/ |
| WebAssembly Serverless Edge | 2026-03 | https://www.birjob.com/blog/webassembly-serverless-eating-kubernetes-edge |
| Server-side WASM Runtimes 2026 | 2026-06 | https://omeronal.com/webassembly-wasm-server-side-2026/ |
| Research on WebAssembly Runtimes | 2024-04 | https://arxiv.org/html/2404.12621v1 |
| Comparative Study of WASM Runtimes | 2024 | https://ojs.bonviewpress.com/index.php/AAES/article/download/4965/1367/29227 |
| Wasmtime Security Model (BoxAgnts) | 2026-06 | https://dev.to/guyoung/boxagnts-tool-system-2-the-security-model-of-wasmtime-sandboxing-1cj0 |
| Zylos: WASM Sandboxing for AI | 2026-03 | https://zylos.ai/research/2026-03-12-wasm-sandboxing-ai-agent-runtime-isolation |
| Microsandbox: Hardware Isolation | 2026-02 | https://prompts.brightcoding.dev/blog/microsandbox-hardware-isolated-sandboxes-for-ai-agents |
| Supabase Edge Functions Docs | 2026-06 | https://supabase.com/docs/guides/functions |

---

*Document compiled: June 2026*
*Research scope: Microsandbox, WASM sandboxing, edge function platforms, process-based vs instruction-based isolation*
