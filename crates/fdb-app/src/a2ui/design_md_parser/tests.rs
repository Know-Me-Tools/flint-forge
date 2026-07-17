use super::*;

const SAMPLE: &str = r##"
# Design System — Acme Corp

## 1. Color

```json
{
  "primary": "#2563eb",
  "surface": "#ffffff",
  "text": "#0f172a"
}
```

## 2. Typography

font_family: Inter, system-ui, sans-serif
size_md: 14px
size_lg: 16px

## 3. Spacing

xs: 4px
sm: 8px
md: 16px
lg: 24px

## 4. Layout

max_width: 1280px
sidebar_width: 240px

## 5. Components

### button

```json
{ "variant": "primary", "size": "md" }
```

```json
{ "--btn-primary-bg": "#1d4ed8" }
```

react_component: "@acme/ui/Button"

### data-grid

```json
{ "pageSize": 25 }
```

## 6. Motion

duration_fast: 150ms
easing: ease-in-out

## 7. Voice

Friendly, clear, and concise.

## 8. Brand

Acme Corp: professional with warmth.

## 9. Anti-patterns

- Never use red for primary actions.
- Avoid all-caps labels.
"##;

#[test]
fn parses_title() {
    let doc = parse(SAMPLE).expect("parse");
    assert_eq!(doc.name, "Acme Corp");
}

#[test]
fn parses_color_section() {
    let doc = parse(SAMPLE).expect("parse");
    assert_eq!(doc.tokens["color"]["primary"], "#2563eb");
}

#[test]
fn parses_typography_kv() {
    let doc = parse(SAMPLE).expect("parse");
    assert_eq!(
        doc.tokens["typography"]["font_family"],
        "Inter, system-ui, sans-serif"
    );
}

#[test]
fn parses_spacing_kv() {
    let doc = parse(SAMPLE).expect("parse");
    assert_eq!(doc.tokens["spacing"]["md"], "16px");
}

#[test]
fn parses_component_overrides() {
    let doc = parse(SAMPLE).expect("parse");
    assert_eq!(doc.component_overrides.len(), 2);
    let btn = doc
        .component_overrides
        .iter()
        .find(|c| c.slug == "button")
        .expect("button");
    assert_eq!(btn.prop_defaults["variant"], "primary");
    assert_eq!(btn.css_vars["--btn-primary-bg"], "#1d4ed8");
    assert_eq!(btn.react_component.as_deref(), Some("@acme/ui/Button"));
}

#[test]
fn parses_voice_brand_anti_patterns() {
    let doc = parse(SAMPLE).expect("parse");
    assert!(doc.voice.contains("Friendly"));
    assert!(doc.brand.contains("Acme Corp"));
    assert!(doc.anti_patterns.contains("red"));
}

#[test]
fn raw_sections_stored() {
    let doc = parse(SAMPLE).expect("parse");
    assert!(doc.raw_sections.contains_key(&1));
    assert!(doc.raw_sections.contains_key(&9));
}

#[test]
fn w3c_token_mapper_flattens_nested() {
    let input = r##"{
      "color": {
        "$type": "color",
        "primary": { "$value": "#2563eb" },
        "brand": {
          "dark": { "$value": "#1d4ed8" }
        }
      },
      "spacing": {
        "md": { "$value": "16px" }
      }
    }"##;
    let result = map_w3c_tokens(input).expect("map");
    assert_eq!(result["color"]["primary"], "#2563eb");
    assert_eq!(result["color"]["brand-dark"], "#1d4ed8");
    assert_eq!(result["spacing"]["md"], "16px");
}

#[test]
fn missing_title_returns_error() {
    let result = parse("## 1. Color\nprimary: #000");
    assert!(matches!(result, Err(ParseError::MissingTitle)));
}
