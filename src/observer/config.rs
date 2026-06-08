use serde::{Deserialize, Serialize};

use crate::core::types::{ModeConfig, ObservationConcept, ObservationType};

pub fn get_code_mode() -> ModeConfig {
    ModeConfig {
        name: "Code Development".into(),
        description: "Default mode for software development memories".into(),
        version: "1.0.0".into(),
        observation_types: vec![
            ObservationType {
                id: "bugfix".into(),
                label: "Bug Fix".into(),
                description: "Something was broken, now fixed".into(),
            },
            ObservationType {
                id: "feature".into(),
                label: "Feature".into(),
                description: "New capability or functionality added".into(),
            },
            ObservationType {
                id: "refactor".into(),
                label: "Refactor".into(),
                description: "Code restructured, behavior unchanged".into(),
            },
            ObservationType {
                id: "change".into(),
                label: "Change".into(),
                description: "Generic modification (docs, config, misc)".into(),
            },
            ObservationType {
                id: "discovery".into(),
                label: "Discovery".into(),
                description: "Learning about existing system".into(),
            },
            ObservationType {
                id: "decision".into(),
                label: "Decision".into(),
                description: "Architectural/design choice with rationale".into(),
            },
            ObservationType {
                id: "security_alert".into(),
                label: "Security Alert".into(),
                description: "A security issue that needs attention".into(),
            },
            ObservationType {
                id: "security_note".into(),
                label: "Security Note".into(),
                description: "A security-relevant observation, not urgent".into(),
            },
        ],
        observation_concepts: vec![
            ObservationConcept {
                id: "how-it-works".into(),
                label: "How It Works".into(),
                description: "Explanation of mechanism".into(),
            },
            ObservationConcept {
                id: "why-it-exists".into(),
                label: "Why It Exists".into(),
                description: "Reason for existence".into(),
            },
            ObservationConcept {
                id: "what-changed".into(),
                label: "What Changed".into(),
                description: "Summary of changes made".into(),
            },
            ObservationConcept {
                id: "problem-solution".into(),
                label: "Problem → Solution".into(),
                description: "Problem and its resolution".into(),
            },
            ObservationConcept {
                id: "gotcha".into(),
                label: "Gotcha".into(),
                description: "Tricky or unexpected behavior".into(),
            },
            ObservationConcept {
                id: "pattern".into(),
                label: "Pattern".into(),
                description: "Reusable pattern or idiom".into(),
            },
            ObservationConcept {
                id: "trade-off".into(),
                label: "Trade-off".into(),
                description: "Design trade-off".into(),
            },
        ],
    }
}

pub const SYSTEM_IDENTITY: &str = r#"You are a memory observer. Your job is to analyze tool executions and
classify them into structured observations.

You do NOT have access to any tools. You only output XML.
Do NOT write explanations outside the XML tags.
Do NOT suggest code or actions.
Only classify what you observe."#;

pub const OBSERVER_ROLE: &str = r#"Classify the observation below into one of these types:
- bugfix: something was broken, now fixed
- feature: new capability added
- refactor: code restructured, behavior unchanged
- change: generic modification (docs, config, misc)
- discovery: learning about existing system
- decision: architectural/design choice with rationale
- security_alert: security issue needing attention
- security_note: security-relevant observation, not urgent

Tag with concepts:
- how-it-works: explanation of mechanism
- why-it-exists: reason for existence
- what-changed: summary of changes
- problem-solution: problem and resolution
- gotcha: tricky or unexpected behavior
- pattern: reusable pattern
- trade-off: design trade-off

Output ONLY the <observation> XML block."#;

pub const RECORDING_FOCUS: &str = r#"Focus on:
1. Decisions the user made and WHY
2. Problems encountered and HOW they were solved
3. New information discovered about the codebase
4. Patterns and conventions learned
5. Security issues identified

Skip or classify as "change":
- Routine file reads without analysis
- Trivial linting/formatting
- Simple variable renames
- Boilerplate additions
- Standard import statements"#;

pub const SKIP_GUIDANCE: &str = r#"When in doubt, classify as "change".
Only use "bugfix" when something was clearly broken and now fixed.
Only use "feature" when new capability was added.
Only use "discovery" when genuinely new information was learned.
Only use "security_alert" when an active security issue is found."#;

pub const FOOTER: &str = "Output only valid XML. No markdown, no explanations outside the tags.";

pub const VALID_OBSERVATION_TYPES: &[&str] = &[
    "bugfix", "feature", "refactor", "change", "discovery",
    "decision", "security_alert", "security_note",
];

pub const VALID_CONCEPTS: &[&str] = &[
    "how-it-works", "why-it-exists", "what-changed",
    "problem-solution", "gotcha", "pattern", "trade-off",
];

pub fn validate_type(t: &str) -> &str {
    let t = t.trim();
    if VALID_OBSERVATION_TYPES.contains(&t) {
        t
    } else {
        "change"
    }
}

pub fn validate_concept(c: &str) -> Option<&str> {
    let c = c.trim();
    if VALID_CONCEPTS.contains(&c) {
        Some(c)
    } else {
        None
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservationOutput {
    pub observation_type: String,
    pub title: String,
    pub subtitle: String,
    pub facts: Vec<String>,
    pub narrative: String,
    pub concepts: Vec<String>,
    pub files_read: Vec<String>,
    pub files_modified: Vec<String>,
}

impl Default for ObservationOutput {
    fn default() -> Self {
        Self {
            observation_type: "change".into(),
            title: "Observation".into(),
            subtitle: String::new(),
            facts: Vec::new(),
            narrative: String::new(),
            concepts: Vec::new(),
            files_read: Vec::new(),
            files_modified: Vec::new(),
        }
    }
}
