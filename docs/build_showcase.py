#!/usr/bin/env python3
"""Generate comprehensive A2UI Component Showcase HTMLX document with HTMX + Alpine.js"""
import os

OUT = '/Users/gqadonis/Projects/prometheus/flint-forge/docs/FLINT-A2UI-COMPONENT-SHOWCASE.html'

def write_html():
    parts = []
    
    # CSS styles (Flint/KnowMe dark theme)
    css = """
:root{--bg:#0B0F14;--surface-1:#131A22;--surface-2:#1A232E;--surface-3:#0E141B;--surface-4:#202B38;--surface-5:#2A3542;--text:#E8EDF3;--text-muted:#B4BECB;--text-dim:#97A1AE;--ember:#FF6A3D;--ember-deep:#E04E28;--ember-tint:rgba(255,106,61,0.10);--ember-glow:rgba(255,106,61,0.25);--cyan:#34CFE6;--cyan-tint:rgba(52,207,230,0.10);--green:#4FD18B;--green-tint:rgba(79,209,139,0.10);--green-glow:rgba(79,209,139,0.25);--yellow:#F4B942;--yellow-tint:rgba(244,185,66,0.10);--red:#F05D5D;--red-tint:rgba(240,93,93,0.10);--code-bg:#0E141B;--code-text:#D8E4F0;--line:#28333F;--line-light:#3A4556;--good:#4FD18B;--warn:#F4B942;--shadow:0 4px 24px rgba(0,0,0,0.4);--shadow-sm:0 2px 8px rgba(0,0,0,0.3);--font-display:'Space Grotesk',sans-serif;--font-body:'Inter',sans-serif;--font-mono:'JetBrains Mono',monospace;--radius-sm:6px;--radius-md:10px;--radius-lg:14px;--radius-xl:20px;--transition:all 0.2s cubic-bezier(0.4,0,0.2,1);--transition-slow:all 0.4s cubic-bezier(0.4,0,0.2,1);}
*{margin:0;padding:0;box-sizing:border-box;}html{scroll-behavior:smooth;}
body{font-family:var(--font-body);background:var(--bg);color:var(--text-muted);font-size:15px;line-height:1.7;-webkit-font-smoothing:antialiased;overflow-x:hidden;}

/* Typography */
h1,h2,h3,h4,h5,h6{font-family:var(--font-display);font-weight:700;color:var(--text);letter-spacing:-0.02em;line-height:1.2;}
h1{font-size:2.8rem;}h2{font-size:2rem;margin-top:3rem;margin-bottom:1rem;}h3{font-size:1.4rem;margin-top:2rem;margin-bottom:0.75rem;}h4{font-size:1.1rem;margin-top:1.5rem;margin-bottom:0.5rem;}
p{margin-bottom:1rem;max-width:70ch;}a{color:var(--cyan);text-decoration:none;transition:var(--transition);}a:hover{color:var(--text);text-decoration:underline;}
code{font-family:var(--font-mono);font-size:0.85rem;color:var(--code-text);background:var(--code-bg);padding:2px 6px;border-radius:4px;border:1px solid var(--line);}pre code{display:block;padding:0;background:none;border:none;}

/* Layout */
.container{max-width:1400px;margin:0 auto;padding:0 24px;}.grid-2{display:grid;grid-template-columns:repeat(2,1fr);gap:24px;}.grid-3{display:grid;grid-template-columns:repeat(3,1fr);gap:24px;}.grid-4{display:grid;grid-template-columns:repeat(4,1fr);gap:16px;}
@media(max-width:900px){.grid-2,.grid-3,.grid-4{grid-template-columns:1fr;}}
.flex{display:flex;}.flex-col{flex-direction:column;}.items-center{align-items:center;}.justify-between{justify-content:space-between;}.gap-2{gap:8px;}.gap-4{gap:16px;}.gap-6{gap:24px;}

/* Cards */
.card{background:var(--surface-1);border:1px solid var(--line);border-radius:var(--radius-lg);padding:24px;transition:var(--transition);box-shadow:var(--shadow-sm);}
.card:hover{border-color:var(--line-light);box-shadow:var(--shadow);transform:translateY(-2px);}
.card-header{display:flex;align-items:center;justify-content:space-between;margin-bottom:16px;}
.card-title{font-family:var(--font-display);font-size:1.1rem;font-weight:600;color:var(--text);}
.card-subtitle{font-size:0.85rem;color:var(--text-dim);}

/* Surface levels */
.surface-1{background:var(--surface-1);border:1px solid var(--line);border-radius:var(--radius-md);padding:20px;}
.surface-2{background:var(--surface-2);border:1px solid var(--line);border-radius:var(--radius-md);padding:20px;}
.surface-3{background:var(--surface-3);border:1px solid var(--line);border-radius:var(--radius-md);padding:20px;}

/* Buttons */
.btn{display:inline-flex;align-items:center;gap:8px;padding:10px 20px;border-radius:var(--radius-sm);font-family:var(--font-display);font-weight:600;font-size:0.9rem;cursor:pointer;border:none;transition:var(--transition);position:relative;overflow:hidden;}
.btn-primary{background:linear-gradient(135deg,var(--ember),var(--ember-deep));color:#fff;box-shadow:0 4px 16px var(--ember-glow);}
.btn-primary:hover{transform:translateY(-1px);box-shadow:0 6px 24px var(--ember-glow);}
.btn-primary:active{transform:translateY(0);}
.btn-secondary{background:var(--surface-2);color:var(--text);border:1px solid var(--line);}
.btn-secondary:hover{background:var(--surface-4);border-color:var(--line-light);}
.btn-ghost{background:transparent;color:var(--text-dim);}
.btn-ghost:hover{color:var(--text);background:var(--surface-2);}
.btn-sm{padding:6px 14px;font-size:0.8rem;}.btn-lg{padding:14px 28px;font-size:1rem;}
.btn-icon{width:40px;height:40px;padding:0;justify-content:center;border-radius:var(--radius-sm);}

/* Inputs */
.input{width:100%;padding:10px 14px;background:var(--surface-3);border:1px solid var(--line);border-radius:var(--radius-sm);color:var(--text);font-family:var(--font-body);font-size:0.95rem;transition:var(--transition);}
.input:focus{outline:none;border-color:var(--cyan);box-shadow:0 0 0 3px var(--cyan-tint);}
.input::placeholder{color:var(--text-dim);}
.select{appearance:none;background-image:url('data:image/svg+xml,<svg xmlns=%22http://www.w3.org/2000/svg%22 width=%2216%22 height=%2216%22 fill=%22%2397A1AE%22><path d=%22M4 6l4 4 4-4%22/></svg>');background-repeat:no-repeat;background-position:right 12px center;padding-right:36px;}
.textarea{min-height:120px;resize:vertical;}

/* Form elements */
.label{display:block;font-family:var(--font-display);font-weight:600;font-size:0.85rem;color:var(--text-muted);margin-bottom:6px;text-transform:uppercase;letter-spacing:0.05em;}
.form-group{margin-bottom:20px;}
.form-hint{font-size:0.8rem;color:var(--text-dim);margin-top:4px;}
.form-error{font-size:0.8rem;color:var(--red);margin-top:4px;display:none;}
.form-group.has-error .form-error{display:block;}
.form-group.has-error .input{border-color:var(--red);box-shadow:0 0 0 3px var(--red-tint);}

/* Checkbox / Switch */
.checkbox{display:flex;align-items:center;gap:10px;cursor:pointer;}
.checkbox input[type=checkbox]{appearance:none;width:20px;height:20px;border:2px solid var(--line);border-radius:var(--radius-sm);background:var(--surface-3);cursor:pointer;transition:var(--transition);position:relative;}
.checkbox input[type=checkbox]:checked{background:var(--green);border-color:var(--green);box-shadow:0 0 8px var(--green-glow);}
.checkbox input[type=checkbox]:checked::after{content:'';position:absolute;left:5px;top:1px;width:6px;height:11px;border:solid #fff;border-width:0 2px 2px 0;transform:rotate(45deg);}
.switch{position:relative;display:inline-block;width:44px;height:24px;}
.switch input{opacity:0;width:0;height:0;}
.switch-slider{position:absolute;cursor:pointer;top:0;left:0;right:0;bottom:0;background:var(--line);border-radius:24px;transition:var(--transition);}
.switch-slider::before{position:absolute;content:'';height:18px;width:18px;left:3px;bottom:3px;background:var(--text);border-radius:50%;transition:var(--transition);}
.switch input:checked + .switch-slider{background:var(--green);}
.switch input:checked + .switch-slider::before{transform:translateX(20px);}

/* Table */
.table{width:100%;border-collapse:collapse;font-size:0.9rem;}
.table th{text-align:left;padding:12px 16px;background:var(--surface-3);color:var(--text);font-family:var(--font-display);font-weight:600;font-size:0.85rem;letter-spacing:0.03em;border-bottom:2px solid var(--line);white-space:nowrap;}
.table td{padding:12px 16px;border-bottom:1px solid var(--line);vertical-align:middle;}
.table tr:hover{background:var(--surface-2);transition:var(--transition);}
.table tr:last-child td{border-bottom:none;}
.table-badge{display:inline-block;padding:3px 10px;border-radius:100px;font-size:0.75rem;font-weight:600;}
.badge-green{background:var(--green-tint);color:var(--green);}
.badge-yellow{background:var(--yellow-tint);color:var(--yellow);}
.badge-red{background:var(--red-tint);color:var(--red);}
.badge-ember{background:var(--ember-tint);color:var(--ember);}

/* Tabs */
.tabs{display:flex;gap:4px;padding:4px;background:var(--surface-3);border-radius:var(--radius-md);margin-bottom:20px;}
.tab-btn{padding:10px 20px;border-radius:var(--radius-sm);font-family:var(--font-display);font-weight:600;font-size:0.9rem;color:var(--text-dim);background:transparent;border:none;cursor:pointer;transition:var(--transition);}
.tab-btn:hover{color:var(--text);}
.tab-btn.active{background:var(--surface-1);color:var(--text);box-shadow:var(--shadow-sm);}
.tab-panel{display:none;}
.tab-panel.active{display:block;animation:fadeIn 0.3s ease;}

/* Accordion */
.accordion-item{border:1px solid var(--line);border-radius:var(--radius-sm);margin-bottom:8px;overflow:hidden;transition:var(--transition);}
.accordion-item:hover{border-color:var(--line-light);}
.accordion-header{padding:14px 18px;display:flex;align-items:center;justify-content:space-between;cursor:pointer;background:var(--surface-2);transition:var(--transition);}
.accordion-header:hover{background:var(--surface-4);}
.accordion-title{font-family:var(--font-display);font-weight:600;font-size:0.95rem;color:var(--text);}
.accordion-icon{width:20px;height:20px;transition:var(--transition);color:var(--text-dim);}
.accordion-item.is-open .accordion-icon{transform:rotate(180deg);}
.accordion-body{padding:0 18px;max-height:0;overflow:hidden;transition:max-height 0.3s ease,padding 0.3s ease;}
.accordion-item.is-open .accordion-body{padding:14px 18px;max-height:500px;}

/* Modal / Drawer */
.modal-overlay{position:fixed;inset:0;background:rgba(0,0,0,0.6);backdrop-filter:blur(4px);z-index:100;display:none;align-items:center;justify-content:center;opacity:0;transition:opacity 0.3s ease;}
.modal-overlay.is-open{display:flex;opacity:1;}
.modal-content{background:var(--surface-1);border:1px solid var(--line);border-radius:var(--radius-lg);width:100%;max-width:600px;max-height:90vh;overflow-y:auto;box-shadow:var(--shadow);transform:scale(0.95);transition:transform 0.3s ease;}
.modal-overlay.is-open .modal-content{transform:scale(1);}
.modal-header{padding:20px 24px;border-bottom:1px solid var(--line);display:flex;justify-content:space-between;align-items:center;}
.modal-title{font-family:var(--font-display);font-weight:700;font-size:1.2rem;}
.modal-close{width:32px;height:32px;border-radius:var(--radius-sm);background:var(--surface-2);border:none;color:var(--text-dim);cursor:pointer;display:grid;place-items:center;transition:var(--transition);}
.modal-close:hover{color:var(--text);background:var(--surface-4);}
.modal-body{padding:24px;}
.modal-footer{padding:16px 24px;border-top:1px solid var(--line);display:flex;justify-content:flex-end;gap:12px;}

.drawer{position:fixed;top:0;right:0;width:400px;max-width:90vw;height:100vh;background:var(--surface-1);border-left:1px solid var(--line);z-index:101;transform:translateX(100%);transition:transform 0.3s cubic-bezier(0.4,0,0.2,1);box-shadow:var(--shadow);}
.drawer.is-open{transform:translateX(0);}
.drawer-header{padding:20px 24px;border-bottom:1px solid var(--line);display:flex;justify-content:space-between;align-items:center;}
.drawer-body{padding:24px;overflow-y:auto;height:calc(100vh - 80px);}

/* Sidebar / Nav */
.sidebar-nav{width:260px;height:100vh;background:var(--surface-3);border-right:1px solid var(--line);position:fixed;left:0;top:0;z-index:50;transition:var(--transition-slow);}
.sidebar-nav.collapsed{width:72px;}
.sidebar-header{padding:20px 24px;border-bottom:1px solid var(--line);}
.sidebar-logo{font-family:var(--font-display);font-weight:700;font-size:1.5rem;color:var(--text);display:flex;align-items:center;gap:12px;}
.sidebar-logo-icon{width:40px;height:40px;border-radius:var(--radius-sm);background:linear-gradient(135deg,var(--ember),var(--ember-deep));display:grid;place-items:center;color:#fff;font-weight:700;font-size:1.2rem;}
.sidebar-nav-item{display:flex;align-items:center;gap:12px;padding:12px 24px;color:var(--text-dim);font-weight:500;transition:var(--transition);cursor:pointer;border:none;background:none;width:100%;font-size:0.95rem;}
.sidebar-nav-item:hover{color:var(--text);background:var(--surface-2);}
.sidebar-nav-item.active{color:var(--ember);background:var(--ember-tint);border-right:3px solid var(--ember);}
.sidebar-nav-item svg{width:20px;height:20px;flex-shrink:0;}
.main-content{margin-left:260px;transition:var(--transition-slow);}
.sidebar-nav.collapsed + .main-content{margin-left:72px;}

/* Top Bar */
.top-bar{height:64px;background:var(--surface-1);border-bottom:1px solid var(--line);display:flex;align-items:center;justify-content:space-between;padding:0 24px;position:sticky;top:0;z-index:40;}
.top-bar-left{display:flex;align-items:center;gap:16px;}
.top-bar-right{display:flex;align-items:center;gap:16px;}
.breadcrumb{display:flex;align-items:center;gap:8px;font-size:0.85rem;color:var(--text-dim);}
.breadcrumb a{color:var(--text-dim);} .breadcrumb a:hover{color:var(--text);}
.breadcrumb-sep{color:var(--line-light);}

/* Avatar */
.avatar{width:40px;height:40px;border-radius:50%;background:linear-gradient(135deg,var(--cyan),var(--green));display:grid;place-items:center;color:#fff;font-weight:600;font-size:0.9rem;box-shadow:0 2px 8px rgba(0,0,0,0.3);}
.avatar-sm{width:32px;height:32px;font-size:0.8rem;}
.avatar-lg{width:56px;height:56px;font-size:1.2rem;}
.avatar-group{display:flex;}.avatar-group .avatar{margin-left:-12px;border:2px solid var(--bg);}
.avatar-group .avatar:first-child{margin-left:0;}

/* Progress */
.progress-bar{height:8px;background:var(--surface-3);border-radius:100px;overflow:hidden;}
.progress-fill{height:100%;background:linear-gradient(90deg,var(--green),var(--cyan));border-radius:100px;transition:width 0.6s ease;box-shadow:0 0 8px var(--green-glow);}
.progress-bar.ember .progress-fill{background:linear-gradient(90deg,var(--ember),var(--ember-deep));box-shadow:0 0 8px var(--ember-glow);}

/* Chat / Messaging */
.chat-container{display:flex;flex-direction:column;height:100%;}
.chat-messages{flex:1;overflow-y:auto;padding:24px;display:flex;flex-direction:column;gap:16px;}
.chat-message{display:flex;gap:12px;max-width:80%;}
.chat-message.own{flex-direction:row-reverse;align-self:flex-end;}
.chat-bubble{padding:12px 16px;border-radius:var(--radius-md);background:var(--surface-2);border:1px solid var(--line);}
.chat-message.own .chat-bubble{background:var(--ember-tint);border-color:var(--ember);}
.chat-meta{font-size:0.75rem;color:var(--text-dim);margin-top:4px;}
.chat-composer{padding:16px 24px;border-top:1px solid var(--line);display:flex;gap:12px;}
.chat-input{flex:1;}

/* Timeline */
.timeline{position:relative;padding-left:32px;}
.timeline::before{content:'';position:absolute;left:11px;top:0;bottom:0;width:2px;background:var(--line);}
.timeline-item{position:relative;margin-bottom:24px;}
.timeline-dot{position:absolute;left:-26px;width:24px;height:24px;border-radius:50%;background:var(--surface-1);border:2px solid var(--cyan);display:grid;place-items:center;z-index:1;}
.timeline-dot svg{width:12px;height:12px;color:var(--cyan);}
.timeline-content{background:var(--surface-2);border:1px solid var(--line);border-radius:var(--radius-md);padding:16px;}
.timeline-time{font-size:0.8rem;color:var(--text-dim);margin-bottom:4px;}
.timeline-title{font-family:var(--font-display);font-weight:600;color:var(--text);}

/* Kanban */
.kanban-board{display:flex;gap:16px;overflow-x:auto;padding-bottom:16px;}
.kanban-column{min-width:280px;background:var(--surface-3);border:1px solid var(--line);border-radius:var(--radius-md);padding:16px;}
.kanban-column-header{display:flex;align-items:center;justify-content:space-between;margin-bottom:12px;}
.kanban-column-title{font-family:var(--font-display);font-weight:600;font-size:0.95rem;}
.kanban-count{font-size:0.8rem;color:var(--text-dim);background:var(--surface-2);padding:2px 8px;border-radius:100px;}
.kanban-card{background:var(--surface-2);border:1px solid var(--line);border-radius:var(--radius-sm);padding:14px;margin-bottom:10px;cursor:grab;transition:var(--transition);}
.kanban-card:hover{border-color:var(--line-light);box-shadow:var(--shadow-sm);}
.kanban-card-title{font-weight:600;color:var(--text);font-size:0.9rem;margin-bottom:6px;}
.kanban-card-meta{font-size:0.8rem;color:var(--text-dim);display:flex;gap:12px;}

/* Metric */
.metric-card{text-align:center;padding:32px 24px;}
.metric-value{font-family:var(--font-display);font-size:2.5rem;font-weight:700;color:var(--text);line-height:1;}
.metric-label{font-size:0.9rem;color:var(--text-dim);margin-top:8px;}
.metric-change{font-size:0.85rem;font-weight:600;margin-top:6px;}
.metric-change.positive{color:var(--green);}
.metric-change.negative{color:var(--red);}

/* Chart area */
.chart-placeholder{height:200px;background:var(--surface-3);border:1px solid var(--line);border-radius:var(--radius-md);display:flex;align-items:center;justify-content:center;position:relative;overflow:hidden;}
.chart-bar{position:absolute;bottom:0;width:24px;background:linear-gradient(to top,var(--cyan),transparent);border-radius:4px 4px 0 0;opacity:0.6;animation:barGrow 1s ease forwards;}

/* Toast / Notification */
.toast-container{position:fixed;top:24px;right:24px;z-index:200;display:flex;flex-direction:column;gap:12px;}
.toast{background:var(--surface-1);border:1px solid var(--line);border-radius:var(--radius-md);padding:16px 20px;min-width:320px;box-shadow:var(--shadow);display:flex;align-items:flex-start;gap:12px;transform:translateX(120%);transition:transform 0.4s cubic-bezier(0.4,0,0.2,1);}
.toast.is-visible{transform:translateX(0);}
.toast-icon{width:20px;height:20px;flex-shrink:0;margin-top:2px;}
.toast-success{border-color:var(--green);}
.toast-success .toast-icon{color:var(--green);}
.toast-error{border-color:var(--red);}
.toast-error .toast-icon{color:var(--red);}
.toast-warning{border-color:var(--yellow);}
.toast-warning .toast-icon{color:var(--yellow);}
.toast-title{font-weight:600;color:var(--text);font-size:0.95rem;}
.toast-body{font-size:0.85rem;color:var(--text-dim);margin-top:2px;}

/* Loading states */
.skeleton{background:linear-gradient(90deg,var(--surface-3),var(--surface-4),var(--surface-3));background-size:200% 100%;animation:skeleton 1.5s ease-in-out infinite;border-radius:var(--radius-sm);}
.spinner{width:20px;height:20px;border:2px solid var(--line);border-top-color:var(--cyan);border-radius:50%;animation:spin 0.8s linear infinite;}
.spinner-sm{width:14px;height:14px;border-width:2px;}
.spinner-lg{width:32px;height:32px;border-width:3px;}

/* HTMX specific animations */
.htmx-added{animation:slideIn 0.3s ease;}
.htmx-swapping{animation:fadeOut 0.2s ease forwards;}

/* Section headers */
.section-header{margin:48px 0 24px;padding-bottom:12px;border-bottom:1px solid var(--line);display:flex;align-items:center;gap:16px;}
.section-number{width:40px;height:40px;border-radius:var(--radius-sm);background:var(--ember-tint);color:var(--ember);display:grid;place-items:center;font-family:var(--font-display);font-weight:700;font-size:1.1rem;}
.section-title{font-family:var(--font-display);font-size:1.5rem;font-weight:700;color:var(--text);}

/* Demo area */
.demo-area{background:var(--surface-3);border:1px solid var(--line);border-radius:var(--radius-lg);padding:32px;position:relative;overflow:hidden;}
.demo-area::before{content:'Demo';position:absolute;top:12px;right:16px;font-family:var(--font-mono);font-size:0.7rem;color:var(--text-dim);text-transform:uppercase;letter-spacing:0.1em;}
.demo-label{font-family:var(--font-mono);font-size:0.75rem;color:var(--text-dim);text-transform:uppercase;letter-spacing:0.1em;margin-bottom:16px;}

/* Code preview */
.code-preview{background:var(--code-bg);border:1px solid var(--line);border-radius:var(--radius-md);padding:20px;overflow-x:auto;font-size:0.85rem;line-height:1.6;}
.code-preview-header{display:flex;gap:8px;padding:8px 16px;background:var(--surface-2);border-bottom:1px solid var(--line);border-radius:var(--radius-md) var(--radius-md) 0 0;}
.code-dot{width:10px;height:10px;border-radius:50%;}
.code-dot-red{background:var(--red);} .code-dot-yellow{background:var(--yellow);} .code-dot-green{background:var(--green);}

/* Hero */
.hero{padding:80px 0 60px;text-align:center;}
.hero-badge{display:inline-block;padding:6px 16px;background:var(--ember-tint);border:1px solid var(--ember);border-radius:100px;font-family:var(--font-mono);font-size:0.75rem;color:var(--ember);text-transform:uppercase;letter-spacing:0.1em;margin-bottom:24px;}
.hero h1{font-size:3.5rem;margin-bottom:16px;}
.hero p{font-size:1.2rem;color:var(--text-dim);max-width:600px;margin:0 auto 40px;}

/* Component showcase grid */
.showcase-grid{display:grid;grid-template-columns:repeat(auto-fill,minmax(300px,1fr));gap:24px;}
.showcase-item{background:var(--surface-1);border:1px solid var(--line);border-radius:var(--radius-lg);padding:24px;transition:var(--transition);}
.showcase-item:hover{border-color:var(--line-light);box-shadow:var(--shadow);}
.showcase-item-title{font-family:var(--font-display);font-weight:600;color:var(--text);margin-bottom:8px;font-size:1rem;}
.showcase-item-desc{font-size:0.85rem;color:var(--text-dim);margin-bottom:16px;}

/* Stepper */
.stepper{display:flex;align-items:center;gap:8px;}
.step{flex:1;text-align:center;position:relative;}
.step::after{content:'';position:absolute;top:20px;left:50%;right:-50%;height:2px;background:var(--line);z-index:0;}
.step:last-child::after{display:none;}
.step-circle{width:40px;height:40px;border-radius:50%;background:var(--surface-3);border:2px solid var(--line);display:grid;place-items:center;margin:0 auto 8px;position:relative;z-index:1;font-family:var(--font-display);font-weight:600;color:var(--text-dim);}
.step.active .step-circle{background:var(--ember-tint);border-color:var(--ember);color:var(--ember);box-shadow:0 0 12px var(--ember-glow);}
.step.completed .step-circle{background:var(--green);border-color:var(--green);color:#fff;}
.step-label{font-size:0.8rem;color:var(--text-dim);}
.step.active .step-label{color:var(--text);}

/* Pagination */
.pagination{display:flex;align-items:center;gap:4px;}
.page-btn{padding:8px 14px;border-radius:var(--radius-sm);background:var(--surface-2);border:1px solid var(--line);color:var(--text-dim);font-family:var(--font-display);font-weight:600;cursor:pointer;transition:var(--transition);}
.page-btn:hover{background:var(--surface-4);color:var(--text);}
.page-btn.active{background:var(--ember-tint);border-color:var(--ember);color:var(--ember);}
.page-btn:disabled{opacity:0.4;cursor:not-allowed;}

/* Filter bar */
.filter-bar{display:flex;align-items:center;gap:12px;padding:12px 16px;background:var(--surface-2);border:1px solid var(--line);border-radius:var(--radius-md);margin-bottom:20px;}
.filter-tag{display:inline-flex;align-items:center;gap:6px;padding:4px 12px;background:var(--surface-3);border:1px solid var(--line);border-radius:100px;font-size:0.8rem;color:var(--text);}
.filter-tag-remove{width:14px;height:14px;border-radius:50%;border:none;background:var(--line);color:var(--text-dim);cursor:pointer;display:grid;place-items:center;font-size:10px;}
.filter-tag-remove:hover{background:var(--red);color:#fff;}

/* Command palette */
.command-palette-overlay{position:fixed;inset:0;background:rgba(0,0,0,0.7);backdrop-filter:blur(8px);z-index:200;display:none;align-items:flex-start;justify-content:center;padding-top:120px;opacity:0;transition:opacity 0.2s ease;}
.command-palette-overlay.is-open{display:flex;opacity:1;}
.command-palette{background:var(--surface-1);border:1px solid var(--line);border-radius:var(--radius-lg);width:100%;max-width:600px;box-shadow:var(--shadow);transform:translateY(-20px);transition:transform 0.3s ease;}
.command-palette-overlay.is-open .command-palette{transform:translateY(0);}
.command-input{width:100%;padding:16px 24px;background:transparent;border:none;border-bottom:1px solid var(--line);color:var(--text);font-size:1.1rem;outline:none;}
.command-input::placeholder{color:var(--text-dim);}
.command-list{max-height:400px;overflow-y:auto;padding:8px 0;}
.command-item{padding:12px 24px;display:flex;align-items:center;gap:12px;cursor:pointer;transition:var(--transition);}
.command-item:hover{background:var(--surface-2);}
.command-item-icon{width:32px;height:32px;border-radius:var(--radius-sm);background:var(--surface-3);display:grid;place-items:center;color:var(--text-dim);}
.command-item:hover .command-item-icon{color:var(--cyan);}
.command-item-text{font-weight:500;color:var(--text);}
.command-item-desc{font-size:0.8rem;color:var(--text-dim);}
.command-shortcut{margin-left:auto;font-family:var(--font-mono);font-size:0.75rem;color:var(--text-dim);background:var(--surface-3);padding:2px 8px;border-radius:4px;}

/* EMR specific */
.emr-vital-sign{display:flex;align-items:center;gap:16px;padding:16px;background:var(--surface-2);border:1px solid var(--line);border-radius:var(--radius-md);}
.emr-vital-icon{width:40px;height:40px;border-radius:var(--radius-sm);background:var(--cyan-tint);display:grid;place-items:center;color:var(--cyan);}
.emr-vital-value{font-family:var(--font-display);font-size:1.5rem;font-weight:700;color:var(--text);}
.emr-vital-label{font-size:0.85rem;color:var(--text-dim);}
.emr-vital-trend{font-size:0.8rem;}
.emr-vital-trend.up{color:var(--green);} .emr-vital-trend.down{color:var(--red);}

/* Generative AI interfaces */
.gen-prompt-area{background:var(--surface-3);border:1px solid var(--line);border-radius:var(--radius-lg);padding:24px;}
.gen-image-grid{display:grid;grid-template-columns:repeat(2,1fr);gap:12px;}
.gen-image-item{aspect-ratio:1;background:var(--surface-2);border:1px solid var(--line);border-radius:var(--radius-md);overflow:hidden;position:relative;cursor:pointer;transition:var(--transition);}
.gen-image-item:hover{border-color:var(--cyan);box-shadow:0 0 16px var(--cyan-tint);}
.gen-image-item::after{content:'';position:absolute;inset:0;background:linear-gradient(180deg,transparent 60%,rgba(0,0,0,0.6));}
.gen-image-actions{position:absolute;bottom:8px;left:8px;right:8px;display:flex;gap:8px;z-index:1;opacity:0;transition:opacity 0.2s ease;}
.gen-image-item:hover .gen-image-actions{opacity:1;}

/* Video player mock */
.video-player{aspect-ratio:16/9;background:var(--surface-3);border-radius:var(--radius-md);position:relative;overflow:hidden;display:flex;align-items:center;justify-content:center;}
.video-player::before{content:'';position:absolute;inset:0;background:linear-gradient(135deg,var(--surface-3),var(--surface-4));}
.video-play-btn{width:80px;height:80px;border-radius:50%;background:rgba(255,255,255,0.1);backdrop-filter:blur(8px);border:2px solid rgba(255,255,255,0.3);display:grid;place-items:center;cursor:pointer;transition:var(--transition);z-index:1;}
.video-play-btn:hover{background:rgba(255,255,255,0.2);transform:scale(1.1);}
.video-controls{position:absolute;bottom:0;left:0;right:0;padding:16px 20px;background:linear-gradient(transparent,rgba(0,0,0,0.8));display:flex;align-items:center;gap:12px;}
.video-timeline{flex:1;height:4px;background:rgba(255,255,255,0.2);border-radius:100px;cursor:pointer;position:relative;}
.video-timeline-fill{height:100%;width:35%;background:var(--ember);border-radius:100px;}
.video-time{font-family:var(--font-mono);font-size:0.8rem;color:#fff;}

/* Animations */
@keyframes fadeIn{from{opacity:0;transform:translateY(10px);}to{opacity:1;transform:translateY(0);}}
@keyframes slideIn{from{opacity:0;transform:translateX(-20px);}to{opacity:1;transform:translateX(0);}}
@keyframes fadeOut{from{opacity:1;}to{opacity:0;}}
@keyframes spin{to{transform:rotate(360deg);}}
@keyframes skeleton{0%{background-position:200% 0;}100%{background-position:-200% 0;}}
@keyframes barGrow{from{height:0;}to{height:var(--h,60%);}}
@keyframes pulse{0%,100%{opacity:1;}50%{opacity:0.5;}}
@keyframes glow{0%,100%{box-shadow:0 0 5px var(--cyan-tint);}50%{box-shadow:0 0 20px var(--cyan-tint);}}

/* Scrollbar */
::-webkit-scrollbar{width:8px;height:8px;}::-webkit-scrollbar-track{background:var(--surface-3);}::-webkit-scrollbar-thumb{background:var(--line-light);border-radius:4px;}::-webkit-scrollbar-thumb:hover{background:var(--text-dim);}

/* Selection */
::selection{background:var(--ember-tint);color:var(--text);}

/* Focus visible */
:focus-visible{outline:2px solid var(--cyan);outline-offset:2px;}

/* HTMX request states */
.htmx-request .spinner{display:inline-block;}
.htmx-request .btn-content{opacity:0.6;}

/* Print */
@media print{body{background:#fff;color:#000;}.sidebar-nav,.top-bar{display:none;}.main-content{margin-left:0;}}
"""
    return css

def build_head():
    return f'''<!DOCTYPE html>
<html lang="en" x-data="{{}}">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>Flint A2UI Component Showcase — KnowMe Design System</title>
<script src="https://cdn.tailwindcss.com"></script>
<script src="https://unpkg.com/htmx.org@2.0.4"></script>
<script defer src="https://cdn.jsdelivr.net/npm/alpinejs@3.x.x/dist/cdn.min.js"></script>
<link rel="preconnect" href="https://fonts.googleapis.com">
<link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
<link href="https://fonts.googleapis.com/css2?family=Space+Grotesk:wght@400;500;600;700&family=Inter:wght@300;400;500;600;700&family=JetBrains+Mono:wght@400;500;600&display=swap" rel="stylesheet">
<style>{get_css()}</style>
</head>'''

def get_css():
    return build_css()

# Call the CSS builder
def build_css():
    return """
:root{--bg:#0B0F14;--surface-1:#131A22;--surface-2:#1A232E;--surface-3:#0E141B;--surface-4:#202B38;--surface-5:#2A3542;--text:#E8EDF3;--text-muted:#B4BECB;--text-dim:#97A1AE;--ember:#FF6A3D;--ember-deep:#E04E28;--ember-tint:rgba(255,106,61,0.10);--ember-glow:rgba(255,106,61,0.25);--cyan:#34CFE6;--cyan-tint:rgba(52,207,230,0.10);--green:#4FD18B;--green-tint:rgba(79,209,139,0.10);--green-glow:rgba(79,209,139,0.25);--yellow:#F4B942;--yellow-tint:rgba(244,185,66,0.10);--red:#F05D5D;--red-tint:rgba(240,93,93,0.10);--code-bg:#0E141B;--code-text:#D8E4F0;--line:#28333F;--line-light:#3A4556;--good:#4FD18B;--warn:#F4B942;--shadow:0 4px 24px rgba(0,0,0,0.4);--shadow-sm:0 2px 8px rgba(0,0,0,0.3);--font-display:'Space Grotesk',sans-serif;--font-body:'Inter',sans-serif;--font-mono:'JetBrains Mono',monospace;--radius-sm:6px;--radius-md:10px;--radius-lg:14px;--radius-xl:20px;--transition:all 0.2s cubic-bezier(0.4,0,0.2,1);--transition-slow:all 0.4s cubic-bezier(0.4,0,0.2,1);}
*{margin:0;padding:0;box-sizing:border-box;}html{scroll-behavior:smooth;}
body{font-family:var(--font-body);background:var(--bg);color:var(--text-muted);font-size:15px;line-height:1.7;-webkit-font-smoothing:antialiased;overflow-x:hidden;}
h1,h2,h3,h4,h5,h6{font-family:var(--font-display);font-weight:700;color:var(--text);letter-spacing:-0.02em;line-height:1.2;}
h1{font-size:2.8rem;}h2{font-size:2rem;margin-top:3rem;margin-bottom:1rem;}h3{font-size:1.4rem;margin-top:2rem;margin-bottom:0.75rem;}h4{font-size:1.1rem;margin-top:1.5rem;margin-bottom:0.5rem;}
p{margin-bottom:1rem;max-width:70ch;}a{color:var(--cyan);text-decoration:none;transition:var(--transition);}a:hover{color:var(--text);text-decoration:underline;}
code{font-family:var(--font-mono);font-size:0.85rem;color:var(--code-text);background:var(--code-bg);padding:2px 6px;border-radius:4px;border:1px solid var(--line);}
.container{max-width:1400px;margin:0 auto;padding:0 24px;}
.card{background:var(--surface-1);border:1px solid var(--line);border-radius:var(--radius-lg);padding:24px;transition:var(--transition);box-shadow:var(--shadow-sm);}
.card:hover{border-color:var(--line-light);box-shadow:var(--shadow);transform:translateY(-2px);}
.btn{display:inline-flex;align-items:center;gap:8px;padding:10px 20px;border-radius:var(--radius-sm);font-family:var(--font-display);font-weight:600;font-size:0.9rem;cursor:pointer;border:none;transition:var(--transition);}
.btn-primary{background:linear-gradient(135deg,var(--ember),var(--ember-deep));color:#fff;box-shadow:0 4px 16px var(--ember-glow);}
.btn-primary:hover{transform:translateY(-1px);box-shadow:0 6px 24px var(--ember-glow);}
.btn-secondary{background:var(--surface-2);color:var(--text);border:1px solid var(--line);}
.btn-secondary:hover{background:var(--surface-4);}
.btn-ghost{background:transparent;color:var(--text-dim);}
.btn-ghost:hover{color:var(--text);background:var(--surface-2);}
.input{width:100%;padding:10px 14px;background:var(--surface-3);border:1px solid var(--line);border-radius:var(--radius-sm);color:var(--text);font-size:0.95rem;transition:var(--transition);}
.input:focus{outline:none;border-color:var(--cyan);box-shadow:0 0 0 3px var(--cyan-tint);}
.label{display:block;font-family:var(--font-display);font-weight:600;font-size:0.85rem;color:var(--text-muted);margin-bottom:6px;text-transform:uppercase;letter-spacing:0.05em;}
.form-group{margin-bottom:20px;}
.table{width:100%;border-collapse:collapse;font-size:0.9rem;}
.table th{text-align:left;padding:12px 16px;background:var(--surface-3);color:var(--text);font-family:var(--font-display);font-weight:600;font-size:0.85rem;border-bottom:2px solid var(--line);}
.table td{padding:12px 16px;border-bottom:1px solid var(--line);vertical-align:middle;}
.table tr:hover{background:var(--surface-2);}
.table-badge{display:inline-block;padding:3px 10px;border-radius:100px;font-size:0.75rem;font-weight:600;}
.badge-green{background:var(--green-tint);color:var(--green);}
.badge-yellow{background:var(--yellow-tint);color:var(--yellow);}
.badge-red{background:var(--red-tint);color:var(--red);}
.badge-ember{background:var(--ember-tint);color:var(--ember);}
.tabs{display:flex;gap:4px;padding:4px;background:var(--surface-3);border-radius:var(--radius-md);margin-bottom:20px;}
.tab-btn{padding:10px 20px;border-radius:var(--radius-sm);font-family:var(--font-display);font-weight:600;font-size:0.9rem;color:var(--text-dim);background:transparent;border:none;cursor:pointer;transition:var(--transition);}
.tab-btn:hover{color:var(--text);}
.tab-btn.active{background:var(--surface-1);color:var(--text);box-shadow:var(--shadow-sm);}
.accordion-item{border:1px solid var(--line);border-radius:var(--radius-sm);margin-bottom:8px;overflow:hidden;}
.accordion-header{padding:14px 18px;display:flex;align-items:center;justify-content:space-between;cursor:pointer;background:var(--surface-2);}
.accordion-title{font-family:var(--font-display);font-weight:600;color:var(--text);}
.accordion-body{padding:0 18px;max-height:0;overflow:hidden;transition:max-height 0.3s ease,padding 0.3s ease;}
.accordion-item.is-open .accordion-body{padding:14px 18px;max-height:500px;}
.modal-overlay{position:fixed;inset:0;background:rgba(0,0,0,0.6);backdrop-filter:blur(4px);z-index:100;display:none;align-items:center;justify-content:center;opacity:0;transition:opacity 0.3s ease;}
.modal-overlay.is-open{display:flex;opacity:1;}
.modal-content{background:var(--surface-1);border:1px solid var(--line);border-radius:var(--radius-lg);width:100%;max-width:600px;max-height:90vh;overflow-y:auto;box-shadow:var(--shadow);transform:scale(0.95);transition:transform 0.3s ease;}
.modal-overlay.is-open .modal-content{transform:scale(1);}
.sidebar-nav{width:260px;height:100vh;background:var(--surface-3);border-right:1px solid var(--line);position:fixed;left:0;top:0;z-index:50;transition:var(--transition-slow);}
.main-content{margin-left:260px;transition:var(--transition-slow);}
.top-bar{height:64px;background:var(--surface-1);border-bottom:1px solid var(--line);display:flex;align-items:center;justify-content:space-between;padding:0 24px;position:sticky;top:0;z-index:40;}
.avatar{width:40px;height:40px;border-radius:50%;background:linear-gradient(135deg,var(--cyan),var(--green));display:grid;place-items:center;color:#fff;font-weight:600;font-size:0.9rem;}
.progress-bar{height:8px;background:var(--surface-3);border-radius:100px;overflow:hidden;}
.progress-fill{height:100%;background:linear-gradient(90deg,var(--green),var(--cyan));border-radius:100px;transition:width 0.6s ease;}
.kanban-column{min-width:280px;background:var(--surface-3);border:1px solid var(--line);border-radius:var(--radius-md);padding:16px;}
.kanban-card{background:var(--surface-2);border:1px solid var(--line);border-radius:var(--radius-sm);padding:14px;margin-bottom:10px;cursor:grab;}
.chat-message{display:flex;gap:12px;max-width:80%;}
.chat-message.own{flex-direction:row-reverse;align-self:flex-end;}
.chat-bubble{padding:12px 16px;border-radius:var(--radius-md);background:var(--surface-2);border:1px solid var(--line);}
.chat-message.own .chat-bubble{background:var(--ember-tint);border-color:var(--ember);}
.metric-value{font-family:var(--font-display);font-size:2.5rem;font-weight:700;color:var(--text);}
.step-circle{width:40px;height:40px;border-radius:50%;background:var(--surface-3);border:2px solid var(--line);display:grid;place-items:center;margin:0 auto 8px;font-family:var(--font-display);font-weight:600;color:var(--text-dim);}
.step.active .step-circle{background:var(--ember-tint);border-color:var(--ember);color:var(--ember);}
.step.completed .step-circle{background:var(--green);border-color:var(--green);color:#fff;}
.toast-container{position:fixed;top:24px;right:24px;z-index:200;display:flex;flex-direction:column;gap:12px;}
.toast{background:var(--surface-1);border:1px solid var(--line);border-radius:var(--radius-md);padding:16px 20px;min-width:320px;box-shadow:var(--shadow);transform:translateX(120%);transition:transform 0.4s ease;}
.toast.is-visible{transform:translateX(0);}
.video-player{aspect-ratio:16/9;background:var(--surface-3);border-radius:var(--radius-md);position:relative;overflow:hidden;display:flex;align-items:center;justify-content:center;}
.gen-image-item{aspect-ratio:1;background:var(--surface-2);border:1px solid var(--line);border-radius:var(--radius-md);overflow:hidden;position:relative;cursor:pointer;}
.command-palette-overlay{position:fixed;inset:0;background:rgba(0,0,0,0.7);backdrop-filter:blur(8px);z-index:200;display:none;align-items:flex-start;justify-content:center;padding-top:120px;opacity:0;transition:opacity 0.2s ease;}
.command-palette-overlay.is-open{display:flex;opacity:1;}
.command-palette{background:var(--surface-1);border:1px solid var(--line);border-radius:var(--radius-lg);width:100%;max-width:600px;box-shadow:var(--shadow);}
.command-input{width:100%;padding:16px 24px;background:transparent;border:none;border-bottom:1px solid var(--line);color:var(--text);font-size:1.1rem;outline:none;}
.skeleton{background:linear-gradient(90deg,var(--surface-3),var(--surface-4),var(--surface-3));background-size:200% 100%;animation:skeleton 1.5s ease-in-out infinite;border-radius:var(--radius-sm);}
.spinner{width:20px;height:20px;border:2px solid var(--line);border-top-color:var(--cyan);border-radius:50%;animation:spin 0.8s linear infinite;}
@keyframes fadeIn{from{opacity:0;transform:translateY(10px);}to{opacity:1;transform:translateY(0);}}
@keyframes slideIn{from{opacity:0;transform:translateX(-20px);}to{opacity:1;transform:translateX(0);}}
@keyframes spin{to{transform:rotate(360deg);}}
@keyframes skeleton{0%{background-position:200% 0;}100%{background-position:-200% 0;}}
::-webkit-scrollbar{width:8px;}::-webkit-scrollbar-track{background:var(--surface-3);}::-webkit-scrollbar-thumb{background:var(--line-light);border-radius:4px;}
::selection{background:var(--ember-tint);color:var(--text);}
:focus-visible{outline:2px solid var(--cyan);outline-offset:2px;}
@media(max-width:900px){.grid-2,.grid-3,.grid-4{grid-template-columns:1fr;}}
"""

print("CSS generated")

# Now build the full HTML document
import html

def escape_html(s):
    return html.escape(s)

sections = []

def add_section(title, content):
    sections.append(f'<section class="container" style="padding:40px 24px;"><h2>{escape_html(title)}</h2>{content}</section>')

def add_subsection(title, content):
    sections.append(f'<div style="margin-top:32px;"><h3>{escape_html(title)}</h3>{content}</div>')

def card(title, desc, demo_html):
    return f'''<div class="card" style="margin-bottom:24px;">
    <div class="card-header">
        <div>
            <div class="card-title">{escape_html(title)}</div>
            <div class="card-subtitle">{escape_html(desc)}</div>
        </div>
    </div>
    <div class="demo-area" style="margin-bottom:16px;">
        {demo_html}
    </div>
</div>'''

def showcase_grid(items):
    inner = '\n'.join([f'<div class="showcase-item"><div class="showcase-item-title">{escape_html(i["title"])}</div><div class="showcase-item-desc">{escape_html(i["desc"])}</div>{i["html"]}</div>' for i in items])
    return f'<div class="showcase-grid">{inner}</div>'

# === SECTION 1: HERO ===
hero_html = '''<div class="hero" style="background:linear-gradient(160deg,#11181F 0%,#0B0F14 70%);padding:80px 24px 60px;text-align:center;border-bottom:1px solid var(--line);">
<div class="container">
<div class="hero-badge">Component Showcase</div>
<h1 style="font-size:3.5rem;margin-bottom:16px;">Flint A2UI Components</h1>
<p style="font-size:1.2rem;color:var(--text-dim);max-width:600px;margin:0 auto 40px;">
    A comprehensive visual reference of all A2UI base primitives rendered with the KnowMe design system — dark-themed, ember-accented, AI-native.
</p>
<div style="display:flex;gap:16px;justify-content:center;flex-wrap:wrap;">
    <a href="#layout" class="btn btn-primary">Explore Components</a>
    <a href="#domains" class="btn btn-secondary">Domain Examples</a>
</div>
</div>
</div>'''

# === SECTION 2: LAYOUT PRIMITIVES ===
layout_items = [
    {"title": "Stack", "desc": "Vertical/horizontal grouping of children", "html": '''<div style="display:flex;flex-direction:column;gap:8px;"><div style="padding:12px;background:var(--surface-2);border-radius:var(--radius-sm);border:1px solid var(--line);">Item 1</div><div style="padding:12px;background:var(--surface-2);border-radius:var(--radius-sm);border:1px solid var(--line);">Item 2</div><div style="padding:12px;background:var(--surface-2);border-radius:var(--radius-sm);border:1px solid var(--line);">Item 3</div></div>'''},
    {"title": "Card", "desc": "Bordered container with depth", "html": '''<div style="padding:20px;background:var(--surface-2);border:1px solid var(--line);border-radius:var(--radius-md);box-shadow:var(--shadow-sm);"><div style="font-weight:600;color:var(--text);margin-bottom:8px;">Card Title</div><div style="font-size:0.9rem;color:var(--text-dim);">Card content with surface-2 background and subtle border.</div></div>'''},
    {"title": "Grid", "desc": "Multi-column responsive layout", "html": '''<div style="display:grid;grid-template-columns:repeat(3,1fr);gap:8px;"><div style="padding:16px;background:var(--surface-2);border-radius:var(--radius-sm);text-align:center;">A</div><div style="padding:16px;background:var(--surface-2);border-radius:var(--radius-sm);text-align:center;">B</div><div style="padding:16px;background:var(--surface-2);border-radius:var(--radius-sm);text-align:center;">C</div></div>'''},
    {"title": "Split Pane", "desc": "Resizable sidebar + main", "html": '''<div style="display:flex;gap:1px;border:1px solid var(--line);border-radius:var(--radius-md);overflow:hidden;"><div style="width:120px;padding:16px;background:var(--surface-2);font-size:0.8rem;">Sidebar</div><div style="flex:1;padding:16px;background:var(--surface-3);font-size:0.8rem;">Main content area</div></div>'''},
    {"title": "Tabs", "desc": "Tabbed content switching", "html": '''<div x-data="{tab:'one'}"><div style="display:flex;gap:4px;padding:4px;background:var(--surface-3);border-radius:var(--radius-md);margin-bottom:12px;"><button @click="tab='one'" :class="tab=='one'?'active':''" class="tab-btn">First</button><button @click="tab='two'" :class="tab=='two'?'active':''" class="tab-btn">Second</button><button @click="tab='three'" :class="tab=='three'?'active':''" class="tab-btn">Third</button></div><div x-show="tab=='one'" x-transition.duration.300ms class="surface-2" style="padding:16px;">Tab One Content</div><div x-show="tab=='two'" x-transition.duration.300ms class="surface-2" style="padding:16px;">Tab Two Content</div><div x-show="tab=='three'" x-transition.duration.300ms class="surface-2" style="padding:16px;">Tab Three Content</div></div>'''},
    {"title": "Accordion", "desc": "Collapsible sections", "html": '''<div x-data="{open:1}"><div class="accordion-item" :class="open==1?'is-open':''"><div @click="open=1" class="accordion-header"><span class="accordion-title">Section One</span><span class="accordion-icon">▼</span></div><div class="accordion-body"><p>Content for section one. This expands and collapses with smooth animation.</p></div></div><div class="accordion-item" :class="open==2?'is-open':''"><div @click="open=2" class="accordion-header"><span class="accordion-title">Section Two</span><span class="accordion-icon">▼</span></div><div class="accordion-body"><p>Content for section two. Only one open at a time.</p></div></div></div>'''},
    {"title": "Modal", "desc": "Overlay dialog", "html": '''<div x-data="{show:false}"><button @click="show=true" class="btn btn-primary">Open Modal</button><div x-show="show" x-transition:enter="transition ease-out duration-300" x-transition:enter-start="opacity-0" x-transition:enter-end="opacity-100" x-transition:leave="transition ease-in duration-200" x-transition:leave-start="opacity-100" x-transition:leave-end="opacity-0" class="modal-overlay" style="display:flex;" :class="show?'is-open':''"><div @click.away="show=false" class="modal-content"><div class="modal-header"><span class="modal-title">Modal Title</span><button @click="show=false" class="modal-close">✕</button></div><div class="modal-body"><p>Modal content goes here. Use @click.away to close when clicking outside.</p></div><div class="modal-footer"><button @click="show=false" class="btn btn-secondary">Cancel</button><button @click="show=false" class="btn btn-primary">Confirm</button></div></div></div></div>'''},
    {"title": "Drawer", "desc": "Slide-out panel", "html": '''<div x-data="{drawer:false}"><button @click="drawer=true" class="btn btn-secondary">Open Drawer</button><div x-show="drawer" class="modal-overlay" :class="drawer?'is-open':''"><div class="drawer" :class="drawer?'is-open':''"><div class="drawer-header"><span style="font-family:var(--font-display);font-weight:700;">Drawer Panel</span><button @click="drawer=false" class="modal-close">✕</button></div><div class="drawer-body"><p>Drawer content with scrollable body.</p></div></div></div></div>'''},
]
layout_section = showcase_grid(layout_items)

# === SECTION 3: DATA DISPLAY ===
data_display_items = [
    {"title": "Text", "desc": "Headings, paragraphs, labels", "html": '''<div><h4 style="margin-top:0;">Heading Text</h4><p style="margin-bottom:0;">Body text with <strong>bold</strong> and <em>italic</em> styling.</p></div>'''},
    {"title": "Badge", "desc": "Status indicators", "html": '''<div style="display:flex;gap:8px;flex-wrap:wrap;"><span class="table-badge badge-green">Active</span><span class="table-badge badge-yellow">Pending</span><span class="table-badge badge-red">Error</span><span class="table-badge badge-ember">Warning</span></div>'''},
    {"title": "Avatar", "desc": "User representations", "html": '''<div class="avatar-group"><div class="avatar">JD</div><div class="avatar" style="background:linear-gradient(135deg,var(--ember),var(--yellow));">AL</div><div class="avatar" style="background:linear-gradient(135deg,var(--cyan),var(--green));">MK</div></div>'''},
    {"title": "Progress", "desc": "Progress indicators", "html": '''<div style="display:flex;flex-direction:column;gap:12px;"><div class="progress-bar"><div class="progress-fill" style="width:75%;"></div></div><div class="progress-bar ember"><div class="progress-fill" style="width:45%;"></div></div></div>'''},
    {"title": "Metric", "desc": "Key numbers with trends", "html": '''<div class="metric-card"><div class="metric-value">2,847</div><div class="metric-label">Total Users</div><div class="metric-change positive">↑ 12.5%</div></div>'''},
    {"title": "Table", "desc": "Tabular data display", "html": '''<table class="table" style="font-size:0.8rem;"><thead><tr><th>Name</th><th>Status</th><th>Role</th></tr></thead><tbody><tr><td>Alice Chen</td><td><span class="table-badge badge-green">Active</span></td><td>Admin</td></tr><tr><td>Bob Smith</td><td><span class="table-badge badge-yellow">Pending</span></td><td>Editor</td></tr></tbody></table>'''},
    {"title": "Timeline", "desc": "Chronological events", "html": '''<div class="timeline"><div class="timeline-item"><div class="timeline-dot">●</div><div class="timeline-content"><div class="timeline-time">2 min ago</div><div class="timeline-title">Deployment started</div></div></div><div class="timeline-item"><div class="timeline-dot" style="border-color:var(--green);">✓</div><div class="timeline-content"><div class="timeline-time">5 min ago</div><div class="timeline-title">Build completed</div></div></div></div>'''},
    {"title": "Kanban", "desc": "Drag-and-drop status board", "html": '''<div class="kanban-board"><div class="kanban-column"><div class="kanban-column-header"><span class="kanban-column-title">To Do</span><span class="kanban-count">3</span></div><div class="kanban-card"><div class="kanban-card-title">Design review</div><div class="kanban-card-meta">Due today</div></div></div><div class="kanban-column"><div class="kanban-column-header"><span class="kanban-column-title">Done</span><span class="kanban-count">1</span></div><div class="kanban-card"><div class="kanban-card-title">Setup repo</div><div class="kanban-card-meta">Completed</div></div></div></div>'''},
]
data_display_section = showcase_grid(data_display_items)

# === SECTION 4: INPUT PRIMITIVES ===
input_items = [
    {"title": "Text Field", "desc": "Single-line input", "html": '''<div class="form-group"><label class="label">Username</label><input type="text" class="input" placeholder="Enter username" value="flint_user"></div>'''},
    {"title": "Text Area", "desc": "Multi-line input", "html": '''<div class="form-group"><label class="label">Description</label><textarea class="input textarea" placeholder="Enter description..."></textarea></div>'''},
    {"title": "Number", "desc": "Numeric input", "html": '''<div class="form-group"><label class="label">Quantity</label><input type="number" class="input" value="42" min="0" max="100"></div>'''},
    {"title": "Switch", "desc": "Boolean toggle", "html": '''<div class="form-group"><label class="checkbox"><div class="switch"><input type="checkbox" checked><span class="switch-slider"></span></div><span>Enable notifications</span></label></div>'''},
    {"title": "Checkbox", "desc": "Multiple selection", "html": '''<div style="display:flex;flex-direction:column;gap:8px;"><label class="checkbox"><input type="checkbox" checked><span>Option A</span></label><label class="checkbox"><input type="checkbox"><span>Option B</span></label></div>'''},
    {"title": "Select", "desc": "Dropdown selection", "html": '''<div class="form-group"><label class="label">Category</label><select class="input select"><option>Database</option><option>API</option><option>Frontend</option></select></div>'''},
    {"title": "Search", "desc": "Search with autocomplete", "html": '''<div class="form-group"><input type="search" class="input" placeholder="Search components..." style="background-image:url(\'data:image/svg+xml,<svg xmlns=%22http://www.w3.org/2000/svg%22 width=%2216%22 height=%2216%22 fill=%22%2397A1AE%22><path d=%22M11.742 10.344a6.5 6.5 0 1 0-1.397 1.398h-.001l3.85 3.85a1 1 0 0 0 1.415-1.414l-3.85-3.85zm-5.242.656a5 5 0 1 1 0-10 5 5 0 0 1 0 10z%22/></svg>\');background-repeat:no-repeat;background-position:12px center;padding-left:40px;"></div>'''},
    {"title": "Date Picker", "desc": "Date selection", "html": '''<div class="form-group"><label class="label">Start Date</label><input type="date" class="input" value="2026-06-30"></div>'''},
]
input_section = showcase_grid(input_items)

# === SECTION 5: ACTION PRIMITIVES ===
action_items = [
    {"title": "Button", "desc": "Primary action trigger", "html": '''<div style="display:flex;gap:12px;flex-wrap:wrap;"><button class="btn btn-primary">Primary</button><button class="btn btn-secondary">Secondary</button><button class="btn btn-ghost">Ghost</button></div>'''},
    {"title": "Button Group", "desc": "Mutually exclusive actions", "html": '''<div style="display:flex;gap:4px;background:var(--surface-3);padding:4px;border-radius:var(--radius-md);"><button class="btn btn-primary" style="flex:1;justify-content:center;">Day</button><button class="btn btn-ghost" style="flex:1;justify-content:center;color:var(--text-dim);">Week</button><button class="btn btn-ghost" style="flex:1;justify-content:center;color:var(--text-dim);">Month</button></div>'''},
    {"title": "Icon Button", "desc": "Compact action", "html": '''<div style="display:flex;gap:8px;"><button class="btn btn-icon btn-secondary">✏️</button><button class="btn btn-icon btn-secondary">🗑️</button><button class="btn btn-icon btn-secondary">📋</button></div>'''},
    {"title": "Pagination", "desc": "Page navigation", "html": '''<div class="pagination"><button class="page-btn" disabled>←</button><button class="page-btn active">1</button><button class="page-btn">2</button><button class="page-btn">3</button><button class="page-btn">→</button></div>'''},
]
action_section = showcase_grid(action_items)

# === SECTION 6: AGENT PRIMITIVES ===
agent_items = [
    {"title": "Agent Chat", "desc": "Chat interface with streaming", "html": '''<div class="chat-container" style="height:280px;border:1px solid var(--line);border-radius:var(--radius-md);overflow:hidden;"><div class="chat-messages"><div class="chat-message"><div class="avatar" style="width:32px;height:32px;font-size:0.75rem;flex-shrink:0;">AI</div><div class="chat-bubble"><p>Hello! How can I help you today?</p></div></div><div class="chat-message own"><div class="chat-bubble"><p>Generate a dashboard for my sales data.</p></div><div class="avatar" style="width:32px;height:32px;font-size:0.75rem;flex-shrink:0;background:linear-gradient(135deg,var(--ember),var(--yellow));">U</div></div></div><div class="chat-composer"><input type="text" class="input chat-input" placeholder="Type a message..."><button class="btn btn-primary">Send</button></div></div>'''},
    {"title": "Tool Call", "desc": "Tool invocation card", "html": '''<div class="surface-2" style="border-left:3px solid var(--cyan);"><div style="display:flex;align-items:center;gap:12px;margin-bottom:8px;"><span style="color:var(--cyan);">🔧</span><span style="font-weight:600;color:var(--text);">a2ui_generate_grid</span><span class="table-badge badge-green" style="margin-left:auto;">Running</span></div><div style="font-size:0.85rem;color:var(--text-dim);">Generating data grid for table <code>public.customers</code>...</div><div class="progress-bar" style="margin-top:12px;"><div class="progress-fill" style="width:60%;"></div></div></div>'''},
    {"title": "Artifact", "desc": "Generated artifact display", "html": '''<div class="surface-2" style="border:1px solid var(--line);border-radius:var(--radius-md);padding:16px;"><div style="display:flex;align-items:center;gap:12px;margin-bottom:12px;"><span style="font-size:1.5rem;">📄</span><div><div style="font-weight:600;color:var(--text);">Sales Dashboard</div><div style="font-size:0.8rem;color:var(--text-dim);">Generated 2 min ago</div></div></div><div style="height:100px;background:var(--surface-3);border-radius:var(--radius-sm);display:flex;align-items:center;justify-content:center;color:var(--text-dim);">Artifact preview</div></div>'''},
    {"title": "Streaming Text", "desc": "Live text generation", "html": '''<div class="surface-2"><div style="display:flex;align-items:center;gap:8px;margin-bottom:8px;"><div class="spinner spinner-sm"></div><span style="font-size:0.85rem;color:var(--text-dim);">Generating response...</span></div><div style="font-size:0.95rem;color:var(--text);line-height:1.6;">The sales data shows a <strong>15% increase</strong> in Q2 compared to Q1, with the highest growth in the enterprise segment...</div></div>'''},
]
agent_section = showcase_grid(agent_items)

# === SECTION 7: NAVIGATION PRIMITIVES ===
nav_items = [
    {"title": "Breadcrumb", "desc": "Hierarchical navigation", "html": '''<div class="breadcrumb"><a href="#">Home</a><span class="breadcrumb-sep">/</span><a href="#">Settings</a><span class="breadcrumb-sep">/</span><span>Profile</span></div>'''},
    {"title": "Stepper", "desc": "Multi-step progress", "html": '''<div class="stepper"><div class="step completed"><div class="step-circle">✓</div><div class="step-label">Account</div></div><div class="step active"><div class="step-circle">2</div><div class="step-label">Profile</div></div><div class="step"><div class="step-circle">3</div><div class="step-label">Review</div></div></div>'''},
    {"title": "Filter Bar", "desc": "Active filter display", "html": '''<div class="filter-bar"><span class="filter-tag">Status: Active<button class="filter-tag-remove">✕</button></span><span class="filter-tag">Role: Admin<button class="filter-tag-remove">✕</button></span><button class="btn btn-sm btn-ghost">+ Add Filter</button></div>'''},
]
nav_section = showcase_grid(nav_items)

# === SECTION 8: ADMIN DASHBOARD SHELL ===
admin_shell = '''<div class="card" style="padding:0;overflow:hidden;">
<div style="display:flex;height:400px;">
<div style="width:200px;background:var(--surface-3);border-right:1px solid var(--line);padding:16px;flex-shrink:0;">
<div style="font-family:var(--font-display);font-weight:700;font-size:1.2rem;color:var(--text);margin-bottom:24px;">⚡ Flint</div>
<div style="display:flex;flex-direction:column;gap:4px;"><button class="sidebar-nav-item active">📊 Dashboard</button><button class="sidebar-nav-item">👥 Users</button><button class="sidebar-nav-item">⚙️ Settings</button></div>
</div>
<div style="flex:1;display:flex;flex-direction:column;">
<div class="top-bar" style="position:static;">
<div class="top-bar-left"><div class="breadcrumb"><a href="#">Dashboard</a></div></div>
<div class="top-bar-right"><div class="avatar" style="width:32px;height:32px;font-size:0.75rem;">U</div></div>
</div>
<div style="padding:24px;flex:1;overflow-y:auto;">
<div class="grid-4" style="margin-bottom:24px;"><div class="metric-card surface-2"><div class="metric-value">1.2K</div><div class="metric-label">Users</div></div><div class="metric-card surface-2"><div class="metric-value" style="color:var(--green);">98.9%</div><div class="metric-label">Uptime</div></div><div class="metric-card surface-2"><div class="metric-value" style="color:var(--cyan);">45ms</div><div class="metric-label">Latency</div></div><div class="metric-card surface-2"><div class="metric-value" style="color:var(--ember);">3</div><div class="metric-label">Alerts</div></div></div>
<table class="table"><thead><tr><th>Service</th><th>Status</th><th>Latency</th></tr></thead><tbody><tr><td>flint-gate</td><td><span class="table-badge badge-green">Healthy</span></td><td>12ms</td></tr><tr><td>flint-forge</td><td><span class="table-badge badge-green">Healthy</span></td><td>8ms</td></tr><tr><td>flint-realtime</td><td><span class="table-badge badge-yellow">Degraded</span></td><td>120ms</td></tr></tbody></table>
</div>
</div>
</div>
</div>'''

# === SECTION 9: CHAT INTERFACE ===
chat_interface = '''<div class="card" style="padding:0;overflow:hidden;">
<div style="display:flex;height:400px;">
<div style="width:180px;background:var(--surface-3);border-right:1px solid var(--line);padding:16px;flex-shrink:0;">
<div style="font-weight:600;color:var(--text);margin-bottom:16px;">Rooms</div>
<div style="display:flex;flex-direction:column;gap:8px;"><div class="sidebar-nav-item active" style="padding:8px 12px;border-radius:var(--radius-sm);"># general</div><div class="sidebar-nav-item" style="padding:8px 12px;border-radius:var(--radius-sm);"># dev</div><div class="sidebar-nav-item" style="padding:8px 12px;border-radius:var(--radius-sm);"># design</div></div>
</div>
<div style="flex:1;display:flex;flex-direction:column;">
<div style="padding:12px 20px;border-bottom:1px solid var(--line);display:flex;align-items:center;justify-content:space-between;"><span style="font-weight:600;color:var(--text);"># general</span><span style="font-size:0.8rem;color:var(--text-dim);">3 members online</span></div>
<div class="chat-messages" style="flex:1;"><div class="chat-message"><div class="avatar" style="width:32px;height:32px;font-size:0.75rem;flex-shrink:0;">AI</div><div><div style="font-size:0.8rem;color:var(--text-dim);margin-bottom:4px;">Agent <span style="color:var(--cyan);">●</span></div><div class="chat-bubble"><p>I've generated the dashboard. Take a look at the metrics panel.</p></div></div></div><div class="chat-message own"><div><div style="font-size:0.8rem;color:var(--text-dim);margin-bottom:4px;text-align:right;">You</div><div class="chat-bubble"><p>Looks great! Can you add a latency chart?</p></div></div><div class="avatar" style="width:32px;height:32px;font-size:0.75rem;flex-shrink:0;background:linear-gradient(135deg,var(--ember),var(--yellow));">U</div></div></div>
<div class="chat-composer"><input type="text" class="input chat-input" placeholder="Type a message..." style="flex:1;"><button class="btn btn-primary">Send</button></div>
</div>
</div>
</div>'''

# === SECTION 10: CRM INTERFACE ===
crm_interface = '''<div class="card" style="padding:0;overflow:hidden;">
<div style="padding:20px 24px;border-bottom:1px solid var(--line);display:flex;align-items:center;justify-content:space-between;"><div><div style="font-family:var(--font-display);font-weight:700;font-size:1.2rem;color:var(--text);">Contacts</div><div style="font-size:0.85rem;color:var(--text-dim);">1,247 total contacts</div></div><div style="display:flex;gap:12px;"><input type="search" class="input" placeholder="Search contacts..." style="width:240px;"><button class="btn btn-primary">+ Add Contact</button></div></div>
<div style="padding:24px;">
<table class="table"><thead><tr><th><label class="checkbox"><input type="checkbox"></label></th><th>Name</th><th>Company</th><th>Status</th><th>Last Activity</th><th>Actions</th></tr></thead><tbody><tr><td><label class="checkbox"><input type="checkbox"></label></td><td><div style="display:flex;align-items:center;gap:10px;"><div class="avatar" style="width:32px;height:32px;font-size:0.75rem;">AC</div><div><div style="font-weight:600;color:var(--text);">Alice Chen</div><div style="font-size:0.8rem;color:var(--text-dim);">alice@example.com</div></div></div></td><td>Acme Corp</td><td><span class="table-badge badge-green">Customer</span></td><td>2 hours ago</td><td><button class="btn btn-icon btn-ghost" style="width:32px;height:32px;">✏️</button></td></tr><tr><td><label class="checkbox"><input type="checkbox"></label></td><td><div style="display:flex;align-items:center;gap:10px;"><div class="avatar" style="width:32px;height:32px;font-size:0.75rem;background:linear-gradient(135deg,var(--ember),var(--yellow));">BS</div><div><div style="font-weight:600;color:var(--text);">Bob Smith</div><div style="font-size:0.8rem;color:var(--text-dim);">bob@example.com</div></div></div></td><td>TechStart</td><td><span class="table-badge badge-yellow">Lead</span></td><td>1 day ago</td><td><button class="btn btn-icon btn-ghost" style="width:32px;height:32px;">✏️</button></td></tr></tbody></table>
<div class="pagination" style="margin-top:20px;justify-content:center;"><button class="page-btn" disabled>←</button><button class="page-btn active">1</button><button class="page-btn">2</button><button class="page-btn">3</button><button class="page-btn">→</button></div>
</div>
</div>'''

# === SECTION 11: EMR INTERFACE ===
emr_interface = '''<div class="card" style="padding:0;overflow:hidden;">
<div style="padding:20px 24px;border-bottom:1px solid var(--line);display:flex;align-items:center;gap:16px;"><div class="avatar" style="width:48px;height:48px;font-size:1.1rem;">JP</div><div><div style="font-family:var(--font-display);font-weight:700;font-size:1.2rem;color:var(--text);">John Patient</div><div style="font-size:0.85rem;color:var(--text-dim);">DOB: 1985-03-12 | MRN: 12345678</div></div><span class="table-badge badge-green" style="margin-left:auto;">Active</span></div>
<div style="padding:24px;">
<div class="grid-4" style="margin-bottom:24px;"><div class="emr-vital-sign"><div class="emr-vital-icon">❤️</div><div><div class="emr-vital-value">72</div><div class="emr-vital-label">HR bpm</div><div class="emr-vital-trend up">↑ 2</div></div></div><div class="emr-vital-sign"><div class="emr-vital-icon" style="background:var(--green-tint);color:var(--green);">🫁</div><div><div class="emr-vital-value">16</div><div class="emr-vital-label">RR /min</div></div></div><div class="emr-vital-sign"><div class="emr-vital-icon" style="background:var(--yellow-tint);color:var(--yellow);">🌡️</div><div><div class="emr-vital-value">98.6</div><div class="emr-vital-label">Temp °F</div></div></div><div class="emr-vital-sign"><div class="emr-vital-icon" style="background:var(--cyan-tint);color:var(--cyan);">💉</div><div><div class="emr-vital-value">120/80</div><div class="emr-vital-label">BP mmHg</div></div></div></div>
<div style="display:flex;gap:4px;padding:4px;background:var(--surface-3);border-radius:var(--radius-md);margin-bottom:20px;"><button class="tab-btn active">Vitals</button><button class="tab-btn">Medications</button><button class="tab-btn">Labs</button><button class="tab-btn">Notes</button></div>
<div class="timeline"><div class="timeline-item"><div class="timeline-dot" style="border-color:var(--green);">✓</div><div class="timeline-content"><div class="timeline-time">Today 09:00</div><div class="timeline-title">Routine checkup completed</div></div></div><div class="timeline-item"><div class="timeline-dot" style="border-color:var(--cyan);">💊</div><div class="timeline-content"><div class="timeline-time">Yesterday 14:00</div><div class="timeline-title">Prescription updated: Lisinopril 10mg</div></div></div></div>
</div>
</div>'''

# === SECTION 12: GENERATIVE AI INTERFACES ===
gen_image = '''<div class="card" style="padding:0;overflow:hidden;">
<div style="padding:20px 24px;border-bottom:1px solid var(--line);"><div style="font-family:var(--font-display);font-weight:700;font-size:1.2rem;color:var(--text);">Text-to-Image</div></div>
<div style="padding:24px;">
<div class="gen-prompt-area" style="margin-bottom:24px;">
<div class="form-group"><label class="label">Prompt</label><textarea class="input textarea" placeholder="A futuristic cityscape at sunset with neon lights..." style="min-height:80px;">A futuristic cityscape at sunset with neon lights reflecting off glass towers, cyberpunk aesthetic, high detail</textarea></div>
<div style="display:flex;gap:12px;align-items:flex-end;">
<div class="form-group" style="flex:1;margin-bottom:0;"><label class="label">Style</label><select class="input select"><option>Cyberpunk</option><option>Realistic</option><option>Anime</option><option>Oil Painting</option></select></div>
<div class="form-group" style="flex:1;margin-bottom:0;"><label class="label">Aspect Ratio</label><select class="input select"><option>16:9</option><option>1:1</option><option>9:16</option></select></div>
<button class="btn btn-primary" style="height:46px;"><span class="spinner" style="display:none;"></span> Generate</button>
</div>
</div>
<div class="gen-image-grid">
<div class="gen-image-item" style="background:linear-gradient(135deg,var(--surface-4),var(--surface-3));display:flex;align-items:center;justify-content:center;"><span style="font-size:3rem;opacity:0.3;">🖼️</span></div>
<div class="gen-image-item" style="background:linear-gradient(135deg,var(--surface-4),var(--surface-3));display:flex;align-items:center;justify-content:center;"><span style="font-size:3rem;opacity:0.3;">🖼️</span></div>
<div class="gen-image-item" style="background:linear-gradient(135deg,var(--surface-4),var(--surface-3));display:flex;align-items:center;justify-content:center;"><span style="font-size:3rem;opacity:0.3;">🖼️</span></div>
<div class="gen-image-item" style="background:linear-gradient(135deg,var(--surface-4),var(--surface-3));display:flex;align-items:center;justify-content:center;"><span style="font-size:3rem;opacity:0.3;">🖼️</span></div>
</div>
</div>
</div>'''

# === SECTION 13: VIDEO STREAMING ===
video_streaming = '''<div class="card" style="padding:0;overflow:hidden;">
<div style="display:flex;flex-wrap:wrap;">
<div style="flex:1;min-width:300px;">
<div class="video-player">
<div class="video-play-btn">▶</div>
<div class="video-controls">
<span class="video-time">12:34</span>
<div class="video-timeline"><div class="video-timeline-fill"></div></div>
<span class="video-time">36:15</span>
</div>
</div>
<div style="padding:20px 24px;"><div style="font-family:var(--font-display);font-weight:700;font-size:1.2rem;color:var(--text);margin-bottom:8px;">Building AI-Native Interfaces with HTMX</div><div style="font-size:0.85rem;color:var(--text-dim);">1.2K views · 2 days ago</div></div>
</div>
<div style="width:300px;min-width:300px;border-left:1px solid var(--line);padding:16px;">
<div style="font-weight:600;color:var(--text);margin-bottom:12px;">Up Next</div>
<div style="display:flex;flex-direction:column;gap:12px;">
<div style="display:flex;gap:12px;align-items:center;"><div style="width:100px;height:60px;background:var(--surface-3);border-radius:var(--radius-sm);flex-shrink:0;"></div><div><div style="font-weight:500;color:var(--text);font-size:0.85rem;">Introduction to A2UI</div><div style="font-size:0.75rem;color:var(--text-dim);">15 min</div></div></div>
<div style="display:flex;gap:12px;align-items:center;"><div style="width:100px;height:60px;background:var(--surface-3);border-radius:var(--radius-sm);flex-shrink:0;"></div><div><div style="font-weight:500;color:var(--text);font-size:0.85rem;">Component Registry Deep Dive</div><div style="font-size:0.75rem;color:var(--text-dim);">28 min</div></div></div>
</div>
</div>
</div>
</div>'''

# Build the full page
page_parts = [hero_html]

# Component Primitives section
page_parts.append('''<section class="container" id="layout" style="padding:60px 24px 40px;">
<div class="section-header"><div class="section-number">01</div><div class="section-title">Layout Primitives</div></div>
''' + layout_section + '''</section>''')

page_parts.append('''<section class="container" style="padding:40px 24px;">
<div class="section-header"><div class="section-number">02</div><div class="section-title">Data Display</div></div>
''' + data_display_section + '''</section>''')

page_parts.append('''<section class="container" style="padding:40px 24px;">
<div class="section-header"><div class="section-number">03</div><div class="section-title">Input Primitives</div></div>
''' + input_section + '''</section>''')

page_parts.append('''<section class="container" style="padding:40px 24px;">
<div class="section-header"><div class="section-number">04</div><div class="section-title">Action Primitives</div></div>
''' + action_section + '''</section>''')

page_parts.append('''<section class="container" style="padding:40px 24px;">
<div class="section-header"><div class="section-number">05</div><div class="section-title">Agent Primitives</div></div>
''' + agent_section + '''</section>''')

page_parts.append('''<section class="container" style="padding:40px 24px;">
<div class="section-header"><div class="section-number">06</div><div class="section-title">Navigation</div></div>
''' + nav_section + '''</section>''')

# Domain Examples section
page_parts.append('''<section class="container" id="domains" style="padding:60px 24px 40px;">
<div class="section-header"><div class="section-number">07</div><div class="section-title">Domain Examples</div></div>
<h3>Admin Dashboard Shell</h3>''' + admin_shell + '''<div style="height:40px;"></div>
<h3>Chat / Messaging Interface</h3>''' + chat_interface + '''<div style="height:40px;"></div>
<h3>CRM Interface</h3>''' + crm_interface + '''<div style="height:40px;"></div>
<h3>EMR Medical Interface</h3>''' + emr_interface + '''<div style="height:40px;"></div>
<h3>Generative AI (Text-to-Image)</h3>''' + gen_image + '''<div style="height:40px;"></div>
<h3>Video Streaming</h3>''' + video_streaming + '''</section>''')

# Footer
page_parts.append('''<div style="padding:60px 24px;text-align:center;border-top:1px solid var(--line);margin-top:40px;">
<div style="font-family:var(--font-display);font-weight:700;font-size:1.5rem;color:var(--text);margin-bottom:12px;">Flint A2UI Component Registry</div>
<div style="font-size:0.9rem;color:var(--text-dim);margin-bottom:24px;">KnowMe Design System · Dark Theme · Ember & Cyan</div>
<div style="font-family:var(--font-mono);font-size:0.8rem;color:var(--text-dim);">RFC-FORGE-A2UI-001 · June 2026</div>
</div>''')

# Command palette (global)
page_parts.append('''<div class="command-palette-overlay" x-data="{open:false}" :class="open?'is-open':''" @keydown.window.meta.k.prevent="open=true">
<div class="command-palette">
<input type="text" class="command-input" placeholder="Search components..." x-ref="cmdInput" @keydown.escape="open=false">
<div class="command-list">
<div class="command-item"><div class="command-item-icon">📐</div><div><div class="command-item-text">Layout Primitives</div><div class="command-item-desc">Stack, Card, Grid, Tabs, Modal...</div></div></div>
<div class="command-item"><div class="command-item-icon">📊</div><div><div class="command-item-text">Data Display</div><div class="command-item-desc">Table, Badge, Progress, Timeline...</div></div></div>
<div class="command-item"><div class="command-item-icon">⌨️</div><div><div class="command-item-text">Input Primitives</div><div class="command-item-desc">TextField, Select, Switch, DatePicker...</div></div></div>
</div>
</div>
</div>''')

# Toast container
page_parts.append('''<div class="toast-container">
<div class="toast toast-success is-visible"><div class="toast-icon">✓</div><div><div class="toast-title">Success</div><div class="toast-body">Component saved successfully</div></div></div>
</div>''')

# Full page assembly
html_content = '<body>\n' + '\n'.join(page_parts) + '\n</body>\n</html>'

# Build head
head = build_head()

# Write output
full_output = head + '\n' + html_content
with open(OUT, 'w') as f:
    f.write(full_output)

print(f"Wrote {len(full_output)} bytes to {OUT}")
print(f"Sections: {len(page_parts)}")

if __name__ == '__main__':
    write_html()
