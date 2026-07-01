# UI Patterns & Component Compositions — Technical Research Brief
**Date:** 2025-08-06
**Domains Covered:** 12
**Focus:** HTMX-native interface architectures, component primitives, layout structures, data flow patterns, and accessibility considerations

---

## 1. Admin Dashboard / Shell Interface

### Key Screen/Component Breakdown
| Component | Primitive(s) | Role |
|-----------|-------------|------|
| Sidebar Navigation | `<nav>`, `<ul>`, `<a>`, Collapsible Group | Primary wayfinding, module hierarchy |
| Top Bar | `<header>`, Search Input, Notification Bell, User Avatar | Global actions, identity, context |
| Content Area | `<main>`, `<section>`, Breadcrumb | Page-specific content container |
| Breadcrumbs | `<nav aria-label="breadcrumb">`, `<ol>` | Hierarchical wayfinding, history |
| Command Palette | Modal Overlay, `<input>`, Result List | Keyboard-driven navigation (⌘+K) |
| Status Indicators | Badge, Dot, Tooltip, Toast | System health, notifications, alerts |

### Layout Structure
```
┌─────────────────────────────────────────────────┐
│ [≡] [Logo]  [Search]  [Alerts] [User]          │  Top Bar (fixed, h-16)
├──────────┬──────────────────────────────────────┤
│          │ [Home] > [Users] > [Edit]           │  Breadcrumb (h-12)
│ Sidebar  ├──────────────────────────────────────┤
│ (w-64)   │                                      │
│  ─────   │         Content Area                 │
│  Nav     │         (flex-1, overflow-auto)      │
│  Items   │                                      │
│  ─────   │                                      │
│  ─────   │                                      │
│          │                                      │
└──────────┴──────────────────────────────────────┘
```
- **Grid:** CSS Grid with `grid-template-columns: auto 1fr` for sidebar + content
- **Flex:** Column flex for top bar stack; row flex for sidebar content groups
- **Collapsible:** Sidebar uses `transform: translateX` or `width` transition (prefers `translateX` for performance)

### Data Flow Patterns
| Pattern | HTMX Strategy | Endpoint Design |
|---------|--------------|-----------------|
| Navigation | `hx-boost="true"` on nav links | Full page swaps with partial content fragments |
| Command Palette | `hx-get` on input debounce | `/api/search?q={input}` returns `<ul>` of results |
| Notifications | `hx-get` polling or SSE | `/api/notifications` returns badge count + list |
| Status Indicators | WebSocket or polling | Real-time connection status via `/api/health` |

### Special UX Considerations
- **Accessibility:** Sidebar needs `aria-expanded`, focus trapping in command palette, skip links
- **Performance:** Sidebar state persisted in `localStorage`; command palette lazy-loaded
- **Mobile:** Sidebar becomes off-canvas drawer (`<dialog>` or `transform`); top bar compresses to hamburger + logo + avatar
- **Keyboard:** `⌘/Ctrl+K` for command palette, `Escape` to close modals, arrow keys for navigation

### Recommended HTMX Swap Strategies
| Interaction | HTMX Attribute | Swap Target | Swap Style |
|-------------|---------------|-------------|------------|
| Nav click (boosted) | `hx-boost="true"` | `<body>` | `innerHTML` (full page) |
| Search command | `hx-get="/api/search"` | `#command-results` | `innerHTML` with debounce |
| Notification bell | `hx-get="/api/notifications"` | `#notification-dropdown` | `outerHTML` on click |
| User menu | `hx-get` | `#user-menu` | `toggle` visibility |
| Breadcrumb nav | `hx-boost` or `hx-get` | `main` | `innerHTML` with URL push |

---

## 2. Chat / Messaging Interface (Matrix.org Style)

### Key Screen/Component Breakdown
| Component | Primitive(s) | Role |
|-----------|-------------|------|
| Message List | `<ul>` / `<div>`, Virtual Scroller | Chronological message display |
| Message Bubble | `<div>`, Avatar, Timestamp, Content | Individual message container |
| Composer | `<textarea>`, Send Button, Attachment | Message input and submission |
| Reactions | Button group, Emoji, Count | Quick sentiment on messages |
| Thread View | Collapsible panel, Reply list | Threaded conversation branching |
| Presence Indicators | Dot, Tooltip, Status text | Online/away/offline status |
| File Attachments | Card, Progress bar, Preview | Uploaded media display |
| Read Receipts | Avatar stack, Tooltip | Delivery acknowledgment |

### Layout Structure
```
┌────────────────────────────────────────┐
│ [Room List]  [Message Area]            │
│  ┌─────┐    ┌────────────────────┐    │
│  │Room1│    │ [Message]          │    │
│  │Room2│    │ [Message]          │    │
│  │Room3│    │ [Message]          │    │
│  │ ... │    │         ↑ scroll    │    │
│  │ ... │    │ [Message]          │    │
│  └─────┘    │ [Message]          │    │
│             ├────────────────────┤    │
│             │ [Composer] [+][📎]  │    │
│             └────────────────────┘    │
└────────────────────────────────────────┘
```
- **Flex:** Column flex for message list (scrolls up); row flex for main container
- **Virtual Scroll:** Essential for rooms with 10k+ messages (only render visible items)
- **Sticky:** Composer sticks to bottom; unread banner sticks to top

### Data Flow Patterns
| Pattern | Implementation | Notes |
|---------|---------------|-------|
| Real-time sync | WebSocket / SSE (Matrix: /sync) | Incremental sync with sync tokens |
| Message sending | Optimistic UI + WebSocket | Show immediately, confirm server receipt |
| History loading | `hx-get` with scroll trigger | Paginate backwards (`/messages?from=token`) |
| Thread loading | `hx-get` on expand | Load thread replies on demand |
| File upload | `hx-post` with `multipart/form-data` | Progress via `hx-trigger` on upload progress |

### Special UX Considerations
- **Accessibility:** Screen reader announcements for new messages (`aria-live="polite"`), keyboard navigation between messages
- **Performance:** Virtual scrolling is mandatory; image lazy loading; collapse old threads
- **Mobile:** Room list becomes bottom nav or drawer; composer becomes full-width with attachment FAB
- **Security:** End-to-end encryption indicators (shield icon); verification flows for devices
- **RTL:** Message alignment flips for RTL languages; timestamps move to opposite side

### Recommended HTMX Swap Strategies
| Interaction | HTMX Attribute | Swap Target | Swap Style |
|-------------|---------------|-------------|------------|
| New message (WS) | `ws-send` / `hx-swap-oob` | Append to `#messages` | `beforeend` with scroll |
| Load older messages | `hx-get` + scroll trigger | `#messages` | `afterbegin` (prepend) |
| Send message | `hx-post` + optimistic | `#messages` | `beforeend` + composer reset |
| Thread expand | `hx-get` | `#thread-panel` | `innerHTML` |
| Reaction add | `hx-post` | `#reactions-{msgId}` | `outerHTML` |
| Room switch | `hx-get` | `#message-area` | `innerHTML` with URL update |

---

## 3. Video Streaming Interface

### Key Screen/Component Breakdown
| Component | Primitive(s) | Role |
|-----------|-------------|------|
| Video Player | `<video>` or custom canvas | Core media playback |
| Playlist | `<ul>`, Thumbnail, Title, Duration | Up next / queue management |
| Controls | Overlay bar, Buttons, Sliders | Play/pause, seek, volume, settings |
| Chat Overlay | Side panel or bottom overlay | Live chat (if streaming) |
| Quality Selector | Dropdown, Badge (current) | Resolution / bitrate selection |
| Theater Mode | Expanded width layout | Distraction-free viewing |
| Picture-in-Picture | `<video>` API or floating div | Floating mini player |

### Layout Structure
```
┌────────────────────────────────────────────────────┐
│  [Video Player]                          [Chat]   │
│  ┌────────────────────────┐           ┌─────────┐│
│  │                        │           │ Chat    ││
│  │      Video Area        │           │ Message ││
│  │                        │           │  List   ││
│  │  [Controls Overlay]    │           │         ││
│  └────────────────────────┘           │ [Composer││
│  [Title] [Views] [Like] [Share]       └─────────┘│
│  ───────────────────────────────────────────────── │
│  [Up Next]  [Playlist Item] [Playlist Item]...   │
└────────────────────────────────────────────────────┘
```
- **Grid:** `1fr 300px` for video + chat; collapses to single column on mobile
- **Flex:** Column for controls overlay; row for action buttons below video
- **Theater Mode:** Expands video to `100vw` or max-width; collapses secondary panels

### Data Flow Patterns
| Pattern | Implementation | Notes |
|---------|---------------|-------|
| Playback | Native `<video>` API + HLS/DASH | Adaptive bitrate streaming |
| Chat (live) | WebSocket | Real-time message stream |
| Playlist | `hx-get` on load | `/api/playlist?videoId=xyz` |
| Quality switch | Client-side video API | `video.setVideoQuality()` or HLS level switch |
| Theater mode | Client-side toggle | CSS class toggle, no server request |
| PiP | `requestPictureInPicture()` | Native API, no server request |

### Special UX Considerations
- **Accessibility:** Keyboard controls (space=play, arrows=seek/volume), captions/subtitles track, focus visible on controls
- **Performance:** Lazy load playlist thumbnails; buffer management; cleanup on unmount
- **Mobile:** Native fullscreen on rotate; controls auto-hide; swipe gestures for seek
- **Bandwidth:** Auto quality based on `connection.effectiveType`; manual override available
- **DRM:** Encrypted media extensions (EME) for protected content

### Recommended HTMX Swap Strategies
| Interaction | HTMX Attribute | Swap Target | Swap Style |
|-------------|---------------|-------------|------------|
| Load playlist | `hx-get` | `#playlist` | `innerHTML` |
| Chat message (live) | WebSocket + `hx-swap-oob` | `#chat-messages` | `beforeend` |
| Chat history | `hx-get` | `#chat-messages` | `afterbegin` (pagination) |
| Video metadata | `hx-get` | `#video-info` | `innerHTML` |
| Like/Subscribe | `hx-post` | `#action-buttons` | `outerHTML` |
| Theater mode | Client-side toggle | `body` | CSS class (no HTMX) |

---

## 4. Image Display & Gallery

### Key Screen/Component Breakdown
| Component | Primitive(s) | Role |
|-----------|-------------|------|
| Lightbox | Modal overlay, `<img>` or `<canvas>` | Full-screen image viewing |
| Grid | CSS Grid, `<img>`, Lazy loading | Thumbnail browsing |
| Carousel | Horizontal scroll, Buttons, Indicators | Sequential image browsing |
| Zoom | Pan/pinch container, `<img>` | Detail inspection |
| Metadata Overlay | Panel, `<dl>`, Tags | EXIF, caption, location info |
| EXIF Display | Table, `<details>` | Camera settings, GPS, timestamp |

### Layout Structure
```
Grid View:
┌────┬────┬────┬────┐
│ T1 │ T2 │ T3 │ T4 │   ← CSS Grid / Masonry
├────┼────┼────┼────┤
│ T5 │ T6 │ T7 │ T8 │
└────┴────┴────┴────┘

Lightbox View:
┌────────────────────────────┐
│ [X] [←]    [Image]    [→] │
│                            │
│    [Caption / Metadata]    │
└────────────────────────────┘
```
- **Grid:** `grid-template-columns: repeat(auto-fill, minmax(250px, 1fr))`
- **Masonry:** CSS `columns` or JS masonry library for uneven heights
- **Lightbox:** Fixed overlay, `object-fit: contain` for image

### Data Flow Patterns
| Pattern | Implementation | Notes |
|---------|---------------|-------|
| Grid loading | `hx-get` on scroll (infinite) | `/api/gallery?page=N` returns thumbnail grid |
| Lightbox open | `hx-get` on click | `/api/image/{id}` returns full image + metadata |
| Navigation | `hx-get` or client-side | Preload adjacent images |
| EXIF data | `hx-get` on expand | `/api/image/{id}/exif` returns metadata table |
| Zoom | Client-side (panzoom.js) | Transform scale/translate on image |

### Special UX Considerations
- **Accessibility:** `alt` text for all images, `aria-label` for nav buttons, focus trap in lightbox, `Escape` to close
- **Performance:** Lazy loading (`loading="lazy"`), responsive images (`srcset`), WebP/AVIF format, progressive loading
- **Mobile:** Pinch to zoom, swipe between images, swipe down to close lightbox
- **Keyboard:** Arrow keys for navigation, `+/-` for zoom, `Escape` for close
- **Print:** Optional print stylesheet for metadata

### Recommended HTMX Swap Strategies
| Interaction | HTMX Attribute | Swap Target | Swap Style |
|-------------|---------------|-------------|------------|
| Grid page load | `hx-get` + scroll trigger | `#gallery-grid` | `beforeend` (infinite scroll) |
| Lightbox open | `hx-get` | `#lightbox-content` | `innerHTML` (modal) |
| Next/prev image | `hx-get` | `#lightbox-content` | `innerHTML` with preload |
| EXIF expand | `hx-get` | `#exif-panel` | `innerHTML` |
| Tag filter | `hx-get` | `#gallery-grid` | `innerHTML` |
| Zoom | Client-side | `#image` | CSS transform (no HTMX) |

---

## 5. Image Editing Interface

### Key Screen/Component Breakdown
| Component | Primitive(s) | Role |
|-----------|-------------|------|
| Toolbar | `<nav>`, Icon Buttons, Tool Groups | Quick access to common tools |
| Canvas | `<canvas>` or `<svg>` | Main editing surface |
| Layers Panel | Sortable list, Visibility toggle, Opacity | Layer management |
| Adjustments Panel | Sliders, Presets, Histogram | Color/exposure correction |
| Filter Previews | Thumbnail grid, Before/After | Filter application preview |
| Crop/Rotate Controls | Overlay handles, Angle input, Aspect ratio | Composition adjustment |
| History Panel | Undo/Redo stack, State snapshots | Non-destructive editing |

### Layout Structure
```
┌─────────────────────────────────────────────────────┐
│ [File] [Edit] [View] [Filters] [Export]             │  Toolbar
├────────┬──────────────────────────┬─────────────────┤
│        │                          │                 │
│ Layers │      Canvas Area         │  Adjustments   │
│ Panel  │                          │  Panel         │
│        │   ┌────────────────┐     │                │
│ [+]    │   │                │     │  [Brightness]  │
│ [Layer1│   │   Image/Canvas  │     │  [Contrast]    │
│ [Layer2│   │                │     │  [Saturation]  │
│ ...    │   └────────────────┘     │  [Filters]     │
│        │                          │                │
└────────┴──────────────────────────┴─────────────────┘
```
- **Grid:** `auto 1fr auto` for panels + canvas + panels
- **Canvas:** `position: relative` with overlay controls absolutely positioned
- **Panels:** Resizable width (drag handle), collapsible groups

### Data Flow Patterns
| Pattern | Implementation | Notes |
|---------|---------------|-------|
| Tool selection | Client-side state | Update cursor, canvas mode, active tool |
| Adjustment sliders | Debounced `hx-post` or WebWorker | `/api/preview` with adjustment params |
| Filter preview | `hx-get` or client-side WebGL | Generate preview thumbnails |
| Layer operations | Client-side canvas stack | Merge, reorder, opacity changes locally |
| Export | `hx-post` with blob | `/api/export` with format settings |
| History | Client-side stack | Undo/redo canvas states |

### Special UX Considerations
- **Accessibility:** Keyboard shortcuts for all tools, ARIA labels for sliders, high contrast mode for canvas
- **Performance:** WebGL for filters, OffscreenCanvas for processing, requestAnimationFrame for smooth updates
- **Mobile:** Touch gestures for pan/zoom, simplified toolbar, bottom sheet for panels
- **Precision:** Input fields alongside sliders for exact values; zoom to pixel level
- **Non-destructive:** All edits stored as operations; original preserved; export generates new file

### Recommended HTMX Swap Strategies
| Interaction | HTMX Attribute | Swap Target | Swap Style |
|-------------|---------------|-------------|------------|
| Load image | `hx-get` | `#canvas-container` | `innerHTML` |
| Apply filter | `hx-post` | `#canvas-container` | `innerHTML` (preview) |
| Save preset | `hx-post` | `#presets-list` | `beforeend` |
| Export image | `hx-post` | `#download-link` | `innerHTML` (returns link) |
| Load history | `hx-get` | `#history-panel` | `innerHTML` |
| Tool switch | Client-side | — | State change (no HTMX) |

---

## 6. Video Editing Interface

### Key Screen/Component Breakdown
| Component | Primitive(s) | Role |
|-----------|-------------|------|
| Timeline | Horizontal track, Clips, Playhead | Temporal sequence of media |
| Preview | `<video>` or WebGL canvas | Frame-accurate preview |
| Media Pool | Grid/list, Thumbnails, Metadata | Available source media |
| Effects Panel | Parameters, Keyframes, Presets | Visual/audio effects control |
| Transport Controls | Play/pause, Jog/shuttle, Timecode | Playback control |
| Markers | Flag indicators, Notes, Color | Annotation and navigation points |
| Tracks | Layered lanes, Mute/solo, Lock | Audio/video track management |

### Layout Structure
```
┌────────────────────────────────────────────────────┐
│ [File] [Edit] [Effects] [Export]                 │  Toolbar
├────────────────────────────────────────────────────┤
│ ┌──────────────────────────┐ ┌──────────────────┐ │
│ │                          │ │  [Effects]         │ │
│ │      Preview Area        │ │  [Parameters]     │ │
│ │                          │ │  [Keyframes]       │ │
│ └──────────────────────────┘ └──────────────────┘ │
├────────────────────────────────────────────────────┤
│  Media Pool                                        │
│  ┌────┐┌────┐┌────┐┌────┐...                     │
│  └────┘└────┘└────┘└────┘                        │
├────────────────────────────────────────────────────┤
│  Timeline                                          │
│  ┌──────────────────────────────────────────────┐ │
│  │ V1: [Clip1] [  Clip2  ] [Clip3]              │ │
│  │ V2:       [  Overlay  ]                      │ │
│  │ A1: [Audio1] [Audio2] [Audio3]                │ │
│  │▲ Playhead                                    │ │
│  └──────────────────────────────────────────────┘ │
│  [|<] [<<] [Play] [>>] [>|]    [Timecode]        │  Transport
└────────────────────────────────────────────────────┘
```
- **Grid:** Rows for preview + effects; media pool as sub-row; timeline as fixed-height row
- **Timeline:** Horizontal scroll with fixed track headers; zoom via scale transform
- **Tracks:** Absolute positioning for clips; width = duration × zoom

### Data Flow Patterns
| Pattern | Implementation | Notes |
|---------|---------------|-------|
| Project loading | `hx-get` | `/api/project/{id}` returns full timeline state |
| Media import | `hx-post` with multipart | Upload to media pool, generate proxies |
| Timeline operations | Client-side + sync | Drag, trim, split operations locally; save to server |
| Preview frame | WebSocket or polling | Server-side render frame at playhead position |
| Export | `hx-post` with job queue | `/api/export` returns job ID; poll for progress |
| Auto-save | `hx-post` debounced | Save project state every 30s or on change |

### Special UX Considerations
- **Accessibility:** Keyboard shortcuts for all transport controls, focus indicators on timeline clips, audio description
- **Performance:** Proxy media for editing (low-res), GPU-accelerated preview, background rendering
- **Precision:** Frame-by-frame navigation (arrow keys), timecode display, zoom to sample level for audio
- **Mobile:** Not suitable for mobile; touch on tablet for rough cutting only
- **Keyboard:** J/K/L for shuttle, I/O for in/out points, +/- for timeline zoom
- **Drag & Drop:** Native HTML5 drag API for clip manipulation; visual feedback essential

### Recommended HTMX Swap Strategies
| Interaction | HTMX Attribute | Swap Target | Swap Style |
|-------------|---------------|-------------|------------|
| Project load | `hx-get` | `#app-container` | `innerHTML` |
| Media import | `hx-post` | `#media-pool` | `beforeend` |
| Apply effect | `hx-post` | `#effects-panel` | `innerHTML` |
| Export start | `hx-post` | `#export-status` | `innerHTML` |
| Export progress | `hx-get` (poll) | `#export-status` | `outerHTML` |
| Timeline drag | Client-side + `hx-post` (sync) | `#timeline` | `outerHTML` (after drop) |
| Preview update | WebSocket / SSE | `#preview` | `innerHTML` (frame) |

---

## 7. Text-to-Image Generative AI

### Key Screen/Component Breakdown
| Component | Primitive(s) | Role |
|-----------|-------------|------|
| Prompt Input | `<textarea>`, Token counter, Negative prompt | Text-to-image prompt entry |
| Style Selector | Grid, Dropdown, Tags | Artistic style / model selection |
| Generation Progress | Progress bar, Preview, Queue position | Status of generation job |
| Gallery Grid | CSS Grid, Thumbnails, Actions | Generated image browsing |
| Upscale Controls | Button group, Scale selector | Resolution enhancement |
| Variation Generation | Button, Seed input, Strength | Similar image generation |
| Parameter Panel | Sliders, Dimensions, Steps, CFG | Fine-tuning generation parameters |

### Layout Structure
```
┌────────────────────────────────────────────────────┐
│ [Prompt Input]                         [Generate] │
│  [Negative Prompt]                                  │
│  [Style: ▼] [Model: ▼] [Dimensions: ▼]            │
├────────────────────────────────────────────────────┤
│  Generation Progress                               │
│  ┌────────────────────────────────────────────┐     │
│  │ [████████████████████░░░░]  80% ETA: 0:12 │     │
│  │ [Preview thumbnail]                        │     │
│  └────────────────────────────────────────────┘     │
├────────────────────────────────────────────────────┤
│  Gallery                                             │
│  ┌────┐┌────┐┌────┐┌────┐                         │
│  │Img1││Img2││Img3││Img4│                         │
│  │[♥] ││[♥] ││[♥] ││[♥] │                         │
│  │[U] ││[V] ││[U] ││[V] │                         │
│  └────┘└────┘└────┘└────┘                         │
└────────────────────────────────────────────────────┘
```
- **Flex:** Column layout for input → progress → gallery
- **Grid:** `repeat(auto-fill, minmax(256px, 1fr))` for gallery
- **Progress:** Collapsible panel that appears during generation

### Data Flow Patterns
| Pattern | Implementation | Notes |
|---------|---------------|-------|
| Prompt submission | `hx-post` | `/api/generate` with prompt + params; returns job ID |
| Progress polling | `hx-get` (poll) or SSE | `/api/jobs/{id}/progress` returns progress % + preview |
| Gallery loading | `hx-get` on load | `/api/gallery` returns generated images |
| Upscale | `hx-post` | `/api/upscale` with image ID + scale factor |
| Variation | `hx-post` | `/api/variation` with image ID + seed + strength |
| Model switch | `hx-get` | `/api/models` returns available models + styles |

### Special UX Considerations
- **Accessibility:** Prompt input with clear labels, progress announced via `aria-live`, gallery with `alt` text
- **Performance:** Lazy load gallery images, cache generated results, preview thumbnails while generating
- **Mobile:** Stacked layout, prompt input expands, gallery becomes 2-column
- **Queue Management:** Show queue position, allow cancel, batch generation support
- **Safety:** Content moderation on prompts, blur/nsfw toggle for results
- **Persistence:** Save prompt history, favorite generations, seed values for reproducibility

### Recommended HTMX Swap Strategies
| Interaction | HTMX Attribute | Swap Target | Swap Style |
|-------------|---------------|-------------|------------|
| Generate | `hx-post` | `#generation-progress` | `innerHTML` (shows job) |
| Progress update | `hx-get` (poll) or SSE | `#progress-bar` | `outerHTML` |
| Preview update | `hx-get` (poll) | `#preview` | `innerHTML` (new image) |
| Gallery add | `hx-swap-oob` (SSE) | `#gallery` | `afterbegin` (new result) |
| Upscale | `hx-post` | `#gallery` | `afterbegin` (new upscaled) |
| Variation | `hx-post` | `#generation-progress` | `innerHTML` (new job) |
| Load more | `hx-get` + scroll | `#gallery` | `beforeend` |

---

## 8. Text-to-Video Generative AI

### Key Screen/Component Breakdown
| Component | Primitive(s) | Role |
|-----------|-------------|------|
| Prompt Input | `<textarea>`, Scene separation | Text-to-video prompt entry |
| Scene Editor | Timeline, Scene cards, Duration | Multi-scene composition |
| Frame Preview | Grid, Strip, Keyframe selection | Preview frames from generation |
| Generation Progress | Progress bar, ETA, Preview | Long-running video generation status |
| Storyboard | Grid of keyframes, Scene labels | Visual overview of video structure |
| Parameter Panel | Duration, FPS, Resolution, Motion | Video generation parameters |
| Asset Library | Uploaded images, clips, music | Source material for generation |

### Layout Structure
```
┌────────────────────────────────────────────────────┐
│ [Prompt / Scene Editor]            [Generate] [+] │
│  [Scene 1: "A cat walking..."] [Duration: 3s] [X] │
│  [Scene 2: "Then jumping..."] [Duration: 2s] [X] │
├────────────────────────────────────────────────────┤
│  Storyboard                                        │
│  ┌────┐┌────┐┌────┐┌────┐                         │
│  │F1  ││F2  ││F3  ││F4  │                         │
│  │[S1]││[S1]││[S2]││[S2]│                         │
│  └────┘└────┘└────┘└────┘                         │
├────────────────────────────────────────────────────┤
│  Frame Preview                                     │
│  ┌────────────────────────────────────────────┐     │
│  │ [Frame 1] [Frame 2] [Frame 3] ...         │     │
│  │ [Frame 5] [Frame 6] [Frame 7] ...         │     │
│  └────────────────────────────────────────────┘     │
├────────────────────────────────────────────────────┤
│  Generation Progress                               │
│  [████████████████░░░░]  60% ETA: 4:32             │
└────────────────────────────────────────────────────┘
```
- **Flex:** Column layout for input → storyboard → frames → progress
- **Storyboard:** Horizontal scrollable strip of keyframes
- **Frame Preview:** Grid of selectable frames with scene grouping

### Data Flow Patterns
| Pattern | Implementation | Notes |
|---------|---------------|-------|
| Scene composition | Client-side + `hx-post` on save | Build scenes locally; submit for generation |
| Generation | `hx-post` with job queue | `/api/generate-video` returns job ID |
| Progress polling | `hx-get` (poll) or SSE | `/api/jobs/{id}/progress` with frame previews |
| Frame preview | `hx-get` on scene select | `/api/scenes/{id}/frames` returns frame grid |
| Storyboard update | `hx-swap-oob` (SSE) | Update storyboard as generation progresses |
| Video playback | `hx-get` on complete | `/api/videos/{id}` returns playable video |

### Special UX Considerations
- **Accessibility:** Long jobs need status announcements, progress bar with ARIA, keyboard scene reordering
- **Performance:** Long generation times (minutes); need progress indication, cancellation, background processing
- **Mobile:** Simplified scene editor, reduced frame preview, generation notifications
- **Cost Awareness:** Show estimated cost/credits before generation, confirm on expensive operations
- **Preview Quality:** Low-res preview during generation, final render on complete
- **Persistence:** Auto-save scenes, save drafts, allow resuming interrupted generations

### Recommended HTMX Swap Strategies
| Interaction | HTMX Attribute | Swap Target | Swap Style |
|-------------|---------------|-------------|------------|
| Add scene | Client-side + `hx-post` | `#scene-list` | `beforeend` |
| Reorder scenes | `hx-post` (drag end) | `#scene-list` | `innerHTML` |
| Generate | `hx-post` | `#progress-panel` | `innerHTML` |
| Progress update | `hx-get` (poll) or SSE | `#progress-bar` | `outerHTML` |
| Frame preview | `hx-get` | `#frame-preview` | `innerHTML` |
| Storyboard update | SSE | `#storyboard` | `outerHTML` |
| Video complete | SSE | `#video-player` | `innerHTML` (show player) |
| Load project | `hx-get` | `#app` | `innerHTML` |

---

## 9. Document Management Workflow

### Key Screen/Component Breakdown
| Component | Primitive(s) | Role |
|-----------|-------------|------|
| Document List | Table, Grid, Filters, Sort | Browse and search documents |
| Upload | Drop zone, Progress, File picker | File upload with validation |
| Version History | Timeline, Diff view, Restore | Track document changes |
| Approval Workflow | Flowchart, Status badges, Actions | Multi-step approval chain |
| Comments | Threaded list, Annotations, Mentions | Collaborative feedback |
| Preview | `<iframe>`, PDF viewer, Thumbnail | Document content preview |
| Metadata | Form, Tags, Categories, Permissions | Document properties |

### Layout Structure
```
┌────────────────────────────────────────────────────┐
│ [Documents] [Upload] [Filters] [Search]            │
├────────────────────────────────────────────────────┤
│  ┌────────────┐  ┌────────────────────────────┐   │
│  │            │  │  [Preview]                 │   │
│  │ Document   │  │                            │   │
│  │  List      │  │  [Tabs: Content|Versions|  │   │
│  │  ┌────┐   │  │         Comments|Workflow]  │   │
│  │  │Doc1│   │  │                            │   │
│  │  │Doc2│   │  │  [Selected tab content]     │   │
│  │  │Doc3│   │  │                            │   │
│  │  └────┘   │  │                            │   │
│  └────────────┘  └────────────────────────────┘   │
└────────────────────────────────────────────────────┘
```
- **Grid:** `300px 1fr` for list + preview; collapsible on mobile
- **Preview:** `<iframe>` or object embed for PDFs; fallback to download link
- **Tabs:** Horizontal tabs for switching between content, versions, comments, workflow

### Data Flow Patterns
| Pattern | Implementation | Notes |
|---------|---------------|-------|
| List loading | `hx-get` with filters | `/api/documents?folder=X&sort=date` |
| Upload | `hx-post` with `multipart/form-data` | Chunked upload for large files; progress bar |
| Preview | `hx-get` | `/api/documents/{id}/preview` returns viewer HTML |
| Version compare | `hx-get` | `/api/documents/{id}/versions?compare=A,B` |
| Approval action | `hx-post` | `/api/documents/{id}/approve` or `/reject` |
| Comment | `hx-post` | `/api/documents/{id}/comments` with thread ID |
| Workflow update | `hx-get` (poll) or SSE | `/api/documents/{id}/workflow` |

### Special UX Considerations
- **Accessibility:** Keyboard navigation for list, focus management on preview, ARIA live for status updates
- **Performance:** Lazy load previews, thumbnail generation, pagination for large lists
- **Mobile:** List becomes full-screen; preview in modal; simplified workflow actions
- **Security:** Permission-based access, audit logging, watermark previews, download restrictions
- **Versioning:** Clear version numbering, visual diff for text documents, rollback capability
- **Offline:** Queue uploads when offline; sync when reconnected
- **Bulk Actions:** Multi-select with checkbox, bulk download, bulk move, bulk delete

### Recommended HTMX Swap Strategies
| Interaction | HTMX Attribute | Swap Target | Swap Style |
|-------------|---------------|-------------|------------|
| List filter | `hx-get` | `#document-list` | `innerHTML` |
| Document select | `hx-get` | `#preview-panel` | `innerHTML` |
| Tab switch | `hx-get` | `#tab-content` | `innerHTML` |
| Upload progress | `hx-post` with progress | `#upload-list` | `beforeend` |
| Upload complete | `hx-trigger` on complete | `#document-list` | `beforeend` (new doc) |
| Comment add | `hx-post` | `#comments-list` | `beforeend` |
| Approval action | `hx-post` | `#workflow-panel` | `outerHTML` |
| Version restore | `hx-post` | `#preview-panel` | `innerHTML` |

---

## 10. CRM Interface

### Key Screen/Component Breakdown
| Component | Primitive(s) | Role |
|-----------|-------------|------|
| Contact List | Table, Card grid, Filters, Search | Browse and filter contacts |
| Detail View | Tabs, Profile card, Custom fields | Contact information display |
| Activity Timeline | Vertical timeline, Icons, Filters | Chronological interaction history |
| Pipeline/Kanban | Column board, Cards, Drag-drop | Sales/deal stage tracking |
| Notes | `<textarea>`, Timestamp, Author | Free-form contact notes |
| Tasks | Checkbox, Due date, Priority, Assignee | Action items tied to contacts |
| Email Integration | Inbox view, Compose, Templates | Email history and sending |

### Layout Structure
```
┌────────────────────────────────────────────────────┐
│ [Contacts] [Deals] [Tasks] [Email] [Search]        │
├────────────────────────────────────────────────────┤
│  ┌──────────┐  ┌────────────────────────────────┐  │
│  │ Contact  │  │ [Contact Name] [Edit] [Actions]│  │
│  │  List    │  │                                │  │
│  │  ┌────┐  │  │ [Overview] [Activity] [Deals] │  │
│  │  │John│  │  │ [Notes] [Tasks] [Email]        │  │
│  │  │Jane│  │  │                                │  │
│  │  │Bob │  │  │ [Selected Tab Content]         │  │
│  │  └────┘  │  │                                │  │
│  └──────────┘  └────────────────────────────────┘  │
│                                                  │
│  Pipeline View (alternative):                    │
│  ┌────────┐┌────────┐┌────────┐┌────────┐        │
│  │ Lead   ││ Contact││ Proposal││ Closed │       │
│  │ [C1]   ││ [C2]   ││ [C3]   ││ [C4]   │       │
│  │ [C5]   ││        ││        ││        │       │
│  └────────┘└────────┘└────────┘└────────┘        │
└────────────────────────────────────────────────────┘
```
- **Grid:** `350px 1fr` for list + detail; full-width for pipeline
- **Pipeline:** Horizontal scrollable columns, CSS Grid or flex
- **Timeline:** Vertical flex with alternating sides or single column

### Data Flow Patterns
| Pattern | Implementation | Notes |
|---------|---------------|-------|
| Contact list | `hx-get` with filters | `/api/contacts?stage=lead&owner=me` |
| Contact detail | `hx-get` on select | `/api/contacts/{id}` returns full profile |
| Activity load | `hx-get` on tab select | `/api/contacts/{id}/activity` |
| Pipeline move | `hx-post` (drag end) | `/api/deals/{id}/stage` with new stage |
| Task create | `hx-post` | `/api/tasks` with contact ID |
| Email send | `hx-post` | `/api/emails` with contact ID; async send |
| Note add | `hx-post` | `/api/contacts/{id}/notes` |
| Deal update | `hx-post` | `/api/deals/{id}` with field changes |

### Special UX Considerations
- **Accessibility:** Keyboard navigation between contacts, focus on detail view, ARIA for timeline
- **Performance:** Virtual scroll for large contact lists; lazy load activities; cache contact details
- **Mobile:** Contact list becomes full-screen; detail in modal or push navigation; pipeline becomes vertical stack
- **Customization:** Custom fields per contact type; configurable pipeline stages; saved views/filters
- **Integrations:** Email sync, calendar sync, phone call logging; external system webhooks
- **Bulk Actions:** Merge duplicates, bulk email, bulk stage update, bulk assign
- **Security:** Role-based access (owner, team, org); field-level permissions; audit log

### Recommended HTMX Swap Strategies
| Interaction | HTMX Attribute | Swap Target | Swap Style |
|-------------|---------------|-------------|------------|
| Contact select | `hx-get` | `#detail-panel` | `innerHTML` |
| List filter | `hx-get` | `#contact-list` | `innerHTML` |
| Tab switch | `hx-get` | `#tab-content` | `innerHTML` |
| Pipeline move | `hx-post` (drag) | `#pipeline` | `innerHTML` |
| Task toggle | `hx-post` | `#task-item` | `outerHTML` |
| Note add | `hx-post` | `#notes-list` | `beforeend` |
| Email send | `hx-post` | `#email-history` | `beforeend` |
| Deal quick edit | `hx-post` | `#deal-card` | `outerHTML` |

---

## 11. ERP Interface

### Key Screen/Component Breakdown
| Component | Primitive(s) | Role |
|-----------|-------------|------|
| Inventory | Table, Stock levels, Alerts, Locations | Stock management |
| Orders | Table, Status badges, Actions, Filters | Order lifecycle management |
| Invoices | Table, PDF preview, Payment status | Billing and collections |
| Reports | Charts, Tables, Filters, Export | Business intelligence |
| Multi-step Workflows | Stepper, Forms, Validation | Guided business processes |
| Approval Chains | Status flow, Delegation, Escalation | Authorization workflows |
| Audit Trails | Timeline, Filters, Export | Change tracking and compliance |

### Layout Structure
```
┌────────────────────────────────────────────────────┐
│ [Inventory] [Orders] [Invoices] [Reports] [Admin]  │
├────────────────────────────────────────────────────┤
│  Filters: [Status: ▼] [Date: ▼] [Location: ▼]     │
├────────────────────────────────────────────────────┤
│  Data Table                                        │
│  ┌────────────────────────────────────────────┐     │
│  │ ID  │ Name      │ Qty │ Status  │ Actions │     │
│  │ 001 │ Widget A  │ 150 │ OK      │ [View]  │     │
│  │ 002 │ Widget B  │  12 │ LOW     │ [Order] │     │
│  │ 003 │ Widget C  │   0 │ OUT     │ [Alert] │     │
│  └────────────────────────────────────────────┘     │
│  [1] [2] [3] ... [Next] [Export]                   │
├────────────────────────────────────────────────────┤
│  Reports (alternative view):                       │
│  ┌────────────┐  ┌────────────┐  ┌────────────┐   │
│  │ [Chart 1]  │  │ [Chart 2]  │  │ [Chart 3]  │   │
│  │ Sales Trend│  │ Inventory  │  │ Revenue    │   │
│  └────────────┘  └────────────┘  └────────────┘   │
└────────────────────────────────────────────────────┘
```
- **Grid:** Full-width tables; dashboard grid for reports
- **Table:** Fixed headers, horizontal scroll for many columns, sticky action column
- **Workflows:** Stepper at top, form content below, navigation at bottom

### Data Flow Patterns
| Pattern | Implementation | Notes |
|---------|---------------|-------|
| Data loading | `hx-get` with query params | `/api/inventory?status=low&location=warehouse1` |
| CRUD operations | `hx-post` / `hx-put` / `hx-delete` | Standard REST with HTML fragment returns |
| Reports | `hx-get` with filter params | `/api/reports/sales?from=2024-01&to=2024-12` |
| Workflow step | `hx-post` | Submit step form; return next step or summary |
| Approval | `hx-post` | `/api/approvals/{id}` with approve/reject/comment |
| Audit trail | `hx-get` | `/api/audit?entity=order&entityId=123` |
| Bulk operations | `hx-post` with selected IDs | `/api/orders/bulk-update` with array of IDs |

### Special UX Considerations
- **Accessibility:** Table sorting via keyboard, focus on action buttons, ARIA labels for status badges
- **Performance:** Pagination (not infinite scroll) for large datasets; server-side sorting/filtering; column visibility toggle
- **Mobile:** Tables become cards (responsive table pattern); reports become stacked charts; workflows simplified
- **Data Density:** Users need to see maximum information; compact table mode; configurable columns; Excel-like interactions
- **Validation:** Strict form validation before workflow advancement; inline validation; cross-field checks
- **Export:** CSV, Excel, PDF export for all tables and reports; print-friendly CSS
- **Concurrency:** Optimistic locking or last-write-wins with conflict notification
- **Localization:** Currency formatting, date formats, number formats, multi-language support

### Recommended HTMX Swap Strategies
| Interaction | HTMX Attribute | Swap Target | Swap Style |
|-------------|---------------|-------------|------------|
| Table filter | `hx-get` | `#data-table` | `innerHTML` |
| Table sort | `hx-get` | `#data-table` | `innerHTML` |
| Table pagination | `hx-get` | `#data-table` | `innerHTML` |
| Row edit | `hx-get` | `#row-{id}` | `outerHTML` (inline edit) |
| Row save | `hx-post` | `#row-{id}` | `outerHTML` |
| Workflow step | `hx-post` | `#workflow-form` | `innerHTML` (next step) |
| Approval action | `hx-post` | `#approval-panel` | `outerHTML` |
| Report filter | `hx-get` | `#report-container` | `innerHTML` |
| Audit load | `hx-get` | `#audit-timeline` | `innerHTML` |
| Bulk action | `hx-post` | `#data-table` | `innerHTML` |

---

## 12. EMR (Electronic Medical Record)

### Key Screen/Component Breakdown
| Component | Primitive(s) | Role |
|-----------|-------------|------|
| Patient List | Table, Filters, Alerts, Search | Patient roster and selection |
| Chart View | Timeline, Summary cards, Problem list | Patient health overview |
| Vital Signs Timeline | Line chart, Data table, Threshold alerts | Physiological measurements over time |
| Medication List | Table, Status, Adherence, Interactions | Current and historical medications |
| Allergies | Alert banner, List, Severity icons | Allergy and intolerance tracking |
| Lab Results | Table, Reference ranges, Trend charts | Diagnostic test results |
| Visit Notes | Structured form, Free text, Templates | Clinical encounter documentation |

### Layout Structure
```
┌────────────────────────────────────────────────────┐
│ [Patient List] [Search] [Alerts] [User]            │
├────────────────────────────────────────────────────┤
│  Patient: [Name], [DOB], [MRN], [Alert Banner]     │
├────────────────────────────────────────────────────┤
│  ┌──────────┐  ┌────────────────────────────────┐  │
│  │  Patient │  │ [Summary] [Vitals] [Meds]     │  │
│  │  Photo   │  │ [Labs] [Notes] [Allergies]    │  │
│  │          │  │                                │  │
│  │  [Demog] │  │ [Selected Tab Content]         │  │
│  │  [Contact│  │                                │  │
│  │  [Insurance]│                                │  │
│  └──────────┘  └────────────────────────────────┘  │
└────────────────────────────────────────────────────┘
```
- **Grid:** `280px 1fr` for patient info + chart; full-width for patient list
- **Timeline:** Vertical scroll for chart history; horizontal for vital signs
- **Alerts:** Fixed banner at top for critical allergies or conditions

### Data Flow Patterns
| Pattern | Implementation | Notes |
|---------|---------------|-------|
| Patient list | `hx-get` with filters | `/api/patients?ward=ICU&provider=DrSmith` |
| Chart load | `hx-get` on select | `/api/patients/{id}/chart` |
| Vitals load | `hx-get` on tab select | `/api/patients/{id}/vitals` with time range |
| Lab results | `hx-get` | `/api/patients/{id}/labs` |
| Medication update | `hx-post` | `/api/patients/{id}/medications` with reconciliation |
| Visit note | `hx-post` | `/api/patients/{id}/notes` with structured data |
| Allergy alert | `hx-get` on patient load | Always loaded with patient context |
| Real-time vitals | WebSocket or SSE | ICU monitoring; push updates to chart |

### Special UX Considerations
- **Accessibility:** WCAG 2.1 AA compliance; high contrast for vital signs; screen reader support for alerts; keyboard-only navigation for sterile environments
- **Performance:** Fast load of critical data (allergies, alerts); progressive loading of historical data; cache recent patients
- **Mobile:** Tablet-first for bedside use; simplified views for quick data entry; barcode scanning for medication administration
- **Safety:** Critical alerts (allergies, drug interactions) must be prominent and unmissable; confirmation dialogs for high-risk actions; audit trail for all data changes
- **Privacy:** HIPAA compliance; automatic logout; role-based access; audit logging; no PHI in URLs or logs
- **Data Entry:** Structured templates for common conditions; voice-to-text for notes; smart defaults; autocomplete for medications and diagnoses
- **Interoperability:** HL7 FHIR integration; external lab result importing; pharmacy system connectivity
- **Offline:** Limited offline capability for chart review; sync when reconnected; queue data entry
- **Clinical Decision Support:** Drug interaction alerts, dosage range checks, allergy warnings, guideline recommendations

### Recommended HTMX Swap Strategies
| Interaction | HTMX Attribute | Swap Target | Swap Style |
|-------------|---------------|-------------|------------|
| Patient select | `hx-get` | `#chart-container` | `innerHTML` |
| List filter | `hx-get` | `#patient-list` | `innerHTML` |
| Tab switch | `hx-get` | `#tab-content` | `innerHTML` |
| Vitals range | `hx-get` | `#vitals-chart` | `innerHTML` |
| Med add | `hx-post` | `#medication-list` | `beforeend` |
| Med reconcile | `hx-post` | `#medication-list` | `innerHTML` |
| Note save | `hx-post` | `#notes-list` | `beforeend` |
| Lab load | `hx-get` | `#lab-results` | `innerHTML` |
| Real-time vitals | SSE | `#vitals-panel` | `outerHTML` (update values) |
| Alert dismiss | `hx-post` | `#alert-banner` | `remove` |

---

## Cross-Domain Pattern Summary

### HTMX Strategy Matrix
| Domain | Primary Pattern | Real-time Needs | Heavy Client JS |
|--------|---------------|-----------------|-----------------|
| Admin Dashboard | `hx-boost` + polling | Medium (notifications) | Low |
| Chat/Messaging | WebSocket + SSE | High (real-time) | High (virtual scroll) |
| Video Streaming | Native `<video>` + SSE | Medium (live chat) | Medium (custom controls) |
| Image Gallery | `hx-get` + lazy load | Low | Low |
| Image Editing | Client-side + `hx-post` | Low | High (canvas/WebGL) |
| Video Editing | Client-side + WebSocket | Medium (preview) | Very High (timeline) |
| Text-to-Image | `hx-post` + polling/SSE | Medium (generation) | Low |
| Text-to-Video | `hx-post` + SSE | Medium (generation) | Medium |
| Document Management | `hx-get` + `hx-post` | Low | Low |
| CRM | `hx-get` + `hx-post` | Low | Medium (pipeline drag) |
| ERP | `hx-get` + `hx-post` | Low | Medium (tables) |
| EMR | `hx-get` + WebSocket/SSE | High (vitals) | Medium (charts) |

### Shared Component Primitives
| Primitive | Used In | HTMX Notes |
|-----------|---------|------------|
| Modal/Dialog | All | `hx-target="#modal"` + `innerHTML` |
| Data Table | ERP, CRM, EMR, Document | `hx-get` for sort/filter/paginate |
| Form | All | `hx-post` with validation; `hx-target` for error display |
| Tabs | Admin, CRM, EMR, Document | `hx-get` with `innerHTML` swap |
| Timeline | CRM, EMR, Video Editing | `hx-get` for load more; SSE for real-time |
| Progress Bar | Text-to-Image, Text-to-Video, Upload | `hx-swap-oob` or `outerHTML` for updates |
| Toast/Notification | All | `hx-swap-oob` or SSE for ephemeral alerts |
| Dropdown/Menu | All | `hx-get` for lazy-loaded content |
| Drag & Drop | CRM (pipeline), Video Editing | Native DnD + `hx-post` on drop |
| Infinite Scroll | Chat, Gallery, Document | `hx-get` with scroll trigger + `beforeend` |

### Recommended HTMX Extensions per Domain
| Extension | Domains | Purpose |
|-----------|---------|---------|
| `ws` (WebSocket) | Chat, Video Editing, EMR | Bidirectional real-time communication |
| `sse` (Server-Sent Events) | Chat, Video Streaming, Generative AI | Server-push updates |
| `json-enc` | All (API endpoints) | Send JSON instead of form-encoded |
| `class-tools` | Admin, CRM | Toggle CSS classes for UI states |
| `loading-states` | All | Show loading indicators during requests |
| `path-deps` | ERP, CRM | Refresh dependent elements on data change |
| `confirm` | EMR, ERP | Confirmation dialogs for destructive actions |
| `multi-swap` | Video Editing, Image Editing | Update multiple targets from one response |
| `preload` | Gallery, Document | Preload content on hover for faster perceived performance |
| `morph` (Idiomorph) | All | DOM morphing for smoother transitions |

---

*End of Technical Brief*
