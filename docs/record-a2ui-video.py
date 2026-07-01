from playwright.sync_api import sync_playwright
import os, sys

BASE = 'http://localhost:8743'
OUT_DIR = '/Users/gqadonis/Projects/prometheus/flint-forge/docs/_video_tmp'
os.makedirs(OUT_DIR, exist_ok=True)

def show_title_card(page, text, sub=None, hold=1.3):
    page.evaluate('''(args) => {
        const d=document.createElement('div');
        d.id='__tc';
        d.style.cssText='position:fixed;inset:0;z-index:999999;background:rgba(11,15,20,0.97);'
            +'display:flex;flex-direction:column;align-items:center;justify-content:center;color:#fff;'
            +'font-family:Inter,system-ui,sans-serif;transition:opacity .25s ease;opacity:0;text-align:center;padding:40px;';
        d.innerHTML = '<div style="font-size:40px;font-weight:800;letter-spacing:-1px;">'+args.text+'</div>'
            + (args.sub ? '<div style="margin-top:14px;font-size:18px;color:#9aa3af;max-width:700px;">'+args.sub+'</div>' : '');
        document.body.appendChild(d);
        requestAnimationFrame(()=>{ d.style.opacity='1'; });
    }''', {'text': text, 'sub': sub or ''})
    page.wait_for_timeout(int(hold*1000))
    page.evaluate('''() => {
        const d=document.getElementById('__tc');
        if(d){ d.style.opacity='0'; setTimeout(()=>d.remove(), 300); }
    }''')
    page.wait_for_timeout(300)

def safe(label, fn):
    try:
        fn()
        return True
    except Exception as e:
        print(f'    [skip] {label}: {type(e).__name__}')
        return False

def probe_section(page, max_actions=3):
    """Bounded, defensive pass: exercise whatever real interactive controls
    are visible right now without navigating away or breaking layout."""
    acted = 0

    def try_selects():
        nonlocal acted
        for loc in page.locator('select:visible').all()[:2]:
            if acted >= max_actions: return
            if safe('select', lambda l=loc: l.select_option(index=1)):
                page.wait_for_timeout(220); acted += 1

    def try_text_inputs():
        nonlocal acted
        for loc in page.locator('input[type="text"]:visible, input[type="search"]:visible, textarea:visible').all()[:2]:
            if acted >= max_actions: return
            if safe('text-input', lambda l=loc: (l.click(), l.fill(''), l.type('Demo input', delay=18))):
                page.wait_for_timeout(220); acted += 1

    def try_checks():
        nonlocal acted
        for loc in page.locator('input[type="checkbox"]:visible, input[type="radio"]:visible').all()[:2]:
            if acted >= max_actions: return
            if safe('checkbox/radio', lambda l=loc: l.click()):
                page.wait_for_timeout(180); acted += 1

    def try_accordion():
        nonlocal acted
        for loc in page.locator('.acc-h:visible').all()[:1]:
            if acted >= max_actions: return
            if safe('accordion', lambda l=loc: l.click()):
                page.wait_for_timeout(280); acted += 1

    def try_switch():
        nonlocal acted
        for loc in page.locator('.sw-i:visible, .sw-l:visible').all()[:1]:
            if acted >= max_actions: return
            if safe('switch', lambda l=loc: l.click()):
                page.wait_for_timeout(200); acted += 1

    def try_dropdown_menu():
        nonlocal acted
        for loc in page.locator('.mn-i:visible').all()[:1]:
            if acted >= max_actions: return
            if safe('dropdown-menu', lambda l=loc: l.click()):
                page.wait_for_timeout(350)
                safe('close-menu', lambda: page.keyboard.press('Escape'))
                acted += 1

    def try_rating():
        nonlocal acted
        for loc in page.locator('[\\:class*="rating"]:visible, .ic[aria-hidden]').all()[:0]:
            pass  # rating stars are SVG icons without a stable hook; skip safely

    def try_hx_buttons():
        nonlocal acted
        for loc in page.locator('button[hx-post]:visible, button[hx-get]:visible').all()[:1]:
            if acted >= max_actions: return
            if safe('hx-button', lambda l=loc: l.click()):
                page.wait_for_timeout(450); acted += 1

    def try_buttons_generic():
        nonlocal acted
        # avoid hero/top nav buttons that navigate to other files
        for loc in page.locator('.demo button.btn:visible, .comp-item button.btn:visible').all()[:2]:
            if acted >= max_actions: return
            txt = ''
            try: txt = (loc.inner_text() or '').strip().lower()
            except: pass
            if any(k in txt for k in ('delete', 'remove', 'sign out', 'logout')):
                continue
            if safe('button', lambda l=loc: l.click()):
                page.wait_for_timeout(220); acted += 1

    for fn in (try_selects, try_text_inputs, try_checks, try_accordion,
               try_switch, try_dropdown_menu, try_hx_buttons, try_buttons_generic):
        if acted >= max_actions: break
        fn()
    return acted

def tour_file(page, filename, title, sub):
    print(f'== {filename} ==')
    page.goto(f'{BASE}/{filename}?v=vid', wait_until='networkidle')
    page.wait_for_timeout(300)
    show_title_card(page, title, sub)

    nav_links = page.locator('.snav-i[href^="#"]').all()
    if not nav_links:
        # no left-nav (rare) -> just scroll the page in steps
        height = page.evaluate('document.body.scrollHeight')
        steps = max(4, height // 700)
        for i in range(steps):
            page.mouse.wheel(0, 700)
            page.wait_for_timeout(450)
            probe_section(page, max_actions=2)
        return

    print(f'   {len(nav_links)} nav sections')
    for i in range(len(nav_links)):
        # re-query each time: clicking can cause layout/Alpine state changes
        links = page.locator('.snav-i[href^="#"]').all()
        if i >= len(links):
            break
        label = ''
        safe('label', lambda: None)
        try:
            label = links[i].inner_text().strip()
        except Exception:
            pass
        print(f'   -> section: {label!r}')
        if not safe('nav-click', lambda l=links[i]: l.click()):
            continue
        page.wait_for_timeout(550)
        probe_section(page, max_actions=3)
        page.wait_for_timeout(250)

def mobile_segment(page):
    print('== mobile drawer demo ==')
    page.goto(f'{BASE}/FLINT-A2UI-REGISTRY-SPEC.html?v=vid', wait_until='networkidle')
    page.wait_for_timeout(300)
    show_title_card(page, 'Mobile Layout', 'Responsive sidebar — offcanvas drawer', hold=1.4)
    safe('burger-click', lambda: page.locator('.snav-burger').click())
    page.wait_for_timeout(900)
    safe('nav-click-mobile', lambda: page.locator('.snav-i[href^="#"]').nth(3).click())
    page.wait_for_timeout(700)
    for _ in range(3):
        page.mouse.wheel(0, 500)
        page.wait_for_timeout(450)

def main():
    files = [
        ('FLINT-A2UI-REGISTRY-SPEC.html', 'Flint Component Registry', 'A2UI base primitives, schema, and API surface'),
        ('FLINT-A2UI-SITUATIONAL.html',   'Situational Examples', 'Forms, lists, tabs, admin shell, chat, matrix, video, image'),
        ('FLINT-A2UI-CRM.html',           'CRM Interface', 'Pipeline, contacts, and deal management'),
        ('FLINT-A2UI-DOCS.html',          'Document Management', 'Upload, library, and collaboration'),
        ('FLINT-A2UI-EMR.html',           'EMR Interface', 'Patient records and clinical workflows'),
        ('FLINT-A2UI-ERP.html',           'ERP Interface', 'Inventory, orders, and operations'),
        ('FLINT-A2UI-VIDEO-GEN.html',     'Video Generative AI', 'Prompt-to-video, library, and batch generation'),
    ]

    with sync_playwright() as p:
        browser = p.chromium.launch()

        # Desktop tour (one continuous context/video)
        ctx = browser.new_context(
            viewport={'width': 1600, 'height': 1000},
            record_video_dir=OUT_DIR,
            record_video_size={'width': 1600, 'height': 1000},
        )
        page = ctx.new_page()
        page.goto('about:blank')
        page.evaluate("document.body.style.background='#0b0f14'")
        show_title_card(page, 'Flint A2UI', 'Component Showcase — Desktop Walkthrough', hold=1.6)
        for fn, title, sub in files:
            tour_file(page, fn, title, sub)
        desktop_video_path = page.video.path()
        ctx.close()
        print('Desktop video saved:', desktop_video_path)

        # Mobile segment (separate context/video at narrow viewport)
        ctx2 = browser.new_context(
            viewport={'width': 480, 'height': 900},
            record_video_dir=OUT_DIR,
            record_video_size={'width': 480, 'height': 900},
        )
        page2 = ctx2.new_page()
        mobile_segment(page2)
        mobile_video_path = page2.video.path()
        ctx2.close()
        print('Mobile video saved:', mobile_video_path)

        browser.close()

        with open(os.path.join(OUT_DIR, '_paths.txt'), 'w') as f:
            f.write(desktop_video_path + '\n')
            f.write(mobile_video_path + '\n')

if __name__ == '__main__':
    main()
