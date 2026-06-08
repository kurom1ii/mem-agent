use crate::observer::config::{
    FOOTER, OBSERVER_ROLE, RECORDING_FOCUS, SKIP_GUIDANCE, SYSTEM_IDENTITY,
};

const MAX_TOOL_RESPONSE_LENGTH: usize = 16000;
const TRUNCATION_MARKER: &str = "\n\n<truncated>\n\n";

pub fn truncate_tool_output(text: &str) -> String {
    if text.len() <= MAX_TOOL_RESPONSE_LENGTH {
        return text.to_string();
    }

    let head_len = (MAX_TOOL_RESPONSE_LENGTH as f64 * 0.6) as usize;
    let tail_len = (MAX_TOOL_RESPONSE_LENGTH as f64 * 0.3) as usize;

    let head = &text[..head_len.min(text.len())];
    let tail_start = text.len().saturating_sub(tail_len).max(head_len);
    let tail = &text[tail_start..];

    format!("{head}{TRUNCATION_MARKER}{tail}")
}

pub fn build_observation_prompt(
    tool_name: &str,
    tool_input: &str,
    tool_output: &str,
    cwd: &str,
    timestamp: &str,
    user_prompt: Option<&str>,
) -> String {
    let truncated = truncate_tool_output(tool_output);

    let mut prompt = String::new();
    prompt.push_str(SYSTEM_IDENTITY);
    prompt.push_str("\n\n");

    prompt.push_str("<observed_from_primary_session>\n");
    prompt.push_str(&format!(
        "  <what_happened>{tool_name}</what_happened>\n"
    ));

    if let Some(p) = user_prompt {
        prompt.push_str(&format!("  <user_request>{p}</user_request>\n"));
    }

    prompt.push_str(&format!("  <occurred_at>{timestamp}</occurred_at>\n"));
    prompt.push_str(&format!(
        "  <working_directory>{cwd}</working_directory>\n"
    ));

    if !tool_input.is_empty() && tool_input != "{}" {
        prompt.push_str("  <parameters>\n");
        prompt.push_str(&format!("{tool_input}\n"));
        prompt.push_str("  </parameters>\n");
    }

    prompt.push_str("  <outcome>\n");
    prompt.push_str(&truncated);
    prompt.push_str("\n  </outcome>\n");
    prompt.push_str("</observed_from_primary_session>\n\n");

    prompt.push_str(OBSERVER_ROLE);
    prompt.push_str("\n\n");
    prompt.push_str(RECORDING_FOCUS);
    prompt.push_str("\n\n");
    prompt.push_str(SKIP_GUIDANCE);
    prompt.push_str("\n\n");
    prompt.push_str(FOOTER);
    prompt.push_str("\n\n");

    prompt.push_str("Respond with exactly one <observation> block:\n");
    prompt.push_str("<observation>\n");
    prompt.push_str("  <type>bugfix|feature|refactor|change|discovery|decision|security_alert|security_note</type>\n");
    prompt.push_str("  <title>brief title</title>\n");
    prompt.push_str("  <subtitle>one line summary</subtitle>\n");
    prompt.push_str("  <facts>\n");
    prompt.push_str(
        "    <fact>key factual statement about what happened</fact>\n",
    );
    prompt.push_str("  </facts>\n");
    prompt.push_str("  <narrative>what was done and why</narrative>\n");
    prompt.push_str("  <concepts>\n");
    prompt.push_str("    <concept>how-it-works|why-it-exists|what-changed|problem-solution|gotcha|pattern|trade-off</concept>\n");
    prompt.push_str("  </concepts>\n");
    prompt.push_str("  <files_read>\n");
    prompt.push_str(
        "    <file>/relative/path</file>\n",
    );
    prompt.push_str("  </files_read>\n");
    prompt.push_str("  <files_modified>\n");
    prompt.push_str(
        "    <file>/relative/path</file>\n",
    );
    prompt.push_str("  </files_modified>\n");
    prompt.push_str("</observation>\n");

    prompt
}

pub fn build_summary_prompt(session_title: &str, observation_count: usize) -> String {
    let mut prompt = String::new();
    prompt.push_str(SYSTEM_IDENTITY);
    prompt.push_str("\n\n");
    prompt.push_str("--- MODE SWITCH: PROGRESS SUMMARY ---\n\n");
    prompt.push_str(&format!(
        "Summarize the session \"{session_title}\" which has {observation_count} observations.\n\n"
    ));
    prompt.push_str("Respond with exactly one <summary> block:\n");
    prompt.push_str("<summary>\n");
    prompt.push_str("  <request>what the user asked for</request>\n");
    prompt.push_str("  <investigated>what was investigated</investigated>\n");
    prompt.push_str("  <learned>what was learned</learned>\n");
    prompt.push_str("  <completed>what was completed</completed>\n");
    prompt.push_str("  <next_steps>suggested next steps</next_steps>\n");
    prompt.push_str("  <notes>additional notes</notes>\n");
    prompt.push_str("</summary>\n");

    prompt
}
