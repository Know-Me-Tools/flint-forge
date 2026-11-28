//! Public DESIGN.md import contract for both token and renderer consumers.
use fdb_app::a2ui::{design_md_parser::ParseError, parse_design_md};
use serde_json::json;

#[test]
fn json_token_sections_report_the_offending_section() {
    for section in 1..=3 {
        let input = format!("# Sample\n## {section}. Tokens\n```json\n{{\n```");
        assert!(
            matches!(parse_design_md(&input),Err(ParseError::InvalidJson {section:actual,..}) if actual==section)
        );
    }
}

#[test]
fn token_objects_and_renderer_directives_survive_import() {
    let document = parse_design_md("# Design System: Interop\n## 1. Color\nPrimary Surface: navy\n## 2. Typography\n```json\n{\"family\":\"Inter\"}\n```\n## 3. Spacing\n```json\n{\"base\":4}\n```\n## 5. Components\n### Empty Slot\n### Submit Button\nflutter_widget: ConfirmButton\nhtmx_template: \"forms/confirm\"").expect("valid design document");
    assert_eq!(document.name, "Interop");
    assert_eq!(
        document.tokens,
        json!({"color":{"primary_surface":"navy"},"typography":{"family":"Inter"},"spacing":{"base":4}})
    );
    let [component] = document.component_overrides.as_slice() else {
        panic!("one nonempty component override expected")
    };
    assert_eq!(component.slug, "submit-button");
    assert_eq!(component.flutter_widget.as_deref(), Some("ConfirmButton"));
    assert_eq!(component.htmx_template.as_deref(), Some("forms/confirm"));
    assert!(component.react_component.is_none());
}
