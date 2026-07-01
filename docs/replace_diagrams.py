#!/usr/bin/env python3
"""
Replace all ASCII/line diagrams in COMPETITIVE-ANALYSIS.html with branded SVGs.
Brand colors: bg=#0B0F14, surface-1=#131A22, surface-2=#1A232E, text=#E8EDF3,
              text-muted=#B4BECB, ember=#FF6A3D, cyan=#34CFE6, green=#4FD18B,
              yellow=#F4B942, line=#28333F
"""
import re

with open('/Users/gqadonis/Projects/prometheus/flint-forge/docs/COMPETITIVE-ANALYSIS.html', 'r') as f:
    html = f.read()

# ------------------------------------------------------------------
# SVG 1: Supabase Architecture
# ------------------------------------------------------------------
supabase_arch = '''<figure>
<svg viewBox="0 0 860 420" xmlns="http://www.w3.org/2000/svg" style="max-width:860px">
  <defs>
    <filter id="sh1" x="-5%" y="-5%" width="110%" height="110%">
      <feDropShadow dx="0" dy="2" stdDeviation="3" flood-color="#000" flood-opacity="0.35"/>
    </filter>
    <linearGradient id="grad-sb" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0%" stop-color="#1A232E"/>
      <stop offset="100%" stop-color="#131A22"/>
    </linearGradient>
    <linearGradient id="grad-db" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0%" stop-color="#1A232E"/>
      <stop offset="100%" stop-color="#0E141B"/>
    </linearGradient>
  </defs>
  <!-- Outer container -->
  <rect x="10" y="10" width="840" height="400" rx="12" fill="url(#grad-sb)" stroke="#28333F" stroke-width="1.5"/>
  <text x="430" y="38" text-anchor="middle" font-family="Space Grotesk, sans-serif" font-size="15" font-weight="700" fill="#E8EDF3" letter-spacing="0.08em">SUPABASE PLATFORM</text>

  <!-- Top row: 4 service boxes -->
  <g transform="translate(0,0)">
    <!-- Auth -->
    <rect x="40" y="55" width="170" height="70" rx="8" fill="#131A22" stroke="#34CFE6" stroke-width="1.5" filter="url(#sh1)"/>
    <text x="125" y="82" text-anchor="middle" font-family="Space Grotesk, sans-serif" font-size="13" font-weight="600" fill="#E8EDF3">Auth</text>
    <text x="125" y="100" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="10" fill="#34CFE6">GoTrue (Go)</text>
    <text x="125" y="114" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="9" fill="#97A1AE">JWT-based</text>

    <!-- REST/GraphQL -->
    <rect x="235" y="55" width="170" height="70" rx="8" fill="#131A22" stroke="#34CFE6" stroke-width="1.5" filter="url(#sh1)"/>
    <text x="320" y="82" text-anchor="middle" font-family="Space Grotesk, sans-serif" font-size="13" font-weight="600" fill="#E8EDF3">REST / GraphQL</text>
    <text x="320" y="100" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="10" fill="#34CFE6">PostgREST</text>
    <text x="320" y="114" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="9" fill="#97A1AE">Auto-generated</text>

    <!-- Realtime -->
    <rect x="430" y="55" width="170" height="70" rx="8" fill="#131A22" stroke="#34CFE6" stroke-width="1.5" filter="url(#sh1)"/>
    <text x="515" y="82" text-anchor="middle" font-family="Space Grotesk, sans-serif" font-size="13" font-weight="600" fill="#E8EDF3">Realtime</text>
    <text x="515" y="100" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="10" fill="#34CFE6">Phoenix (Elixir)</text>
    <text x="515" y="114" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="9" fill="#97A1AE">WAL polling</text>

    <!-- Edge Functions -->
    <rect x="625" y="55" width="170" height="70" rx="8" fill="#131A22" stroke="#34CFE6" stroke-width="1.5" filter="url(#sh1)"/>
    <text x="710" y="82" text-anchor="middle" font-family="Space Grotesk, sans-serif" font-size="13" font-weight="600" fill="#E8EDF3">Edge Functions</text>
    <text x="710" y="100" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="10" fill="#34CFE6">Deno Runtime</text>
    <text x="710" y="114" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="9" fill="#97A1AE">V8 isolate</text>
  </g>

  <!-- Connector lines down to Postgres -->
  <line x1="125" y1="125" x2="125" y2="155" stroke="#28333F" stroke-width="1.5"/>
  <line x1="320" y1="125" x2="320" y2="155" stroke="#28333F" stroke-width="1.5"/>
  <line x1="515" y1="125" x2="515" y2="155" stroke="#28333F" stroke-width="1.5"/>
  <line x1="710" y1="125" x2="710" y2="155" stroke="#28333F" stroke-width="1.5"/>
  <line x1="125" y1="155" x2="710" y2="155" stroke="#28333F" stroke-width="1.5"/>
  <!-- Vertical down to Postgres -->
  <line x1="430" y1="155" x2="430" y2="175" stroke="#28333F" stroke-width="1.5"/>
  <!-- Arrow -->
  <polygon points="426,170 430,178 434,170" fill="#28333F"/>

  <!-- PostgreSQL box -->
  <rect x="210" y="178" width="440" height="90" rx="10" fill="url(#grad-db)" stroke="#FF6A3D" stroke-width="2" filter="url(#sh1)"/>
  <text x="430" y="208" text-anchor="middle" font-family="Space Grotesk, sans-serif" font-size="16" font-weight="700" fill="#E8EDF3">PostgreSQL</text>
  <text x="430" y="228" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="11" fill="#FF6A3D">+ Supavisor Pooler</text>
  <text x="430" y="250" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="10" fill="#97A1AE">50+ extensions · pgvector · pg_graphql · pg_net · pgsodium</text>

  <!-- Connector down to Storage -->
  <line x1="430" y1="268" x2="430" y2="298" stroke="#28333F" stroke-width="1.5"/>
  <polygon points="426,293 430,301 434,293" fill="#28333F"/>

  <!-- Storage box -->
  <rect x="305" y="301" width="250" height="60" rx="8" fill="#131A22" stroke="#34CFE6" stroke-width="1.5" filter="url(#sh1)"/>
  <text x="430" y="328" text-anchor="middle" font-family="Space Grotesk, sans-serif" font-size="13" font-weight="600" fill="#E8EDF3">Storage</text>
  <text x="430" y="346" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="10" fill="#97A1AE">S3-compatible · CDN · Node/TS</text>

  <!-- Key characteristic label -->
  <text x="430" y="390" text-anchor="middle" font-family="Inter, sans-serif" font-size="11" fill="#97A1AE"><tspan font-style="italic">Service-oriented</tspan> — Postgres is the integration hub</text>
</svg>
<figcaption><b>Figure 1.1</b> Supabase platform architecture — four services atop PostgreSQL with Storage below.</figcaption>
</figure>'''

# ------------------------------------------------------------------
# SVG 2: Flint Architecture
# ------------------------------------------------------------------
flint_arch = '''<figure>
<svg viewBox="0 0 900 480" xmlns="http://www.w3.org/2000/svg" style="max-width:900px">
  <defs>
    <filter id="sh2" x="-5%" y="-5%" width="110%" height="110%">
      <feDropShadow dx="0" dy="2" stdDeviation="3" flood-color="#000" flood-opacity="0.35"/>
    </filter>
    <linearGradient id="grad-flint" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0%" stop-color="#1A232E"/>
      <stop offset="100%" stop-color="#131A22"/>
    </linearGradient>
    <linearGradient id="grad-pg" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0%" stop-color="#1A232E"/>
      <stop offset="100%" stop-color="#0E141B"/>
    </linearGradient>
  </defs>
  <rect x="10" y="10" width="880" height="460" rx="12" fill="url(#grad-flint)" stroke="#28333F" stroke-width="1.5"/>
  <text x="450" y="38" text-anchor="middle" font-family="Space Grotesk, sans-serif" font-size="15" font-weight="700" fill="#E8EDF3" letter-spacing="0.08em">FLINT PLATFORM</text>

  <!-- Flint Gate (left column) -->
  <rect x="35" y="55" width="190" height="200" rx="10" fill="#131A22" stroke="#FF6A3D" stroke-width="2" filter="url(#sh2)"/>
  <text x="130" y="80" text-anchor="middle" font-family="Space Grotesk, sans-serif" font-size="14" font-weight="700" fill="#E8EDF3">Flint Gate</text>
  <text x="130" y="96" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="10" fill="#FF6A3D">Axum / Rust</text>
  <text x="50" y="120" font-family="Inter, sans-serif" font-size="10" fill="#B4BECB">• Kratos auth</text>
  <text x="50" y="138" font-family="Inter, sans-serif" font-size="10" fill="#B4BECB">• JWT mint</text>
  <text x="50" y="156" font-family="Inter, sans-serif" font-size="10" fill="#B4BECB">• SSE streaming</text>
  <text x="50" y="174" font-family="Inter, sans-serif" font-size="10" fill="#B4BECB">• AG-UI / A2UI</text>
  <text x="50" y="192" font-family="Inter, sans-serif" font-size="10" fill="#B4BECB">• Token metering</text>
  <text x="50" y="210" font-family="Inter, sans-serif" font-size="10" fill="#B4BECB">• Backpressure</text>
  <text x="50" y="228" font-family="Inter, sans-serif" font-size="10" fill="#B4BECB">• Cedar policy</text>

  <!-- Connection: Gate → Forge (horizontal) -->
  <line x1="225" y1="125" x2="265" y2="125" stroke="#FF6A3D" stroke-width="2" stroke-dasharray="4,3"/>
  <text x="245" y="118" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="9" fill="#FF6A3D">RLS JWT</text>
  <polygon points="260,121 268,125 260,129" fill="#FF6A3D"/>

  <!-- Flint Forge (right block) -->
  <rect x="270" y="55" width="595" height="200" rx="10" fill="#131A22" stroke="#34CFE6" stroke-width="1.5" filter="url(#sh2)"/>
  <text x="567" y="80" text-anchor="middle" font-family="Space Grotesk, sans-serif" font-size="14" font-weight="700" fill="#E8EDF3">Flint Forge</text>
  <text x="567" y="96" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="10" fill="#34CFE6">Data + Edge Compute Plane</text>

  <!-- Quarry -->
  <rect x="290" y="110" width="170" height="60" rx="8" fill="#0E141B" stroke="#34CFE6" stroke-width="1.2" filter="url(#sh2)"/>
  <text x="375" y="135" text-anchor="middle" font-family="Space Grotesk, sans-serif" font-size="12" font-weight="600" fill="#E8EDF3">Quarry</text>
  <text x="375" y="153" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="10" fill="#34CFE6">REST / GraphQL Gateway</text>

  <!-- Kiln -->
  <rect x="480" y="110" width="170" height="60" rx="8" fill="#0E141B" stroke="#FF6A3D" stroke-width="1.2" filter="url(#sh2)"/>
  <text x="565" y="135" text-anchor="middle" font-family="Space Grotesk, sans-serif" font-size="12" font-weight="600" fill="#E8EDF3">Kiln</text>
  <text x="565" y="153" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="10" fill="#FF6A3D">WASM Edge Functions</text>

  <!-- Postgres 18 (wide box) -->
  <rect x="290" y="185" width="555" height="55" rx="8" fill="url(#grad-pg)" stroke="#FF6A3D" stroke-width="2" filter="url(#sh2)"/>
  <text x="380" y="210" text-anchor="middle" font-family="Space Grotesk, sans-serif" font-size="13" font-weight="700" fill="#E8EDF3">PostgreSQL 18</text>
  <text x="380" y="228" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="9" fill="#97A1AE">+ pgrx extensions</text>
  <!-- Extension pills inside Postgres -->
  <rect x="470" y="200" width="50" height="22" rx="4" fill="#0E141B" stroke="#34CFE6" stroke-width="0.8"/>
  <text x="495" y="215" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="8" fill="#34CFE6">Auth</text>
  <rect x="530" y="200" width="50" height="22" rx="4" fill="#0E141B" stroke="#34CFE6" stroke-width="0.8"/>
  <text x="555" y="215" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="8" fill="#34CFE6">Hooks</text>
  <rect x="590" y="200" width="50" height="22" rx="4" fill="#0E141B" stroke="#FF6A3D" stroke-width="0.8"/>
  <text x="615" y="215" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="8" fill="#FF6A3D">LLM</text>
  <rect x="650" y="200" width="55" height="22" rx="4" fill="#0E141B" stroke="#FF6A3D" stroke-width="0.8"/>
  <text x="677" y="215" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="8" fill="#FF6A3D">Ember</text>
  <rect x="715" y="200" width="50" height="22" rx="4" fill="#0E141B" stroke="#4FD18B" stroke-width="0.8"/>
  <text x="740" y="215" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="8" fill="#4FD18B">Vault</text>
  <rect x="775" y="200" width="55" height="22" rx="4" fill="#0E141B" stroke="#4FD18B" stroke-width="0.8"/>
  <text x="802" y="215" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="8" fill="#4FD18B">pgvector</text>

  <!-- Connection: Gate → Realtime Fabric (vertical then horizontal) -->
  <line x1="130" y1="255" x2="130" y2="295" stroke="#FF6A3D" stroke-width="1.5" stroke-dasharray="4,3"/>
  <text x="145" y="280" font-family="JetBrains Mono, monospace" font-size="9" fill="#FF6A3D">WatchEntityType (gRPC)</text>
  <polygon points="126,288 130,296 134,288" fill="#FF6A3D"/>

  <!-- Realtime Fabric (bottom left) -->
  <rect x="35" y="300" width="260" height="130" rx="10" fill="#131A22" stroke="#4FD18B" stroke-width="2" filter="url(#sh2)"/>
  <text x="165" y="325" text-anchor="middle" font-family="Space Grotesk, sans-serif" font-size="14" font-weight="700" fill="#E8EDF3">Realtime Fabric</text>
  <text x="165" y="343" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="10" fill="#4FD18B">Iggy spine · WebSocket mux</text>
  <text x="50" y="370" font-family="Inter, sans-serif" font-size="10" fill="#B4BECB">• CRDT synchronization</text>
  <text x="50" y="388" font-family="Inter, sans-serif" font-size="10" fill="#B4BECB">• Federation gateways</text>
  <text x="50" y="406" font-family="Inter, sans-serif" font-size="10" fill="#B4BECB">• Per-event RLS re-query</text>
  <text x="50" y="424" font-family="Inter, sans-serif" font-size="10" fill="#B4BECB">• AG-UI / A2UI streaming</text>

  <!-- Connection: Forge → Realtime Fabric (not directly, but implied via Postgres) -->
  <!-- Actually, no direct line in the ASCII. Let's add a subtle one. -->
  <line x1="567" y1="240" x2="567" y2="270" stroke="#28333F" stroke-width="1" opacity="0.5"/>
  <line x1="295" y1="270" x2="567" y2="270" stroke="#28333F" stroke-width="1" opacity="0.5"/>
  <line x1="295" y1="270" x2="295" y2="365" stroke="#28333F" stroke-width="1" opacity="0.5"/>
  <polygon points="291,357 295,365 299,357" fill="#28333F" opacity="0.5"/>

  <!-- Key characteristic label -->
  <text x="450" y="465" text-anchor="middle" font-family="Inter, sans-serif" font-size="11" fill="#97A1AE"><tspan font-style="italic">Plane-oriented</tspan> — one identity model, one governance boundary</text>
</svg>
<figcaption><b>Figure 1.2</b> Flint platform architecture — three co-designed planes sharing one identity model and WASM host substrate.</figcaption>
</figure>'''

# ------------------------------------------------------------------
# SVG 3: Deno Runtime Pipeline
# ------------------------------------------------------------------
deno_pipeline = '''<figure>
<svg viewBox="0 0 800 100" xmlns="http://www.w3.org/2000/svg" style="max-width:800px">
  <defs><filter id="shp" x="-3%" y="-10%" width="106%" height="120%"><feDropShadow dx="0" dy="2" stdDeviation="2" flood-color="#000" flood-opacity="0.3"/></filter></defs>
  <!-- Pipeline stages -->
  <g transform="translate(0,20)">
    <rect x="0" y="0" width="110" height="42" rx="6" fill="#131A22" stroke="#F4B942" stroke-width="1.5" filter="url(#shp)"/>
    <text x="55" y="20" text-anchor="middle" font-family="Space Grotesk, sans-serif" font-size="11" font-weight="600" fill="#E8EDF3">Request</text>
    <text x="55" y="34" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="9" fill="#97A1AE">HTTP</text>
  </g>
  <line x1="115" y1="41" x2="140" y2="41" stroke="#F4B942" stroke-width="2"/><polygon points="134,37 142,41 134,45" fill="#F4B942"/>

  <g transform="translate(145,20)">
    <rect x="0" y="0" width="130" height="42" rx="6" fill="#131A22" stroke="#F4B942" stroke-width="1.5" filter="url(#shp)"/>
    <text x="65" y="20" text-anchor="middle" font-family="Space Grotesk, sans-serif" font-size="11" font-weight="600" fill="#E8EDF3">Deno Isolate</text>
    <text x="65" y="34" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="9" fill="#97A1AE">V8 process</text>
  </g>
  <line x1="280" y1="41" x2="305" y2="41" stroke="#F4B942" stroke-width="2"/><polygon points="299,37 307,41 299,45" fill="#F4B942"/>

  <g transform="translate(310,20)">
    <rect x="0" y="0" width="100" height="42" rx="6" fill="#131A22" stroke="#F4B942" stroke-width="1.5" filter="url(#shp)"/>
    <text x="50" y="20" text-anchor="middle" font-family="Space Grotesk, sans-serif" font-size="11" font-weight="600" fill="#E8EDF3">V8 JIT</text>
    <text x="50" y="34" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="9" fill="#97A1AE">JIT compiler</text>
  </g>
  <line x1="415" y1="41" x2="440" y2="41" stroke="#F4B942" stroke-width="2"/><polygon points="434,37 442,41 434,45" fill="#F4B942"/>

  <g transform="translate(445,20)">
    <rect x="0" y="0" width="90" height="42" rx="6" fill="#131A22" stroke="#F4B942" stroke-width="1.5" filter="url(#shp)"/>
    <text x="45" y="20" text-anchor="middle" font-family="Space Grotesk, sans-serif" font-size="11" font-weight="600" fill="#E8EDF3">Syscall</text>
    <text x="45" y="34" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="9" fill="#97A1AE">permission</text>
  </g>
  <line x1="540" y1="41" x2="565" y2="41" stroke="#F4B942" stroke-width="2"/><polygon points="559,37 567,41 559,45" fill="#F4B942"/>

  <g transform="translate(570,20)">
    <rect x="0" y="0" width="110" height="42" rx="6" fill="#131A22" stroke="#F4B942" stroke-width="1.5" filter="url(#shp)"/>
    <text x="55" y="20" text-anchor="middle" font-family="Space Grotesk, sans-serif" font-size="11" font-weight="600" fill="#E8EDF3">Host OS</text>
    <text x="55" y="34" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="9" fill="#97A1AE">full access</text>
  </g>

  <!-- Warning label -->
  <text x="340" y="85" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="10" fill="#F4B942">⚠ Large attack surface · V8 CVEs · JIT spraying · Spectre</text>
</svg>
<figcaption><b>Figure 5.1</b> Deno runtime execution pipeline — V8-based with permission-model syscall gating.</figcaption>
</figure>'''

# ------------------------------------------------------------------
# SVG 4: Wasmtime Runtime Pipeline
# ------------------------------------------------------------------
wasmtime_pipeline = '''<figure>
<svg viewBox="0 0 900 100" xmlns="http://www.w3.org/2000/svg" style="max-width:900px">
  <defs><filter id="shp2" x="-3%" y="-10%" width="106%" height="120%"><feDropShadow dx="0" dy="2" stdDeviation="2" flood-color="#000" flood-opacity="0.3"/></filter></defs>
  <g transform="translate(0,20)">
    <rect x="0" y="0" width="100" height="42" rx="6" fill="#131A22" stroke="#4FD18B" stroke-width="1.5" filter="url(#shp2)"/>
    <text x="50" y="20" text-anchor="middle" font-family="Space Grotesk, sans-serif" font-size="11" font-weight="600" fill="#E8EDF3">Request</text>
    <text x="50" y="34" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="9" fill="#97A1AE">signed</text>
  </g>
  <line x1="105" y1="41" x2="125" y2="41" stroke="#4FD18B" stroke-width="2"/><polygon points="119,37 127,41 119,45" fill="#4FD18B"/>

  <g transform="translate(130,20)">
    <rect x="0" y="0" width="110" height="42" rx="6" fill="#131A22" stroke="#4FD18B" stroke-width="1.5" filter="url(#shp2)"/>
    <text x="55" y="20" text-anchor="middle" font-family="Space Grotesk, sans-serif" font-size="11" font-weight="600" fill="#E8EDF3">Wasmtime</text>
    <text x="55" y="34" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="9" fill="#97A1AE">runtime</text>
  </g>
  <line x1="245" y1="41" x2="265" y2="41" stroke="#4FD18B" stroke-width="2"/><polygon points="259,37 267,41 259,45" fill="#4FD18B"/>

  <g transform="translate(270,20)">
    <rect x="0" y="0" width="130" height="42" rx="6" fill="#131A22" stroke="#4FD18B" stroke-width="1.5" filter="url(#shp2)"/>
    <text x="65" y="20" text-anchor="middle" font-family="Space Grotesk, sans-serif" font-size="11" font-weight="600" fill="#E8EDF3">Cranelift AOT</text>
    <text x="65" y="34" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="9" fill="#97A1AE">no JIT in data plane</text>
  </g>
  <line x1="405" y1="41" x2="425" y2="41" stroke="#4FD18B" stroke-width="2"/><polygon points="419,37 427,41 419,45" fill="#4FD18B"/>

  <g transform="translate(430,20)">
    <rect x="0" y="0" width="170" height="42" rx="6" fill="#131A22" stroke="#4FD18B" stroke-width="1.5" filter="url(#shp2)"/>
    <text x="85" y="20" text-anchor="middle" font-family="Space Grotesk, sans-serif" font-size="11" font-weight="600" fill="#E8EDF3">Native Machine Code</text>
    <text x="85" y="34" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="9" fill="#97A1AE">hardware sandbox</text>
  </g>
  <line x1="605" y1="41" x2="625" y2="41" stroke="#4FD18B" stroke-width="2"/><polygon points="619,37 627,41 619,45" fill="#4FD18B"/>

  <g transform="translate(630,20)">
    <rect x="0" y="0" width="200" height="42" rx="6" fill="#131A22" stroke="#4FD18B" stroke-width="1.5" filter="url(#shp2)"/>
    <text x="100" y="20" text-anchor="middle" font-family="Space Grotesk, sans-serif" font-size="11" font-weight="600" fill="#E8EDF3">Host OS (via WASI)</text>
    <text x="100" y="34" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="9" fill="#97A1AE">preopen capabilities only</text>
  </g>

  <!-- Good label -->
  <text x="415" y="85" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="10" fill="#4FD18B">✓ No JIT in data plane · hardware bounds · signed components · fuel/epoch limits</text>
</svg>
<figcaption><b>Figure 5.2</b> Wasmtime execution pipeline — AOT compilation with hardware-enforced sandbox and capability-based WASI.</figcaption>
</figure>'''

# ------------------------------------------------------------------
# SVG 5: Microsandbox Runtime Pipeline
# ------------------------------------------------------------------
microsandbox_pipeline = '''<figure>
<svg viewBox="0 0 900 100" xmlns="http://www.w3.org/2000/svg" style="max-width:900px">
  <defs><filter id="shp3" x="-3%" y="-10%" width="106%" height="120%"><feDropShadow dx="0" dy="2" stdDeviation="2" flood-color="#000" flood-opacity="0.3"/></filter></defs>
  <g transform="translate(0,20)">
    <rect x="0" y="0" width="100" height="42" rx="6" fill="#131A22" stroke="#34CFE6" stroke-width="1.5" filter="url(#shp3)"/>
    <text x="50" y="20" text-anchor="middle" font-family="Space Grotesk, sans-serif" font-size="11" font-weight="600" fill="#E8EDF3">Request</text>
  </g>
  <line x1="105" y1="41" x2="125" y2="41" stroke="#34CFE6" stroke-width="2"/><polygon points="119,37 127,41 119,45" fill="#34CFE6"/>

  <g transform="translate(130,20)">
    <rect x="0" y="0" width="100" height="42" rx="6" fill="#131A22" stroke="#34CFE6" stroke-width="1.5" filter="url(#shp3)"/>
    <text x="50" y="20" text-anchor="middle" font-family="Space Grotesk, sans-serif" font-size="11" font-weight="600" fill="#E8EDF3">libkrun</text>
    <text x="50" y="34" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="9" fill="#97A1AE">Rust lib</text>
  </g>
  <line x1="235" y1="41" x2="255" y2="41" stroke="#34CFE6" stroke-width="2"/><polygon points="249,37 257,41 249,45" fill="#34CFE6"/>

  <g transform="translate(260,20)">
    <rect x="0" y="0" width="110" height="42" rx="6" fill="#131A22" stroke="#34CFE6" stroke-width="1.5" filter="url(#shp3)"/>
    <text x="55" y="20" text-anchor="middle" font-family="Space Grotesk, sans-serif" font-size="11" font-weight="600" fill="#E8EDF3">KVM / HVF</text>
    <text x="55" y="34" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="9" fill="#97A1AE">hypervisor</text>
  </g>
  <line x1="375" y1="41" x2="395" y2="41" stroke="#34CFE6" stroke-width="2"/><polygon points="389,37 397,41 389,45" fill="#34CFE6"/>

  <g transform="translate(400,20)">
    <rect x="0" y="0" width="100" height="42" rx="6" fill="#131A22" stroke="#34CFE6" stroke-width="1.5" filter="url(#shp3)"/>
    <text x="50" y="20" text-anchor="middle" font-family="Space Grotesk, sans-serif" font-size="11" font-weight="600" fill="#E8EDF3">MicroVM</text>
    <text x="50" y="34" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="9" fill="#97A1AE">hardware iso</text>
  </g>
  <line x1="505" y1="41" x2="525" y2="41" stroke="#34CFE6" stroke-width="2"/><polygon points="519,37 527,41 519,45" fill="#34CFE6"/>

  <g transform="translate(530,20)">
    <rect x="0" y="0" width="140" height="42" rx="6" fill="#131A22" stroke="#34CFE6" stroke-width="1.5" filter="url(#shp3)"/>
    <text x="70" y="20" text-anchor="middle" font-family="Space Grotesk, sans-serif" font-size="11" font-weight="600" fill="#E8EDF3">Linux Kernel</text>
    <text x="70" y="34" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="9" fill="#97A1AE">guest OS</text>
  </g>
  <line x1="675" y1="41" x2="695" y2="41" stroke="#34CFE6" stroke-width="2"/><polygon points="689,37 697,41 689,45" fill="#34CFE6"/>

  <g transform="translate(700,20)">
    <rect x="0" y="0" width="130" height="42" rx="6" fill="#131A22" stroke="#34CFE6" stroke-width="1.5" filter="url(#shp3)"/>
    <text x="65" y="20" text-anchor="middle" font-family="Space Grotesk, sans-serif" font-size="11" font-weight="600" fill="#E8EDF3">User Code</text>
    <text x="65" y="34" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="9" fill="#97A1AE">any binary</text>
  </g>

  <!-- Note label -->
  <text x="415" y="85" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="10" fill="#34CFE6">~100-200ms cold start · hardware isolation · any language binary</text>
</svg>
<figcaption><b>Figure 5.3</b> Microsandbox execution pipeline — microVM-based hardware isolation via KVM/HVF.</figcaption>
</figure>'''

# ------------------------------------------------------------------
# SVG 6: Ports-and-Adapters Database
# ------------------------------------------------------------------
ports_adapters = '''<figure>
<svg viewBox="0 0 800 520" xmlns="http://www.w3.org/2000/svg" style="max-width:800px">
  <defs>
    <filter id="sha" x="-5%" y="-5%" width="110%" height="110%"><feDropShadow dx="0" dy="2" stdDeviation="3" flood-color="#000" flood-opacity="0.35"/></filter>
    <marker id="arrow-ember" markerWidth="8" markerHeight="8" refX="7" refY="4" orient="auto"><polygon points="0,0 8,4 0,8" fill="#FF6A3D"/></marker>
    <marker id="arrow-cyan" markerWidth="8" markerHeight="8" refX="7" refY="4" orient="auto"><polygon points="0,0 8,4 0,8" fill="#34CFE6"/></marker>
  </defs>
  <rect x="10" y="10" width="780" height="500" rx="12" fill="#131A22" stroke="#28333F" stroke-width="1.5"/>

  <!-- Quarry -->
  <rect x="300" y="30" width="200" height="55" rx="8" fill="#0E141B" stroke="#FF6A3D" stroke-width="2" filter="url(#sha)"/>
  <text x="400" y="55" text-anchor="middle" font-family="Space Grotesk, sans-serif" font-size="14" font-weight="700" fill="#E8EDF3">Quarry</text>
  <text x="400" y="74" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="10" fill="#FF6A3D">Axum Gateway</text>

  <!-- Incoming requests label -->
  <text x="140" y="60" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="11" fill="#B4BECB">REST / GraphQL</text>
  <text x="140" y="78" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="10" fill="#97A1AE">requests</text>
  <line x1="210" y1="58" x2="290" y2="58" stroke="#B4BECB" stroke-width="1.5" marker-end="url(#arrow-cyan)"/>

  <!-- Arrow down to fdb-ports -->
  <line x1="400" y1="85" x2="400" y2="115" stroke="#FF6A3D" stroke-width="2" marker-end="url(#arrow-ember)"/>

  <!-- fdb-ports (traits) -->
  <rect x="250" y="120" width="300" height="70" rx="8" fill="#0E141B" stroke="#FF6A3D" stroke-width="1.5" stroke-dasharray="6,4" filter="url(#sha)"/>
  <text x="400" y="145" text-anchor="middle" font-family="Space Grotesk, sans-serif" font-size="13" font-weight="600" fill="#E8EDF3">fdb-ports</text>
  <text x="400" y="163" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="10" fill="#FF6A3D">Trait seams</text>
  <text x="400" y="180" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="9" fill="#97A1AE">DatabaseBackend · SchemaProvider · RestExecutor · GraphQlExecutor · ChangeStreamSource</text>

  <!-- Arrow down to fdb-app -->
  <line x1="400" y1="190" x2="400" y2="220" stroke="#FF6A3D" stroke-width="2" marker-end="url(#arrow-ember)"/>

  <!-- fdb-app -->
  <rect x="300" y="225" width="200" height="55" rx="8" fill="#0E141B" stroke="#FF6A3D" stroke-width="1.5" filter="url(#sha)"/>
  <text x="400" y="250" text-anchor="middle" font-family="Space Grotesk, sans-serif" font-size="13" font-weight="600" fill="#E8EDF3">fdb-app</text>
  <text x="400" y="268" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="10" fill="#FF6A3D">Use-cases layer</text>

  <!-- Arrow down to split -->
  <line x1="400" y1="280" x2="400" y2="310" stroke="#FF6A3D" stroke-width="2" marker-end="url(#arrow-ember)"/>
  <line x1="200" y1="310" x2="600" y2="310" stroke="#FF6A3D" stroke-width="1.5"/>

  <!-- Left branch: fdb-postgres -->
  <line x1="200" y1="310" x2="200" y2="340" stroke="#FF6A3D" stroke-width="1.5" marker-end="url(#arrow-ember)"/>
  <rect x="110" y="345" width="180" height="55" rx="8" fill="#0E141B" stroke="#34CFE6" stroke-width="1.5" filter="url(#sha)"/>
  <text x="200" y="370" text-anchor="middle" font-family="Space Grotesk, sans-serif" font-size="12" font-weight="600" fill="#E8EDF3">fdb-postgres</text>
  <text x="200" y="388" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="10" fill="#34CFE6">Adapter</text>

  <line x1="200" y1="400" x2="200" y2="430" stroke="#34CFE6" stroke-width="1.5" marker-end="url(#arrow-cyan)"/>
  <rect x="80" y="435" width="240" height="55" rx="8" fill="#0E141B" stroke="#4FD18B" stroke-width="2" filter="url(#sha)"/>
  <text x="200" y="460" text-anchor="middle" font-family="Space Grotesk, sans-serif" font-size="13" font-weight="700" fill="#E8EDF3">PostgreSQL</text>
  <text x="200" y="478" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="10" fill="#4FD18B">+ pgrx extensions</text>

  <!-- Right branch: fdb-surrealdb -->
  <line x1="600" y1="310" x2="600" y2="340" stroke="#FF6A3D" stroke-width="1.5" marker-end="url(#arrow-ember)"/>
  <rect x="510" y="345" width="180" height="55" rx="8" fill="#0E141B" stroke="#34CFE6" stroke-width="1.5" filter="url(#sha)"/>
  <text x="600" y="370" text-anchor="middle" font-family="Space Grotesk, sans-serif" font-size="12" font-weight="600" fill="#E8EDF3">fdb-surrealdb</text>
  <text x="600" y="388" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="10" fill="#34CFE6">Adapter (future)</text>

  <line x1="600" y1="400" x2="600" y2="430" stroke="#34CFE6" stroke-width="1.5" marker-end="url(#arrow-cyan)"/>
  <rect x="480" y="435" width="240" height="55" rx="8" fill="#0E141B" stroke="#F4B942" stroke-width="2" filter="url(#sha)"/>
  <text x="600" y="460" text-anchor="middle" font-family="Space Grotesk, sans-serif" font-size="13" font-weight="700" fill="#E8EDF3">SurrealDB 3.x</text>
  <text x="600" y="478" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="10" fill="#F4B942">Graph · Real-time · Edge · AI</text>

  <!-- Adapter label -->
  <text x="400" y="510" text-anchor="middle" font-family="Inter, sans-serif" font-size="11" fill="#97A1AE">One adapter per port — backend-swappable without rewriting business logic</text>
</svg>
<figcaption><b>Figure 6.1</b> Ports-and-adapters database architecture — trait seams enable backend-swappability.</figcaption>
</figure>'''

# ------------------------------------------------------------------
# SVG 7: Hybrid Strategy
# ------------------------------------------------------------------
hybrid_strategy = '''<figure>
<svg viewBox="0 0 700 380" xmlns="http://www.w3.org/2000/svg" style="max-width:700px">
  <defs>
    <filter id="shh" x="-5%" y="-5%" width="110%" height="110%"><feDropShadow dx="0" dy="2" stdDeviation="3" flood-color="#000" flood-opacity="0.35"/></filter>
    <marker id="arr-ember" markerWidth="8" markerHeight="8" refX="7" refY="4" orient="auto"><polygon points="0,0 8,4 0,8" fill="#FF6A3D"/></marker>
    <marker id="arr-cyan" markerWidth="8" markerHeight="8" refX="7" refY="4" orient="auto"><polygon points="0,0 8,4 0,8" fill="#34CFE6"/></marker>
  </defs>
  <rect x="10" y="10" width="680" height="360" rx="12" fill="#131A22" stroke="#28333F" stroke-width="1.5"/>

  <!-- User-Facing App -->
  <rect x="150" y="30" width="400" height="55" rx="8" fill="#0E141B" stroke="#E8EDF3" stroke-width="2" filter="url(#shh)"/>
  <text x="350" y="55" text-anchor="middle" font-family="Space Grotesk, sans-serif" font-size="14" font-weight="700" fill="#E8EDF3">User-Facing App</text>
  <text x="350" y="74" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="10" fill="#97A1AE">Next.js · React · Flutter · etc.</text>

  <!-- Split arrow -->
  <line x1="350" y1="85" x2="350" y2="115" stroke="#E8EDF3" stroke-width="1.5" marker-end="url(#arr-cyan)"/>
  <line x1="200" y1="115" x2="500" y2="115" stroke="#E8EDF3" stroke-width="1.5"/>

  <!-- Supabase branch (left) -->
  <line x1="200" y1="115" x2="200" y2="145" stroke="#34CFE6" stroke-width="1.5" marker-end="url(#arr-cyan)"/>
  <rect x="100" y="150" width="200" height="110" rx="8" fill="#0E141B" stroke="#34CFE6" stroke-width="2" filter="url(#shh)"/>
  <text x="200" y="175" text-anchor="middle" font-family="Space Grotesk, sans-serif" font-size="13" font-weight="700" fill="#E8EDF3">Supabase</text>
  <text x="200" y="195" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="10" fill="#34CFE6">Auth · basic DB · storage</text>
  <text x="120" y="220" font-family="Inter, sans-serif" font-size="10" fill="#B4BECB">• Authentication</text>
  <text x="120" y="238" font-family="Inter, sans-serif" font-size="10" fill="#B4BECB">• User data CRUD</text>
  <text x="120" y="256" font-family="Inter, sans-serif" font-size="10" fill="#B4BECB">• File storage</text>

  <!-- Flint branch (right) -->
  <line x1="500" y1="115" x2="500" y2="145" stroke="#FF6A3D" stroke-width="1.5" marker-end="url(#arr-ember)"/>
  <rect x="400" y="150" width="200" height="110" rx="8" fill="#0E141B" stroke="#FF6A3D" stroke-width="2" filter="url(#shh)"/>
  <text x="500" y="175" text-anchor="middle" font-family="Space Grotesk, sans-serif" font-size="13" font-weight="700" fill="#E8EDF3">Flint</text>
  <text x="500" y="195" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="10" fill="#FF6A3D">AI agent · edge · inference</text>
  <text x="420" y="220" font-family="Inter, sans-serif" font-size="10" fill="#B4BECB">• AI orchestration</text>
  <text x="420" y="238" font-family="Inter, sans-serif" font-size="10" fill="#B4BECB">• In-DB inference</text>
  <text x="420" y="256" font-family="Inter, sans-serif" font-size="10" fill="#B4BECB">• WASM edge tools</text>

  <!-- Merge arrows -->
  <line x1="200" y1="260" x2="200" y2="290" stroke="#34CFE6" stroke-width="1.5" marker-end="url(#arr-cyan)"/>
  <line x1="500" y1="260" x2="500" y2="290" stroke="#FF6A3D" stroke-width="1.5" marker-end="url(#arr-ember)"/>
  <line x1="200" y1="290" x2="500" y2="290" stroke="#28333F" stroke-width="1.5"/>
  <line x1="350" y1="290" x2="350" y2="320" stroke="#28333F" stroke-width="1.5" marker-end="url(#arr-cyan)"/>

  <!-- Shared PostgreSQL -->
  <rect x="200" y="325" width="300" height="50" rx="8" fill="#0E141B" stroke="#4FD18B" stroke-width="2" filter="url(#shh)"/>
  <text x="350" y="348" text-anchor="middle" font-family="Space Grotesk, sans-serif" font-size="13" font-weight="700" fill="#E8EDF3">PostgreSQL</text>
  <text x="350" y="366" text-anchor="middle" font-family="JetBrains Mono, monospace" font-size="10" fill="#4FD18B">+ pgvector (shared or separate)</text>
</svg>
<figcaption><b>Figure 8.1</b> Hybrid deployment strategy — Supabase velocity for standard features, Flint sovereignty for AI-native infrastructure.</figcaption>
</figure>'''

# ------------------------------------------------------------------
# Now do replacements using exact content fingerprints
# ------------------------------------------------------------------

# 1. Supabase Architecture
old_supabase = '''<pre><code class="language-text">┌─────────────────────────────────────────────────────────────┐
│                      Supabase Platform                        │
│  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌───────────────┐  │
│  │  Auth   │  │ REST/   │  │ Realtime│  │ Edge Functions│  │
│  │ GoTrue  │  │ GraphQL │  │ Phoenix │  │ Deno Runtime  │  │
│  │ (Go)    │  │PostgREST│  │(Elixir) │  │ (V8 isolate)  │  │
│  └────┬────┘  └────┬────┘  └────┬────┘  └───────┬───────┘  │
│       │            │            │                │           │
│       └────────────┴────────────┴────────────────┘           │
│                          │                                   │
│                   ┌──────┴──────┐                            │
│                   │ PostgreSQL  │ ←── 50+ extensions          │
│                   │  + Pooler   │     pgvector, pg_graphql,   │
│                   │             │     pg_net, pgsodium, etc.  │
│                   └─────────────┘                            │
│                          │                                   │
│                   ┌──────┴──────┐                            │
│                   │   Storage   │ ←── S3-compatible, CDN     │
│                   │ (Node/TS)   │                            │
│                   └─────────────┘                            │
└─────────────────────────────────────────────────────────────┘</code></pre>'''

if old_supabase in html:
    html = html.replace(old_supabase, supabase_arch)
    print("Replaced Supabase Architecture")
else:
    print("WARNING: Supabase Architecture not found for exact replacement")

# 2. Flint Architecture
old_flint = '''<pre><code class="language-text">┌─────────────────────────────────────────────────────────────┐
│                      Flint Platform                         │
│                                                             │
│  ┌──────────────┐         ┌─────────────────────────────┐  │
│  │  Flint Gate  │◄───────►│      Flint Forge            │  │
│  │  (Axum/Rust) │  RLS JWT│  ┌─────────┐  ┌──────────┐  │  │
│  │              │         │  │ Quarry  │  │  Kiln    │  │  │
│  │ • Kratos auth│         │  │ REST/   │  │ WASM    │  │  │
│  │ • JWT mint   │         │  │ GraphQL │  │ Edge    │  │  │
│  │ • SSE stream │         │  │ gateway │  │ Functions│  │  │
│  │ • AG-UI/A2UI │         │  └────┬────┘  └────┬─────┘  │  │
│  │ • Token meter│         │       │            │        │  │
│  │ • Backpres.  │         │  ┌────┴────────────┴────┐   │  │
│  └──────────────┘         │  │     Postgres 18      │   │  │
│          │                │  │  ┌────┐┌────┐┌────┐  │   │  │
│          │ WatchEntityType│  │  │Auth││Hook││LLM │  │   │  │
│          │   (gRPC)       │  │  │    ││s   ││Ember│  │   │  │
│          ▼                │  │  └────┘└────┘└────┘  │   │  │
│  ┌──────────────────┐   │  │  ┌────┐┌──────────┐  │   │  │
│  │ Realtime Fabric  │   │  │  │Vault││  pgvector │  │   │  │
│  │ (Iggy spine,     │   │  │  │(KMS)││  pg_graphql│  │   │  │
│  │  WebSocket mux,   │   │  │  └────┘└──────────┘  │   │  │
│  │  CRDT, Federation)│   │  └────────────────────────┘   │  │
│  └──────────────────┘   └─────────────────────────────────┘  │
│                                                             │
└─────────────────────────────────────────────────────────────┘</code></pre>'''

if old_flint in html:
    html = html.replace(old_flint, flint_arch)
    print("Replaced Flint Architecture")
else:
    print("WARNING: Flint Architecture not found for exact replacement")

# 3. Deno Pipeline
old_deno = '''<pre><code class="language-text">Request → Deno Isolate → V8 JIT → Syscall → Host OS</code></pre>'''
if old_deno in html:
    html = html.replace(old_deno, deno_pipeline)
    print("Replaced Deno Pipeline")
else:
    print("WARNING: Deno Pipeline not found")

# 4. Ports-and-Adapters
old_ports = '''<pre><code class="language-text">                    ┌─────────────┐
     REST/GraphQL  │   Quarry    │
        requests   │   (Axum)    │
            │      └──────┬──────┘
            │             │
            │      ┌──────┴──────┐
            │      │  fdb-ports  │ ←── Trait seams
            │      │  (traits)   │     DatabaseBackend
            │      └──────┬──────┘     SchemaProvider
            │             │             RestExecutor
            │      ┌──────┴──────┐       GraphQlExecutor
            │      │  fdb-app    │       ChangeStreamSource
            │      │ (use-cases) │
            │      └──────┬──────┘
            │             │
       ┌────┴─────────────┴────┐
       │                     │
  ┌────┴────┐           ┌────┴────────┐
  │fdb-     │           │ fdb-        │ ←── Adapters
  │postgres │           │ surrealdb   │     (one per port)
  │         │           │  (future)   │
  └────┬────┘           └────┬────────┘
       │                     │
  ┌────┴────┐           ┌────┴────────┐
  │Postgres │           │  SurrealDB  │
  │  + pgrx │           │  3.x        │
  └─────────┘           └─────────────┘</code></pre>'''

if old_ports in html:
    html = html.replace(old_ports, ports_adapters)
    print("Replaced Ports-and-Adapters")
else:
    print("WARNING: Ports-and-Adapters not found")

# 5. Wasmtime Pipeline
old_wasmtime = '''<pre><code class="language-text">Request → Wasmtime → Cranelift AOT → Native Machine Code → Host OS (via WASI)</code></pre>'''
if old_wasmtime in html:
    html = html.replace(old_wasmtime, wasmtime_pipeline)
    print("Replaced Wasmtime Pipeline")
else:
    print("WARNING: Wasmtime Pipeline not found")

# 6. Microsandbox Pipeline
old_micro = '''<pre><code class="language-text">Request → libkrun → KVM/HVF → MicroVM → Linux Kernel → User Code</code></pre>'''
if old_micro in html:
    html = html.replace(old_micro, microsandbox_pipeline)
    print("Replaced Microsandbox Pipeline")
else:
    print("WARNING: Microsandbox Pipeline not found")

# 7. Hybrid Strategy
old_hybrid = '''<pre><code class="language-text">┌─────────────────────────────────────────┐
│           User-Facing App               │
│  (Next.js, React, Flutter, etc.)       │
└──────────────────┬──────────────────────┘
                   │
         ┌─────────┴─────────┐
         │                   │
    ┌────┴────┐         ┌────┴────┐
    │Supabase │         │  Flint  │
    │ (Auth,  │         │ (AI     │
    │  basic  │         │  agent  │
    │  DB,    │         │  infra, │
    │  storage│         │  edge   │
    │  )      │         │  tools) │
    └────┬────┘         └────┬────┘
         │                   │
         └─────────┬─────────┘
                   │
            ┌──────┴──────┐
            │  PostgreSQL │ ←── Shared database
            │  (pgvector) │     (or separate instances)
            └─────────────┘</code></pre>'''

if old_hybrid in html:
    html = html.replace(old_hybrid, hybrid_strategy)
    print("Replaced Hybrid Strategy")
else:
    print("WARNING: Hybrid Strategy not found")

# Write output
with open('/Users/gqadonis/Projects/prometheus/flint-forge/docs/COMPETITIVE-ANALYSIS.html', 'w') as f:
    f.write(html)

print(f"\nFinal file size: {len(html)} bytes")
print("Done.")
PYEOF