use crate::observer::config::{ObservationOutput, validate_concept, validate_type};

fn extract_tag_content(xml: &str, tag: &str) -> String {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");

    xml.find(&open)
        .and_then(|start| {
            let content_start = start + open.len();
            xml[content_start..]
                .find(&close)
                .map(|end| xml[content_start..content_start + end].trim().to_string())
        })
        .unwrap_or_default()
}

fn extract_all_tag_contents(xml: &str, tag: &str) -> Vec<String> {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let mut results = Vec::new();
    let mut search_from = 0;

    while let Some(start) = xml[search_from..].find(&open) {
        let abs_start = search_from + start;
        let content_start = abs_start + open.len();

        if let Some(end) = xml[content_start..].find(&close) {
            let content = xml[content_start..content_start + end].trim().to_string();
            if !content.is_empty() {
                results.push(content);
            }
            search_from = content_start + end + close.len();
        } else {
            break;
        }
    }

    results
}

fn extract_block(xml: &str, block_tag: &str) -> String {
    let open = format!("<{block_tag}>");
    let close = format!("</{block_tag}>");

    if let Some(start) = xml.find(&open) {
        let content_start = start + open.len();
        if let Some(end) = xml[content_start..].find(&close) {
            return xml[content_start..content_start + end].to_string();
        }
    }
    String::new()
}

pub fn parse_observation_xml(xml: &str) -> Option<ObservationOutput> {
    let obs_type = extract_tag_content(xml, "type");
    let obs_type = validate_type(&obs_type).to_string();

    let title = extract_tag_content(xml, "title");
    let title = if title.is_empty() { "Observation".to_string() } else { title };

    let subtitle = extract_tag_content(xml, "subtitle");
    let narrative = extract_tag_content(xml, "narrative");

    let facts: Vec<String> = extract_all_tag_contents(xml, "fact");

    let concepts: Vec<String> = extract_all_tag_contents(xml, "concept")
        .into_iter()
        .filter_map(|c| validate_concept(&c).map(String::from))
        .collect();

    let files_read_block = extract_block(xml, "files_read");
    let files_read: Vec<String> = extract_all_tag_contents(&files_read_block, "file");

    let files_modified_block = extract_block(xml, "files_modified");
    let files_modified: Vec<String> = extract_all_tag_contents(&files_modified_block, "file");

    Some(ObservationOutput {
        observation_type: obs_type,
        title,
        subtitle,
        facts,
        narrative,
        concepts,
        files_read,
        files_modified,
    })
}

pub fn parse_summary_xml(xml: &str) -> Option<SummaryOutput> {
    Some(SummaryOutput {
        request: extract_tag_content(xml, "request"),
        investigated: extract_tag_content(xml, "investigated"),
        learned: extract_tag_content(xml, "learned"),
        completed: extract_tag_content(xml, "completed"),
        next_steps: extract_tag_content(xml, "next_steps"),
        notes: extract_tag_content(xml, "notes"),
    })
}

#[derive(Debug, Clone)]
pub struct SummaryOutput {
    pub request: String,
    pub investigated: String,
    pub learned: String,
    pub completed: String,
    pub next_steps: String,
    pub notes: String,
}

pub fn classify_tool_fallback(
    tool_name: &str,
    tool_input: &str,
    _tool_output: &str,
) -> ObservationOutput {
    let (obs_type, title) = match tool_name {
        "Read" | "Glob" | "Grep" | "LS" => ("discovery", format!("Read: {}", preview_text(tool_input, 80))),
        "Edit" | "Write" | "Patch" => ("change", format!("Modified file")),
        "Bash" => {
            if tool_input.contains("fix") || tool_input.contains("bug") || tool_input.contains("error")
            {
                ("bugfix", format!("Fix: {}", preview_text(tool_input, 80)))
            } else if tool_input.contains("refactor") {
                ("refactor", format!("Refactor: {}", preview_text(tool_input, 80)))
            } else {
                ("change", format!("Command: {}", preview_text(tool_input, 80)))
            }
        }
        _ => ("change", format!("{}: {}", tool_name, preview_text(tool_input, 60))),
    };

    ObservationOutput {
        observation_type: obs_type.to_string(),
        title,
        subtitle: String::new(),
        facts: vec![],
        narrative: format!("Executed {tool_name}"),
        concepts: vec![],
        files_read: vec![],
        files_modified: vec![],
    }
}

fn preview_text(text: &str, max_chars: usize) -> String {
    let trimmed = text.trim();
    if trimmed.chars().count() <= max_chars {
        trimmed.to_string()
    } else {
        format!("{}...", trimmed.chars().take(max_chars).collect::<String>())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_complete_observation() {
        let xml = r#"<observation>
  <type>bugfix</type>
  <title>Fix auth middleware null check</title>
  <subtitle>Auth middleware was crashing on null tokens</subtitle>
  <facts>
    <fact>auth middleware checked req.user but not null case</fact>
    <fact>error happened on all anonymous routes</fact>
  </facts>
  <narrative>Fixed the auth middleware by adding a null check before accessing user properties. This was causing crashes on anonymous routes.</narrative>
  <concepts>
    <concept>gotcha</concept>
    <concept>problem-solution</concept>
  </concepts>
  <files_read>
    <file>src/auth/middleware.ts</file>
    <file>src/auth/types.ts</file>
  </files_read>
  <files_modified>
    <file>src/auth/middleware.ts</file>
  </files_modified>
</observation>"#;

        let result = parse_observation_xml(xml).unwrap();
        assert_eq!(result.observation_type, "bugfix");
        assert_eq!(result.title, "Fix auth middleware null check");
        assert_eq!(result.facts.len(), 2);
        assert_eq!(result.concepts.len(), 2);
        assert_eq!(result.concepts[0], "gotcha");
        assert_eq!(result.files_read.len(), 2);
        assert_eq!(result.files_modified.len(), 1);
    }

    #[test]
    fn test_parse_minimal_observation() {
        let xml = r#"<observation>
  <type>change</type>
  <title>Updated config</title>
  <subtitle></subtitle>
  <facts></facts>
  <narrative>Updated the database config</narrative>
  <concepts></concepts>
  <files_read></files_read>
  <files_modified>
    <file>config.yml</file>
  </files_modified>
</observation>"#;

        let result = parse_observation_xml(xml).unwrap();
        assert_eq!(result.observation_type, "change");
        assert_eq!(result.facts.len(), 0);
        assert_eq!(result.concepts.len(), 0);
    }

    #[test]
    fn test_invalid_type_falls_back() {
        let xml = r#"<observation>
  <type>invalid_type</type>
  <title>Test</title>
  <subtitle></subtitle>
  <facts></facts>
  <narrative>Test</narrative>
  <concepts></concepts>
  <files_read></files_read>
  <files_modified></files_modified>
</observation>"#;

        let result = parse_observation_xml(xml).unwrap();
        assert_eq!(result.observation_type, "change");
    }

    #[test]
    fn test_invalid_concept_filtered() {
        let xml = r#"<observation>
  <type>discovery</type>
  <title>Test</title>
  <subtitle></subtitle>
  <facts></facts>
  <narrative>Test</narrative>
  <concepts>
    <concept>gotcha</concept>
    <concept>invalid_concept</concept>
    <concept>how-it-works</concept>
  </concepts>
  <files_read></files_read>
  <files_modified></files_modified>
</observation>"#;

        let result = parse_observation_xml(xml).unwrap();
        assert_eq!(result.concepts.len(), 2);
        assert_eq!(result.concepts[0], "gotcha");
        assert_eq!(result.concepts[1], "how-it-works");
    }

    #[test]
    fn test_classify_tool_fallback() {
        let result = classify_tool_fallback("Read", "src/auth.ts", "content...");
        assert_eq!(result.observation_type, "discovery");

        let result = classify_tool_fallback("Bash", "fix login bug", "output");
        assert_eq!(result.observation_type, "bugfix");

        let result = classify_tool_fallback("Edit", "src/file.ts", "output");
        assert_eq!(result.observation_type, "change");
    }

    #[test]
    fn test_parse_summary() {
        let xml = r#"<summary>
  <request>Fix authentication bug</request>
  <investigated>Looked at middleware chain</investigated>
  <learned>Auth middleware needs null checks</learned>
  <completed>Fixed and tested the middleware</completed>
  <next_steps>Update documentation</next_steps>
  <notes>Similar pattern in payment middleware</notes>
</summary>"#;

        let result = parse_summary_xml(xml).unwrap();
        assert_eq!(result.request, "Fix authentication bug");
        assert_eq!(result.completed, "Fixed and tested the middleware");
    }
}
