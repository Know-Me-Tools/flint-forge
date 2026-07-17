//! H1 title extraction and H2-numbered section splitting for DESIGN.md.

use std::collections::HashMap;

/// Extract the H1 title from the document.
pub(super) fn extract_title(input: &str) -> Option<String> {
    for line in input.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("# ") {
            // Strip "Design System — " prefix if present
            let name = rest
                .trim_start_matches("Design System — ")
                .trim_start_matches("Design System: ")
                .trim()
                .to_owned();
            if !name.is_empty() {
                return Some(name);
            }
        }
    }
    None
}

/// Split the document into numbered sections by H2 headings matching
/// `## <N>.` or `## <N> `. Returns a map of section number → section body text.
pub(super) fn split_sections(input: &str) -> HashMap<u8, String> {
    let mut sections: HashMap<u8, String> = HashMap::new();
    let mut current_num: Option<u8> = None;
    let mut current_body = String::new();

    for line in input.lines() {
        let trimmed = line.trim();
        // Match "## 1. Color" or "## 1 Color"
        if let Some(rest) = trimmed.strip_prefix("## ") {
            if let Some(num) = parse_section_number(rest) {
                if let Some(n) = current_num {
                    sections.insert(n, current_body.trim().to_owned());
                }
                current_num = Some(num);
                current_body = String::new();
                continue;
            }
        }
        if current_num.is_some() {
            current_body.push_str(line);
            current_body.push('\n');
        }
    }
    if let Some(n) = current_num {
        sections.insert(n, current_body.trim().to_owned());
    }
    sections
}

fn parse_section_number(heading: &str) -> Option<u8> {
    let digits: String = heading.chars().take_while(char::is_ascii_digit).collect();
    digits.parse::<u8>().ok().filter(|&n| (1..=9).contains(&n))
}
