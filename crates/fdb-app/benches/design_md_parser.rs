use criterion::{black_box, criterion_group, criterion_main, Criterion};

const SAMPLE_MD: &str = r##"
# Design System — Benchmark Fixture

## 1. Color
```json
{"primary": "#2563eb", "surface": "#ffffff", "text": "#0f172a"}
```

## 2. Typography
font_family: Inter, system-ui
size_md: 14px

## 3. Spacing
xs: 4px
sm: 8px
md: 16px

## 4. Layout
max_width: 1280px

## 5. Components

### button
```json
{"variant": "primary"}
```

### data-grid
```json
{"pageSize": 25}
```

## 6. Motion
duration_fast: 150ms

## 7. Voice
Friendly and clear.

## 8. Brand
Professional with warmth.

## 9. Anti-patterns
- Avoid all-caps.
"##;

fn bench_parse_design_md(c: &mut Criterion) {
    c.bench_function("parse_design_md", |b| {
        b.iter(|| fdb_app::a2ui::parse_design_md(black_box(SAMPLE_MD)).unwrap());
    });
}

criterion_group!(benches, bench_parse_design_md);
criterion_main!(benches);
