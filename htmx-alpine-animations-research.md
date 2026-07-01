
---

## 7. Real-Time Streaming Updates (SSE)

### 7.1 SSE Fundamentals

Server-Sent Events (SSE) provide one-way, server-to-client streaming over standard HTTP. The browser opens a persistent connection, and the server pushes events as they occur.

**Wire format:**
```
event: notification
data: <div class="alert">New message from Alice</div>

id: 123
retry: 5000
data: <div class="update">Price updated</div>
```

### 7.2 HTMX SSE Extension Setup

```html
<head>
  <script src="https://cdn.jsdelivr.net/npm/htmx.org@latest/dist/htmx.min.js"></script>
  <script src="https://cdn.jsdelivr.net/npm/htmx-ext-sse@latest"></script>
</head>
<body hx-ext="sse">
```

**Note:** In HTMX v4.x, the SSE extension has been rewritten. `sse-connect` and `sse-swap` still work but emit deprecation warnings. The new pattern uses `hx-sse:connect`.

### 7.3 Basic SSE Feed (Live Notifications)

```html
<!-- Live notification stream -->
<div hx-ext="sse" sse-connect="/events/notifications" sse-swap="message">
  <ul id="notification-list" aria-live="polite">
    <li class="placeholder">No notifications yet</li>
  </ul>
</div>
```

Server (Go example):
```go
func eventsHandler(w http.ResponseWriter, r *http.Request) {
    w.Header().Set("Content-Type", "text/event-stream")
    w.Header().Set("Cache-Control", "no-cache")
    w.Header().Set("Connection", "keep-alive")
    
    flusher, ok := w.(http.Flusher)
    if !ok {
        http.Error(w, "Streaming unsupported", http.StatusInternalServerError)
        return
    }
    
    for {
        select {
        case msg := <-messageChannel:
            fmt.Fprintf(w, "data: %s

", msg)
            flusher.Flush()
        case <-r.Context().Done():
            return
        }
    }
}
```

### 7.4 Multiple Event Types on One Stream

```html
<div hx-ext="sse" sse-connect="/stream">
  <!-- Stats update -->
  <div sse-swap="statsUpdate"></div>
  
  <!-- Alert banner -->
  <div sse-swap="alertBanner"></div>
  
  <!-- Chat message -->
  <div sse-swap="chatMessage" hx-swap="beforeend"></div>
</div>
```

### 7.5 SSE Triggering HTMX Requests (Signal Pattern)

Instead of swapping HTML directly, use SSE to trigger HTMX requests for full fragments:

```html
<div hx-ext="sse" sse-connect="/events">
  <!-- When "newNotification" event arrives, HTMX fetches updated list -->
  <div hx-get="/notifications" 
       hx-trigger="sse:newNotification"
       hx-target="#notification-list"
       hx-swap="innerHTML">
  </div>
</div>
```

**Benefits:**
- Server can just send lightweight signal events (e.g., `event: newNotification

`)
- HTMX handles fetching the properly rendered HTML fragment
- Separation of concerns: SSE coordinates, HTMX renders

### 7.6 Graceful Close

```html
<div hx-ext="sse" sse-connect="/stream" sse-close="done">
  <!-- Process completes, server sends: event: done

 -->
  <!-- Connection closes automatically -->
</div>
```

### 7.7 Chat with WebSockets (Bidirectional)

For true bidirectional real-time (chat, collaborative editing), use WebSockets instead:

```html
<div hx-ext="ws" ws-connect="/ws/chat/">
  <div id="chat-messages" aria-live="polite"></div>
  <form ws-send>
    <input type="text" name="message" placeholder="Type a message..." />
    <button type="submit">Send</button>
  </form>
</div>
```

### 7.8 SSE vs WebSockets Decision Matrix

| Use SSE when... | Use WebSockets when... |
|---|---|
| Server pushes updates to client | Client and server both send messages |
| Notifications, progress bars, live feeds | Chat, collaborative editing, gaming |
| HTML fragments for HTMX to swap | Binary data or high-frequency messaging |
| Want HTTP semantics (caching, auth, proxies) | Need persistent bidirectional channel |

### 7.9 Performance Considerations

- **HTTP/1.1 connection limit:** Browsers cap connections per domain at ~6. Multiple SSE tabs can exhaust this.
- **HTTP/2 recommended:** Multiplexes streams over a single connection, removing the per-domain cap.
- **Auto-reconnection:** SSE reconnects automatically with exponential backoff. Server can control `retry` interval.
- **Event IDs:** Include `id:` fields so the browser can resume from the last received event after reconnect.
- **Connection cleanup:** Always close SSE connections when components are removed (use `sse-close` or manual cleanup).
- **SSE for signals, not data:** For large payloads, send a signal event and let HTMX fetch the rendered HTML.

---

## 8. Progressive Enhancement for Admin Shell Interfaces

### 8.1 The Progressive Enhancement Philosophy

Start with working HTML forms and links, then layer HTMX on top. When JavaScript fails, the application still works.

```html
<!-- Search form: works with or without JS -->
<form action="/search/" method="GET"
      hx-get="/search/"
      hx-target="#results"
      hx-push-url="true">
  <input type="text" name="q" required />
  <button type="submit">Search</button>
</form>

<div id="results"></div>
```

### 8.2 Server-Side HTMX Detection

The server must differentiate HTMX requests from regular requests:

```python
def search(request):
    query = request.GET.get('q', '')
    results = Product.objects.filter(name__icontains=query)
    context = {'results': results, 'query': query}
    
    if request.headers.get('HX-Request'):
        # HTMX request: return only the fragment
        return render(request, 'search_results.html', context)
    else:
        # Regular request: return full page
        return render(request, 'search_page.html', context)
```

**Key header:** `HX-Request: true` is sent by HTMX on all requests.

### 8.3 hx-boost for SPA-like Navigation

The fastest way to progressively enhance an entire admin shell:

```html
<body hx-boost="true" hx-sync="this:replace">
  <nav>
    <a href="/dashboard">Dashboard</a>
    <a href="/users">Users</a>
    <a href="/settings">Settings</a>
  </nav>
  
  <main id="content">
    <!-- Page content -->
  </main>
</body>
```

**Behavior:**
- With HTMX: clicks become AJAX requests, content swaps, URL updates via `pushState`
- Without HTMX: normal browser navigation
- Add `hx-boost="false"` on links that should remain full page loads (e.g., downloads)

### 8.4 Admin Shell Architecture

```html
<!DOCTYPE html>
<html lang="en" class="dark">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>Admin Dashboard</title>
  <script src="/htmx.min.js"></script>
  <script src="/alpine.min.js" defer></script>
  <link rel="stylesheet" href="/admin.css">
</head>
<body hx-boost="true" hx-sync="this:replace">
  
  <div x-data="adminShell()" class="admin-shell">
    <!-- Sidebar -->
    <aside class="sidebar" :class="{ 'collapsed': !sidebarOpen }">
      <nav class="nav-menu">
        <a href="/dashboard" class="nav-item" 
           :class="{ 'active': currentPage === 'dashboard' }">
          <span class="icon">📊</span>
          <span class="label" x-show="sidebarOpen" x-collapse>Dashboard</span>
        </a>
        <a href="/users" class="nav-item">
          <span class="icon">👥</span>
          <span class="label" x-show="sidebarOpen" x-collapse>Users</span>
        </a>
      </nav>
    </aside>
    
    <!-- Top bar -->
    <header class="top-bar">
      <button @click="sidebarOpen = !sidebarOpen" class="toggle-btn">
        ☰
      </button>
      <div class="breadcrumbs" hx-boost="false">
        <!-- Breadcrumbs for non-JS fallback -->
      </div>
      <div class="actions">
        <!-- Global actions -->
      </div>
    </header>
    
    <!-- Main content -->
    <main id="main-content" class="main-content">
      <!-- Page content loaded here via HTMX boosting or manual swaps -->
    </main>
  </div>
  
  <script>
    function adminShell() {
      return {
        sidebarOpen: localStorage.getItem('sidebar-open') !== 'false',
        currentPage: window.location.pathname.split('/')[1] || 'dashboard',
        init() {
          this.$watch('sidebarOpen', val => {
            localStorage.setItem('sidebar-open', val);
          });
        }
      }
    }
  </script>
</body>
</html>
```

### 8.5 Multi-Target Updates with OOB Swaps

Admin dashboards often need to update multiple areas from a single action:

```html
<!-- Server response to "Approve Order" action -->
<tr id="order-123">
  <td>Order #123</td>
  <td><span class="badge badge-success">Approved</span></td>
</tr>

<!-- OOB: Update stats card -->
<div id="pending-count" hx-swap-oob="true">
  <span class="stat-value">47</span>
  <span class="stat-label">Pending</span>
</div>

<!-- OOB: Add to activity log -->
<li hx-swap-oob="beforeend:#activity-log" class="log-entry htmx-added">
  <span class="time">14:32</span>
  <span class="action">Order #123 approved by Alice</span>
</li>

<!-- OOB: Toast notification -->
<div id="toast-container" hx-swap-oob="beforeend">
  <div class="toast toast-success" role="alert">
    Order approved successfully
  </div>
</div>
```

### 8.6 Error Handling & Resilience

```javascript
document.body.addEventListener('htmx:beforeSwap', function(evt) {
  const status = evt.detail.xhr.status;
  
  if (status === 404) {
    evt.detail.shouldSwap = true;
    evt.detail.target = htmx.find("#not-found-panel");
  } else if (status === 422) {
    // Validation errors: swap the re-rendered form
    evt.detail.shouldSwap = true;
    evt.detail.isError = false;
  } else if (status === 500) {
    evt.detail.shouldSwap = true;
    evt.detail.target = htmx.find("#error-toast");
  }
});

// Global network error handler
document.body.addEventListener('htmx:sendError', function() {
  showToast('Network error. Please check your connection.', 'error');
});
```

### 8.7 Polling for Live Dashboards

```html
<!-- Auto-refresh status dashboard every 30 seconds -->
<div hx-get="/dashboard/stats" 
     hx-trigger="every 30s"
     hx-swap="innerHTML transition:true">
  <!-- Stats content -->
</div>

<!-- Polling status indicator -->
<div class="sync-indicator"
     hx-get="/sync-status"
     hx-trigger="every 5s"
     hx-swap="outerHTML">
  <span class="status-dot status-online"></span>
  <span class="status-text">Last synced: 2s ago</span>
</div>
```

### 8.8 HTMX + Alpine.js for Admin Shell: Decision Matrix

| Feature | HTMX | Alpine.js | Why? |
|---|---|---|---|
| Page navigation | ✅ hx-boost | ❌ | Server renders full pages |
| Form submission | ✅ hx-post | ❌ | Server handles validation/state |
| Sidebar toggle | ❌ | ✅ x-data | Client-only state, no server roundtrip |
| Modal dialogs | ❌ | ✅ x-show + x-trap | Client state, focus management |
| Dropdown menus | ❌ | ✅ x-show + @click.outside | Client-only interaction |
| Data tables (sort, page) | ✅ hx-get | ❌ | Server-side pagination/sorting |
| Live search/filter | ✅ hx-get + delay | ✅ x-model | Debounced input, server results |
| Dark mode toggle | ❌ | ✅ $persist | Client preference, localStorage |
| Toast notifications | ✅ OOB swap | ❌ | Server signals, DOM insertion |
| Inline editing | ✅ hx-put | ❌ | Server persists changes |
| Drag-and-drop reorder | ❌ | ✅ or custom JS | Complex client interaction |
| Charts/graphs | ❌ | ❌ (use library) | D3, Chart.js, etc. |

### 8.9 Best Practices & Gotchas

**DO:**
- Always provide `method` and `action` on forms as fallback
- Use `hx-push-url="true"` for navigation so back/forward buttons work
- Set `HX-Request` detection on server to return fragments vs full pages
- Include `<noscript>` tags with a friendly message for JS-disabled users
- Use `hx-sync="this:replace"` to prevent duplicate requests from rapid clicks
- Add `aria-live` regions for HTMX-updated content that screen readers should announce
- Use skeleton screens instead of spinners for content loading to reduce perceived wait time

**DON'T:**
- Return full pages for HTMX requests - only return HTML fragments
- Forget to set `hx-boost="false"` on links that need full page loads (downloads, external links)
- Use HTMX for purely client-side state (e.g., dark mode toggle) - use Alpine instead
- Block the UI during HTMX requests - use `.htmx-request` styling for feedback instead

### 8.10 Performance Optimization for Admin Shells

- **Preload links:** Use the `preload` extension to prefetch on hover/focus
- **Lazy load tabs:** Use `hx-trigger="revealed once"` to load tab content only when first shown
- **Selective OOB updates:** Only swap elements that actually changed; avoid full dashboard re-renders
- **Debounce search:** Use `keyup changed delay:300ms` for search inputs
- **Morph swaps:** Use `hx-swap="morph"` for complex DOM updates that need to preserve focus/scroll
- **Cache fragments:** Cache commonly rendered fragments server-side (e.g., sidebar navigation)
- **Connection pooling:** For SSE, use HTTP/2 to avoid per-domain connection limits

---

## Appendix A: HTMX + Alpine.js Integration Checklist

```markdown
□ Load Alpine.js with `defer` attribute
□ Load HTMX without `defer` (or with `defer` after Alpine)
□ Include `[x-cloak] { display: none !important; }` CSS rule
□ Set `htmx.config.reportValidityOfForms = true` for HTML5 validation
□ Use `hx-sync="this:replace"` on boosted containers
□ Server detects `HX-Request` header to return fragments
□ Include `x-cloak` on elements with `x-show` that default to hidden
□ Use `x-trap` for modals, `x-collapse` for accordions
□ Use `$persist` for UI state that should survive reloads
□ Use `htmx:beforeSwap` event to handle 422/404/500 status codes
□ Respect `prefers-reduced-motion` for accessibility
□ Test without JavaScript to verify progressive enhancement
```

## Appendix B: Dark Theme Animation Token Reference

| Token | Light | Dark | Usage |
|---|---|---|---|
| `--bg-primary` | `#ffffff` | `#0f172a` | Page background |
| `--bg-secondary` | `#f8fafc` | `#1e293b` | Card/panel background |
| `--bg-tertiary` | `#e2e8f0` | `#334155` | Input/elevated background |
| `--text-primary` | `#0f172a` | `#f8fafc` | Primary text |
| `--text-secondary` | `#475569` | `#94a3b8` | Secondary text |
| `--accent` | `#4f46e5` | `#6366f1` | Primary accent |
| `--accent-glow` | `rgba(79,70,229,0.15)` | `rgba(99,102,241,0.3)` | Glow/shadow |
| `--border` | `#e2e8f0` | `#334155` | Default borders |
| `--border-hover` | `#cbd5e1` | `#475569` | Hover borders |
| `--transition-fast` | `150ms` | `150ms` | Micro-interactions |
| `--transition-base` | `200ms` | `200ms` | Standard transitions |
| `--transition-slow` | `300ms` | `300ms` | Page/section transitions |

---

> **Sources:** This research was compiled from HTMX documentation (htmx.org), Alpine.js documentation (alpinejs.dev), Ben Nadel's HTMX+Alpine integration guide, HTMX examples and essays, OpenReplay HTMX tutorials, and various community implementations including fasthx-admin, Django Unfold, and Hypermedia Systems patterns.
