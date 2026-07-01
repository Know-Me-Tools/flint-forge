#!/usr/bin/env python3
"""Generate the branded HTMLX artifact for FLINT-META-EXTENSION-PLAN.md"""
import html as html_module
import re
import textwrap

with open('/Users/gqadonis/Projects/prometheus/flint-forge/docs/FLINT-META-EXTENSION-PLAN.md', 'r') as f:
    md = f.read()

# === SVG DIAGRAMS ===

# SVG 1: Architecture Overview (Two-Layer Stack)
arch_svg = '''<figure>
<svg viewBox="0 0 900 580" xmlns="http://www.w3.org/2000/svg" style="max-width:900px">
  <defs>
    <filter id="sh" x="-3%" y="-3%" width="106%" height="106%"><feDropShadow dx="0" dy="2" stdDeviation="3" flood-color="#000" flood-opacity="0.35"/></filter>
    <marker id="a1" markerWidth="8" markerHeight="8" refX="7" refY="4" orient="auto"><polygon points="0,0 8,4 0,8" fill="#FF6A3D"/></marker>
    <marker id="a2" markerWidth="8" markerHeight="8" refX="7" refY="4" orient="auto"><polygon points="0,0 8,4 0,8" fill="#34CFE6"/></marker>
  </defs>

  <!-- LAYER 1: Inside PostgreSQL -->
  <rect x="10" y="10" width="880" height="280" rx="12" fill="#131A22" stroke="#28333F" stroke-width="1.5"/>
  <text x="450" y="32" text-anchor="middle" font-family="Space Grotesk, sans-serif" font-size="14" font-weight="700" fill="#E8EDF3" letter-spacing="0.08em">LAYER 1: INSIDE POSTGRESQL</text>

  <!-- flint_meta box -->
  <rect x="30" y="45" width="840" height="180" rx="10" fill="#0E141B" stroke="#FF6A3D" stroke-width="2" filter="url(#sh)"/>
  <text x="450" y="65" text-anchor="middle" font-family="Space Grotesk, sans-serif" font-size="12" font-weight="700" fill="#E8EDF3">flint_meta (pgrx extension)</text>

  <!-- Cache tables -->
  <rect x="45" y="78" width="110" height="45" rx="6" fill="#131A22" stroke="#34CFE6" stroke-width="1.2" filter="url(#sh)"/>
  <text x="100" y="97" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="8" fill="#34CFE6">cache_tables</text>
  <text x="100" y="112" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="7" fill="#97A1AE">pre-computed</text>

  <rect x="165" y="78" width="110" height="45" rx="6" fill="#131A22" stroke="#34CFE6" stroke-width="1.2" filter="url(#sh)"/>
  <text x="220" y="97" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="8" fill="#34CFE6">cache_columns</text>
  <text x="220" y="112" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="7" fill="#97A1AE">+ ui_hint</text>

  <rect x="285" y="78" width="110" height="45" rx="6" fill="#131A22" stroke="#34CFE6" stroke-width="1.2" filter="url(#sh)"/>
  <text x="340" y="97" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="8" fill="#34CFE6">cache_functions</text>
  <text x="340" y="112" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="7" fill="#97A1AE">+ rest_path</text>

  <rect x="405" y="78" width="110" height="45" rx="6" fill="#131A22" stroke="#4FD18B" stroke-width="1.2" filter="url(#sh)"/>
  <text x="460" y="97" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="8" fill="#4FD18B">keto_tuples</text>
  <text x="460" y="112" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="7" fill="#97A1AE">permissions</text>

  <rect x="525" y="78" width="110" height="45" rx="6" fill="#131A22" stroke="#4FD18B" stroke-width="1.2" filter="url(#sh)"/>
  <text x="580" y="97" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="8" fill="#4FD18B">vault_keys</text>
  <text x="580" y="112" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="7" fill="#97A1AE">envelope DEK</text>

  <rect x="645" y="78" width="110" height="45" rx="6" fill="#131A22" stroke="#F4B942" stroke-width="1.2" filter="url(#sh)"/>
  <text x="700" y="97" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="8" fill="#F4B942">schema_version</text>
  <text x="700" y="112" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="7" fill="#97A1AE">auto-increment</text>

  <!-- SQL functions label -->
  <text x="450" y="140" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="9" fill="#B4BECB">SQL Functions: meta.tables() · meta.columns() · meta.check_permission() · meta.decrypt_column() · meta.agui()</text>

  <!-- Event triggers label -->
  <text x="450" y="158" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="9" fill="#B4BECB">Event Triggers: ddl_command_end → refresh_cache() · sql_drop → invalidate_cache()</text>

  <!-- NOTIFY channels label -->
  <text x="450" y="176" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="9" fill="#B4BECB">NOTIFY: meta_runtime · keto_changes · vault_rotation · agui_update</text>

  <!-- PostgreSQL core -->
  <rect x="30" y="195" width="840" height="45" rx="6" fill="#131A22" stroke="#28333F" stroke-width="1"/>
  <text x="450" y="215" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="10" fill="#97A1AE">PostgreSQL Core (pg_catalog) + flint_auth · flint_hooks · flint_llm · flint_vault · pg_graphql · pgvector</text>
  <text x="450" y="230" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="8" fill="#97A1AE">+ other extensions</text>

  <!-- Down arrow to Layer 2 -->
  <line x1="450" y1="245" x2="450" y2="270" stroke="#FF6A3D" stroke-width="2" stroke-dasharray="4,3" marker-end="url(#a1)"/>
  <text x="465" y="265" font-family="JetBrains Mono, monospace" font-size="9" fill="#FF6A3D">LISTEN / NOTIFY</text>

  <!-- LAYER 2: Rust Reflection Engine -->
  <rect x="10" y="280" width="880" height="290" rx="12" fill="#131A22" stroke="#28333F" stroke-width="1.5"/>
  <text x="450" y="302" text-anchor="middle" font-family="Space Grotesk, sans-serif" font-size="14" font-weight="700" fill="#E8EDF3" letter-spacing="0.08em">LAYER 2: RUST REFLECTION ENGINE</text>

  <!-- Top row: 4 compiler outputs -->
  <rect x="40" y="315" width="170" height="55" rx="6" fill="#0E141B" stroke="#FF6A3D" stroke-width="1.5" filter="url(#sh)"/>
  <text x="125" y="338" text-anchor="middle" font-family="Space Grotesk, sans-serif" font-size="11" font-weight="600" fill="#E8EDF3">Axum Router</text>
  <text x="125" y="355" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="9" fill="#FF6A3D">(hot-swappable)</text>

  <rect x="225" y="315" width="170" height="55" rx="6" fill="#0E141B" stroke="#34CFE6" stroke-width="1.5" filter="url(#sh)"/>
  <text x="310" y="338" text-anchor="middle" font-family="Space Grotesk, sans-serif" font-size="11" font-weight="600" fill="#E8EDF3">GraphQL Gateway</text>
  <text x="310" y="355" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="9" fill="#34CFE6">async-graphql</text>

  <rect x="410" y="315" width="170" height="55" rx="6" fill="#0E141B" stroke="#4FD18B" stroke-width="1.5" filter="url(#sh)"/>
  <text x="495" y="338" text-anchor="middle" font-family="Space Grotesk, sans-serif" font-size="11" font-weight="600" fill="#E8EDF3">OpenAPI Generator</text>
  <text x="495" y="355" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="9" fill="#4FD18B">utoipa</text>

  <rect x="595" y="315" width="170" height="55" rx="6" fill="#0E141B" stroke="#F4B942" stroke-width="1.5" filter="url(#sh)"/>
  <text x="680" y="338" text-anchor="middle" font-family="Space Grotesk, sans-serif" font-size="11" font-weight="600" fill="#E8EDF3">MCP Manifest</text>
  <text x="680" y="355" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="9" fill="#F4B942">+ AG-UI Compiler</text>

  <!-- ArcSwap IR box -->
  <rect x="120" y="385" width="660" height="70" rx="8" fill="#0E141B" stroke="#FF6A3D" stroke-width="2" filter="url(#sh)"/>
  <text x="450" y="408" text-anchor="middle" font-family="Space Grotesk, sans-serif" font-size="12" font-weight="700" fill="#E8EDF3">Immutable IR (ArcSwap)</text>
  <text x="450" y="428" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="10" fill="#FF6A3D">DatabaseModel → Router → GraphQL → OpenAPI → MCP → AG-UI</text>
  <text x="450" y="444" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="9" fill="#97A1AE">Atomic hot-swap: zero downtime, zero locking</text>

  <!-- SQL Compiler -->
  <rect x="120" y="470" width="300" height="55" rx="6" fill="#0E141B" stroke="#34CFE6" stroke-width="1.5" filter="url(#sh)"/>
  <text x="270" y="493" text-anchor="middle" font-family="Space Grotesk, sans-serif" font-size="11" font-weight="600" fill="#E8EDF3">SQL Compiler</text>
  <text x="270" y="510" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="9" fill="#34CFE6">HTTP → AST → Query AST → SQL → Prepared → JSON</text>

  <!-- Token Propagation -->
  <rect x="450" y="470" width="330" height="55" rx="6" fill="#0E141B" stroke="#4FD18B" stroke-width="1.5" filter="url(#sh)"/>
  <text x="615" y="493" text-anchor="middle" font-family="Space Grotesk, sans-serif" font-size="11" font-weight="600" fill="#E8EDF3">Token Propagation</text>
  <text x="615" y="510" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="9" fill="#4FD18B">JWT Claims · Keto Subject · Vault Key Ref → SET LOCAL</text>

  <!-- Flint Gate -->
  <rect x="120" y="540" width="280" height="35" rx="6" fill="#131A22" stroke="#FF6A3D" stroke-width="1.5" filter="url(#sh)"/>
  <text x="260" y="562" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="10" fill="#FF6A3D">Flint Gate (Axum) → JWT mint → Kratos + Keto + Cedar</text>

  <!-- Realtime Fabric -->
  <rect x="420" y="540" width="280" height="35" rx="6" fill="#131A22" stroke="#4FD18B" stroke-width="1.5" filter="url(#sh)"/>
  <text x="560" y="562" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="10" fill="#4FD18B">Realtime Fabric (Iggy) → WebSocket mux → CRDT → SSE</text>
</svg>
<figcaption><b>Figure 3.1</b> Two-layer architecture: PostgreSQL owns metadata via pgrx extension; Rust reflects it into compiled executable artifacts.</figcaption>
</figure>'''

# SVG 2: Compiler Pipeline
compiler_svg = '''<figure>
<svg viewBox="0 0 800 520" xmlns="http://www.w3.org/2000/svg" style="max-width:800px">
  <defs><filter id="shc" x="-3%" y="-3%" width="106%" height="106%"><feDropShadow dx="0" dy="2" stdDeviation="2" flood-color="#000" flood-opacity="0.3"/></filter>
    <marker id="ac" markerWidth="8" markerHeight="8" refX="7" refY="4" orient="auto"><polygon points="0,0 8,4 0,8" fill="#FF6A3D"/></marker>
  </defs>
  <rect x="10" y="10" width="780" height="500" rx="12" fill="#131A22" stroke="#28333F" stroke-width="1.5"/>

  <!-- PostgreSQL input -->
  <rect x="280" y="25" width="240" height="40" rx="6" fill="#0E141B" stroke="#4FD18B" stroke-width="2" filter="url(#shc)"/>
  <text x="400" y="45" text-anchor="middle" font-family="Space Grotesk, sans-serif" font-size="12" font-weight="700" fill="#E8EDF3">PostgreSQL (flint_meta schema)</text>
  <text x="400" y="58" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="9" fill="#4FD18B">SELECT * FROM meta.tables(), meta.columns(), etc.</text>

  <line x1="400" y1="65" x2="400" y2="85" stroke="#FF6A3D" stroke-width="2" marker-end="url(#ac)"/>

  <!-- Reflection -->
  <rect x="280" y="90" width="240" height="35" rx="6" fill="#0E141B" stroke="#FF6A3D" stroke-width="1.5" filter="url(#shc)"/>
  <text x="400" y="112" text-anchor="middle" font-family="Space Grotesk, sans-serif" font-size="11" font-weight="600" fill="#E8EDF3">ReflectionEngine::reflect()</text>

  <line x1="400" y1="125" x2="400" y2="145" stroke="#FF6A3D" stroke-width="2" marker-end="url(#ac)"/>

  <!-- DatabaseModel -->
  <rect x="280" y="150" width="240" height="35" rx="6" fill="#0E141B" stroke="#FF6A3D" stroke-width="2" filter="url(#shc)"/>
  <text x="400" y="172" text-anchor="middle" font-family="Space Grotesk, sans-serif" font-size="11" font-weight="700" fill="#E8EDF3">DatabaseModel (Immutable IR)</text>

  <!-- Branch to 6 compiler stages -->
  <line x1="400" y1="185" x2="400" y2="205" stroke="#FF6A3D" stroke-width="1.5"/>
  <line x1="100" y1="205" x2="700" y2="205" stroke="#FF6A3D" stroke-width="1.5"/>

  <!-- Stage 1: Normalization -->
  <line x1="100" y1="205" x2="100" y2="225" stroke="#FF6A3D" stroke-width="1.5" marker-end="url(#ac)"/>
  <rect x="40" y="230" width="120" height="40" rx="6" fill="#0E141B" stroke="#34CFE6" stroke-width="1.2" filter="url(#shc)"/>
  <text x="100" y="248" text-anchor="middle" font-family="Space Grotesk, sans-serif" font-size="10" font-weight="600" fill="#E8EDF3">Normalization</text>
  <text x="100" y="262" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="8" fill="#34CFE6">Resolve domains</text>

  <!-- Stage 2: Validation -->
  <line x1="220" y1="205" x2="220" y2="225" stroke="#FF6A3D" stroke-width="1.5" marker-end="url(#ac)"/>
  <rect x="160" y="230" width="120" height="40" rx="6" fill="#0E141B" stroke="#34CFE6" stroke-width="1.2" filter="url(#shc)"/>
  <text x="220" y="248" text-anchor="middle" font-family="Space Grotesk, sans-serif" font-size="10" font-weight="600" fill="#E8EDF3">Validation</text>
  <text x="220" y="262" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="8" fill="#34CFE6">Detect cycles</text>

  <!-- Stage 3: Permission Analysis -->
  <line x1="340" y1="205" x2="340" y2="225" stroke="#FF6A3D" stroke-width="1.5" marker-end="url(#ac)"/>
  <rect x="280" y="230" width="120" height="40" rx="6" fill="#0E141B" stroke="#34CFE6" stroke-width="1.2" filter="url(#shc)"/>
  <text x="340" y="248" text-anchor="middle" font-family="Space Grotesk, sans-serif" font-size="10" font-weight="600" fill="#E8EDF3">Permission Analysis</text>
  <text x="340" y="262" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="8" fill="#34CFE6">Keto/RLS/Cedar</text>

  <!-- Stage 4: Endpoint Generation -->
  <line x1="460" y1="205" x2="460" y2="225" stroke="#FF6A3D" stroke-width="1.5" marker-end="url(#ac)"/>
  <rect x="400" y="230" width="120" height="40" rx="6" fill="#0E141B" stroke="#34CFE6" stroke-width="1.2" filter="url(#shc)"/>
  <text x="460" y="248" text-anchor="middle" font-family="Space Grotesk, sans-serif" font-size="10" font-weight="600" fill="#E8EDF3">Endpoint Generation</text>
  <text x="460" y="262" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="8" fill="#34CFE6">REST routes</text>

  <!-- Stage 5: OpenAPI Compiler -->
  <line x1="580" y1="205" x2="580" y2="225" stroke="#FF6A3D" stroke-width="1.5" marker-end="url(#ac)"/>
  <rect x="520" y="230" width="120" height="40" rx="6" fill="#0E141B" stroke="#34CFE6" stroke-width="1.2" filter="url(#shc)"/>
  <text x="580" y="248" text-anchor="middle" font-family="Space Grotesk, sans-serif" font-size="10" font-weight="600" fill="#E8EDF3">OpenAPI Compiler</text>
  <text x="580" y="262" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="8" fill="#34CFE6">utoipa</text>

  <!-- Stage 6: GraphQL SDL -->
  <line x1="700" y1="205" x2="700" y2="225" stroke="#FF6A3D" stroke-width="1.5" marker-end="url(#ac)"/>
  <rect x="640" y="230" width="120" height="40" rx="6" fill="#0E141B" stroke="#34CFE6" stroke-width="1.2" filter="url(#shc)"/>
  <text x="700" y="248" text-anchor="middle" font-family="Space Grotesk, sans-serif" font-size="10" font-weight="600" fill="#E8EDF3">GraphQL SDL</text>
  <text x="700" y="262" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="8" fill="#34CFE6">schema.graphql</text>

  <!-- Merge line -->
  <line x1="100" y1="270" x2="700" y2="270" stroke="#FF6A3D" stroke-width="1.5"/>
  <line x1="400" y1="270" x2="400" y2="290" stroke="#FF6A3D" stroke-width="2" marker-end="url(#ac)"/>

  <!-- MCP + AG-UI compilers -->
  <rect x="200" y="295" width="200" height="35" rx="6" fill="#0E141B" stroke="#F4B942" stroke-width="1.5" filter="url(#shc)"/>
  <text x="300" y="315" text-anchor="middle" font-family="Space Grotesk, sans-serif" font-size="10" font-weight="600" fill="#E8EDF3">MCP Compiler</text>
  <text x="300" y="328" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="8" fill="#F4B942">Tool manifest</text>

  <rect x="420" y="295" width="200" height="35" rx="6" fill="#0E141B" stroke="#F4B942" stroke-width="1.5" filter="url(#shc)"/>
  <text x="520" y="315" text-anchor="middle" font-family="Space Grotesk, sans-serif" font-size="10" font-weight="600" fill="#E8EDF3">AG-UI Compiler</text>
  <text x="520" y="328" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="8" fill="#F4B942">A2UI descriptors</text>

  <!-- Merge to ArcSwap -->
  <line x1="300" y1="330" x2="400" y2="360" stroke="#F4B942" stroke-width="1.5"/>
  <line x1="520" y1="330" x2="400" y2="360" stroke="#F4B942" stroke-width="1.5"/>
  <line x1="400" y1="360" x2="400" y2="380" stroke="#FF6A3D" stroke-width="2" marker-end="url(#ac)"/>

  <!-- ArcSwap Hot-Swap -->
  <rect x="280" y="385" width="240" height="45" rx="8" fill="#0E141B" stroke="#FF6A3D" stroke-width="2" filter="url(#shc)"/>
  <text x="400" y="408" text-anchor="middle" font-family="Space Grotesk, sans-serif" font-size="12" font-weight="700" fill="#E8EDF3">ArcSwap Hot-Swap</text>
  <text x="400" y="425" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="9" fill="#FF6A3D">Atomic replacement: zero downtime, zero locking</text>

  <!-- Outputs -->
  <line x1="400" y1="430" x2="400" y2="450" stroke="#FF6A3D" stroke-width="1.5"/>
  <line x1="100" y1="450" x2="700" y2="450" stroke="#FF6A3D" stroke-width="1.5"/>

  <line x1="100" y1="450" x2="100" y2="470" stroke="#4FD18B" stroke-width="1.5" marker-end="url(#ac)"/>
  <rect x="40" y="475" width="120" height="30" rx="6" fill="#0E141B" stroke="#4FD18B" stroke-width="1.2" filter="url(#shc)"/>
  <text x="100" y="494" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="9" fill="#4FD18B">Router</text>

  <line x1="220" y1="450" x2="220" y2="470" stroke="#4FD18B" stroke-width="1.5" marker-end="url(#ac)"/>
  <rect x="160" y="475" width="120" height="30" rx="6" fill="#0E141B" stroke="#4FD18B" stroke-width="1.2" filter="url(#shc)"/>
  <text x="220" y="494" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="9" fill="#4FD18B">OpenAPI</text>

  <line x1="340" y1="450" x2="340" y2="470" stroke="#4FD18B" stroke-width="1.5" marker-end="url(#ac)"/>
  <rect x="280" y="475" width="120" height="30" rx="6" fill="#0E141B" stroke="#4FD18B" stroke-width="1.2" filter="url(#shc)"/>
  <text x="340" y="494" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="9" fill="#4FD18B">GraphQL</text>

  <line x1="460" y1="450" x2="460" y2="470" stroke="#4FD18B" stroke-width="1.5" marker-end="url(#ac)"/>
  <rect x="400" y="475" width="120" height="30" rx="6" fill="#0E141B" stroke="#4FD18B" stroke-width="1.2" filter="url(#shc)"/>
  <text x="460" y="494" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="9" fill="#4FD18B">MCP Tools</text>

  <line x1="580" y1="450" x2="580" y2="470" stroke="#4FD18B" stroke-width="1.5" marker-end="url(#ac)"/>
  <rect x="520" y="475" width="120" height="30" rx="6" fill="#0E141B" stroke="#4FD18B" stroke-width="1.2" filter="url(#shc)"/>
  <text x="580" y="494" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="9" fill="#4FD18B">AG-UI/A2UI</text>

  <line x1="700" y1="450" x2="700" y2="470" stroke="#4FD18B" stroke-width="1.5" marker-end="url(#ac)"/>
  <rect x="640" y="475" width="120" height="30" rx="6" fill="#0E141B" stroke="#4FD18B" stroke-width="1.2" filter="url(#shc)"/>
  <text x="700" y="494" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="9" fill="#4FD18B">Agent Tools</text>
</svg>
<figcaption><b>Figure 5.1</b> Compiler pipeline: DatabaseModel → Normalization → Validation → Permission Analysis → Endpoint Generation → OpenAPI → GraphQL SDL → MCP → AG-UI → ArcSwap hot-swap.</figcaption>
</figure>'''

# SVG 3: JWT Flow
jwt_svg = '''<figure>
<svg viewBox="0 0 800 340" xmlns="http://www.w3.org/2000/svg" style="max-width:800px">
  <defs><filter id="shj" x="-3%" y="-3%" width="106%" height="106%"><feDropShadow dx="0" dy="2" stdDeviation="2" flood-color="#000" flood-opacity="0.3"/></filter>
    <marker id="aj" markerWidth="8" markerHeight="8" refX="7" refY="4" orient="auto"><polygon points="0,0 8,4 0,8" fill="#FF6A3D"/></marker>
  </defs>
  <rect x="10" y="10" width="780" height="320" rx="12" fill="#131A22" stroke="#28333F" stroke-width="1.5"/>

  <!-- User Request -->
  <rect x="30" y="25" width="120" height="35" rx="6" fill="#0E141B" stroke="#E8EDF3" stroke-width="1.5" filter="url(#shj)"/>
  <text x="90" y="45" text-anchor="middle" font-family="Space Grotesk, sans-serif" font-size="11" font-weight="600" fill="#E8EDF3">User Request</text>
  <text x="90" y="58" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="8" fill="#97A1AE">with JWT</text>

  <line x1="155" y1="42" x2="175" y2="42" stroke="#E8EDF3" stroke-width="1.5" marker-end="url(#aj)"/>

  <!-- Flint Gate -->
  <rect x="180" y="25" width="140" height="35" rx="6" fill="#0E141B" stroke="#FF6A3D" stroke-width="2" filter="url(#shj)"/>
  <text x="250" y="45" text-anchor="middle" font-family="Space Grotesk, sans-serif" font-size="11" font-weight="600" fill="#E8EDF3">Flint Gate</text>
  <text x="250" y="58" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="8" fill="#FF6A3D">Axum / Rust</text>

  <line x1="325" y1="42" x2="345" y2="42" stroke="#FF6A3D" stroke-width="1.5" marker-end="url(#aj)"/>

  <!-- Kratos -->
  <rect x="350" y="25" width="120" height="35" rx="6" fill="#0E141B" stroke="#34CFE6" stroke-width="1.2" filter="url(#shj)"/>
  <text x="410" y="45" text-anchor="middle" font-family="Space Grotesk, sans-serif" font-size="10" font-weight="600" fill="#E8EDF3">Kratos</text>
  <text x="410" y="58" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="8" fill="#34CFE6">Validate session</text>

  <line x1="475" y1="42" x2="495" y2="42" stroke="#34CFE6" stroke-width="1.5" marker-end="url(#aj)"/>

  <!-- Keto -->
  <rect x="500" y="25" width="120" height="35" rx="6" fill="#0E141B" stroke="#4FD18B" stroke-width="1.2" filter="url(#shj)"/>
  <text x="560" y="45" text-anchor="middle" font-family="Space Grotesk, sans-serif" font-size="10" font-weight="600" fill="#E8EDF3">Keto</text>
  <text x="560" y="58" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="8" fill="#4FD18B">Coarse check</text>

  <line x1="625" y1="42" x2="645" y2="42" stroke="#4FD18B" stroke-width="1.5" marker-end="url(#aj)"/>

  <!-- Cedar -->
  <rect x="650" y="25" width="120" height="35" rx="6" fill="#0E141B" stroke="#F4B942" stroke-width="1.2" filter="url(#shj)"/>
  <text x="710" y="45" text-anchor="middle" font-family="Space Grotesk, sans-serif" font-size="10" font-weight="600" fill="#E8EDF3">Cedar</text>
  <text x="710" y="58" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="8" fill="#F4B942">Capabilities</text>

  <!-- JWT minted arrow -->
  <line x1="250" y1="65" x2="250" y2="90" stroke="#FF6A3D" stroke-width="1.5" marker-end="url(#aj)"/>

  <!-- Enriched JWT -->
  <rect x="140" y="95" width="220" height="55" rx="6" fill="#0E141B" stroke="#FF6A3D" stroke-width="1.5" filter="url(#shj)"/>
  <text x="250" y="112" text-anchor="middle" font-family="Space Grotesk, sans-serif" font-size="10" font-weight="600" fill="#E8EDF3">Enriched JWT</text>
  <text x="250" y="128" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="8" fill="#97A1AE">sub, roles, keto_subject, vault_key_id</text>
  <text x="250" y="142" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="7" fill="#97A1AE">+ Cedar capabilities</text>

  <line x1="365" y1="122" x2="385" y2="122" stroke="#FF6A3D" stroke-width="1.5" marker-end="url(#aj)"/>

  <!-- Flint Forge -->
  <rect x="390" y="100" width="140" height="45" rx="6" fill="#0E141B" stroke="#FF6A3D" stroke-width="2" filter="url(#shj)"/>
  <text x="460" y="120" text-anchor="middle" font-family="Space Grotesk, sans-serif" font-size="11" font-weight="600" fill="#E8EDF3">Flint Forge</text>
  <text x="460" y="138" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="8" fill="#FF6A3D">SET LOCAL GUC</text>

  <line x1="535" y1="122" x2="555" y2="122" stroke="#FF6A3D" stroke-width="1.5" marker-end="url(#aj)"/>

  <!-- PostgreSQL -->
  <rect x="560" y="100" width="200" height="45" rx="6" fill="#0E141B" stroke="#4FD18B" stroke-width="2" filter="url(#shj)"/>
  <text x="660" y="120" text-anchor="middle" font-family="Space Grotesk, sans-serif" font-size="11" font-weight="600" fill="#E8EDF3">PostgreSQL</text>
  <text x="660" y="138" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="8" fill="#4FD18B">flint_meta extension</text>

  <!-- Down to inner checks -->
  <line x1="660" y1="150" x2="660" y2="170" stroke="#4FD18B" stroke-width="1.5" marker-end="url(#aj)"/>

  <!-- Permission checks -->
  <rect x="520" y="175" width="280" height="55" rx="6" fill="#0E141B" stroke="#34CFE6" stroke-width="1.5" filter="url(#shj)"/>
  <text x="660" y="195" text-anchor="middle" font-family="Space Grotesk, sans-serif" font-size="10" font-weight="600" fill="#E8EDF3">Permission Checks</text>
  <text x="660" y="212" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="8" fill="#34CFE6">keto_check() → vault.decrypt_column() → RLS</text>
  <text x="660" y="226" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="7" fill="#97A1AE">All inline, zero round-trips</text>

  <!-- Down to SQL execution -->
  <line x1="660" y1="235" x2="660" y2="255" stroke="#34CFE6" stroke-width="1.5" marker-end="url(#aj)"/>

  <rect x="540" y="260" width="240" height="35" rx="6" fill="#0E141B" stroke="#4FD18B" stroke-width="1.5" filter="url(#shj)"/>
  <text x="660" y="280" text-anchor="middle" font-family="Space Grotesk, sans-serif" font-size="10" font-weight="600" fill="#E8EDF3">SQL Execution</text>
  <text x="660" y="292" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="8" fill="#4FD18B">Prepared statement → JSON result</text>

  <!-- Realtime fanout -->
  <line x1="660" y1="300" x2="660" y2="315" stroke="#4FD18B" stroke-width="1.5"/>
  <line x1="460" y1="315" x2="660" y2="315" stroke="#4FD18B" stroke-width="1.5" marker-end="url(#aj)"/>
  <rect x="260" y="300" width="200" height="25" rx="6" fill="#131A22" stroke="#4FD18B" stroke-width="1.2" filter="url(#shj)"/>
  <text x="360" y="315" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="9" fill="#4FD18B">Iggy → WebSocket / SSE / CRDT</text>
</svg>
<figcaption><b>Figure 6.1</b> JWT identity flow: Kratos validates, Keto checks, Cedar evaluates, then enriched JWT propagates via SET LOCAL into PostgreSQL for inline permission, encryption, and RLS.</figcaption>
</figure>'''

# SVG 4: Realtime Notification Architecture
realtime_svg = '''<figure>
<svg viewBox="0 0 800 280" xmlns="http://www.w3.org/2000/svg" style="max-width:800px">
  <defs><filter id="shr" x="-3%" y="-3%" width="106%" height="106%"><feDropShadow dx="0" dy="2" stdDeviation="2" flood-color="#000" flood-opacity="0.3"/></filter>
    <marker id="ar" markerWidth="8" markerHeight="8" refX="7" refY="4" orient="auto"><polygon points="0,0 8,4 0,8" fill="#FF6A3D"/></marker>
  </defs>
  <rect x="10" y="10" width="780" height="260" rx="12" fill="#131A22" stroke="#28333F" stroke-width="1.5"/>
  <text x="400" y="32" text-anchor="middle" font-family="Space Grotesk, sans-serif" font-size="14" font-weight="700" fill="#E8EDF3" letter-spacing="0.08em">REALTIME NOTIFICATION ARCHITECTURE</text>

  <!-- PostgreSQL NOTIFY -->
  <rect x="30" y="50" width="140" height="45" rx="6" fill="#0E141B" stroke="#4FD18B" stroke-width="2" filter="url(#shr)"/>
  <text x="100" y="70" text-anchor="middle" font-family="Space Grotesk, sans-serif" font-size="11" font-weight="600" fill="#E8EDF3">PostgreSQL</text>
  <text x="100" y="86" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="9" fill="#4FD18B">NOTIFY channels</text>

  <line x1="175" y1="72" x2="195" y2="72" stroke="#4FD18B" stroke-width="2" marker-end="url(#ar)"/>

  <!-- flint-reflection listener -->
  <rect x="200" y="50" width="160" height="45" rx="6" fill="#0E141B" stroke="#FF6A3D" stroke-width="2" filter="url(#shr)"/>
  <text x="280" y="70" text-anchor="middle" font-family="Space Grotesk, sans-serif" font-size="11" font-weight="600" fill="#E8EDF3">Listener</text>
  <text x="280" y="86" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="9" fill="#FF6A3D">flint-reflection</text>

  <line x1="365" y1="72" x2="385" y2="72" stroke="#FF6A3D" stroke-width="2" marker-end="url(#ar)"/>

  <!-- Iggy Producer -->
  <rect x="390" y="50" width="140" height="45" rx="6" fill="#0E141B" stroke="#FF6A3D" stroke-width="1.5" filter="url(#shr)"/>
  <text x="460" y="70" text-anchor="middle" font-family="Space Grotesk, sans-serif" font-size="11" font-weight="600" fill="#E8EDF3">Iggy Producer</text>
  <text x="460" y="86" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="9" fill="#FF6A3D">topic: meta.changes</text>

  <line x1="535" y1="72" x2="555" y2="72" stroke="#FF6A3D" stroke-width="2" marker-end="url(#ar)"/>

  <!-- Iggy Spine -->
  <rect x="560" y="50" width="140" height="45" rx="6" fill="#0E141B" stroke="#FF6A3D" stroke-width="2" filter="url(#shr)"/>
  <text x="630" y="70" text-anchor="middle" font-family="Space Grotesk, sans-serif" font-size="11" font-weight="600" fill="#E8EDF3">Iggy Spine</text>
  <text x="630" y="86" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="9" fill="#FF6A3D">Event bus</text>

  <!-- Fanout arrows -->
  <line x1="630" y1="95" x2="630" y2="115" stroke="#FF6A3D" stroke-width="1.5"/>
  <line x1="150" y1="115" x2="630" y2="115" stroke="#FF6A3D" stroke-width="1.5"/>

  <line x1="150" y1="115" x2="150" y2="135" stroke="#34CFE6" stroke-width="1.5" marker-end="url(#ar)"/>
  <rect x="70" y="140" width="160" height="40" rx="6" fill="#0E141B" stroke="#34CFE6" stroke-width="1.5" filter="url(#shr)"/>
  <text x="150" y="160" text-anchor="middle" font-family="Space Grotesk, sans-serif" font-size="10" font-weight="600" fill="#E8EDF3">WebSocket mux</text>
  <text x="150" y="174" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="8" fill="#34CFE6">AG-UI updates</text>

  <line x1="290" y1="115" x2="290" y2="135" stroke="#34CFE6" stroke-width="1.5" marker-end="url(#ar)"/>
  <rect x="210" y="140" width="160" height="40" rx="6" fill="#0E141B" stroke="#34CFE6" stroke-width="1.5" filter="url(#shr)"/>
  <text x="290" y="160" text-anchor="middle" font-family="Space Grotesk, sans-serif" font-size="10" font-weight="600" fill="#E8EDF3">SSE Stream</text>
  <text x="290" y="174" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="8" fill="#34CFE6">Schema updates</text>

  <line x1="430" y1="115" x2="430" y2="135" stroke="#34CFE6" stroke-width="1.5" marker-end="url(#ar)"/>
  <rect x="350" y="140" width="160" height="40" rx="6" fill="#0E141B" stroke="#34CFE6" stroke-width="1.5" filter="url(#shr)"/>
  <text x="430" y="160" text-anchor="middle" font-family="Space Grotesk, sans-serif" font-size="10" font-weight="600" fill="#E8EDF3">gRPC Fanout</text>
  <text x="430" y="174" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="8" fill="#34CFE6">MCP tool updates</text>

  <line x1="570" y1="115" x2="570" y2="135" stroke="#34CFE6" stroke-width="1.5" marker-end="url(#ar)"/>
  <rect x="490" y="140" width="160" height="40" rx="6" fill="#0E141B" stroke="#34CFE6" stroke-width="1.5" filter="url(#shr)"/>
  <text x="570" y="160" text-anchor="middle" font-family="Space Grotesk, sans-serif" font-size="10" font-weight="600" fill="#E8EDF3">CRDT Sync</text>
  <text x="570" y="174" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="8" fill="#34CFE6">Federation</text>

  <line x1="710" y1="115" x2="710" y2="135" stroke="#34CFE6" stroke-width="1.5" marker-end="url(#ar)"/>
  <rect x="630" y="140" width="160" height="40" rx="6" fill="#0E141B" stroke="#34CFE6" stroke-width="1.5" filter="url(#shr)"/>
  <text x="710" y="160" text-anchor="middle" font-family="Space Grotesk, sans-serif" font-size="10" font-weight="600" fill="#E8EDF3">Audit Log</text>
  <text x="710" y="174" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="8" fill="#34CFE6">Compliance</text>

  <!-- Channel labels -->
  <rect x="30" y="200" width="170" height="40" rx="6" fill="#131A22" stroke="#FF6A3D" stroke-width="1" filter="url(#shr)"/>
  <text x="115" y="215" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="9" fill="#FF6A3D">meta_runtime</text>
  <text x="115" y="230" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="8" fill="#97A1AE">DDL changes</text>

  <rect x="210" y="200" width="170" height="40" rx="6" fill="#131A22" stroke="#4FD18B" stroke-width="1" filter="url(#shr)"/>
  <text x="295" y="215" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="9" fill="#4FD18B">keto_changes</text>
  <text x="295" y="230" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="8" fill="#97A1AE">Permission changes</text>

  <rect x="390" y="200" width="170" height="40" rx="6" fill="#131A22" stroke="#F4B942" stroke-width="1" filter="url(#shr)"/>
  <text x="475" y="215" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="9" fill="#F4B942">vault_rotation</text>
  <text x="475" y="230" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="8" fill="#97A1AE">Key rotation events</text>

  <rect x="570" y="200" width="170" height="40" rx="6" fill="#131A22" stroke="#34CFE6" stroke-width="1" filter="url(#shr)"/>
  <text x="655" y="215" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="9" fill="#34CFE6">agui_update</text>
  <text x="655" y="230" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="8" fill="#97A1AE">UI descriptor updates</text>
</svg>
<figcaption><b>Figure 8.1</b> Realtime notification architecture: four NOTIFY channels feed into Iggy, which fans out to WebSocket, SSE, gRPC, CRDT, and audit consumers.</figcaption>
</figure>'''

# SVG 5: Flint Ecosystem Integration
ecosystem_svg = '''<figure>
<svg viewBox="0 0 800 340" xmlns="http://www.w3.org/2000/svg" style="max-width:800px">
  <defs><filter id="she" x="-3%" y="-3%" width="106%" height="106%"><feDropShadow dx="0" dy="2" stdDeviation="2" flood-color="#000" flood-opacity="0.3"/></filter>
    <marker id="ae" markerWidth="8" markerHeight="8" refX="7" refY="4" orient="auto"><polygon points="0,0 8,4 0,8" fill="#FF6A3D"/></marker>
  </defs>
  <rect x="10" y="10" width="780" height="320" rx="12" fill="#131A22" stroke="#28333F" stroke-width="1.5"/>
  <text x="400" y="32" text-anchor="middle" font-family="Space Grotesk, sans-serif" font-size="14" font-weight="700" fill="#E8EDF3" letter-spacing="0.08em">FLINT ECOSYSTEM INTEGRATION</text>

  <!-- Flint Gate -->
  <rect x="30" y="50" width="160" height="55" rx="8" fill="#0E141B" stroke="#FF6A3D" stroke-width="2" filter="url(#she)"/>
  <text x="110" y="72" text-anchor="middle" font-family="Space Grotesk, sans-serif" font-size="12" font-weight="700" fill="#E8EDF3">Flint Gate</text>
  <text x="110" y="90" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="9" fill="#FF6A3D">Axum / Rust</text>
  <text x="110" y="100" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="8" fill="#97A1AE">Kratos + Keto + Cedar + Vault</text>

  <line x1="195" y1="77" x2="215" y2="77" stroke="#FF6A3D" stroke-width="2" marker-end="url(#ae)"/>
  <text x="205" y="68" font-family="JetBrains Mono, monospace" font-size="8" fill="#FF6A3D">JWT</text>

  <!-- Flint Forge -->
  <rect x="220" y="50" width="160" height="55" rx="8" fill="#0E141B" stroke="#FF6A3D" stroke-width="2" filter="url(#she)"/>
  <text x="300" y="72" text-anchor="middle" font-family="Space Grotesk, sans-serif" font-size="12" font-weight="700" fill="#E8EDF3">Flint Forge</text>
  <text x="300" y="90" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="9" fill="#FF6A3D">Meta + Reflection</text>
  <text x="300" y="100" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="8" fill="#97A1AE">REST + GraphQL + OpenAPI</text>

  <line x1="385" y1="77" x2="405" y2="77" stroke="#FF6A3D" stroke-width="2" marker-end="url(#ae)"/>
  <text x="395" y="68" font-family="JetBrains Mono, monospace" font-size="8" fill="#FF6A3D">SQL</text>

  <!-- Realtime Fabric -->
  <rect x="410" y="50" width="180" height="55" rx="8" fill="#0E141B" stroke="#4FD18B" stroke-width="2" filter="url(#she)"/>
  <text x="500" y="72" text-anchor="middle" font-family="Space Grotesk, sans-serif" font-size="12" font-weight="700" fill="#E8EDF3">Realtime Fabric</text>
  <text x="500" y="90" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="9" fill="#4FD18B">Iggy / WebSocket</text>
  <text x="500" y="100" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="8" fill="#97A1AE">CRDT + SSE + gRPC</text>

  <!-- Down to PostgreSQL -->
  <line x1="300" y1="105" x2="300" y2="125" stroke="#FF6A3D" stroke-width="2" marker-end="url(#ae)"/>
  <line x1="500" y1="105" x2="500" y2="125" stroke="#4FD18B" stroke-width="2" marker-end="url(#ae)"/>

  <!-- PostgreSQL box -->
  <rect x="100" y="130" width="600" height="110" rx="10" fill="#0E141B" stroke="#4FD18B" stroke-width="2" filter="url(#she)"/>
  <text x="400" y="150" text-anchor="middle" font-family="Space Grotesk, sans-serif" font-size="13" font-weight="700" fill="#E8EDF3">PostgreSQL 18</text>

  <!-- Extension pills -->
  <rect x="120" y="160" width="80" height="28" rx="4" fill="#131A22" stroke="#FF6A3D" stroke-width="1"/>
  <text x="160" y="178" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="8" fill="#FF6A3D">flint_meta</text>

  <rect x="210" y="160" width="80" height="28" rx="4" fill="#131A22" stroke="#34CFE6" stroke-width="1"/>
  <text x="250" y="178" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="8" fill="#34CFE6">flint_auth</text>

  <rect x="300" y="160" width="80" height="28" rx="4" fill="#131A22" stroke="#34CFE6" stroke-width="1"/>
  <text x="340" y="178" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="8" fill="#34CFE6">flint_hooks</text>

  <rect x="390" y="160" width="80" height="28" rx="4" fill="#131A22" stroke="#34CFE6" stroke-width="1"/>
  <text x="430" y="178" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="8" fill="#34CFE6">flint_llm</text>

  <rect x="480" y="160" width="80" height="28" rx="4" fill="#131A22" stroke="#34CFE6" stroke-width="1"/>
  <text x="520" y="178" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="8" fill="#34CFE6">flint_vault</text>

  <rect x="570" y="160" width="80" height="28" rx="4" fill="#131A22" stroke="#F4B942" stroke-width="1"/>
  <text x="610" y="178" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="8" fill="#F4B942">pg_graphql</text>

  <rect x="120" y="195" width="80" height="28" rx="4" fill="#131A22" stroke="#F4B942" stroke-width="1"/>
  <text x="160" y="213" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="8" fill="#F4B942">pgvector</text>

  <rect x="210" y="195" width="80" height="28" rx="4" fill="#131A22" stroke="#F4B942" stroke-width="1"/>
  <text x="250" y="213" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="8" fill="#F4B942">pg_cron</text>

  <rect x="300" y="195" width="80" height="28" rx="4" fill="#131A22" stroke="#F4B942" stroke-width="1"/>
  <text x="340" y="213" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="8" fill="#F4B942">pg_net</text>

  <rect x="390" y="195" width="80" height="28" rx="4" fill="#131A22" stroke="#F4B942" stroke-width="1"/>
  <text x="430" y="213" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="8" fill="#F4B942">pgsodium</text>

  <rect x="480" y="195" width="80" height="28" rx="4" fill="#131A22" stroke="#F4B942" stroke-width="1"/>
  <text x="520" y="213" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="8" fill="#F4B942">pg_jsonschema</text>

  <!-- External services -->
  <rect x="30" y="260" width="180" height="55" rx="8" fill="#131A22" stroke="#34CFE6" stroke-width="1.5" filter="url(#she)"/>
  <text x="120" y="280" text-anchor="middle" font-family="Space Grotesk, sans-serif" font-size="10" font-weight="600" fill="#E8EDF3">External Services</text>
  <text x="120" y="298" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="8" fill="#34CFE6">AWS KMS / HashiCorp Vault</text>
  <text x="120" y="310" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="8" fill="#34CFE6">Ory Keto / Ory Kratos</text>

  <rect x="590" y="260" width="180" height="55" rx="8" fill="#131A22" stroke="#34CFE6" stroke-width="1.5" filter="url(#she)"/>
  <text x="680" y="280" text-anchor="middle" font-family="Space Grotesk, sans-serif" font-size="10" font-weight="600" fill="#E8EDF3">AI Clients</text>
  <text x="680" y="298" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="8" fill="#34CFE6">AG-UI / A2UI / MCP</text>
  <text x="680" y="310" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="8" fill="#34CFE6">Code generators / IDEs</text>
</svg>
<figcaption><b>Figure 12.1</b> Flint ecosystem integration: Gate authenticates, Forge reflects, Fabric streams, PostgreSQL owns all metadata and extensions.</figcaption>
</figure>'''

# Save SVGs for later insertion
svgs = {
    'arch': arch_svg,
    'compiler': compiler_svg,
    'jwt': jwt_svg,
    'realtime': realtime_svg,
    'ecosystem': ecosystem_svg,
}

print("SVGs prepared")
print(f"arch: {len(arch_svg)} chars")
print(f"compiler: {len(compiler_svg)} chars")
print(f"jwt: {len(jwt_svg)} chars")
print(f"realtime: {len(realtime_svg)} chars")
print(f"ecosystem: {len(ecosystem_svg)} chars")
PYEOF
