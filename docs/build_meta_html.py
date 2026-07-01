#!/usr/bin/env python3
"""Generate complete branded HTMLX artifact for FLINT-META-EXTENSION-PLAN.md"""
import re
import html as html_module

# Read the markdown
with open('/Users/gqadonis/Projects/prometheus/flint-forge/docs/FLINT-META-EXTENSION-PLAN.md', 'r') as f:
    md = f.read()

# === SVG DIAGRAMS ===
arch_svg = """<figure>
<svg viewBox="0 0 900 580" xmlns="http://www.w3.org/2000/svg" style="max-width:900px">
  <defs>
    <filter id="sh" x="-3%" y="-3%" width="106%" height="106%"><feDropShadow dx="0" dy="2" stdDeviation="3" flood-color="#000" flood-opacity="0.35"/></filter>
    <marker id="a1" markerWidth="8" markerHeight="8" refX="7" refY="4" orient="auto"><polygon points="0,0 8,4 0,8" fill="#FF6A3D"/></marker>
    <marker id="a2" markerWidth="8" markerHeight="8" refX="7" refY="4" orient="auto"><polygon points="0,0 8,4 0,8" fill="#34CFE6"/></marker>
  </defs>
  <rect x="10" y="10" width="880" height="280" rx="12" fill="#131A22" stroke="#28333F" stroke-width="1.5"/>
  <text x="450" y="32" text-anchor="middle" font-family="Space Grotesk, sans-serif" font-size="14" font-weight="700" fill="#E8EDF3" letter-spacing="0.08em">LAYER 1: INSIDE POSTGRESQL</text>
  <rect x="30" y="45" width="840" height="180" rx="10" fill="#0E141B" stroke="#FF6A3D" stroke-width="2" filter="url(#sh)"/>
  <text x="450" y="65" text-anchor="middle" font-family="Space Grotesk, sans-serif" font-size="12" font-weight="700" fill="#E8EDF3">flint_meta (pgrx extension)</text>
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
  <text x="450" y="140" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="9" fill="#B4BECB">SQL Functions: meta.tables() . meta.columns() . meta.check_permission() . meta.decrypt_column() . meta.agui()</text>
  <text x="450" y="158" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="9" fill="#B4BECB">Event Triggers: ddl_command_end refresh_cache() . sql_drop invalidate_cache()</text>
  <text x="450" y="176" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="9" fill="#B4BECB">NOTIFY: meta_runtime . keto_changes . vault_rotation . agui_update</text>
  <rect x="30" y="195" width="840" height="45" rx="6" fill="#131A22" stroke="#28333F" stroke-width="1"/>
  <text x="450" y="215" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="10" fill="#97A1AE">PostgreSQL Core (pg_catalog) + flint_auth . flint_hooks . flint_llm . flint_vault . pg_graphql . pgvector</text>
  <text x="450" y="230" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="8" fill="#97A1AE">+ other extensions</text>
  <line x1="450" y1="245" x2="450" y2="270" stroke="#FF6A3D" stroke-width="2" stroke-dasharray="4,3" marker-end="url(#a1)"/>
  <text x="465" y="265" font-family="JetBrains Mono, monospace" font-size="9" fill="#FF6A3D">LISTEN / NOTIFY</text>
  <rect x="10" y="280" width="880" height="290" rx="12" fill="#131A22" stroke="#28333F" stroke-width="1.5"/>
  <text x="450" y="302" text-anchor="middle" font-family="Space Grotesk, sans-serif" font-size="14" font-weight="700" fill="#E8EDF3" letter-spacing="0.08em">LAYER 2: RUST REFLECTION ENGINE</text>
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
  <rect x="120" y="385" width="660" height="70" rx="8" fill="#0E141B" stroke="#FF6A3D" stroke-width="2" filter="url(#sh)"/>
  <text x="450" y="408" text-anchor="middle" font-family="Space Grotesk, sans-serif" font-size="12" font-weight="700" fill="#E8EDF3">Immutable IR (ArcSwap)</text>
  <text x="450" y="428" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="10" fill="#FF6A3D">DatabaseModel . Router . GraphQL . OpenAPI . MCP . AG-UI</text>
  <text x="450" y="444" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="9" fill="#97A1AE">Atomic hot-swap: zero downtime, zero locking</text>
  <rect x="120" y="470" width="300" height="55" rx="6" fill="#0E141B" stroke="#34CFE6" stroke-width="1.5" filter="url(#sh)"/>
  <text x="270" y="493" text-anchor="middle" font-family="Space Grotesk, sans-serif" font-size="11" font-weight="600" fill="#E8EDF3">SQL Compiler</text>
  <text x="270" y="510" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="9" fill="#34CFE6">HTTP . AST . Query AST . SQL . Prepared . JSON</text>
  <rect x="450" y="470" width="330" height="55" rx="6" fill="#0E141B" stroke="#4FD18B" stroke-width="1.5" filter="url(#sh)"/>
  <text x="615" y="493" text-anchor="middle" font-family="Space Grotesk, sans-serif" font-size="11" font-weight="600" fill="#E8EDF3">Token Propagation</text>
  <text x="615" y="510" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="9" fill="#4FD18B">JWT Claims . Keto Subject . Vault Key Ref . SET LOCAL</text>
  <rect x="120" y="540" width="280" height="35" rx="6" fill="#131A22" stroke="#FF6A3D" stroke-width="1.5" filter="url(#sh)"/>
  <text x="260" y="562" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="10" fill="#FF6A3D">Flint Gate (Axum) . JWT mint . Kratos + Keto + Cedar</text>
  <rect x="420" y="540" width="280" height="35" rx="6" fill="#131A22" stroke="#4FD18B" stroke-width="1.5" filter="url(#sh)"/>
  <text x="560" y="562" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="10" fill="#4FD18B">Realtime Fabric (Iggy) . WebSocket mux . CRDT . SSE</text>
</svg>
<figcaption><b>Figure 3.1</b> Two-layer architecture: PostgreSQL owns metadata via pgrx extension; Rust reflects it into compiled executable artifacts.</figcaption>
</figure>"""

compiler_svg = """<figure>
<svg viewBox="0 0 800 520" xmlns="http://www.w3.org/2000/svg" style="max-width:800px">
  <defs><filter id="shc" x="-3%" y="-3%" width="106%" height="106%"><feDropShadow dx="0" dy="2" stdDeviation="2" flood-color="#000" flood-opacity="0.3"/></filter>
    <marker id="ac" markerWidth="8" markerHeight="8" refX="7" refY="4" orient="auto"><polygon points="0,0 8,4 0,8" fill="#FF6A3D"/></marker>
  </defs>
  <rect x="10" y="10" width="780" height="500" rx="12" fill="#131A22" stroke="#28333F" stroke-width="1.5"/>
  <rect x="280" y="25" width="240" height="40" rx="6" fill="#0E141B" stroke="#4FD18B" stroke-width="2" filter="url(#shc)"/>
  <text x="400" y="45" text-anchor="middle" font-family="Space Grotesk, sans-serif" font-size="12" font-weight="700" fill="#E8EDF3">PostgreSQL (flint_meta schema)</text>
  <text x="400" y="58" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="9" fill="#4FD18B">SELECT * FROM meta.tables(), meta.columns(), etc.</text>
  <line x1="400" y1="65" x2="400" y2="85" stroke="#FF6A3D" stroke-width="2" marker-end="url(#ac)"/>
  <rect x="280" y="90" width="240" height="35" rx="6" fill="#0E141B" stroke="#FF6A3D" stroke-width="1.5" filter="url(#shc)"/>
  <text x="400" y="112" text-anchor="middle" font-family="Space Grotesk, sans-serif" font-size="11" font-weight="600" fill="#E8EDF3">ReflectionEngine::reflect()</text>
  <line x1="400" y1="125" x2="400" y2="145" stroke="#FF6A3D" stroke-width="2" marker-end="url(#ac)"/>
  <rect x="280" y="150" width="240" height="35" rx="6" fill="#0E141B" stroke="#FF6A3D" stroke-width="2" filter="url(#shc)"/>
  <text x="400" y="172" text-anchor="middle" font-family="Space Grotesk, sans-serif" font-size="11" font-weight="700" fill="#E8EDF3">DatabaseModel (Immutable IR)</text>
  <line x1="400" y1="185" x2="400" y2="205" stroke="#FF6A3D" stroke-width="1.5"/>
  <line x1="100" y1="205" x2="700" y2="205" stroke="#FF6A3D" stroke-width="1.5"/>
  <line x1="100" y1="205" x2="100" y2="225" stroke="#FF6A3D" stroke-width="1.5" marker-end="url(#ac)"/>
  <rect x="40" y="230" width="120" height="40" rx="6" fill="#0E141B" stroke="#34CFE6" stroke-width="1.2" filter="url(#shc)"/>
  <text x="100" y="248" text-anchor="middle" font-family="Space Grotesk, sans-serif" font-size="10" font-weight="600" fill="#E8EDF3">Normalization</text>
  <text x="100" y="262" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="8" fill="#34CFE6">Resolve domains</text>
  <line x1="220" y1="205" x2="220" y2="225" stroke="#FF6A3D" stroke-width="1.5" marker-end="url(#ac)"/>
  <rect x="160" y="230" width="120" height="40" rx="6" fill="#0E141B" stroke="#34CFE6" stroke-width="1.2" filter="url(#shc)"/>
  <text x="220" y="248" text-anchor="middle" font-family="Space Grotesk, sans-serif" font-size="10" font-weight="600" fill="#E8EDF3">Validation</text>
  <text x="220" y="262" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="8" fill="#34CFE6">Detect cycles</text>
  <line x1="340" y1="205" x2="340" y2="225" stroke="#FF6A3D" stroke-width="1.5" marker-end="url(#ac)"/>
  <rect x="280" y="230" width="120" height="40" rx="6" fill="#0E141B" stroke="#34CFE6" stroke-width="1.2" filter="url(#shc)"/>
  <text x="340" y="248" text-anchor="middle" font-family="Space Grotesk, sans-serif" font-size="10" font-weight="600" fill="#E8EDF3">Permission Analysis</text>
  <text x="340" y="262" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="8" fill="#34CFE6">Keto/RLS/Cedar</text>
  <line x1="460" y1="205" x2="460" y2="225" stroke="#FF6A3D" stroke-width="1.5" marker-end="url(#ac)"/>
  <rect x="400" y="230" width="120" height="40" rx="6" fill="#0E141B" stroke="#34CFE6" stroke-width="1.2" filter="url(#shc)"/>
  <text x="460" y="248" text-anchor="middle" font-family="Space Grotesk, sans-serif" font-size="10" font-weight="600" fill="#E8EDF3">Endpoint Generation</text>
  <text x="460" y="262" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="8" fill="#34CFE6">REST routes</text>
  <line x1="580" y1="205" x2="580" y2="225" stroke="#FF6A3D" stroke-width="1.5" marker-end="url(#ac)"/>
  <rect x="520" y="230" width="120" height="40" rx="6" fill="#0E141B" stroke="#34CFE6" stroke-width="1.2" filter="url(#shc)"/>
  <text x="580" y="248" text-anchor="middle" font-family="Space Grotesk, sans-serif" font-size="10" font-weight="600" fill="#E8EDF3">OpenAPI Compiler</text>
  <text x="580" y="262" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="8" fill="#34CFE6">utoipa</text>
  <line x1="700" y1="205" x2="700" y2="225" stroke="#FF6A3D" stroke-width="1.5" marker-end="url(#ac)"/>
  <rect x="640" y="230" width="120" height="40" rx="6" fill="#0E141B" stroke="#34CFE6" stroke-width="1.2" filter="url(#shc)"/>
  <text x="700" y="248" text-anchor="middle" font-family="Space Grotesk, sans-serif" font-size="10" font-weight="600" fill="#E8EDF3">GraphQL SDL</text>
  <text x="700" y="262" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="8" fill="#34CFE6">schema.graphql</text>
  <line x1="100" y1="270" x2="700" y2="270" stroke="#FF6A3D" stroke-width="1.5"/>
  <line x1="400" y1="270" x2="400" y2="290" stroke="#FF6A3D" stroke-width="2" marker-end="url(#ac)"/>
  <rect x="200" y="295" width="200" height="35" rx="6" fill="#0E141B" stroke="#F4B942" stroke-width="1.5" filter="url(#shc)"/>
  <text x="300" y="315" text-anchor="middle" font-family="Space Grotesk, sans-serif" font-size="10" font-weight="600" fill="#E8EDF3">MCP Compiler</text>
  <text x="300" y="328" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="8" fill="#F4B942">Tool manifest</text>
  <rect x="420" y="295" width="200" height="35" rx="6" fill="#0E141B" stroke="#F4B942" stroke-width="1.5" filter="url(#shc)"/>
  <text x="520" y="315" text-anchor="middle" font-family="Space Grotesk, sans-serif" font-size="10" font-weight="600" fill="#E8EDF3">AG-UI Compiler</text>
  <text x="520" y="328" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="8" fill="#F4B942">A2UI descriptors</text>
  <line x1="300" y1="330" x2="400" y2="360" stroke="#F4B942" stroke-width="1.5"/>
  <line x1="520" y1="330" x2="400" y2="360" stroke="#F4B942" stroke-width="1.5"/>
  <line x1="400" y1="360" x2="400" y2="380" stroke="#FF6A3D" stroke-width="2" marker-end="url(#ac)"/>
  <rect x="280" y="385" width="240" height="45" rx="8" fill="#0E141B" stroke="#FF6A3D" stroke-width="2" filter="url(#shc)"/>
  <text x="400" y="408" text-anchor="middle" font-family="Space Grotesk, sans-serif" font-size="12" font-weight="700" fill="#E8EDF3">ArcSwap Hot-Swap</text>
  <text x="400" y="425" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="9" fill="#FF6A3D">Atomic replacement: zero downtime, zero locking</text>
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
<figcaption><b>Figure 5.1</b> Compiler pipeline: DatabaseModel . Normalization . Validation . Permission Analysis . Endpoint Generation . OpenAPI . GraphQL SDL . MCP . AG-UI . ArcSwap hot-swap.</figcaption>
</figure>"""

jwt_svg = """<figure>
<svg viewBox="0 0 800 340" xmlns="http://www.w3.org/2000/svg" style="max-width:800px">
  <defs><filter id="shj" x="-3%" y="-3%" width="106%" height="106%"><feDropShadow dx="0" dy="2" stdDeviation="2" flood-color="#000" flood-opacity="0.3"/></filter>
    <marker id="aj" markerWidth="8" markerHeight="8" refX="7" refY="4" orient="auto"><polygon points="0,0 8,4 0,8" fill="#FF6A3D"/></marker>
  </defs>
  <rect x="10" y="10" width="780" height="320" rx="12" fill="#131A22" stroke="#28333F" stroke-width="1.5"/>
  <rect x="30" y="25" width="120" height="35" rx="6" fill="#0E141B" stroke="#E8EDF3" stroke-width="1.5" filter="url(#shj)"/>
  <text x="90" y="45" text-anchor="middle" font-family="Space Grotesk, sans-serif" font-size="11" font-weight="600" fill="#E8EDF3">User Request</text>
  <text x="90" y="58" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="8" fill="#97A1AE">with JWT</text>
  <line x1="155" y1="42" x2="175" y2="42" stroke="#E8EDF3" stroke-width="1.5" marker-end="url(#aj)"/>
  <rect x="180" y="25" width="140" height="35" rx="6" fill="#0E141B" stroke="#FF6A3D" stroke-width="2" filter="url(#shj)"/>
  <text x="250" y="45" text-anchor="middle" font-family="Space Grotesk, sans-serif" font-size="11" font-weight="600" fill="#E8EDF3">Flint Gate</text>
  <text x="250" y="58" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="8" fill="#FF6A3D">Axum / Rust</text>
  <line x1="325" y1="42" x2="345" y2="42" stroke="#FF6A3D" stroke-width="1.5" marker-end="url(#aj)"/>
  <rect x="350" y="25" width="120" height="35" rx="6" fill="#0E141B" stroke="#34CFE6" stroke-width="1.2" filter="url(#shj)"/>
  <text x="410" y="45" text-anchor="middle" font-family="Space Grotesk, sans-serif" font-size="10" font-weight="600" fill="#E8EDF3">Kratos</text>
  <text x="410" y="58" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="8" fill="#34CFE6">Validate session</text>
  <line x1="475" y1="42" x2="495" y2="42" stroke="#34CFE6" stroke-width="1.5" marker-end="url(#aj)"/>
  <rect x="500" y="25" width="120" height="35" rx="6" fill="#0E141B" stroke="#4FD18B" stroke-width="1.2" filter="url(#shj)"/>
  <text x="560" y="45" text-anchor="middle" font-family="Space Grotesk, sans-serif" font-size="10" font-weight="600" fill="#E8EDF3">Keto</text>
  <text x="560" y="58" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="8" fill="#4FD18B">Coarse check</text>
  <line x1="625" y1="42" x2="645" y2="42" stroke="#4FD18B" stroke-width="1.5" marker-end="url(#aj)"/>
  <rect x="650" y="25" width="120" height="35" rx="6" fill="#0E141B" stroke="#F4B942" stroke-width="1.2" filter="url(#shj)"/>
  <text x="710" y="45" text-anchor="middle" font-family="Space Grotesk, sans-serif" font-size="10" font-weight="600" fill="#E8EDF3">Cedar</text>
  <text x="710" y="58" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="8" fill="#F4B942">Capabilities</text>
  <line x1="250" y1="65" x2="250" y2="90" stroke="#FF6A3D" stroke-width="1.5" marker-end="url(#aj)"/>
  <rect x="140" y="95" width="220" height="55" rx="6" fill="#0E141B" stroke="#FF6A3D" stroke-width="1.5" filter="url(#shj)"/>
  <text x="250" y="112" text-anchor="middle" font-family="Space Grotesk, sans-serif" font-size="10" font-weight="600" fill="#E8EDF3">Enriched JWT</text>
  <text x="250" y="128" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="8" fill="#97A1AE">sub, roles, keto_subject, vault_key_id</text>
  <text x="250" y="142" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="7" fill="#97A1AE">+ Cedar capabilities</text>
  <line x1="365" y1="122" x2="385" y2="122" stroke="#FF6A3D" stroke-width="1.5" marker-end="url(#aj)"/>
  <rect x="390" y="100" width="140" height="45" rx="6" fill="#0E141B" stroke="#FF6A3D" stroke-width="2" filter="url(#shj)"/>
  <text x="460" y="120" text-anchor="middle" font-family="Space Grotesk, sans-serif" font-size="11" font-weight="600" fill="#E8EDF3">Flint Forge</text>
  <text x="460" y="138" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="8" fill="#FF6A3D">SET LOCAL GUC</text>
  <line x1="535" y1="122" x2="555" y2="122" stroke="#FF6A3D" stroke-width="1.5" marker-end="url(#aj)"/>
  <rect x="560" y="100" width="200" height="45" rx="6" fill="#0E141B" stroke="#4FD18B" stroke-width="2" filter="url(#shj)"/>
  <text x="660" y="120" text-anchor="middle" font-family="Space Grotesk, sans-serif" font-size="11" font-weight="600" fill="#E8EDF3">PostgreSQL</text>
  <text x="660" y="138" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="8" fill="#4FD18B">flint_meta extension</text>
  <line x1="660" y1="150" x2="660" y2="170" stroke="#4FD18B" stroke-width="1.5" marker-end="url(#aj)"/>
  <rect x="520" y="175" width="280" height="55" rx="6" fill="#0E141B" stroke="#34CFE6" stroke-width="1.5" filter="url(#shj)"/>
  <text x="660" y="195" text-anchor="middle" font-family="Space Grotesk, sans-serif" font-size="10" font-weight="600" fill="#E8EDF3">Permission Checks</text>
  <text x="660" y="212" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="8" fill="#34CFE6">keto_check() . vault.decrypt_column() . RLS</text>
  <text x="660" y="226" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="7" fill="#97A1AE">All inline, zero round-trips</text>
  <line x1="660" y1="235" x2="660" y2="255" stroke="#34CFE6" stroke-width="1.5" marker-end="url(#aj)"/>
  <rect x="540" y="260" width="240" height="35" rx="6" fill="#0E141B" stroke="#4FD18B" stroke-width="1.5" filter="url(#shj)"/>
  <text x="660" y="280" text-anchor="middle" font-family="Space Grotesk, sans-serif" font-size="10" font-weight="600" fill="#E8EDF3">SQL Execution</text>
  <text x="660" y="292" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="8" fill="#4FD18B">Prepared statement . JSON result</text>
  <line x1="660" y1="300" x2="660" y2="315" stroke="#4FD18B" stroke-width="1.5"/>
  <line x1="460" y1="315" x2="660" y2="315" stroke="#4FD18B" stroke-width="1.5" marker-end="url(#aj)"/>
  <rect x="260" y="300" width="200" height="25" rx="6" fill="#131A22" stroke="#4FD18B" stroke-width="1.2" filter="url(#shj)"/>
  <text x="360" y="315" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="9" fill="#4FD18B">Iggy . WebSocket / SSE / CRDT</text>
</svg>
<figcaption><b>Figure 6.1</b> JWT identity flow: Kratos validates, Keto checks, Cedar evaluates, then enriched JWT propagates via SET LOCAL into PostgreSQL for inline permission, encryption, and RLS.</figcaption>
</figure>"""

realtime_svg = """<figure>
<svg viewBox="0 0 800 280" xmlns="http://www.w3.org/2000/svg" style="max-width:800px">
  <defs><filter id="shr" x="-3%" y="-3%" width="106%" height="106%"><feDropShadow dx="0" dy="2" stdDeviation="2" flood-color="#000" flood-opacity="0.3"/></filter>
    <marker id="ar" markerWidth="8" markerHeight="8" refX="7" refY="4" orient="auto"><polygon points="0,0 8,4 0,8" fill="#FF6A3D"/></marker>
  </defs>
  <rect x="10" y="10" width="780" height="260" rx="12" fill="#131A22" stroke="#28333F" stroke-width="1.5"/>
  <text x="400" y="32" text-anchor="middle" font-family="Space Grotesk, sans-serif" font-size="14" font-weight="700" fill="#E8EDF3" letter-spacing="0.08em">REALTIME NOTIFICATION ARCHITECTURE</text>
  <rect x="30" y="50" width="140" height="45" rx="6" fill="#0E141B" stroke="#4FD18B" stroke-width="2" filter="url(#shr)"/>
  <text x="100" y="70" text-anchor="middle" font-family="Space Grotesk, sans-serif" font-size="11" font-weight="600" fill="#E8EDF3">PostgreSQL</text>
  <text x="100" y="86" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="9" fill="#4FD18B">NOTIFY channels</text>
  <line x1="175" y1="72" x2="195" y2="72" stroke="#4FD18B" stroke-width="2" marker-end="url(#ar)"/>
  <rect x="200" y="50" width="160" height="45" rx="6" fill="#0E141B" stroke="#FF6A3D" stroke-width="2" filter="url(#shr)"/>
  <text x="280" y="70" text-anchor="middle" font-family="Space Grotesk, sans-serif" font-size="11" font-weight="600" fill="#E8EDF3">Listener</text>
  <text x="280" y="86" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="9" fill="#FF6A3D">flint-reflection</text>
  <line x1="365" y1="72" x2="385" y2="72" stroke="#FF6A3D" stroke-width="2" marker-end="url(#ar)"/>
  <rect x="390" y="50" width="140" height="45" rx="6" fill="#0E141B" stroke="#FF6A3D" stroke-width="1.5" filter="url(#shr)"/>
  <text x="460" y="70" text-anchor="middle" font-family="Space Grotesk, sans-serif" font-size="11" font-weight="600" fill="#E8EDF3">Iggy Producer</text>
  <text x="460" y="86" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="9" fill="#FF6A3D">topic: meta.changes</text>
  <line x1="535" y1="72" x2="555" y2="72" stroke="#FF6A3D" stroke-width="2" marker-end="url(#ar)"/>
  <rect x="560" y="50" width="140" height="45" rx="6" fill="#0E141B" stroke="#FF6A3D" stroke-width="2" filter="url(#shr)"/>
  <text x="630" y="70" text-anchor="middle" font-family="Space Grotesk, sans-serif" font-size="11" font-weight="600" fill="#E8EDF3">Iggy Spine</text>
  <text x="630" y="86" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="9" fill="#FF6A3D">Event bus</text>
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
</figure>"""

ecosystem_svg = """<figure>
<svg viewBox="0 0 800 340" xmlns="http://www.w3.org/2000/svg" style="max-width:800px">
  <defs><filter id="she" x="-3%" y="-3%" width="106%" height="106%"><feDropShadow dx="0" dy="2" stdDeviation="2" flood-color="#000" flood-opacity="0.3"/></filter>
    <marker id="ae" markerWidth="8" markerHeight="8" refX="7" refY="4" orient="auto"><polygon points="0,0 8,4 0,8" fill="#FF6A3D"/></marker>
  </defs>
  <rect x="10" y="10" width="780" height="320" rx="12" fill="#131A22" stroke="#28333F" stroke-width="1.5"/>
  <text x="400" y="32" text-anchor="middle" font-family="Space Grotesk, sans-serif" font-size="14" font-weight="700" fill="#E8EDF3" letter-spacing="0.08em">FLINT ECOSYSTEM INTEGRATION</text>
  <rect x="30" y="50" width="160" height="55" rx="8" fill="#0E141B" stroke="#FF6A3D" stroke-width="2" filter="url(#she)"/>
  <text x="110" y="72" text-anchor="middle" font-family="Space Grotesk, sans-serif" font-size="12" font-weight="700" fill="#E8EDF3">Flint Gate</text>
  <text x="110" y="90" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="9" fill="#FF6A3D">Axum / Rust</text>
  <text x="110" y="100" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="8" fill="#97A1AE">Kratos + Keto + Cedar + Vault</text>
  <line x1="195" y1="77" x2="215" y2="77" stroke="#FF6A3D" stroke-width="2" marker-end="url(#ae)"/>
  <text x="205" y="68" font-family="JetBrains Mono, monospace" font-size="8" fill="#FF6A3D">JWT</text>
  <rect x="220" y="50" width="160" height="55" rx="8" fill="#0E141B" stroke="#FF6A3D" stroke-width="2" filter="url(#she)"/>
  <text x="300" y="72" text-anchor="middle" font-family="Space Grotesk, sans-serif" font-size="12" font-weight="700" fill="#E8EDF3">Flint Forge</text>
  <text x="300" y="90" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="9" fill="#FF6A3D">Meta + Reflection</text>
  <text x="300" y="100" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="8" fill="#97A1AE">REST + GraphQL + OpenAPI</text>
  <line x1="385" y1="77" x2="405" y2="77" stroke="#FF6A3D" stroke-width="2" marker-end="url(#ae)"/>
  <text x="395" y="68" font-family="JetBrains Mono, monospace" font-size="8" fill="#FF6A3D">SQL</text>
  <rect x="410" y="50" width="180" height="55" rx="8" fill="#0E141B" stroke="#4FD18B" stroke-width="2" filter="url(#she)"/>
  <text x="500" y="72" text-anchor="middle" font-family="Space Grotesk, sans-serif" font-size="12" font-weight="700" fill="#E8EDF3">Realtime Fabric</text>
  <text x="500" y="90" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="9" fill="#4FD18B">Iggy / WebSocket</text>
  <text x="500" y="100" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="8" fill="#97A1AE">CRDT + SSE + gRPC</text>
  <line x1="300" y1="105" x2="300" y2="125" stroke="#FF6A3D" stroke-width="2" marker-end="url(#ae)"/>
  <line x1="500" y1="105" x2="500" y2="125" stroke="#4FD18B" stroke-width="2" marker-end="url(#ae)"/>
  <rect x="100" y="130" width="600" height="110" rx="10" fill="#0E141B" stroke="#4FD18B" stroke-width="2" filter="url(#she)"/>
  <text x="400" y="150" text-anchor="middle" font-family="Space Grotesk, sans-serif" font-size="13" font-weight="700" fill="#E8EDF3">PostgreSQL 18</text>
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
</figure>"""

# Map diagram IDs to SVG content
svg_map = {
    'arch': arch_svg,
    'compiler': compiler_svg,
    'jwt': jwt_svg,
    'realtime': realtime_svg,
    'ecosystem': ecosystem_svg,
}

# Convert markdown to HTML
lines = md.split('\n')
output = []
in_code = False
code_lines = []
code_lang = ''
in_table = False
table_headers = []
table_rows = []
in_list = False
list_type = None
list_items = []
pre_counter = 0
svg_insertion_points = {}  # We'll track which pre blocks to replace

# Scan for diagram insertion points first
for idx, line in enumerate(lines):
    if line.strip().startswith('```') and not in_code:
        # Check if next lines contain diagram markers
        for j in range(idx+1, min(idx+5, len(lines))):
            if 'LAYER 1' in lines[j] or 'LAYER 2' in lines[j]:
                svg_insertion_points[pre_counter] = 'arch'
                break
            elif 'PostgreSQL (flint_meta schema)' in lines[j] and 'ReflectionEngine' in lines[j+1] if j+1 < len(lines) else False:
                svg_insertion_points[pre_counter] = 'compiler'
                break
            elif 'User Request' in lines[j] and 'Flint Gate' in lines[j+1] if j+1 < len(lines) else False:
                svg_insertion_points[pre_counter] = 'jwt'
                break
            elif 'REALTIME NOTIFICATION' in lines[j] or 'PostgreSQL' in lines[j] and 'NOTIFY channels' in lines[j+1] if j+1 < len(lines) else False:
                svg_insertion_points[pre_counter] = 'realtime'
                break
            elif 'FLINT ECOSYSTEM' in lines[j]:
                svg_insertion_points[pre_counter] = 'ecosystem'
                break

i = 0
pre_counter = 0
while i < len(lines):
    line = lines[i]
    stripped = line.strip()
    
    if stripped.startswith('```'):
        if in_code:
            # End of code block
            if pre_counter in svg_insertion_points:
                svg_id = svg_insertion_points[pre_counter]
                output.append(svg_map[svg_id])
                code_lines = []
                code_lang = ''
                in_code = False
                pre_counter += 1
                i += 1
                continue
            else:
                lang = code_lang or 'text'
                code_content = html_module.escape('\n'.join(code_lines))
                output.append(f'<pre><code class="language-{lang}">{code_content}</code></pre>')
                code_lines = []
                code_lang = ''
                in_code = False
                pre_counter += 1
                i += 1
                continue
        else:
            flush_table()
            flush_list()
            in_code = True
            code_lang = stripped[3:].strip()
            i += 1
            continue
    
    if in_code:
        code_lines.append(line)
        i += 1
        continue
    
    # Horizontal rule
    if stripped == '---':
        flush_table()
        flush_list()
        output.append('<div class="divider"></div>')
        i += 1
        continue
    
    # Table
    if '|' in stripped and stripped.count('|') >= 2 and not stripped.startswith('#'):
        flush_list()
        cells = [c.strip() for c in stripped.split('|') if c.strip() or c == '']
        is_sep = all(set(c.strip()) <= set(' -|:') for c in stripped.split('|') if c.strip())
        if is_sep:
            pass
        elif cells and not table_headers:
            table_headers = cells
        elif cells:
            table_rows.append(cells)
        in_table = True
        i += 1
        continue
    elif in_table and stripped:
        flush_table()
    
    # Headers
    if stripped.startswith('# ') and not stripped.startswith('## '):
        flush_table()
        flush_list()
        title = stripped[2:].strip()
        output.append(f'<section id="summary"><div class="sec-num">Summary</div><h2>{title}</h2>')
        i += 1
        continue
    elif stripped.startswith('## '):
        flush_table()
        flush_list()
        title = stripped[3:].strip()
        id_text = re.sub(r'[^a-z0-9-]', '', re.sub(r'[\s/]+', '-', title.lower()))[:60]
        output.append(f'</section>\n<section id="{id_text}"><div class="sec-num">{title[:2] if title[0].isdigit() else "##"}</div><h2>{title}</h2>')
        i += 1
        continue
    elif stripped.startswith('### '):
        flush_table()
        flush_list()
        title = stripped[4:].strip()
        output.append(f'<h3>{title}</h3>')
        i += 1
        continue
    elif stripped.startswith('#### '):
        flush_table()
        flush_list()
        title = stripped[5:].strip()
        output.append(f'<h4>{title}</h4>')
        i += 1
        continue
    
    # Lists
    if stripped.startswith('- ') or stripped.startswith('* '):
        flush_table()
        item_text = stripped[2:].strip()
        item_text = re.sub(r'\*\*(.+?)\*\*', r'<strong>\1</strong>', item_text)
        item_text = re.sub(r'\*(.+?)\*', r'<em>\1</em>', item_text)
        item_text = re.sub(r'`([^`]+?)`', r'<code>\1</code>', item_text)
        if not in_list or list_type != 'ul':
            flush_list()
            in_list = True
            list_type = 'ul'
        list_items.append(item_text)
        i += 1
        continue
    elif re.match(r'^\d+\.\s', stripped):
        flush_table()
        item_text = re.sub(r'^\d+\.\s', '', stripped)
        item_text = re.sub(r'\*\*(.+?)\*\*', r'<strong>\1</strong>', item_text)
        item_text = re.sub(r'\*(.+?)\*', r'<em>\1</em>', item_text)
        item_text = re.sub(r'`([^`]+?)`', r'<code>\1</code>', item_text)
        if not in_list or list_type != 'ol':
            flush_list()
            in_list = True
            list_type = 'ol'
        list_items.append(item_text)
        i += 1
        continue
    elif in_list and stripped == '':
        flush_list()
    elif in_list and not stripped.startswith('- ') and not stripped.startswith('* ') and not re.match(r'^\d+\.\s', stripped):
        flush_list()
    
    # Empty lines
    if stripped == '':
        flush_table()
        flush_list()
        i += 1
        continue
    
    # Paragraphs
    flush_table()
    flush_list()
    text = stripped
    text = re.sub(r'`([^`]+?)`', r'<code>\1</code>', text)
    text = re.sub(r'\*\*(.+?)\*\*', r'<strong>\1</strong>', text)
    text = re.sub(r'\*(.+?)\*', r'<em>\1</em>', text)
    text = re.sub(r'\[([^\]]+)\]\(([^)]+)\)', r'<a href="\2">\1</a>', text)
    output.append(f'<p>{text}</p>')
    i += 1

flush_table()
flush_list()

def flush_table():
    global table_headers, table_rows, in_table
    if table_headers:
        output.append('<div class="tw"><table>')
        output.append('<thead><tr>')
        for h in table_headers:
            output.append(f'<th>{h}</th>')
        output.append('</tr></thead>')
        if table_rows:
            output.append('<tbody>')
            for row in table_rows:
                first_cell = row[0] if row else ''
                cls = ''
                if 'Flint' in first_cell or '**Flint' in first_cell:
                    cls = ' class="flint-win"'
                elif 'PostgREST' in first_cell or '**PostgREST' in first_cell:
                    cls = ' class="sb-win"'
                output.append(f'<tr{cls}>')
                for cell in row:
                    output.append(f'<td>{cell}</td>')
                output.append('</tr>')
            output.append('</tbody>')
        output.append('</table></div>')
        table_headers = []
        table_rows = []
        in_table = False

def flush_list():
    global list_items, in_list, list_type
    if list_items:
        tag = 'ol' if list_type == 'ol' else 'ul'
        output.append(f'<{tag}>')
        for item in list_items:
            output.append(f'<li>{item}</li>')
        output.append(f'</{tag}>')
        list_items = []
        in_list = False
        list_type = None

body = '\n'.join(output)

# Now build the full HTML document
head_css = '''<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>Flint Meta Extension — Architecture and Implementation Plan</title>
<style>
@import url('https://fonts.googleapis.com/css2?family=Space+Grotesk:wght@400;500;600;700&family=Inter:wght@300;400;500;600;700&family=JetBrains+Mono:wght@400;500;600&display=swap');

:root{
  --bg:#0B0F14; --surface-1:#131A22; --surface-2:#1A232E; --surface-3:#0E141B;
  --surface-4:#202B38;
  --text:#E8EDF3; --text-muted:#B4BECB; --text-dim:#97A1AE;
  --ember:#FF6A3D; --ember-deep:#E04E28; --ember-tint:rgba(255,106,61,0.10);
  --ember-tint-2:rgba(255,106,61,0.16);
  --cyan:#34CFE6; --cyan-tint:rgba(52,207,230,0.10);
  --green:#4FD18B; --green-tint:rgba(79,209,139,0.10);
  --yellow:#F4B942; --yellow-tint:rgba(244,185,66,0.10);
  --red:#F05D5D; --red-tint:rgba(240,93,93,0.10);
  --code-bg:#0E141B; --code-text:#D8E4F0;
  --line:#28333F; --good:#4FD18B; --warn:#F4B942;
}
*{margin:0;padding:0;box-sizing:border-box;}
html{scroll-behavior:smooth;}
body{
  font-family:'Inter',sans-serif; background:var(--bg); color:var(--text-muted);
  font-size:15.5px; line-height:1.72; -webkit-font-smoothing:antialiased;
}
.layout{display:grid; grid-template-columns:300px 1fr; max-width:1500px; margin:0 auto;}

.sidebar{
  position:sticky; top:0; align-self:start; height:100vh; overflow-y:auto;
  background:var(--surface-3); padding:30px 22px 60px; border-right:1px solid var(--line);
}
.sidebar .brand{display:flex; align-items:center; gap:12px; margin-bottom:26px;}
.sidebar .brand .wm{font-family:'Space Grotesk',sans-serif; font-weight:700; font-size:21px; color:var(--text); letter-spacing:-0.03em; line-height:1;}
.sidebar .brand .sub{font-family:'JetBrains Mono',monospace; font-size:9px; color:var(--text-dim); letter-spacing:0.1em; margin-top:3px; text-transform:uppercase;}
.toc-label{font-family:'JetBrains Mono',monospace; font-size:10px; letter-spacing:0.14em; text-transform:uppercase; color:var(--text-dim); margin:22px 0 10px;}
.sidebar nav a{
  display:block; font-size:13px; color:var(--text-dim); text-decoration:none;
  padding:5px 10px; border-radius:6px; line-height:1.4; transition:all .15s;
}
.sidebar nav a:hover{color:var(--text); background:var(--surface-1);}
.sidebar nav a.sub{padding-left:22px; font-size:12px;}
.sidebar nav a.active{color:var(--ember); background:var(--ember-tint);}

main{padding:0; min-width:0;}
.cover{padding:64px 60px 48px; background:linear-gradient(160deg,#11181F 0%,#0B0F14 70%);}
.cover .eyebrow{font-family:'JetBrains Mono',monospace; font-size:11px; font-weight:600; letter-spacing:0.16em; text-transform:uppercase; color:var(--ember); margin-bottom:18px;}
.cover .lockup{display:flex; align-items:center; gap:18px; margin-bottom:30px;}
.cover h1{font-family:'Space Grotesk',sans-serif; font-size:54px; font-weight:700; letter-spacing:-0.035em; line-height:1.02; color:var(--text); margin-bottom:18px;}
.cover .lede{font-size:18px; font-weight:300; line-height:1.6; max-width:680px; color:var(--text-muted);}
.cover .metarow{display:flex; flex-wrap:wrap; gap:10px; margin-top:30px;}
.cover .chip{font-family:'JetBrains Mono',monospace; font-size:11px; color:var(--text-muted); background:var(--surface-2); padding:7px 13px; border-radius:6px;}
.cover .chip b{color:var(--ember);}

.content{padding:8px 60px 80px; max-width:980px;}
section{padding:40px 0 8px; scroll-margin-top:20px;}
.sec-num{font-family:'JetBrains Mono',monospace; font-size:12px; color:var(--ember); letter-spacing:0.1em; font-weight:600;}
h2{font-family:'Space Grotesk',sans-serif; font-size:32px; font-weight:700; letter-spacing:-0.03em; color:var(--text); margin:6px 0 18px; line-height:1.1;}
h3{font-family:'Space Grotesk',sans-serif; font-size:21px; font-weight:600; letter-spacing:-0.02em; color:var(--text); margin:34px 0 12px;}
h4{font-family:'Space Grotesk',sans-serif; font-size:16px; font-weight:600; color:var(--text); margin:24px 0 8px;}
p{margin:0 0 14px; max-width:74ch;}
strong{color:var(--text); font-weight:600;}
a{color:var(--ember);}
ul,ol{margin:0 0 16px; padding-left:22px; max-width:74ch;}
li{margin-bottom:7px;}
code{font-family:'JetBrains Mono',monospace; font-size:0.86em; background:var(--surface-2); color:var(--code-text); padding:2px 6px; border-radius:4px;}
pre{background:var(--code-bg); border-radius:10px; padding:18px 20px; overflow-x:auto; margin:0 0 18px;}
pre code{background:none; padding:0; font-size:13px; line-height:1.6; color:var(--code-text); white-space:pre;}

.callout{background:var(--surface-1); border-radius:10px; padding:18px 22px; margin:0 0 18px; position:relative; overflow:hidden;}
.callout::before{content:""; position:absolute; left:0; top:0; bottom:0; width:4px; background:var(--ember);}
.callout .k{font-family:'JetBrains Mono',monospace; font-size:10px; letter-spacing:0.12em; text-transform:uppercase; color:var(--ember); font-weight:600; display:block; margin-bottom:7px;}
.callout.warn::before{background:var(--warn);} .callout.warn .k{color:var(--warn);}
.callout.good::before{background:var(--good);} .callout.good .k{color:var(--good);}
.callout.cyan::before{background:var(--cyan);} .callout.cyan .k{color:var(--cyan);}
.callout p:last-child{margin-bottom:0;}

.tw{overflow-x:auto; margin:0 0 20px; border-radius:10px;}
table{border-collapse:collapse; width:100%; font-size:13.5px; background:var(--surface-1);}
th{background:var(--surface-4); color:var(--text); text-align:left; font-family:'Space Grotesk',sans-serif; font-weight:600; font-size:12.5px; padding:11px 14px; white-space:nowrap;}
td{padding:10px 14px; color:var(--text-muted); vertical-align:top;}
tbody tr:nth-child(even){background:var(--surface-2);}
td code{font-size:12px; background:var(--code-bg);}

figure{margin:24px 0 26px; background:var(--surface-1); border-radius:12px; padding:24px 22px 14px;}
figure svg{display:block; width:100%; height:auto;}
figcaption{font-family:'JetBrains Mono',monospace; font-size:11px; color:var(--text-dim); margin-top:12px; letter-spacing:0.04em;}
figcaption b{color:var(--ember); font-weight:600;}

.divider{height:3px; width:54px; background:var(--ember); border-radius:2px; margin:46px 0 0;}

.flint-win td:first-child{color:var(--green); font-weight:600;}
.sb-win td:first-child{color:var(--cyan); font-weight:600;}

footer{background:var(--surface-3); padding:40px 60px; color:var(--text-dim); font-size:13px; border-top:1px solid var(--line);}
footer .fl{display:flex; align-items:center; gap:14px; margin-bottom:14px;}

.toc-toggle{display:none;}
@media(max-width:860px){
  .layout{grid-template-columns:1fr;}
  .sidebar{position:fixed; top:0; left:0; right:0; height:auto; max-height:80vh; z-index:50; padding:16px 20px; display:none; border-right:none; border-bottom:1px solid var(--line);}
  .sidebar.open{display:block;}
  .toc-toggle{display:flex; position:sticky; top:0; z-index:40; align-items:center; gap:10px; background:var(--surface-3); color:var(--text); padding:12px 20px; font-family:'JetBrains Mono',monospace; font-size:12px; cursor:pointer; letter-spacing:0.08em; border-bottom:1px solid var(--line);}
  .cover{padding:40px 24px 32px;} .cover h1{font-size:38px;}
  .content{padding:8px 24px 60px;} footer{padding:30px 24px;}
}
@media(max-width:540px){
  body{font-size:14.5px;} .cover h1{font-size:30px;} h2{font-size:25px;} h3{font-size:18px;}
  .content{padding:8px 16px 50px;} pre code{font-size:12px;} table{font-size:12.5px;}
  th,td{padding:8px 10px;}
}
</style>
</head>
<body>
<div class="toc-toggle" onclick="document.querySelector('.sidebar').classList.toggle('open')">☰ Contents</div>
<div class="layout">
<aside class="sidebar">
  <div class="brand">
    <svg viewBox="0 0 200 200" width="38" height="38" xmlns="http://www.w3.org/2000/svg">
      <rect x="10" y="10" width="180" height="180" rx="40" ry="40" fill="none" stroke="#E8EDF3" stroke-width="4.5"/>
      <g transform="translate(50,38)">
        <rect x="0" y="0" width="20" height="124" rx="10" fill="#E8EDF3"/>
        <path d="M 16,62 C 26,52 44,32 62,18 C 72,10 80,8 84,14 C 86,18 82,24 76,28 C 60,40 40,56 28,66 C 22,70 18,68 16,64 Z" fill="#E8EDF3"/>
        <path d="M 16,62 C 26,72 44,92 62,106 C 72,114 80,116 84,110 C 86,106 82,100 76,96 C 60,84 40,68 28,58 C 22,54 18,56 16,60 Z" fill="#E8EDF3"/>
        <circle cx="18" cy="62" r="15" fill="#FF6A3D" opacity="0.12"/>
        <circle cx="18" cy="62" r="10" fill="#FF6A3D"/>
      </g>
    </svg>
    <div><div class="wm">KnowMe</div><div class="sub">Meta Extension Plan</div></div>
  </div>
  <div class="toc-label">Contents</div>
  <nav>
    <a href="#summary">Executive Summary</a>
    <a href="#philosophy">02 · Philosophy</a>
    <a href="#architecture-overview" class="sub">Architecture</a>
    <a href="#the-meta-extension" class="sub">Meta Extension</a>
    <a href="#the-rust-reflection-engine" class="sub">Rust Reflection</a>
    <a href="#jwt-integration-with-keto">04 · JWT + Keto</a>
    <a href="#key-vault-and-column-level-encryption">05 · Vault + Encryption</a>
    <a href="#realtime-metadata-propagation">06 · Realtime</a>
    <a href="#agui-a2ui-metadata-generation">07 · AG-UI / A2UI</a>
    <a href="#crate-structure">08 · Crate Structure</a>
    <a href="#implementation-roadmap">09 · Roadmap</a>
    <a href="#integration-with-the-flint-ecosystem">10 · Ecosystem</a>
    <a href="#security-considerations">11 · Security</a>
    <a href="#performance-targets">12 · Performance</a>
  </nav>
</aside>
<main>
<div class="cover">
  <div class="eyebrow">RFC-FORGE-META-001 · June 2026</div>
  <div class="lockup">
    <svg viewBox="0 0 200 200" width="64" height="64" xmlns="http://www.w3.org/2000/svg">
      <rect x="10" y="10" width="180" height="180" rx="40" ry="40" fill="none" stroke="#E8EDF3" stroke-width="4.5"/>
      <g transform="translate(50,38)">
        <rect x="0" y="0" width="20" height="124" rx="10" fill="#E8EDF3"/>
        <path d="M 16,62 C 26,52 44,32 62,18 C 72,10 80,8 84,14 C 86,18 82,24 76,28 C 60,40 40,56 28,66 C 22,70 18,68 16,64 Z" fill="#E8EDF3"/>
        <path d="M 16,62 C 26,72 44,92 62,106 C 72,114 80,116 84,110 C 86,106 82,100 76,96 C 60,84 40,68 28,58 C 22,54 18,56 16,60 Z" fill="#E8EDF3"/>
        <circle cx="18" cy="62" r="15" fill="#FF6A3D" opacity="0.12"/>
        <circle cx="18" cy="62" r="10" fill="#FF6A3D"/>
      </g>
    </svg>
    <div>
      <div style="font-family:'Space Grotesk',sans-serif; font-weight:700; font-size:24px; color:#E8EDF3; letter-spacing:-0.03em; line-height:1;">KnowMe</div>
      <div style="font-family:'JetBrains Mono',monospace; font-size:11px; color:#97A1AE; margin-top:4px; letter-spacing:0.06em;">a Prometheus AGS platform</div>
    </div>
  </div>
  <h1>Flint Meta Extension</h1>
  <p class="lede">Architecture and implementation plan for a database-driven compiler that replaces PostgREST with a pgrx extension inside PostgreSQL and a Rust reflection engine — with JWT identity propagation, Keto permissions, Vault encryption, and AG-UI/A2UI metadata generation.</p>
  <div class="metarow">
    <span class="chip"><b>pgrx</b> Extension</span>
    <span class="chip"><b>Flint-reflection</b> Rust Compiler</span>
    <span class="chip"><b>Keto</b> Permission Engine</span>
    <span class="chip"><b>Vault</b> Key Management</span>
    <span class="chip"><b>AG-UI/A2UI</b> AI Interface</span>
  </div>
</div>
<div class="content">
'''

footer = '''
</div>
<footer>
  <div class="fl">
    <svg viewBox="0 0 200 200" width="28" height="28" xmlns="http://www.w3.org/2000/svg">
      <rect x="10" y="10" width="180" height="180" rx="40" ry="40" fill="none" stroke="#97A1AE" stroke-width="4.5"/>
      <g transform="translate(50,38)">
        <rect x="0" y="0" width="20" height="124" rx="10" fill="#97A1AE"/>
        <path d="M 16,62 C 26,52 44,32 62,18 C 72,10 80,8 84,14 C 86,18 82,24 76,28 C 60,40 40,56 28,66 C 22,70 18,68 16,64 Z" fill="#97A1AE"/>
        <path d="M 16,62 C 26,72 44,92 62,106 C 72,114 80,116 84,110 C 86,106 82,100 76,96 C 60,84 40,68 28,58 C 22,54 18,56 16,60 Z" fill="#97A1AE"/>
        <circle cx="18" cy="62" r="15" fill="#FF6A3D" opacity="0.12"/>
        <circle cx="18" cy="62" r="10" fill="#FF6A3D"/>
      </g>
    </svg>
    <div style="font-family:'Space Grotesk',sans-serif; font-size:16px; color:#E8EDF3; font-weight:600;">KnowMe</div>
  </div>
  <p>Flint Meta Extension Architecture and Implementation Plan · RFC-FORGE-META-001 · June 2026</p>
  <p>Generated by Prometheus AGS Platform Intelligence. This architecture reflects research on PostgREST, postgres-meta, pg_graphql, pgrx, Ory Keto, and AG-UI/A2UI standards.</p>
</footer>
</main>
</div>
</body>
</html>
'''

# Build full HTML
full_html = head_css + body + footer

# Fix any remaining markdown bold patterns in the HTML (outside of code blocks)
# We need to be careful not to break code blocks
import re

def fix_bold_outside_code(text):
    # Simple approach: replace **text** with <strong>text</strong> 
    # but avoid code blocks
    result = []
    in_code = False
    for line in text.split('\n'):
        if '<pre>' in line or '<code' in line:
            in_code = True
        if '</pre>' in line or '</code>' in line:
            in_code = False
        
        if not in_code and not line.strip().startswith('<') and '**' in line:
            line = re.sub(r'\*\*(.+?)\*\*', r'<strong>\1</strong>', line)
            line = re.sub(r'(?<!\*)\*(?!\*)(.+?)(?<!\*)\*(?!\*)', r'<em>\1</em>', line)
        
        result.append(line)
    return '\n'.join(result)

full_html = fix_bold_outside_code(full_html)

output_path = '/Users/gqadonis/Projects/prometheus/flint-forge/docs/FLINT-META-EXTENSION-PLAN.html'
with open(output_path, 'w') as f:
    f.write(full_html)

print(f"Wrote {len(full_html)} bytes to {output_path}")

# Verify
remaining_bold = len(re.findall(r'\*\*', full_html))
print(f"Remaining ** patterns: {remaining_bold}")

# Count SVGs
svg_count = full_html.count('<svg')
print(f"SVG count: {svg_count}")

# Count tables
table_count = full_html.count('<table')
print(f"Table count: {table_count}")

# Count pre blocks
pre_count = full_html.count('<pre>')
print(f"Pre count: {pre_count}")

# Count figures
fig_count = full_html.count('<figure>')
print(f"Figure count: {fig_count}")
