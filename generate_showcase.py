#!/usr/bin/env python3
"""Generate FLINT A2UI Component Showcase HTML document."""
import os
import re
import html

MARKDOWN_PATH = "/Users/gqadonis/Projects/prometheus/flint-forge/docs/FLINT-A2UI-REGISTRY-SPEC.md"
OUTPUT_PATH = "/Users/gqadonis/Projects/prometheus/flint-forge/docs/FLINT-A2UI-COMPONENT-SHOWCASE.html"

# --- Design Tokens ---
BG = "#0B0F14"
SURFACE = "#131A22"
SURFACE_ELEVATED = "#1B2430"
SURFACE_ELEVATED2 = "#222E3D"
ACCENT = "#FF6A3D"
CYAN = "#34CFE6"
GREEN = "#4FD18B"
YELLOW = "#FFD166"
PURPLE = "#A78BFA"
TEXT_PRIMARY = "#E2E8F0"
TEXT_SECONDARY = "#94A3B8"
TEXT_MUTED = "#64748B"
BORDER = "#2D3748"

# --- Helpers ---

def escape_html(text):
    return html.escape(text)

def slugify(text):
    return re.sub(r'[^a-z0-9]+', '-', text.lower().strip('-'))

def parse_markdown(md):
    """Parse markdown into sections with h1/h2/h3 headers."""
    lines = md.split('\n')
    sections = []
    current = {"level": 0, "title": "", "content": []}
    
    for line in lines:
        if line.startswith('# '):
            if current["content"]:
                sections.append(current)
            current = {"level": 1, "title": line[2:].strip(), "content": []}
        elif line.startswith('## '):
            if current["content"]:
                sections.append(current)
            current = {"level": 2, "title": line[3:].strip(), "content": []}
        elif line.startswith('### '):
            if current["content"]:
                sections.append(current)
            current = {"level": 3, "title": line[4:].strip(), "content": []}
        else:
            current["content"].append(line)
    
    if current["content"]:
        sections.append(current)
    
    return sections

def markdown_to_html(lines, is_inline=False):
    """Convert markdown lines to HTML."""
    md = '\n'.join(lines)
    
    # Code blocks
    md = re.sub(
        r'```(\w+)?\n(.*?)```',
        lambda m: f'<pre class="code-block"><code class="language-{m.group(1) or "text"}">{escape_html(m.group(2))}</code></pre>',
        md, flags=re.DOTALL
    )
    
    # Inline code
    md = re.sub(r'`([^`]+)`', r'<code class="inline-code">\1</code>', md)
    
    # Bold
    md = re.sub(r'\*\*(.+?)\*\*', r'<strong>\1</strong>', md)
    
    # Italic
    md = re.sub(r'\*(.+?)\*', r'<em>\1</em>', md)
    
    # Tables
    def table_repl(m):
        rows = m.group(0).strip().split('\n')
        if len(rows) < 2:
            return m.group(0)
        html_rows = []
        for i, row in enumerate(rows):
            cells = [c.strip() for c in row.split('|') if c.strip()]
            if i == 0:
                html_rows.append('<thead><tr>' + ''.join(f'<th>{c}</th>' for c in cells) + '</tr></thead>')
            elif i == 1 and all('-' in c for c in cells):
                continue
            else:
                html_rows.append('<tr>' + ''.join(f'<td>{c}</td>' for c in cells) + '</tr>')
        return '<table class="data-table">' + ''.join(html_rows) + '</table>'
    
    # Table pattern: lines starting with | and containing |
    table_pattern = r'(?:^\|.*\|\n?)+'
    md = re.sub(table_pattern, table_repl, md, flags=re.MULTILINE)
    
    # Blockquotes
    md = re.sub(r'^>\s*(.+)$', r'<blockquote>\1</blockquote>', md, flags=re.MULTILINE)
    
    # Paragraphs (simple)
    if not is_inline:
        paragraphs = []
        current_para = []
        for line in md.split('\n'):
            if line.strip() == '':
                if current_para:
                    paragraphs.append('<p>' + ' '.join(current_para) + '</p>')
                    current_para = []
            elif line.startswith('<') or line.startswith('---'):
                if current_para:
                    paragraphs.append('<p>' + ' '.join(current_para) + '</p>')
                    current_para = []
                paragraphs.append(line)
            else:
                current_para.append(line.strip())
        if current_para:
            paragraphs.append('<p>' + ' '.join(current_para) + '</p>')
        md = '\n'.join(paragraphs)
    
    # Horizontal rules
    md = md.replace('---', '<hr class="section-divider">')
    
    # ASCII art / diagrams (preserve in pre blocks)
    def diagram_repl(m):
        return f'<pre class="diagram">{escape_html(m.group(1))}</pre>'
    
    # Detect multi-line ASCII diagrams (lines with box-drawing characters or specific patterns)
    ascii_pattern = r'```\n([┌┐└┘├┤┬┴┼─│┏┓┗┛┣┫┳┻╋━┃╔╗╚╝╠╣╦╩╬═║\s\w\/\-><│┌─┐│└┘│↑↓→←\(\)\[\]\{\}\|\\\/\.:,;\-_+=\*&\^%$#@!~`\'\"\?\s\w\d\n]+?)```'
    md = re.sub(ascii_pattern, diagram_repl, md, flags=re.DOTALL)
    
    return md


def generate_head():
    return f"""<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>Flint A2UI Component Registry — Interactive Showcase</title>
<script src="https://cdn.tailwindcss.com"></script>
<script src="https://unpkg.com/htmx.org@2.0.0"></script>
<script defer src="https://cdn.jsdelivr.net/npm/alpinejs@3.14.3/dist/cdn.min.js"></script>
<link href="https://fonts.googleapis.com/css2?family=Space+Grotesk:wght@400;500;600;700&family=Inter:wght@400;500;600;700&family=JetBrains+Mono:wght@400;500;600&display=swap" rel="stylesheet">
<script>
  tailwind.config = {{
    theme: {{
      extend: {{
        colors: {{
          'flint-bg': '{BG}',
          'flint-surface': '{SURFACE}',
          'flint-surface-2': '{SURFACE_ELEVATED}',
          'flint-surface-3': '{SURFACE_ELEVATED2}',
          'flint-accent': '{ACCENT}',
          'flint-cyan': '{CYAN}',
          'flint-green': '{GREEN}',
          'flint-yellow': '{YELLOW}',
          'flint-purple': '{PURPLE}',
          'flint-text': '{TEXT_PRIMARY}',
          'flint-text-2': '{TEXT_SECONDARY}',
          'flint-text-3': '{TEXT_MUTED}',
          'flint-border': '{BORDER}',
        }},
        fontFamily: {{
          'display': ['Space Grotesk', 'sans-serif'],
          'body': ['Inter', 'sans-serif'],
          'mono': ['JetBrains Mono', 'monospace'],
        }}
      }}
    }}
  }}
</script>
<style>
  :root {{
    --bg: {BG};
    --surface: {SURFACE};
    --surface-2: {SURFACE_ELEVATED};
    --surface-3: {SURFACE_ELEVATED2};
    --accent: {ACCENT};
    --cyan: {CYAN};
    --green: {GREEN};
    --yellow: {YELLOW};
    --purple: {PURPLE};
    --text: {TEXT_PRIMARY};
    --text-2: {TEXT_SECONDARY};
    --text-3: {TEXT_MUTED};
    --border: {BORDER};
  }}
  *::-webkit-scrollbar {{ width: 8px; height: 8px; }}
  *::-webkit-scrollbar-track {{ background: var(--bg); }}
  *::-webkit-scrollbar-thumb {{ background: var(--border); border-radius: 4px; }}
  *::-webkit-scrollbar-thumb:hover {{ background: var(--text-3); }}
  body {{
    font-family: 'Inter', sans-serif;
    background: var(--bg);
    color: var(--text);
  }}
  .font-display {{ font-family: 'Space Grotesk', sans-serif; }}
  .font-mono {{ font-family: 'JetBrains Mono', monospace; }}
  .code-block {{
    background: #0D1117;
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 1.25rem;
    overflow-x: auto;
    font-family: 'JetBrains Mono', monospace;
    font-size: 0.875rem;
    line-height: 1.6;
    color: #E2E8F0;
    margin: 1rem 0;
  }}
  .code-block .keyword {{ color: #FF7B72; }}
  .code-block .string {{ color: #A5D6FF; }}
  .code-block .comment {{ color: #8B949E; font-style: italic; }}
  .code-block .number {{ color: #79C0FF; }}
  .code-block .function {{ color: #D2A8FF; }}
  .inline-code {{
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: 4px;
    padding: 0.15em 0.4em;
    font-family: 'JetBrains Mono', monospace;
    font-size: 0.875em;
    color: var(--cyan);
  }}
  .data-table {{
    width: 100%;
    border-collapse: collapse;
    margin: 1rem 0;
    font-size: 0.875rem;
  }}
  .data-table thead tr {{ background: var(--surface-2); }}
  .data-table th {{
    text-align: left;
    padding: 0.75rem 1rem;
    border-bottom: 2px solid var(--accent);
    font-weight: 600;
    color: var(--text);
    font-family: 'Space Grotesk', sans-serif;
  }}
  .data-table td {{
    padding: 0.75rem 1rem;
    border-bottom: 1px solid var(--border);
    color: var(--text-2);
  }}
  .data-table tr:hover td {{ background: rgba(255,106,61,0.05); }}
  .diagram {{
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 1rem;
    overflow-x: auto;
    font-family: 'JetBrains Mono', monospace;
    font-size: 0.8rem;
    line-height: 1.4;
    color: var(--text-2);
    margin: 1rem 0;
  }}
  .section-divider {{
    border: none;
    height: 1px;
    background: linear-gradient(90deg, transparent, var(--accent), transparent);
    margin: 2rem 0;
  }}
  blockquote {{
    border-left: 3px solid var(--accent);
    background: var(--surface-2);
    padding: 0.75rem 1rem;
    margin: 1rem 0;
    border-radius: 0 8px 8px 0;
    color: var(--text-2);
    font-style: italic;
  }}
  /* Sidebar */
  .sidebar-link {{
    display: block;
    padding: 0.35rem 0.75rem;
    border-radius: 6px;
    color: var(--text-2);
    font-size: 0.85rem;
    transition: all 0.2s;
    text-decoration: none;
  }}
  .sidebar-link:hover {{ background: var(--surface-2); color: var(--accent); }}
  .sidebar-link.h2 {{ font-weight: 600; font-size: 0.9rem; color: var(--text); margin-top: 0.5rem; }}
  .sidebar-link.h3 {{ padding-left: 1.5rem; font-size: 0.8rem; color: var(--text-3); }}
  .sidebar-link.active {{ background: rgba(255,106,61,0.15); color: var(--accent); }}
  
  /* HTMX transitions */
  .htmx-added {{ opacity: 0; transform: translateY(10px); }}
  .htmx-settling {{ opacity: 1; transform: translateY(0); transition: all 0.3s ease; }}
  .fade-in {{ animation: fadeIn 0.4s ease; }}
  @keyframes fadeIn {{ from {{ opacity: 0; transform: translateY(8px); }} to {{ opacity: 1; transform: translateY(0); }} }}
  .slide-in {{ animation: slideIn 0.3s ease; }}
  @keyframes slideIn {{ from {{ opacity: 0; transform: translateX(-20px); }} to {{ opacity: 1; transform: translateX(0); }} }}
  .scale-in {{ animation: scaleIn 0.25s ease; }}
  @keyframes scaleIn {{ from {{ opacity: 0; transform: scale(0.95); }} to {{ opacity: 1; transform: scale(1); }} }}
  
  /* Component showcase cards */
  .showcase-card {{
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: 12px;
    padding: 1.5rem;
    margin: 1rem 0;
  }}
  .showcase-card:hover {{ border-color: rgba(255,106,61,0.3); }}
  .showcase-label {{
    font-family: 'Space Grotesk', sans-serif;
    font-size: 0.75rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--accent);
    margin-bottom: 0.75rem;
  }}
  
  /* Alpine tabs */
  .tab-btn {{
    padding: 0.5rem 1rem;
    border-radius: 6px;
    font-size: 0.85rem;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.2s;
    border: 1px solid transparent;
    background: transparent;
    color: var(--text-2);
  }}
  .tab-btn.active {{ background: rgba(255,106,61,0.15); border-color: var(--accent); color: var(--accent); }}
  .tab-btn:hover {{ background: var(--surface-3); }}
  
  /* Accordion */
  .accordion-item {{ border: 1px solid var(--border); border-radius: 8px; margin-bottom: 0.5rem; overflow: hidden; }}
  .accordion-header {{
    padding: 0.75rem 1rem;
    background: var(--surface-2);
    cursor: pointer;
    display: flex;
    justify-content: space-between;
    align-items: center;
    font-weight: 500;
    transition: background 0.2s;
  }}
  .accordion-header:hover {{ background: var(--surface-3); }}
  .accordion-body {{
    padding: 0 1rem;
    background: var(--surface);
    max-height: 0;
    overflow: hidden;
    transition: max-height 0.3s ease, padding 0.3s ease;
  }}
  .accordion-item.open .accordion-body {{ padding: 1rem; max-height: 500px; }}
  .accordion-icon {{ transition: transform 0.2s; }}
  .accordion-item.open .accordion-icon {{ transform: rotate(180deg); }}
  
  /* Modal */
  .modal-backdrop {{
    position: fixed;
    inset: 0;
    background: rgba(11,15,20,0.85);
    backdrop-filter: blur(4px);
    z-index: 50;
    display: flex;
    align-items: center;
    justify-content: center;
  }}
  .modal-content {{
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: 16px;
    padding: 2rem;
    max-width: 500px;
    width: 90%;
    box-shadow: 0 25px 50px -12px rgba(0,0,0,0.5);
  }}
  
  /* Dropdown */
  .dropdown-menu {{
    position: absolute;
    top: 100%;
    right: 0;
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 0.5rem 0;
    min-width: 200px;
    box-shadow: 0 10px 40px rgba(0,0,0,0.4);
    z-index: 40;
  }}
  .dropdown-item {{
    padding: 0.5rem 1rem;
    font-size: 0.85rem;
    color: var(--text-2);
    cursor: pointer;
    transition: all 0.15s;
  }}
  .dropdown-item:hover {{ background: var(--surface-3); color: var(--accent); }}
  
  /* Form validation */
  .input-field {{
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 0.6rem 0.9rem;
    color: var(--text);
    font-size: 0.9rem;
    transition: border-color 0.2s, box-shadow 0.2s;
    width: 100%;
  }}
  .input-field:focus {{ outline: none; border-color: var(--accent); box-shadow: 0 0 0 3px rgba(255,106,61,0.15); }}
  .input-field.error {{ border-color: #EF4444; }}
  .input-field.success {{ border-color: var(--green); }}
  .input-hint {{ font-size: 0.75rem; margin-top: 0.25rem; color: var(--text-3); }}
  .input-hint.error {{ color: #EF4444; }}
  
  /* Status badges */
  .badge {{
    display: inline-flex;
    align-items: center;
    gap: 0.35rem;
    padding: 0.25rem 0.6rem;
    border-radius: 9999px;
    font-size: 0.75rem;
    font-weight: 600;
  }}
  .badge-stable {{ background: rgba(79,209,139,0.15); color: var(--green); }}
  .badge-beta {{ background: rgba(255,209,102,0.15); color: var(--yellow); }}
  .badge-draft {{ background: rgba(148,163,184,0.15); color: var(--text-3); }}
  .badge-accent {{ background: rgba(255,106,61,0.15); color: var(--accent); }}
  .badge-cyan {{ background: rgba(52,207,230,0.15); color: var(--cyan); }}
  
  /* Progress */
  .progress-bar {{
    height: 8px;
    background: var(--surface-3);
    border-radius: 4px;
    overflow: hidden;
  }}
  .progress-bar-fill {{
    height: 100%;
    background: linear-gradient(90deg, var(--accent), var(--cyan));
    border-radius: 4px;
    transition: width 0.5s ease;
  }}
  
  /* Kanban */
  .kanban-column {{
    background: var(--surface);
    border-radius: 12px;
    padding: 1rem;
    min-width: 260px;
  }}
  .kanban-card {{
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 0.75rem;
    margin-bottom: 0.5rem;
    cursor: grab;
    transition: all 0.2s;
  }}
  .kanban-card:hover {{ border-color: rgba(255,106,61,0.3); transform: translateY(-2px); }}
  
  /* Chart bars */
  .chart-bar {{
    background: linear-gradient(180deg, var(--accent), rgba(255,106,61,0.5));
    border-radius: 4px 4px 0 0;
    transition: height 0.5s ease;
    min-height: 4px;
  }}
  
  /* Toast notifications */
  .toast {{
    position: fixed;
    bottom: 1.5rem;
    right: 1.5rem;
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: 10px;
    padding: 1rem 1.25rem;
    box-shadow: 0 10px 40px rgba(0,0,0,0.4);
    display: flex;
    align-items: center;
    gap: 0.75rem;
    z-index: 60;
  }}
  
  /* Loading spinner */
  .spinner {{
    width: 20px;
    height: 20px;
    border: 2px solid var(--border);
    border-top-color: var(--accent);
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }}
  @keyframes spin {{ to {{ transform: rotate(360deg); }} }}
  
  /* Glow effects */
  .glow-accent {{ box-shadow: 0 0 20px rgba(255,106,61,0.2); }}
  .glow-cyan {{ box-shadow: 0 0 20px rgba(52,207,230,0.2); }}
  
  /* Smooth scroll */
  html {{ scroll-behavior: smooth; }}
  
  /* Section anchors */
  h1, h2, h3 {{ scroll-margin-top: 80px; }}
  
  /* Mobile responsive */
  @media (max-width: 1024px) {{
    .sidebar {{ display: none; }}
    .main-content {{ margin-left: 0; }}
  }}
</style>
</head>
"""

def generate_sidebar(sections):
    """Generate sidebar navigation from all h2/h3 headings."""
    items = []
    for sec in sections:
        if sec["level"] in (2, 3):
            slug = slugify(sec["title"])
            level_class = "h2" if sec["level"] == 2 else "h3"
            items.append(f'<a href="#{slug}" class="sidebar-link {level_class}" @click="open = false">{escape_html(sec["title"])}</a>')
    
    nav_links = '\n'.join(items)
    
    return f"""
<aside class="sidebar fixed left-0 top-0 h-screen w-72 bg-flint-surface border-r border-flint-border overflow-y-auto z-40"
       x-data="{{ open: false }}"
       :class="open ? 'block' : 'hidden lg:block'">
  <div class="p-5 border-b border-flint-border">
    <div class="flex items-center gap-3 mb-1">
      <div class="w-8 h-8 rounded-lg bg-gradient-to-br from-flint-accent to-flint-cyan flex items-center justify-center font-bold text-white font-display">F</div>
      <h1 class="font-display font-bold text-lg text-white">Flint A2UI</h1>
    </div>
    <p class="text-xs text-flint-text-3 font-mono">Component Registry Spec</p>
  </div>
  <nav class="p-3 space-y-0.5">
    {nav_links}
  </nav>
  <div class="p-4 border-t border-flint-border mt-4">
    <div class="flex items-center gap-2 text-xs text-flint-text-3">
      <span class="w-2 h-2 rounded-full bg-flint-green animate-pulse"></span>
      Interactive Demo
    </div>
  </div>
</aside>
<button @click="open = !open" class="lg:hidden fixed top-4 left-4 z-50 p-2 bg-flint-surface-2 rounded-lg border border-flint-border text-white"
        x-data="{{ open: false }}" x-cloak>
  <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 6h16M4 12h16M4 18h16"/></svg>
</button>
"""

def generate_header():
    return """
<header class="fixed top-0 right-0 left-0 lg:left-72 h-16 bg-flint-bg/90 backdrop-blur-md border-b border-flint-border z-30 flex items-center justify-between px-6">
  <div class="flex items-center gap-4">
    <h2 class="font-display font-semibold text-white text-lg">A2UI Component Registry</h2>
    <span class="badge badge-accent">v1.0</span>
  </div>
  <div class="flex items-center gap-3">
    <div class="hidden md:flex items-center gap-2 text-xs text-flint-text-3 font-mono">
      <span class="w-2 h-2 rounded-full bg-flint-green"></span>
      RFC-FORGE-A2UI-001
    </div>
    <div class="hidden md:flex items-center gap-2 text-xs text-flint-text-3 font-mono">
      <span class="w-2 h-2 rounded-full bg-flint-cyan"></span>
      June 2026
    </div>
  </div>
</header>
"""


def generate_showcase_for_section(title):
    """Generate interactive showcase components relevant to the section."""
    showcases = {
        "Executive Summary": lambda: generate_executive_summary_showcase(),
        "Philosophy and Design Principles": lambda: generate_philosophy_showcase(),
        "Architecture Overview": lambda: generate_architecture_showcase(),
        "Base A2UI Component Primitives": lambda: generate_primitives_showcase(),
        "Registry Schema: Metadata-Driven, Extensible Storage": lambda: generate_schema_showcase(),
        "Application Metadata Model": lambda: generate_app_model_showcase(),
        "Database Schema Integration": lambda: generate_db_integration_showcase(),
        "Design System Integration and Open Design Bridge": lambda: generate_design_system_showcase(),
        "REST API Specification": lambda: generate_api_showcase(),
        "A2A Task Definitions": lambda: generate_a2a_showcase(),
        "MCP Server Tools": lambda: generate_mcp_showcase(),
        "Event-Driven Dynamic Component Construction": lambda: generate_event_showcase(),
        "Security and Permissions": lambda: generate_security_showcase(),
        "Implementation Roadmap": lambda: generate_roadmap_showcase(),
        "Integration with Flint Ecosystem": lambda: generate_integration_showcase(),
        "Recommendations and Future Work": lambda: generate_future_showcase(),
    }
    
    for key in showcases:
        if key.lower() in title.lower():
            return showcases[key]()
    
    # Default: generic interactive demo
    return generate_generic_showcase(title)


def generate_executive_summary_showcase():
    return """
<div class="showcase-card">
  <div class="showcase-label">Interactive Demo — Key Differentiators</div>
  <div class="grid md:grid-cols-2 gap-4 mt-4">
    <div class="bg-flint-bg border border-flint-border rounded-lg p-4 hover:border-flint-accent/30 transition-colors">
      <div class="flex items-center gap-2 mb-2">
        <div class="w-8 h-8 rounded-lg bg-flint-accent/15 flex items-center justify-center text-flint-accent text-sm">DB</div>
        <span class="font-semibold text-sm">Database + JSONB + Embeddings</span>
      </div>
      <p class="text-xs text-flint-text-2">Components live in PostgreSQL with vector search — not static files.</p>
    </div>
    <div class="bg-flint-bg border border-flint-border rounded-lg p-4 hover:border-flint-cyan/30 transition-colors">
      <div class="flex items-center gap-2 mb-2">
        <div class="w-8 h-8 rounded-lg bg-flint-cyan/15 flex items-center justify-center text-flint-cyan text-sm">AI</div>
        <span class="font-semibold text-sm">Semantic Vector Search</span>
      </div>
      <p class="text-xs text-flint-text-2">Natural language discovery with pgvector + HNSW indexing.</p>
    </div>
    <div class="bg-flint-bg border border-flint-border rounded-lg p-4 hover:border-flint-green/30 transition-colors">
      <div class="flex items-center gap-2 mb-2">
        <div class="w-8 h-8 rounded-lg bg-flint-green/15 flex items-center justify-center text-flint-green text-sm">⚡</div>
        <span class="font-semibold text-sm">Event-Driven Assembly</span>
      </div>
      <p class="text-xs text-flint-text-2">Real-time component trees assembled from agent events via Iggy.</p>
    </div>
    <div class="bg-flint-bg border border-flint-border rounded-lg p-4 hover:border-flint-purple/30 transition-colors">
      <div class="flex items-center gap-2 mb-2">
        <div class="w-8 h-8 rounded-lg bg-flint-purple/15 flex items-center justify-center text-flint-purple text-sm">🔒</div>
        <span class="font-semibold text-sm">JWT-Scoped Access</span>
      </div>
      <p class="text-xs text-flint-text-2">Component visibility controlled by RLS + Kratos/Keto claims.</p>
    </div>
  </div>
</div>
"""

def generate_philosophy_showcase():
    return """
<div class="showcase-card" x-data="{ active: 'metadata' }">
  <div class="showcase-label">Interactive Demo — Design Principles</div>
  <div class="flex gap-2 mb-4 flex-wrap">
    <button class="tab-btn" :class="active === 'metadata' ? 'active' : ''" @click="active = 'metadata'">Metadata as Source</button>
    <button class="tab-btn" :class="active === 'constrained' ? 'active' : ''" @click="active = 'constrained'">Constrained by Default</button>
    <button class="tab-btn" :class="active === 'jsonb' ? 'active' : ''" @click="active = 'jsonb'">JSONB Extensibility</button>
    <button class="tab-btn" :class="active === 'token' ? 'active' : ''" @click="active = 'token'">Token-Aware</button>
  </div>
  <div class="bg-flint-bg border border-flint-border rounded-lg p-4 min-h-[120px]">
    <div x-show="active === 'metadata'" x-transition:enter="fade-in" x-transition:enter-start="opacity-0" x-transition:enter-end="opacity-100">
      <h4 class="font-semibold text-flint-accent mb-2">Every component is queryable metadata</h4>
      <p class="text-sm text-flint-text-2">Agents query the registry through SQL, REST, A2A tasks, MCP tools, and semantic search. No static files — everything is in PostgreSQL.</p>
    </div>
    <div x-show="active === 'constrained'" x-transition:enter="fade-in" x-transition:enter-start="opacity-0" x-transition:enter-end="opacity-100" style="display:none">
      <h4 class="font-semibold text-flint-cyan mb-2">Pre-approved components only</h4>
      <p class="text-sm text-flint-text-2">Agents select from pre-registered components. Raw HTML/JS is sandboxed in iframes with <code class="inline-code">sandbox="allow-scripts"</code> only when explicitly enabled.</p>
    </div>
    <div x-show="active === 'jsonb'" x-transition:enter="fade-in" x-transition:enter-start="opacity-0" x-transition:enter-end="opacity-100" style="display:none">
      <h4 class="font-semibold text-flint-green mb-2">Schema evolution without migrations</h4>
      <p class="text-sm text-flint-text-2">Component schemas, prop definitions, and design tokens are stored in JSONB columns so the schema evolves without migrations.</p>
    </div>
    <div x-show="active === 'token'" x-transition:enter="fade-in" x-transition:enter-start="opacity-0" x-transition:enter-end="opacity-100" style="display:none">
      <h4 class="font-semibold text-flint-purple mb-2">Design tokens resolved at query time</h4>
      <p class="text-sm text-flint-text-2">Tokens are resolved based on application, tenant, and user preferences. No hardcoded colors — everything is dynamic.</p>
    </div>
  </div>
</div>
"""

def generate_architecture_showcase():
    return """
<div class="showcase-card" x-data="{ layer: 'layer1' }">
  <div class="showcase-label">Interactive Demo — Registry Architecture</div>
  <div class="flex gap-2 mb-4">
    <button class="tab-btn" :class="layer === 'layer1' ? 'active' : ''" @click="layer = 'layer1'">Layer 1: Database</button>
    <button class="tab-btn" :class="layer === 'layer2' ? 'active' : ''" @click="layer = 'layer2'">Layer 2: Rust Engine</button>
    <button class="tab-btn" :class="layer === 'layer3' ? 'active' : ''" @click="layer = 'layer3'">Layer 3: Protocols</button>
  </div>
  <div class="bg-flint-bg border border-flint-border rounded-lg p-4 space-y-3">
    <div x-show="layer === 'layer1'" x-transition:enter="slide-in" x-transition:enter-start="opacity-0 translate-x-[-20px]" x-transition:enter-end="opacity-100 translate-x-0">
      <div class="flex items-center gap-2 mb-2"><div class="w-3 h-3 rounded-full bg-flint-accent"></div><span class="font-semibold text-sm">PostgreSQL 18 — flint_a2ui schema</span></div>
      <div class="grid grid-cols-2 gap-2 text-xs">
        <div class="bg-flint-surface-2 rounded p-2 border border-flint-border">components (JSONB)</div>
        <div class="bg-flint-surface-2 rounded p-2 border border-flint-border">applications (JSONB)</div>
        <div class="bg-flint-surface-2 rounded p-2 border border-flint-border">design_systems (JSONB)</div>
        <div class="bg-flint-surface-2 rounded p-2 border border-flint-border">embeddings (vector)</div>
        <div class="bg-flint-surface-2 rounded p-2 border border-flint-border">schemas (JSONB)</div>
        <div class="bg-flint-surface-2 rounded p-2 border border-flint-border">bindings (JSONB)</div>
      </div>
    </div>
    <div x-show="layer === 'layer2'" x-transition:enter="slide-in" x-transition:enter-start="opacity-0 translate-x-[-20px]" x-transition:enter-end="opacity-100 translate-x-0" style="display:none">
      <div class="flex items-center gap-2 mb-2"><div class="w-3 h-3 rounded-full bg-flint-cyan"></div><span class="font-semibold text-sm">Rust Reflection Engine</span></div>
      <div class="grid grid-cols-2 gap-2 text-xs">
        <div class="bg-flint-surface-2 rounded p-2 border border-flint-border">ArcSwap IR (compiled)</div>
        <div class="bg-flint-surface-2 rounded p-2 border border-flint-border">REST Router (Axum)</div>
        <div class="bg-flint-surface-2 rounded p-2 border border-flint-border">A2A Task Registry</div>
        <div class="bg-flint-surface-2 rounded p-2 border border-flint-border">MCP Tool Server</div>
        <div class="bg-flint-surface-2 rounded p-2 border border-flint-border">Vector Search (pgvector)</div>
        <div class="bg-flint-surface-2 rounded p-2 border border-flint-border">Token Resolver</div>
      </div>
    </div>
    <div x-show="layer === 'layer3'" x-transition:enter="slide-in" x-transition:enter-start="opacity-0 translate-x-[-20px]" x-transition:enter-end="opacity-100 translate-x-0" style="display:none">
      <div class="flex items-center gap-2 mb-2"><div class="w-3 h-3 rounded-full bg-flint-green"></div><span class="font-semibold text-sm">Protocol Surfaces</span></div>
      <div class="grid grid-cols-2 gap-2 text-xs">
        <div class="bg-flint-surface-2 rounded p-2 border border-flint-border">REST API (HTTP)</div>
        <div class="bg-flint-surface-2 rounded p-2 border border-flint-border">A2A Task (HTTP)</div>
        <div class="bg-flint-surface-2 rounded p-2 border border-flint-border">A2UI (JSON)</div>
        <div class="bg-flint-surface-2 rounded p-2 border border-flint-border">AG-UI (SSE)</div>
        <div class="bg-flint-surface-2 rounded p-2 border border-flint-border col-span-2">MCP (HTTP)</div>
      </div>
    </div>
  </div>
</div>
"""

def generate_primitives_showcase():
    return """
<div class="showcase-card" x-data="{ category: 'layout' }">
  <div class="showcase-label">Interactive Demo — A2UI Primitives</div>
  <div class="flex gap-2 mb-4 flex-wrap">
    <template x-for="cat in ['layout','data-display','input','action','agent','navigation']">
      <button class="tab-btn" :class="category === cat ? 'active' : ''" @click="category = cat" x-text="cat.replace('-', ' ')"></button>
    </template>
  </div>
  <div class="bg-flint-bg border border-flint-border rounded-lg p-4">
    <div x-show="category === 'layout'">
      <div class="grid grid-cols-2 md:grid-cols-4 gap-3">
        <div class="bg-flint-surface-2 rounded-lg p-3 border border-flint-border hover:border-flint-accent/30 transition">
          <div class="text-xs font-semibold text-flint-accent mb-1">Stack</div>
          <div class="space-y-1"><div class="h-2 bg-flint-accent/30 rounded"></div><div class="h-2 bg-flint-cyan/30 rounded"></div><div class="h-2 bg-flint-green/30 rounded"></div></div>
        </div>
        <div class="bg-flint-surface-2 rounded-lg p-3 border border-flint-border hover:border-flint-accent/30 transition">
          <div class="text-xs font-semibold text-flint-accent mb-1">Card</div>
          <div class="border border-flint-border rounded p-2"><div class="h-1.5 bg-flint-accent/40 rounded mb-1 w-3/4"></div><div class="h-1.5 bg-flint-text-3/20 rounded"></div></div>
        </div>
        <div class="bg-flint-surface-2 rounded-lg p-3 border border-flint-border hover:border-flint-accent/30 transition">
          <div class="text-xs font-semibold text-flint-accent mb-1">Grid</div>
          <div class="grid grid-cols-2 gap-1"><div class="h-3 bg-flint-accent/20 rounded"></div><div class="h-3 bg-flint-cyan/20 rounded"></div><div class="h-3 bg-flint-green/20 rounded"></div><div class="h-3 bg-flint-purple/20 rounded"></div></div>
        </div>
        <div class="bg-flint-surface-2 rounded-lg p-3 border border-flint-border hover:border-flint-accent/30 transition">
          <div class="text-xs font-semibold text-flint-accent mb-1">Split</div>
          <div class="flex gap-1 h-8"><div class="w-1/3 bg-flint-accent/20 rounded"></div><div class="flex-1 bg-flint-cyan/20 rounded"></div></div>
        </div>
        <div class="bg-flint-surface-2 rounded-lg p-3 border border-flint-border hover:border-flint-accent/30 transition">
          <div class="text-xs font-semibold text-flint-accent mb-1">Tabs</div>
          <div class="flex gap-1"><div class="flex-1 h-4 bg-flint-accent/30 rounded"></div><div class="flex-1 h-4 bg-flint-text-3/10 rounded"></div></div>
        </div>
        <div class="bg-flint-surface-2 rounded-lg p-3 border border-flint-border hover:border-flint-accent/30 transition">
          <div class="text-xs font-semibold text-flint-accent mb-1">Accordion</div>
          <div class="space-y-1"><div class="h-3 bg-flint-accent/20 rounded flex items-center justify-end px-1"><div class="w-2 h-2 border-r border-b border-flint-accent rotate-45"></div></div></div>
        </div>
        <div class="bg-flint-surface-2 rounded-lg p-3 border border-flint-border hover:border-flint-accent/30 transition">
          <div class="text-xs font-semibold text-flint-accent mb-1">Modal</div>
          <div class="relative h-8 bg-flint-bg rounded border border-flint-border flex items-center justify-center"><div class="absolute inset-0 bg-black/30 rounded flex items-center justify-center"><div class="w-8 h-5 bg-flint-surface-2 rounded border border-flint-accent/30"></div></div></div>
        </div>
        <div class="bg-flint-surface-2 rounded-lg p-3 border border-flint-border hover:border-flint-accent/30 transition">
          <div class="text-xs font-semibold text-flint-accent mb-1">Drawer</div>
          <div class="relative h-8 bg-flint-bg rounded border border-flint-border flex items-center justify-center overflow-hidden"><div class="absolute right-0 top-0 bottom-0 w-1/2 bg-flint-surface-2 border-l border-flint-accent/30"></div></div>
        </div>
      </div>
    </div>
    <div x-show="category === 'data-display'" style="display:none">
      <div class="grid grid-cols-2 md:grid-cols-3 gap-3">
        <div class="bg-flint-surface-2 rounded-lg p-3 border border-flint-border"><div class="text-xs font-semibold text-flint-cyan mb-2">Table</div>
          <div class="space-y-1 text-xs"><div class="flex gap-1"><div class="flex-1 h-2 bg-flint-accent/20 rounded"></div><div class="flex-1 h-2 bg-flint-cyan/20 rounded"></div><div class="flex-1 h-2 bg-flint-green/20 rounded"></div></div><div class="flex gap-1"><div class="flex-1 h-2 bg-flint-text-3/10 rounded"></div><div class="flex-1 h-2 bg-flint-text-3/10 rounded"></div><div class="flex-1 h-2 bg-flint-text-3/10 rounded"></div></div></div>
        </div>
        <div class="bg-flint-surface-2 rounded-lg p-3 border border-flint-border"><div class="text-xs font-semibold text-flint-cyan mb-2">DataGrid</div>
          <div class="text-xs text-flint-text-3">Advanced table + pagination</div></div>
        <div class="bg-flint-surface-2 rounded-lg p-3 border border-flint-border"><div class="text-xs font-semibold text-flint-cyan mb-2">Kanban</div>
          <div class="flex gap-1 h-12"><div class="flex-1 bg-flint-accent/10 rounded border border-flint-accent/20"></div><div class="flex-1 bg-flint-cyan/10 rounded border border-flint-cyan/20"></div><div class="flex-1 bg-flint-green/10 rounded border border-flint-green/20"></div></div>
        </div>
        <div class="bg-flint-surface-2 rounded-lg p-3 border border-flint-border"><div class="text-xs font-semibold text-flint-cyan mb-2">Chart</div>
          <div class="flex items-end gap-1 h-12"><div class="flex-1 chart-bar" style="height:60%"></div><div class="flex-1 chart-bar" style="height:80%"></div><div class="flex-1 chart-bar" style="height:40%"></div><div class="flex-1 chart-bar" style="height:90%"></div></div>
        </div>
        <div class="bg-flint-surface-2 rounded-lg p-3 border border-flint-border"><div class="text-xs font-semibold text-flint-cyan mb-2">Metric</div>
          <div class="text-2xl font-bold text-flint-accent">1,247</div><div class="text-xs text-flint-green">↑ 12%</div></div>
        <div class="bg-flint-surface-2 rounded-lg p-3 border border-flint-border"><div class="text-xs font-semibold text-flint-cyan mb-2">Badge</div>
          <div class="flex gap-2"><span class="badge badge-stable">Stable</span><span class="badge badge-beta">Beta</span></div></div>
      </div>
    </div>
    <div x-show="category === 'input'" style="display:none">
      <div class="grid grid-cols-2 md:grid-cols-3 gap-3">
        <div class="bg-flint-surface-2 rounded-lg p-3 border border-flint-border"><div class="text-xs font-semibold text-flint-green mb-2">TextField</div><input type="text" class="input-field text-xs" placeholder="Enter text..." readonly></div>
        <div class="bg-flint-surface-2 rounded-lg p-3 border border-flint-border"><div class="text-xs font-semibold text-flint-green mb-2">Number</div><input type="number" class="input-field text-xs" value="42" readonly></div>
        <div class="bg-flint-surface-2 rounded-lg p-3 border border-flint-border"><div class="text-xs font-semibold text-flint-green mb-2">Switch</div><div class="flex items-center gap-2"><div class="w-10 h-5 bg-flint-accent rounded-full relative"><div class="absolute right-0.5 top-0.5 w-4 h-4 bg-white rounded-full"></div></div><span class="text-xs">On</span></div></div>
        <div class="bg-flint-surface-2 rounded-lg p-3 border border-flint-border"><div class="text-xs font-semibold text-flint-green mb-2">Select</div><select class="input-field text-xs" disabled><option>Option A</option></select></div>
        <div class="bg-flint-surface-2 rounded-lg p-3 border border-flint-border"><div class="text-xs font-semibold text-flint-green mb-2">DatePicker</div><input type="date" class="input-field text-xs" readonly></div>
        <div class="bg-flint-surface-2 rounded-lg p-3 border border-flint-border"><div class="text-xs font-semibold text-flint-green mb-2">FileUpload</div><div class="border-2 border-dashed border-flint-border rounded-lg p-3 text-center text-xs text-flint-text-3">Drop files here</div></div>
      </div>
    </div>
    <div x-show="category === 'action'" style="display:none">
      <div class="grid grid-cols-2 md:grid-cols-3 gap-3">
        <div class="bg-flint-surface-2 rounded-lg p-3 border border-flint-border flex items-center justify-center"><button class="px-4 py-2 bg-flint-accent text-white rounded-lg text-sm font-semibold hover:opacity-90 transition">Button</button></div>
        <div class="bg-flint-surface-2 rounded-lg p-3 border border-flint-border flex items-center justify-center"><button class="px-3 py-2 bg-flint-surface-3 border border-flint-border text-flint-text rounded-lg text-sm hover:border-flint-accent/30 transition">IconButton</button></div>
        <div class="bg-flint-surface-2 rounded-lg p-3 border border-flint-border flex items-center justify-center"><div class="flex rounded-lg overflow-hidden border border-flint-border"><button class="px-3 py-1.5 bg-flint-accent text-white text-xs">Yes</button><button class="px-3 py-1.5 bg-flint-surface text-flint-text-2 text-xs">No</button></div></div>
      </div>
    </div>
    <div x-show="category === 'agent'" style="display:none">
      <div class="grid grid-cols-2 md:grid-cols-3 gap-3">
        <div class="bg-flint-surface-2 rounded-lg p-3 border border-flint-border"><div class="text-xs font-semibold text-flint-purple mb-2">AgentChat</div><div class="space-y-2"><div class="flex gap-2"><div class="w-6 h-6 rounded-full bg-flint-accent/20 flex items-center justify-center text-xs">A</div><div class="bg-flint-surface rounded p-2 text-xs flex-1">How can I help?</div></div><div class="flex gap-2 flex-row-reverse"><div class="w-6 h-6 rounded-full bg-flint-cyan/20 flex items-center justify-center text-xs">U</div><div class="bg-flint-accent/10 rounded p-2 text-xs flex-1">Build a component...</div></div></div></div>
        <div class="bg-flint-surface-2 rounded-lg p-3 border border-flint-border"><div class="text-xs font-semibold text-flint-purple mb-2">ToolCall</div><div class="bg-flint-bg rounded p-2 border border-flint-border text-xs"><div class="flex items-center gap-2 mb-1"><div class="spinner"></div><span>Generating grid...</span></div><div class="progress-bar"><div class="progress-bar-fill" style="width:65%"></div></div></div></div>
        <div class="bg-flint-surface-2 rounded-lg p-3 border border-flint-border"><div class="text-xs font-semibold text-flint-purple mb-2">StreamingText</div><div class="text-xs font-mono text-flint-cyan">The component registry...</div></div>
      </div>
    </div>
    <div x-show="category === 'navigation'" style="display:none">
      <div class="grid grid-cols-2 md:grid-cols-3 gap-3">
        <div class="bg-flint-surface-2 rounded-lg p-3 border border-flint-border"><div class="text-xs font-semibold text-flint-yellow mb-2">Breadcrumb</div><div class="flex items-center gap-1 text-xs text-flint-text-3"><span class="hover:text-flint-accent cursor-pointer">Home</span><span>/</span><span class="hover:text-flint-accent cursor-pointer">Apps</span><span>/</span><span class="text-flint-text">Registry</span></div></div>
        <div class="bg-flint-surface-2 rounded-lg p-3 border border-flint-border"><div class="text-xs font-semibold text-flint-yellow mb-2">Stepper</div><div class="flex items-center gap-2"><div class="w-6 h-6 rounded-full bg-flint-accent text-white flex items-center justify-center text-xs font-bold">1</div><div class="h-px flex-1 bg-flint-border"></div><div class="w-6 h-6 rounded-full bg-flint-border text-flint-text-3 flex items-center justify-center text-xs">2</div></div></div>
        <div class="bg-flint-surface-2 rounded-lg p-3 border border-flint-border"><div class="text-xs font-semibold text-flint-yellow mb-2">Pagination</div><div class="flex items-center gap-1"><button class="w-6 h-6 rounded border border-flint-border text-xs hover:bg-flint-surface-3">&lt;</button><button class="w-6 h-6 rounded bg-flint-accent text-white text-xs">1</button><button class="w-6 h-6 rounded border border-flint-border text-xs hover:bg-flint-surface-3">2</button><button class="w-6 h-6 rounded border border-flint-border text-xs hover:bg-flint-surface-3">&gt;</button></div></div>
      </div>
    </div>
  </div>
</div>
"""

def generate_schema_showcase():
    return """
<div class="showcase-card" x-data="{ table: 'components' }">
  <div class="showcase-label">Interactive Demo — Registry Schema Explorer</div>
  <div class="flex gap-2 mb-4">
    <button class="tab-btn" :class="table === 'components' ? 'active' : ''" @click="table = 'components'">components</button>
    <button class="tab-btn" :class="table === 'applications' ? 'active' : ''" @click="table = 'applications'">applications</button>
    <button class="tab-btn" :class="table === 'design_systems' ? 'active' : ''" @click="table = 'design_systems'">design_systems</button>
    <button class="tab-btn" :class="table === 'embeddings' ? 'active' : ''" @click="table = 'embeddings'">embeddings</button>
  </div>
  <div class="bg-flint-bg border border-flint-border rounded-lg p-4 font-mono text-xs">
    <div x-show="table === 'components'">
      <div class="text-flint-accent mb-2">-- flint_a2ui.components</div>
      <div class="space-y-1 text-flint-text-2">
        <div><span class="text-flint-cyan">id</span> uuid PRIMARY KEY</div>
        <div><span class="text-flint-cyan">slug</span> text NOT NULL UNIQUE</div>
        <div><span class="text-flint-cyan">name</span> text NOT NULL</div>
        <div><span class="text-flint-cyan">description</span> text NOT NULL</div>
        <div><span class="text-flint-cyan">category</span> text CHECK (...)</div>
        <div><span class="text-flint-cyan">primitive_type</span> text NOT NULL</div>
        <div><span class="text-flint-cyan">version</span> text NOT NULL DEFAULT '1.0.0'</div>
        <div><span class="text-flint-cyan">status</span> text DEFAULT 'draft'</div>
        <div><span class="text-flint-cyan">schema</span> jsonb NOT NULL DEFAULT '{{}}'</div>
        <div><span class="text-flint-cyan">ui_hints</span> jsonb NOT NULL DEFAULT '{{}}'</div>
        <div><span class="text-flint-cyan">platforms</span> jsonb NOT NULL DEFAULT '{{}}'</div>
        <div><span class="text-flint-cyan">search_vector</span> tsvector</div>
      </div>
    </div>
    <div x-show="table === 'applications'" style="display:none">
      <div class="text-flint-accent mb-2">-- flint_a2ui.applications</div>
      <div class="space-y-1 text-flint-text-2">
        <div><span class="text-flint-cyan">id</span> uuid PRIMARY KEY</div>
        <div><span class="text-flint-cyan">slug</span> text NOT NULL UNIQUE</div>
        <div><span class="text-flint-cyan">app_type</span> text CHECK (system, platform, user, template, integration)</div>
        <div><span class="text-flint-cyan">owner_id</span> uuid NOT NULL</div>
        <div><span class="text-flint-cyan">config</span> jsonb NOT NULL DEFAULT '{{}}'</div>
        <div><span class="text-flint-cyan">design_system_id</span> uuid REFERENCES design_systems</div>
        <div><span class="text-flint-cyan">jwt_claims_template</span> jsonb NOT NULL DEFAULT '{{}}'</div>
      </div>
    </div>
    <div x-show="table === 'design_systems'" style="display:none">
      <div class="text-flint-accent mb-2">-- flint_a2ui.design_systems</div>
      <div class="space-y-1 text-flint-text-2">
        <div><span class="text-flint-cyan">id</span> uuid PRIMARY KEY</div>
        <div><span class="text-flint-cyan">slug</span> text NOT NULL UNIQUE</div>
        <div><span class="text-flint-cyan">odsf_version</span> text DEFAULT '0.1'</div>
        <div><span class="text-flint-cyan">source_url</span> text</div>
        <div><span class="text-flint-cyan">design_md</span> text</div>
        <div><span class="text-flint-cyan">tokens</span> jsonb NOT NULL DEFAULT '{{}}'</div>
        <div><span class="text-flint-cyan">component_tokens</span> jsonb NOT NULL DEFAULT '{{}}'</div>
        <div><span class="text-flint-cyan">css_output</span> jsonb NOT NULL DEFAULT '{{}}'</div>
      </div>
    </div>
    <div x-show="table === 'embeddings'" style="display:none">
      <div class="text-flint-accent mb-2">-- flint_a2ui.embeddings</div>
      <div class="space-y-1 text-flint-text-2">
        <div><span class="text-flint-cyan">id</span> uuid PRIMARY KEY</div>
        <div><span class="text-flint-cyan">entity_type</span> text CHECK (...)</div>
        <div><span class="text-flint-cyan">entity_id</span> uuid NOT NULL</div>
        <div><span class="text-flint-cyan">aspect</span> text CHECK (description, schema_props, usage_example, ...)</div>
        <div><span class="text-flint-cyan">embedding</span> vector(1536)</div>
        <div><span class="text-flint-cyan">source_text</span> text NOT NULL</div>
        <div><span class="text-flint-cyan">model</span> text DEFAULT 'text-embedding-3-large'</div>
      </div>
    </div>
  </div>
</div>
"""

def generate_app_model_showcase():
    return """
<div class="showcase-card" x-data="{ app: 'flint-admin' }">
  <div class="showcase-label">Interactive Demo — Base Applications</div>
  <div class="grid md:grid-cols-2 gap-4 mb-4">
    <div class="bg-flint-bg border border-flint-border rounded-lg p-4">
      <div class="flex items-center gap-2 mb-3">
        <div class="w-3 h-3 rounded-full bg-flint-accent"></div>
        <span class="font-semibold text-sm">System Applications</span>
      </div>
      <div class="space-y-2">
        <template x-for="a in [
          {slug:'flint-admin', name:'Flint Admin', type:'system', desc:'Platform administration dashboard'},
          {slug:'flint-playground', name:'Flint Playground', type:'system', desc:'Component testing and exploration'},
          {slug:'flint-monitoring', name:'Flint Monitoring', type:'system', desc:'Metrics, logs, health dashboards'},
          {slug:'flint-registry', name:'Flint Registry Manager', type:'system', desc:'Component registry management UI'}
        ]">
          <div class="flex items-center gap-3 p-2 rounded-lg cursor-pointer transition"
               :class="app === a.slug ? 'bg-flint-accent/10 border border-flint-accent/30' : 'hover:bg-flint-surface-2 border border-transparent'"
               @click="app = a.slug">
            <div class="w-8 h-8 rounded-lg bg-flint-surface-2 border border-flint-border flex items-center justify-center text-xs font-mono font-bold" x-text="a.slug.slice(0,2).toUpperCase()"></div>
            <div class="flex-1 min-w-0"><div class="text-sm font-semibold truncate" x-text="a.name"></div><div class="text-xs text-flint-text-3 truncate" x-text="a.desc"></div></div>
            <span class="badge badge-stable text-xs" x-text="a.type"></span>
          </div>
        </template>
      </div>
    </div>
    <div class="bg-flint-bg border border-flint-border rounded-lg p-4">
      <div class="flex items-center gap-2 mb-3">
        <div class="w-3 h-3 rounded-full bg-flint-cyan"></div>
        <span class="font-semibold text-sm">Platform Applications</span>
      </div>
      <div class="space-y-2">
        <template x-for="a in [
          {slug:'flint-gate-console', name:'Flint Gate Console', type:'platform', desc:'Auth proxy management'},
          {slug:'flint-platform-agent', name:'Flint Platform Agent', type:'platform', desc:'Administrative agent interface'}
        ]">
          <div class="flex items-center gap-3 p-2 rounded-lg cursor-pointer transition"
               :class="app === a.slug ? 'bg-flint-cyan/10 border border-flint-cyan/30' : 'hover:bg-flint-surface-2 border border-transparent'"
               @click="app = a.slug">
            <div class="w-8 h-8 rounded-lg bg-flint-surface-2 border border-flint-border flex items-center justify-center text-xs font-mono font-bold" x-text="a.slug.slice(0,2).toUpperCase()"></div>
            <div class="flex-1 min-w-0"><div class="text-sm font-semibold truncate" x-text="a.name"></div><div class="text-xs text-flint-text-3 truncate" x-text="a.desc"></div></div>
            <span class="badge badge-cyan text-xs" x-text="a.type"></span>
          </div>
        </template>
      </div>
    </div>
  </div>
  <div class="bg-flint-bg border border-flint-border rounded-lg p-4 font-mono text-xs">
    <div class="text-flint-accent mb-2">// JWT Claims Resolution for <span x-text="app" class="text-flint-cyan"></span></div>
    <pre class="text-flint-text-2">{{
  "sub": "user-uuid",
  "iss": "flint-gate",
  "aud": "<span x-text="app"></span>",
  "flint": {{
    "roles": ["admin", "editor"],
    "permissions": {{
      "components": ["read:*", "write:custom-*"]
    }},
    "applications": ["<span x-text="app"></span>", "flint-admin"]
  }}
}}</pre>
  </div>
</div>
"""

def generate_db_integration_showcase():
    return """
<div class="showcase-card" x-data="{ pgType: 'text' }">
  <div class="showcase-label">Interactive Demo — Column-to-Component Mapping</div>
  <div class="mb-4">
    <label class="text-xs text-flint-text-3 mb-1 block">Select PostgreSQL type:</label>
    <select class="input-field text-sm" x-model="pgType" style="max-width: 300px;">
      <option value="text">text</option>
      <option value="varchar">varchar</option>
      <option value="uuid">uuid</option>
      <option value="integer">integer</option>
      <option value="bigint">bigint</option>
      <option value="numeric">numeric</option>
      <option value="boolean">boolean</option>
      <option value="timestamp">timestamp</option>
      <option value="timestamptz">timestamptz</option>
      <option value="date">date</option>
      <option value="jsonb">jsonb</option>
      <option value="text[]">text[]</option>
      <option value="bytea">bytea</option>
    </select>
  </div>
  <div class="bg-flint-bg border border-flint-border rounded-lg p-4">
    <div class="flex items-center gap-4 mb-3">
      <div class="text-sm font-mono text-flint-cyan" x-text="pgType"></div>
      <div class="text-flint-text-3">→</div>
      <div class="text-sm font-semibold text-flint-accent" x-text="{
        'text': 'text-field',
        'varchar': 'text-field',
        'uuid': 'text-field (read_only)',
        'integer': 'number',
        'bigint': 'number',
        'numeric': 'number',
        'boolean': 'switch',
        'timestamp': 'date-picker',
        'timestamptz': 'date-picker',
        'date': 'date-picker',
        'jsonb': 'json-editor',
        'text[]': 'multi-select',
        'bytea': 'file-upload'
      }[pgType]"></div>
    </div>
    <div class="text-xs text-flint-text-3" x-text="{
      'text': 'Standard text input with optional validators (email, url, etc.)',
      'varchar': 'Text input with max length constraint',
      'uuid': 'Read-only text field displaying UUID format',
      'integer': 'Numeric input with step: 1, no decimals',
      'bigint': 'Numeric input with step: 1, large number support',
      'numeric': 'Numeric input with precision support',
      'boolean': 'Toggle switch component',
      'timestamp': 'Date picker with time component',
      'timestamptz': 'Date picker with UTC timezone handling',
      'date': 'Date picker without time',
      'jsonb': 'JSON editor with syntax validation',
      'text[]': 'Multi-select with creatable tags',
      'bytea': 'File upload with binary handling'
    }[pgType]"></div>
  </div>
</div>
"""

def generate_design_system_showcase():
    return """
<div class="showcase-card" x-data="{ tab: 'import' }">
  <div class="showcase-label">Interactive Demo — ODSF Import Pipeline</div>
  <div class="flex gap-2 mb-4">
    <button class="tab-btn" :class="tab === 'import' ? 'active' : ''" @click="tab = 'import'">Import ODSF</button>
    <button class="tab-btn" :class="tab === 'tokens' ? 'active' : ''" @click="tab = 'tokens'">Token Resolver</button>
    <button class="tab-btn" :class="tab === 'export' ? 'active' : ''" @click="tab = 'export'">Export ODSF</button>
  </div>
  <div class="bg-flint-bg border border-flint-border rounded-lg p-4">
    <div x-show="tab === 'import'" x-transition:enter="fade-in">
      <div class="flex items-center gap-3 mb-4">
        <div class="bg-flint-surface-2 border border-flint-border rounded-lg p-3 text-center flex-1">
          <div class="text-xs text-flint-text-3 mb-1">Open Design</div>
          <div class="text-sm font-semibold">DESIGN.md</div>
          <div class="text-xs text-flint-text-3 mt-1">tokens.css</div>
        </div>
        <div class="text-flint-accent text-xl">→</div>
        <div class="bg-flint-surface-2 border border-flint-border rounded-lg p-3 text-center flex-1">
          <div class="text-xs text-flint-text-3 mb-1">Flint Registry</div>
          <div class="text-sm font-semibold text-flint-accent">design_systems</div>
          <div class="text-xs text-flint-text-3 mt-1">JSONB tokens</div>
        </div>
      </div>
      <div class="text-xs text-flint-text-2">1. Fetch ODSF bundle → 2. Parse DESIGN.md → 3. Extract tokens.css → 4. Generate component token mappings → 5. Insert design_systems record → 6. Generate embeddings → 7. Emit pg_notify event</div>
    </div>
    <div x-show="tab === 'tokens'" x-transition:enter="fade-in" style="display:none">
      <div class="grid grid-cols-2 md:grid-cols-4 gap-3 mb-3">
        <div class="bg-flint-bg border border-flint-border rounded-lg p-3 text-center"><div class="w-8 h-8 rounded-lg mx-auto mb-1" style="background:#0B0F14;border:1px solid #2D3748"></div><div class="text-xs font-mono">bg</div><div class="text-xs text-flint-text-3">#0B0F14</div></div>
        <div class="bg-flint-bg border border-flint-border rounded-lg p-3 text-center"><div class="w-8 h-8 rounded-lg mx-auto mb-1" style="background:#131A22;border:1px solid #2D3748"></div><div class="text-xs font-mono">surface</div><div class="text-xs text-flint-text-3">#131A22</div></div>
        <div class="bg-flint-bg border border-flint-border rounded-lg p-3 text-center"><div class="w-8 h-8 rounded-lg mx-auto mb-1" style="background:#FF6A3D"></div><div class="text-xs font-mono">accent</div><div class="text-xs text-flint-text-3">#FF6A3D</div></div>
        <div class="bg-flint-bg border border-flint-border rounded-lg p-3 text-center"><div class="w-8 h-8 rounded-lg mx-auto mb-1" style="background:#34CFE6"></div><div class="text-xs font-mono">cyan</div><div class="text-xs text-flint-text-3">#34CFE6</div></div>
      </div>
      <div class="text-xs text-flint-text-2">Tokens are resolved at runtime: base_tokens + component_specific_overrides + user_preferences</div>
    </div>
    <div x-show="tab === 'export'" x-transition:enter="fade-in" style="display:none">
      <div class="flex items-center gap-3 mb-4">
        <div class="bg-flint-surface-2 border border-flint-border rounded-lg p-3 text-center flex-1">
          <div class="text-xs text-flint-text-3 mb-1">Flint Registry</div>
          <div class="text-sm font-semibold text-flint-cyan">components</div>
          <div class="text-xs text-flint-text-3 mt-1">design_systems</div>
        </div>
        <div class="text-flint-cyan text-xl">→</div>
        <div class="bg-flint-surface-2 border border-flint-border rounded-lg p-3 text-center flex-1">
          <div class="text-xs text-flint-text-3 mb-1">Open Design</div>
          <div class="text-sm font-semibold">ODSF Bundle</div>
          <div class="text-xs text-flint-text-3 mt-1">design.md + tokens.css</div>
        </div>
      </div>
      <div class="text-xs text-flint-text-2">Flint can export its component library back to Open Design as an ODSF bundle for sharing.</div>
    </div>
  </div>
</div>
"""

def generate_api_showcase():
    return """
<div class="showcase-card" x-data="{ endpoint: 'components' }">
  <div class="showcase-label">Interactive Demo — REST API Explorer</div>
  <div class="flex gap-2 mb-4 flex-wrap">
    <button class="tab-btn" :class="endpoint === 'components' ? 'active' : ''" @click="endpoint = 'components'">Components</button>
    <button class="tab-btn" :class="endpoint === 'applications' ? 'active' : ''" @click="endpoint = 'applications'">Applications</button>
    <button class="tab-btn" :class="endpoint === 'bindings' ? 'active' : ''" @click="endpoint = 'bindings'">Bindings</button>
    <button class="tab-btn" :class="endpoint === 'design_systems' ? 'active' : ''" @click="endpoint = 'design_systems'">Design Systems</button>
    <button class="tab-btn" :class="endpoint === 'schemas' ? 'active' : ''" @click="endpoint = 'schemas'">Schemas</button>
  </div>
  <div class="bg-flint-bg border border-flint-border rounded-lg p-4 font-mono text-xs">
    <div x-show="endpoint === 'components'">
      <div class="flex items-center gap-2 mb-1"><span class="text-flint-green font-bold">GET</span><span class="text-flint-cyan">/api/v1/components</span></div>
      <div class="text-flint-text-3 mb-2">List components (filtered by app, permissions)</div>
      <div class="flex items-center gap-2 mb-1"><span class="text-flint-green font-bold">GET</span><span class="text-flint-cyan">/api/v1/components/&#123;slug&#125;</span></div>
      <div class="text-flint-text-3 mb-2">Get component definition</div>
      <div class="flex items-center gap-2 mb-1"><span class="text-flint-yellow font-bold">POST</span><span class="text-flint-cyan">/api/v1/components</span></div>
      <div class="text-flint-text-3 mb-2">Register new component</div>
      <div class="flex items-center gap-2 mb-1"><span class="text-flint-purple font-bold">POST</span><span class="text-flint-cyan">/api/v1/components/semantic-search</span></div>
      <div class="text-flint-text-3">Vector semantic search</div>
    </div>
    <div x-show="endpoint === 'applications'" style="display:none">
      <div class="flex items-center gap-2 mb-1"><span class="text-flint-green font-bold">GET</span><span class="text-flint-cyan">/api/v1/applications</span></div>
      <div class="text-flint-text-3 mb-2">List applications</div>
      <div class="flex items-center gap-2 mb-1"><span class="text-flint-green font-bold">GET</span><span class="text-flint-cyan">/api/v1/applications/&#123;slug&#125;/components</span></div>
      <div class="text-flint-text-3 mb-2">Get app-scoped components</div>
      <div class="flex items-center gap-2 mb-1"><span class="text-flint-yellow font-bold">POST</span><span class="text-flint-cyan">/api/v1/applications</span></div>
      <div class="text-flint-text-3">Create application</div>
    </div>
    <div x-show="endpoint === 'bindings'" style="display:none">
      <div class="flex items-center gap-2 mb-1"><span class="text-flint-green font-bold">GET</span><span class="text-flint-cyan">/api/v1/bindings</span></div>
      <div class="text-flint-text-3 mb-2">List bindings</div>
      <div class="flex items-center gap-2 mb-1"><span class="text-flint-yellow font-bold">POST</span><span class="text-flint-cyan">/api/v1/bindings/auto-generate</span></div>
      <div class="text-flint-text-3">Trigger auto-generation for schema</div>
    </div>
    <div x-show="endpoint === 'design_systems'" style="display:none">
      <div class="flex items-center gap-2 mb-1"><span class="text-flint-green font-bold">GET</span><span class="text-flint-cyan">/api/v1/design-systems</span></div>
      <div class="text-flint-text-3 mb-2">List design systems</div>
      <div class="flex items-center gap-2 mb-1"><span class="text-flint-yellow font-bold">POST</span><span class="text-flint-cyan">/api/v1/design-systems/import</span></div>
      <div class="text-flint-text-3">Import ODSF bundle</div>
    </div>
    <div x-show="endpoint === 'schemas'" style="display:none">
      <div class="flex items-center gap-2 mb-1"><span class="text-flint-green font-bold">GET</span><span class="text-flint-cyan">/api/v1/schemas</span></div>
      <div class="text-flint-text-3 mb-2">List schemas</div>
      <div class="flex items-center gap-2 mb-1"><span class="text-flint-yellow font-bold">POST</span><span class="text-flint-cyan">/api/v1/schemas/&#123;slug&#125;/validate</span></div>
      <div class="text-flint-text-3">Validate JSON against schema</div>
    </div>
  </div>
</div>
"""

def generate_a2a_showcase():
    return """
<div class="showcase-card" x-data="{ task: 'register' }">
  <div class="showcase-label">Interactive Demo — A2A Task Catalog</div>
  <div class="grid md:grid-cols-2 gap-3 mb-4">
    <template x-for="t in [
      {id:'a2ui.component.register', name:'Register Component', desc:'Register a new component', input:'Component JSON', output:'Component ID'},
      {id:'a2ui.component.discover', name:'Discover Components', desc:'Find components by description', input:'Natural language query', output:'Component list'},
      {id:'a2ui.component.assemble', name:'Assemble Surface', desc:'Assemble component tree from event', input:'Event type + context', output:'A2UI JSON'},
      {id:'a2ui.design_system.import', name:'Import ODSF', desc:'Import ODSF bundle', input:'Bundle URL', output:'Design system ID'},
      {id:'a2ui.search.semantic', name:'Semantic Search', desc:'Semantic search for components', input:'Query text', output:'Ranked components'},
      {id:'a2ui.token.resolve', name:'Resolve Tokens', desc:'Resolve design tokens', input:'App + component + user', output:'Token map'}
    ]">
      <div class="bg-flint-bg border border-flint-border rounded-lg p-3 cursor-pointer transition"
           :class="task === t.id ? 'border-flint-accent/50 bg-flint-accent/5' : 'hover:border-flint-accent/20'"
           @click="task = t.id">
        <div class="text-xs font-mono text-flint-accent mb-1" x-text="t.id"></div>
        <div class="text-sm font-semibold mb-1" x-text="t.name"></div>
        <div class="text-xs text-flint-text-3" x-text="t.desc"></div>
      </div>
    </template>
  </div>
  <div class="bg-flint-bg border border-flint-border rounded-lg p-4 font-mono text-xs">
    <div class="text-flint-accent mb-2">// Task State Machine</div>
    <div class="flex items-center gap-2">
      <div class="px-2 py-1 rounded bg-flint-surface-2 border border-flint-border">Submitted</div>
      <div class="text-flint-text-3">→</div>
      <div class="px-2 py-1 rounded bg-flint-accent/20 border border-flint-accent/40 text-flint-accent">Working</div>
      <div class="text-flint-text-3">→</div>
      <div class="px-2 py-1 rounded bg-flint-yellow/20 border border-flint-yellow/40 text-flint-yellow">InputRequired</div>
      <div class="text-flint-text-3">→</div>
      <div class="px-2 py-1 rounded bg-flint-green/20 border border-flint-green/40 text-flint-green">Completed</div>
    </div>
    <div class="flex items-center gap-2 mt-2 ml-16">
      <div class="text-flint-text-3">↓</div>
      <div class="px-2 py-1 rounded bg-red-500/20 border border-red-500/40 text-red-400">Failed</div>
    </div>
  </div>
</div>
"""

def generate_mcp_showcase():
    return """
<div class="showcase-card" x-data="{ tool: 'list' }">
  <div class="showcase-label">Interactive Demo — MCP Tool Manifest</div>
  <div class="space-y-2 mb-4">
    <template x-for="t in [
      {id:'a2ui_list_components', name:'List Components', desc:'List available A2UI components filtered by category, application, or permissions', params:['category','application','query']},
      {id:'a2ui_get_component', name:'Get Component', desc:'Get full component definition including schema, examples, and bindings', params:['slug']},
      {id:'a2ui_semantic_search', name:'Semantic Search', desc:'Find components using natural language description', params:['query','limit']},
      {id:'a2ui_generate_form', name:'Generate Form', desc:'Generate a form component for a database table', params:['table_schema','table_name','operation']},
      {id:'a2ui_generate_grid', name:'Generate Grid', desc:'Generate a data grid component for a database table or view', params:['table_schema','table_name','columns']},
      {id:'a2ui_assemble_surface', name:'Assemble Surface', desc:'Assemble an A2UI surface from an event and context', params:['event_type','context','application']}
    ]">
      <div class="bg-flint-bg border border-flint-border rounded-lg p-3 cursor-pointer transition flex items-center gap-3"
           :class="tool === t.id ? 'border-flint-cyan/50 bg-flint-cyan/5' : 'hover:border-flint-cyan/20'"
           @click="tool = t.id">
        <div class="w-8 h-8 rounded-lg bg-flint-surface-2 border border-flint-border flex items-center justify-center text-xs font-mono font-bold text-flint-cyan" x-text="t.id.split('_').pop()[0].toUpperCase()"></div>
        <div class="flex-1 min-w-0">
          <div class="text-sm font-semibold" x-text="t.name"></div>
          <div class="text-xs text-flint-text-3 truncate" x-text="t.desc"></div>
        </div>
      </div>
    </template>
  </div>
  <div class="bg-flint-bg border border-flint-border rounded-lg p-4 font-mono text-xs" x-show="tool">
    <div class="text-flint-cyan mb-2">// Selected Tool Parameters</div>
    <div class="text-flint-text-2" x-text="{
      'a2ui_list_components': '{ category: string, application: string, query: string }',
      'a2ui_get_component': '{ slug: string }',
      'a2ui_semantic_search': '{ query: string, limit: integer (default 5) }',
      'a2ui_generate_form': '{ table_schema: string, table_name: string, operation: enum[create,update,view] }',
      'a2ui_generate_grid': '{ table_schema: string, table_name: string, columns: string[] }',
      'a2ui_assemble_surface': '{ event_type: string, context: object, application: string }'
    }[tool]"></div>
  </div>
</div>
"""

def generate_event_showcase():
    return """
<div class="showcase-card" x-data="{ step: 1, autoPlay: false }" x-init="setInterval(() => { if(autoPlay) { step = step % 4 + 1 } }, 2000)">
  <div class="showcase-label">Interactive Demo — Event Assembly Pipeline</div>
  <div class="flex items-center justify-between mb-4">
    <div class="flex items-center gap-2">
      <button class="tab-btn" :class="autoPlay ? 'active' : ''" @click="autoPlay = !autoPlay">
        <span x-text="autoPlay ? '⏸ Pause' : '▶ Auto Play'"></span>
      </button>
    </div>
    <div class="flex items-center gap-2">
      <button class="w-8 h-8 rounded-lg bg-flint-surface-2 border border-flint-border flex items-center justify-center text-sm hover:bg-flint-accent/20 transition" @click="step = Math.max(1, step - 1)">←</button>
      <span class="text-sm font-mono text-flint-text-3">Step <span x-text="step"></span> / 4</span>
      <button class="w-8 h-8 rounded-lg bg-flint-surface-2 border border-flint-border flex items-center justify-center text-sm hover:bg-flint-accent/20 transition" @click="step = Math.min(4, step + 1)">→</button>
    </div>
  </div>
  <div class="bg-flint-bg border border-flint-border rounded-lg p-4">
    <div class="grid grid-cols-4 gap-2 mb-4">
      <div class="text-center p-2 rounded-lg border transition" :class="step >= 1 ? 'border-flint-accent/50 bg-flint-accent/10' : 'border-flint-border bg-flint-surface-2'">
        <div class="text-xs font-mono mb-1">1. Event Source</div>
        <div class="text-xs text-flint-text-3">Agent infer, Tool call, DB change</div>
      </div>
      <div class="text-center p-2 rounded-lg border transition" :class="step >= 2 ? 'border-flint-cyan/50 bg-flint-cyan/10' : 'border-flint-border bg-flint-surface-2'">
        <div class="text-xs font-mono mb-1">2. Event Router</div>
        <div class="text-xs text-flint-text-3">Matches event type to assembly rule</div>
      </div>
      <div class="text-center p-2 rounded-lg border transition" :class="step >= 3 ? 'border-flint-green/50 bg-flint-green/10' : 'border-flint-border bg-flint-surface-2'">
        <div class="text-xs font-mono mb-1">3. Assembler</div>
        <div class="text-xs text-flint-text-3">Query registry, resolve perms, compose</div>
      </div>
      <div class="text-center p-2 rounded-lg border transition" :class="step >= 4 ? 'border-flint-purple/50 bg-flint-purple/10' : 'border-flint-border bg-flint-surface-2'">
        <div class="text-xs font-mono mb-1">4. Client Render</div>
        <div class="text-xs text-flint-text-3">Receives A2UI JSON → native components</div>
      </div>
    </div>
    <div x-show="step === 1" x-transition:enter="fade-in">
      <div class="text-xs font-mono text-flint-accent mb-2">// Event JSON (tool_call_completed)</div>
      <pre class="code-block text-xs">{{
  "event_type": "tool_call_completed",
  "source": "mcp_tool",
  "source_id": "tool-a2ui_generate_grid",
  "payload": {{
    "tool_name": "a2ui_generate_grid",
    "status": "success",
    "result": {{
      "component": "data-grid",
      "config": {{ "data_source": "public.customers" }}
    }}
  }}
}}</pre>
    </div>
    <div x-show="step === 2" x-transition:enter="fade-in" style="display:none">
      <div class="text-xs font-mono text-flint-cyan mb-2">// Assembly Rule Matched</div>
      <pre class="code-block text-xs">Rule: event_type = "tool_call_completed"
      AND payload.tool_name = "a2ui_generate_grid"
      AND payload.status = "success"
→ surface_type: "modal"
→ root_component: "data-grid"</pre>
    </div>
    <div x-show="step === 3" x-transition:enter="fade-in" style="display:none">
      <div class="text-xs font-mono text-flint-green mb-2">// Generated A2UI Surface</div>
      <pre class="code-block text-xs">{{
  "surface_update": {{
    "surface_id": "main-view",
    "components": [
      {{
        "type": "data-grid",
        "id": "customer-grid",
        "props": {{ "data_source": "public.customers" }},
        "tokens": {{ "accent": "#FF6A3D", "surface": "#131A22" }}
      }}
    ]
  }}
}}</pre>
    </div>
    <div x-show="step === 4" x-transition:enter="fade-in" style="display:none">
      <div class="text-xs font-mono text-flint-purple mb-2">// Client Rendered</div>
      <div class="bg-flint-surface border border-flint-border rounded-lg p-3">
        <div class="flex items-center gap-2 mb-2"><div class="text-xs font-semibold text-flint-accent">Data Grid</div><div class="text-xs text-flint-text-3">public.customers</div></div>
        <div class="flex gap-1"><div class="flex-1 h-2 bg-flint-accent/20 rounded"></div><div class="flex-1 h-2 bg-flint-cyan/20 rounded"></div><div class="flex-1 h-2 bg-flint-green/20 rounded"></div></div>
      </div>
    </div>
  </div>
</div>
"""

def generate_security_showcase():
    return """
<div class="showcase-card" x-data="{ threat: 'unauthorized' }">
  <div class="showcase-label">Interactive Demo — Threat Model</div>
  <div class="grid md:grid-cols-2 gap-3 mb-4">
    <template x-for="t in [
      {id:'unauthorized', name:'Unauthorized Registration', mit:'Only admin roles can register. All audited.'},
      {id:'impersonation', name:'Component Impersonation', mit:'Slug uniqueness enforced at DB level.'},
      {id:'injection', name:'A2UI Injection Attacks', mit:'Constrained generation + sandboxed iframe.'},
      {id:'token', name:'JWT Claim Forgery', mit:'JWT signed by Kratos. Verified by flint-gate.'},
      {id:'escalation', name:'Role Escalation', mit:'Role assignments audited. Inheritance checked.'},
      {id:'tampering', name:'Event Log Tampering', mit:'Append-only. No UPDATE/DELETE via RLS.'}
    ]">
      <div class="bg-flint-bg border border-flint-border rounded-lg p-3 cursor-pointer transition"
           :class="threat === t.id ? 'border-red-400/50 bg-red-400/5' : 'hover:border-red-400/20'"
           @click="threat = t.id">
        <div class="flex items-center gap-2 mb-1">
          <div class="w-2 h-2 rounded-full bg-red-400"></div>
          <div class="text-sm font-semibold" x-text="t.name"></div>
        </div>
        <div class="text-xs text-flint-text-3" x-text="t.mit"></div>
      </div>
    </template>
  </div>
  <div class="bg-flint-bg border border-flint-border rounded-lg p-4 font-mono text-xs">
    <div class="text-flint-accent mb-2">// RLS Policy Example</div>
    <pre class="code-block text-xs">ALTER TABLE flint_a2ui.components ENABLE ROW LEVEL SECURITY;

CREATE POLICY component_access ON flint_a2ui.components
    FOR ALL
    USING (
        is_base = true
        OR application_id IN (
            SELECT application_id FROM flint_a2ui.role_assignments
            WHERE user_id = current_setting('app.jwt_claims')::jsonb->'flint'->>'user_id'
        )
    );</pre>
  </div>
</div>
"""

def generate_roadmap_showcase():
    return """
<div class="showcase-card" x-data="{ milestone: 1 }">
  <div class="showcase-label">Interactive Demo — Implementation Roadmap</div>
  <div class="space-y-3 mb-4">
    <template x-for="m in [
      {num:1, name:'Core Registry', weeks:'Weeks 1-4', status:'complete', progress:100},
      {num:2, name:'Semantic Search', weeks:'Weeks 5-7', status:'complete', progress:100},
      {num:3, name:'Database Binding', weeks:'Weeks 8-11', status:'active', progress:65},
      {num:4, name:'App Model & Perms', weeks:'Weeks 12-14', status:'active', progress:30},
      {num:5, name:'Design System', weeks:'Weeks 15-17', status:'pending', progress:0},
      {num:6, name:'Event Assembly', weeks:'Weeks 18-20', status:'pending', progress:0},
      {num:7, name:'Protocol Surfaces', weeks:'Weeks 21-23', status:'pending', progress:0},
      {num:8, name:'Federation & Scale', weeks:'Weeks 24-26', status:'pending', progress:0}
    ]">
      <div class="flex items-center gap-3 cursor-pointer" @click="milestone = m.num">
        <div class="w-8 h-8 rounded-full flex items-center justify-center text-xs font-bold shrink-0"
             :class="m.status === 'complete' ? 'bg-flint-green/20 text-flint-green border border-flint-green/40' : m.status === 'active' ? 'bg-flint-accent/20 text-flint-accent border border-flint-accent/40' : 'bg-flint-surface-2 text-flint-text-3 border border-flint-border'">
          <span x-text="m.num"></span>
        </div>
        <div class="flex-1 min-w-0">
          <div class="flex items-center justify-between mb-1">
            <div class="text-sm font-semibold" x-text="m.name"></div>
            <div class="text-xs text-flint-text-3" x-text="m.weeks"></div>
          </div>
          <div class="progress-bar"><div class="progress-bar-fill" :style="'width:' + m.progress + '%'"></div></div>
        </div>
      </div>
    </template>
  </div>
</div>
"""

def generate_integration_showcase():
    return """
<div class="showcase-card">
  <div class="showcase-label">Interactive Demo — Ecosystem Integration Matrix</div>
  <div class="overflow-x-auto">
    <table class="data-table text-xs">
      <thead><tr><th>System</th><th>Integration Point</th><th>Mechanism</th></tr></thead>
      <tbody>
        <tr><td class="font-semibold text-flint-accent">flint-gate</td><td>JWT claims, auth proxy</td><td><code class="inline-code">SET LOCAL app.jwt_claims</code></td></tr>
        <tr><td class="font-semibold text-flint-cyan">flint-forge</td><td>Database metadata, reflection</td><td><code class="inline-code">flint_meta</code> → <code class="inline-code">flint_a2ui</code> triggers</td></tr>
        <tr><td class="font-semibold text-flint-green">flint-realtime-fabric</td><td>Real-time UI updates</td><td>Iggy topics: <code class="inline-code">a2ui.events</code></td></tr>
        <tr><td class="font-semibold text-flint-purple">flint-platform-agent</td><td>Administrative interface</td><td>A2A tasks, MCP tools, REST</td></tr>
        <tr><td class="font-semibold text-flint-yellow">flint-vault</td><td>Encrypted column rendering</td><td><code class="inline-code">x-encrypted</code> → Vault key</td></tr>
        <tr><td class="font-semibold text-flint-text">Open Design</td><td>Design system import/export</td><td>ODSF bridge, DESIGN.md parser</td></tr>
        <tr><td class="font-semibold text-flint-text">Keto</td><td>Permission checks</td><td><code class="inline-code">keto_namespace</code> + <code class="inline-code">keto_relation</code></td></tr>
      </tbody>
    </table>
  </div>
</div>
"""

def generate_future_showcase():
    return """
<div class="showcase-card" x-data="{ expanded: null }">
  <div class="showcase-label">Interactive Demo — Future Work Recommendations</div>
  <div class="space-y-2">
    <template x-for="(item, idx) in [
      {title:'Component Test Harness', desc:'Each component should have a test_config JSONB for automated regression testing of generated UIs.'},
      {title:'Multi-modal Embeddings', desc:'Store screenshots/thumbnails alongside text embeddings for visual similarity search.'},
      {title:'Composition Constraints', desc:'Add composition_rules JSONB to enforce valid component trees (e.g., Accordion only contains AccordionItem).'},
      {title:'Animation Definitions', desc:'Add animation JSONB for enter/exit/transition animations per component.'},
      {title:'State Machine Definitions', desc:'Define state machines in JSONB for interactive components (Wizard, Stepper, Form).'},
      {title:'Locale & Internationalization', desc:'Component definitions should support i18n JSONB with translations.'},
      {title:'Dark Mode & Theme Variants', desc:'Design tokens should support light/dark/high-contrast variants.'},
      {title:'Component Performance Budgets', desc:'Add performance JSONB with load time, render time, and bundle size targets.'},
      {title:'Accessibility Compliance Engine', desc:'Automated A11y checks before registration approval.'},
      {title:'Component Marketplace', desc:'Discovery layer where users browse, rate, and install community components.'}
    ]">
      <div class="accordion-item" :class="expanded === idx ? 'open' : ''">
        <div class="accordion-header" @click="expanded = expanded === idx ? null : idx">
          <span class="text-sm" x-text="(idx + 1) + '. ' + item.title"></span>
          <svg class="accordion-icon w-4 h-4 text-flint-text-3" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7"/></svg>
        </div>
        <div class="accordion-body"><div class="text-sm text-flint-text-2" x-text="item.desc"></div></div>
      </div>
    </template>
  </div>
</div>
"""

def generate_generic_showcase(title):
    return f"""
<div class="showcase-card">
  <div class="showcase-label">Interactive Component — {escape_html(title)}</div>
  <div class="bg-flint-bg border border-flint-border rounded-lg p-4 text-center">
    <div class="text-sm text-flint-text-3">Explore this section in the document above for detailed specifications.</div>
  </div>
</div>
"""


def generate_domain_examples():
    """Generate domain-specific application examples."""
    return """
<div class="showcase-card mt-8">
  <div class="showcase-label">Domain-Specific Application Examples</div>
  
  <div class="grid md:grid-cols-2 gap-4 mb-6">
    <!-- Admin Shell -->
    <div class="bg-flint-bg border border-flint-border rounded-lg p-4 hover:border-flint-accent/30 transition">
      <div class="flex items-center gap-2 mb-3">
        <div class="w-8 h-8 rounded-lg bg-flint-accent/15 flex items-center justify-center text-flint-accent text-sm font-bold">A</div>
        <div class="font-semibold text-sm">Admin Shell</div>
        <span class="badge badge-stable text-xs">System</span>
      </div>
      <div class="space-y-2">
        <div class="flex items-center gap-2 text-xs text-flint-text-2"><div class="w-1.5 h-1.5 rounded-full bg-flint-accent"></div>User management & RBAC</div>
        <div class="flex items-center gap-2 text-xs text-flint-text-2"><div class="w-1.5 h-1.5 rounded-full bg-flint-cyan"></div>System health monitoring</div>
        <div class="flex items-center gap-2 text-xs text-flint-text-2"><div class="w-1.5 h-1.5 rounded-full bg-flint-green"></div>Component registry manager</div>
        <div class="flex items-center gap-2 text-xs text-flint-text-2"><div class="w-1.5 h-1.5 rounded-full bg-flint-purple"></div>Audit logs & event stream</div>
      </div>
    </div>
    
    <!-- Chat Interface -->
    <div class="bg-flint-bg border border-flint-border rounded-lg p-4 hover:border-flint-cyan/30 transition">
      <div class="flex items-center gap-2 mb-3">
        <div class="w-8 h-8 rounded-lg bg-flint-cyan/15 flex items-center justify-center text-flint-cyan text-sm font-bold">C</div>
        <div class="font-semibold text-sm">Agent Chat</div>
        <span class="badge badge-cyan text-xs">Agent</span>
      </div>
      <div class="space-y-2">
        <div class="flex items-center gap-2 text-xs text-flint-text-2"><div class="w-1.5 h-1.5 rounded-full bg-flint-accent"></div>Streaming message display</div>
        <div class="flex items-center gap-2 text-xs text-flint-text-2"><div class="w-1.5 h-1.5 rounded-full bg-flint-cyan"></div>Tool call visualization</div>
        <div class="flex items-center gap-2 text-xs text-flint-text-2"><div class="w-1.5 h-1.5 rounded-full bg-flint-green"></div>Code artifact rendering</div>
        <div class="flex items-center gap-2 text-xs text-flint-text-2"><div class="w-1.5 h-1.5 rounded-full bg-flint-purple"></div>Decision prompts</div>
      </div>
    </div>
    
    <!-- CRM -->
    <div class="bg-flint-bg border border-flint-border rounded-lg p-4 hover:border-flint-green/30 transition">
      <div class="flex items-center gap-2 mb-3">
        <div class="w-8 h-8 rounded-lg bg-flint-green/15 flex items-center justify-center text-flint-green text-sm font-bold">R</div>
        <div class="font-semibold text-sm">CRM</div>
        <span class="badge badge-stable text-xs">User</span>
      </div>
      <div class="space-y-2">
        <div class="flex items-center gap-2 text-xs text-flint-text-2"><div class="w-1.5 h-1.5 rounded-full bg-flint-accent"></div>Contact data grid</div>
        <div class="flex items-center gap-2 text-xs text-flint-text-2"><div class="w-1.5 h-1.5 rounded-full bg-flint-cyan"></div>Deal pipeline (Kanban)</div>
        <div class="flex items-center gap-2 text-xs text-flint-text-2"><div class="w-1.5 h-1.5 rounded-full bg-flint-green"></div>Activity timeline</div>
        <div class="flex items-center gap-2 text-xs text-flint-text-2"><div class="w-1.5 h-1.5 rounded-full bg-flint-purple"></div>Communication history</div>
      </div>
    </div>
    
    <!-- ERP -->
    <div class="bg-flint-bg border border-flint-border rounded-lg p-4 hover:border-flint-yellow/30 transition">
      <div class="flex items-center gap-2 mb-3">
        <div class="w-8 h-8 rounded-lg bg-flint-yellow/15 flex items-center justify-center text-flint-yellow text-sm font-bold">E</div>
        <div class="font-semibold text-sm">ERP</div>
        <span class="badge badge-stable text-xs">User</span>
      </div>
      <div class="space-y-2">
        <div class="flex items-center gap-2 text-xs text-flint-text-2"><div class="w-1.5 h-1.5 rounded-full bg-flint-accent"></div>Inventory management</div>
        <div class="flex items-center gap-2 text-xs text-flint-text-2"><div class="w-1.5 h-1.5 rounded-full bg-flint-cyan"></div>Order processing workflows</div>
        <div class="flex items-center gap-2 text-xs text-flint-text-2"><div class="w-1.5 h-1.5 rounded-full bg-flint-green"></div>Financial reporting charts</div>
        <div class="flex items-center gap-2 text-xs text-flint-text-2"><div class="w-1.5 h-1.5 rounded-full bg-flint-purple"></div>Multi-step approval wizard</div>
      </div>
    </div>
    
    <!-- EMR / Healthcare -->
    <div class="bg-flint-bg border border-flint-border rounded-lg p-4 hover:border-red-400/30 transition">
      <div class="flex items-center gap-2 mb-3">
        <div class="w-8 h-8 rounded-lg bg-red-400/15 flex items-center justify-center text-red-400 text-sm font-bold">H</div>
        <div class="font-semibold text-sm">EMR / Healthcare</div>
        <span class="badge badge-beta text-xs">User</span>
      </div>
      <div class="space-y-2">
        <div class="flex items-center gap-2 text-xs text-flint-text-2"><div class="w-1.5 h-1.5 rounded-full bg-flint-accent"></div>Patient records (encrypted)</div>
        <div class="flex items-center gap-2 text-xs text-flint-text-2"><div class="w-1.5 h-1.5 rounded-full bg-flint-cyan"></div>Appointment calendar</div>
        <div class="flex items-center gap-2 text-xs text-flint-text-2"><div class="w-1.5 h-1.5 rounded-full bg-flint-green"></div>Lab results timeline</div>
        <div class="flex items-center gap-2 text-xs text-flint-text-2"><div class="w-1.5 h-1.5 rounded-full bg-flint-purple"></div>Prescription forms</div>
      </div>
    </div>
    
    <!-- Document Management -->
    <div class="bg-flint-bg border border-flint-border rounded-lg p-4 hover:border-flint-purple/30 transition">
      <div class="flex items-center gap-2 mb-3">
        <div class="w-8 h-8 rounded-lg bg-flint-purple/15 flex items-center justify-center text-flint-purple text-sm font-bold">D</div>
        <div class="font-semibold text-sm">Document Management</div>
        <span class="badge badge-stable text-xs">User</span>
      </div>
      <div class="space-y-2">
        <div class="flex items-center gap-2 text-xs text-flint-text-2"><div class="w-1.5 h-1.5 rounded-full bg-flint-accent"></div>File browser (tree view)</div>
        <div class="flex items-center gap-2 text-xs text-flint-text-2"><div class="w-1.5 h-1.5 rounded-full bg-flint-cyan"></div>Document preview (rich text)</div>
        <div class="flex items-center gap-2 text-xs text-flint-text-2"><div class="w-1.5 h-1.5 rounded-full bg-flint-green"></div>Version history</div>
        <div class="flex items-center gap-2 text-xs text-flint-text-2"><div class="w-1.5 h-1.5 rounded-full bg-flint-purple"></div>Collaborative editing</div>
      </div>
    </div>
    
    <!-- Video/Image Editing -->
    <div class="bg-flint-bg border border-flint-border rounded-lg p-4 hover:border-pink-400/30 transition">
      <div class="flex items-center gap-2 mb-3">
        <div class="w-8 h-8 rounded-lg bg-pink-400/15 flex items-center justify-center text-pink-400 text-sm font-bold">M</div>
        <div class="font-semibold text-sm">Media Editor</div>
        <span class="badge badge-beta text-xs">User</span>
      </div>
      <div class="space-y-2">
        <div class="flex items-center gap-2 text-xs text-flint-text-2"><div class="w-1.5 h-1.5 rounded-full bg-flint-accent"></div>Timeline scrubber</div>
        <div class="flex items-center gap-2 text-xs text-flint-text-2"><div class="w-1.5 h-1.5 rounded-full bg-flint-cyan"></div>Layer stack (split panes)</div>
        <div class="flex items-center gap-2 text-xs text-flint-text-2"><div class="w-1.5 h-1.5 rounded-full bg-flint-green"></div>Filter/effect controls</div>
        <div class="flex items-center gap-2 text-xs text-flint-text-2"><div class="w-1.5 h-1.5 rounded-full bg-flint-purple"></div>Export progress modal</div>
      </div>
    </div>
    
    <!-- Generative AI Studio -->
    <div class="bg-flint-bg border border-flint-border rounded-lg p-4 hover:border-flint-cyan/30 transition">
      <div class="flex items-center gap-2 mb-3">
        <div class="w-8 h-8 rounded-lg bg-flint-cyan/15 flex items-center justify-center text-flint-cyan text-sm font-bold">G</div>
        <div class="font-semibold text-sm">Generative AI Studio</div>
        <span class="badge badge-beta text-xs">Platform</span>
      </div>
      <div class="space-y-2">
        <div class="flex items-center gap-2 text-xs text-flint-text-2"><div class="w-1.5 h-1.5 rounded-full bg-flint-accent"></div>Prompt engineering form</div>
        <div class="flex items-center gap-2 text-xs text-flint-text-2"><div class="w-1.5 h-1.5 rounded-full bg-flint-cyan"></div>Output gallery (grid/masonry)</div>
        <div class="flex items-center gap-2 text-xs text-flint-text-2"><div class="w-1.5 h-1.5 rounded-full bg-flint-green"></div>Model comparison (side-by-side)</div>
        <div class="flex items-center gap-2 text-xs text-flint-text-2"><div class="w-1.5 h-1.5 rounded-full bg-flint-purple"></div>Generation history</div>
      </div>
    </div>
  </div>
</div>
"""


def main():
    # Read markdown
    with open(MARKDOWN_PATH, 'r') as f:
        md_content = f.read()
    
    # Parse sections
    sections = parse_markdown(md_content)
    
    # Build HTML
    html_parts = []
    html_parts.append(generate_head())
    
    # Body start
    html_parts.append("<body class="bg-flint-bg text-flint-text min-h-screen">")
    
    # Sidebar
    html_parts.append(generate_sidebar(sections))
    
    # Header
    html_parts.append(generate_header())
    
    # Main content
    html_parts.append("<main class="main-content lg:ml-72 pt-16 pb-20 px-6 max-w-5xl mx-auto">")
    
    # Title section
    html_parts.append("""
<div class="py-10 border-b border-flint-border mb-8">
  <div class="flex items-center gap-3 mb-4">
    <div class="w-12 h-12 rounded-xl bg-gradient-to-br from-flint-accent to-flint-cyan flex items-center justify-center font-bold text-white text-xl font-display shadow-lg shadow-flint-accent/20">F</div>
    <div>
      <h1 class="font-display text-3xl md:text-4xl font-bold text-white">Flint Global A2UI Component Registry</h1>
      <p class="text-flint-text-2 mt-1">Functional Specification, Architecture, and Implementation Plan</p>
    </div>
  </div>
  <div class="flex flex-wrap gap-3 mt-4">
    <span class="badge badge-accent">RFC-FORGE-A2UI-001</span>
    <span class="badge badge-cyan">June 2026</span>
    <span class="badge badge-stable">Architecture Design</span>
    <span class="badge badge-beta">Ready for Implementation</span>
  </div>
</div>
""")
    
    # Process each section
    for sec in sections:
        title = sec["title"]
        level = sec["level"]
        content_lines = sec["content"]
        slug = slugify(title)
        
        # Skip empty sections
        if not any(line.strip() for line in content_lines):
            continue
        
        # Convert content to HTML
        content_html = markdown_to_html(content_lines)
        
        # Determine heading tag
        tag = f"h{level}"
        
        # Section wrapper
        html_parts.append(f'<section id="{slug}" class="mb-10">')
        html_parts.append(f'<{tag} class="font-display font-bold text-white mb-4 mt-8">')
        if level == 1:
            html_parts.append(f'<span class="text-flint-accent mr-2">#</span>')
        elif level == 2:
            html_parts.append(f'<span class="text-flint-cyan mr-2">##</span>')
        elif level == 3:
            html_parts.append(f'<span class="text-flint-green mr-2">###</span>')
        html_parts.append(escape_html(title))
        html_parts.append(f'</{tag}>')
        
        # Content
        html_parts.append('<div class="prose prose-invert prose-sm max-w-none">')
        html_parts.append(content_html)
        html_parts.append('</div>')
        
        # Add showcase for h2 sections
        if level == 2:
            showcase = generate_showcase_for_section(title)
            html_parts.append(showcase)
        
        html_parts.append('</section>')
    
    # Domain examples at the end
    html_parts.append(generate_domain_examples())
    
    # Footer
    html_parts.append("""
<footer class="mt-16 pt-8 border-t border-flint-border text-center pb-10">
  <div class="flex items-center justify-center gap-2 mb-3">
    <div class="w-8 h-8 rounded-lg bg-gradient-to-br from-flint-accent to-flint-cyan flex items-center justify-center font-bold text-white font-display text-sm">F</div>
    <span class="font-display font-semibold text-white">Flint A2UI Registry</span>
  </div>
  <p class="text-sm text-flint-text-3">Document ID: RFC-FORGE-A2UI-001 &nbsp;|&nbsp; Version: 1.0 &nbsp;|&nbsp; Date: June 2026</p>
  <p class="text-xs text-flint-text-3 mt-2">Status: Architecture Design — Ready for Implementation</p>
</footer>
</main>

<!-- Back to top button -->
<button x-data="{ show: false }" x-init="window.addEventListener('scroll', () => show = window.scrollY > 500)" x-show="show" @click="window.scrollTo({top: 0, behavior: 'smooth'})" class="fixed bottom-6 right-6 z-50 w-10 h-10 bg-flint-accent text-white rounded-lg shadow-lg shadow-flint-accent/30 flex items-center justify-center hover:opacity-90 transition" x-cloak>
  <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 10l7-7m0 0l7 7m-7-7v18"/></svg>
</button>

</body>
</html>
""")
    
    # Write output in chunks to avoid memory issues
    output_dir = os.path.dirname(OUTPUT_PATH)
    os.makedirs(output_dir, exist_ok=True)
    
    with open(OUTPUT_PATH, 'w') as f:
        for part in html_parts:
            f.write(part)
            f.flush()
    
    # Get file size
    size = os.path.getsize(OUTPUT_PATH)
    size_kb = size / 1024
    size_mb = size_kb / 1024
    
    print(f"Generated: {OUTPUT_PATH}")
    print(f"File size: {size:,} bytes ({size_kb:.1f} KB / {size_mb:.2f} MB)")
    print(f"Sections processed: {len(sections)}")

if __name__ == "__main__":
    main()
