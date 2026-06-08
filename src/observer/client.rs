use serde_json::{json, Value};

use crate::observer::config::ObservationOutput;
use crate::observer::prompt::build_observation_prompt;

const OPENCODE_GO_API_URL: &str = "https://opencode.ai/zen/go/v1/chat/completions";
const OPENCODE_GO_MODEL: &str = "deepseek-v4-flash";

#[derive(Debug, Clone)]
pub struct ObserverClient {
    api_url: String,
    api_key: String,
    model: String,
}

impl ObserverClient {
    pub fn from_env_or_config() -> Option<Self> {
        let api_key = std::env::var("MEMAGENT_OBSERVER_API_KEY")
            .or_else(|_| std::env::var("DEEPSEEK_API_KEY"))
            .or_else(|_| Self::read_opencode_go_key())
            .ok()?;

        let api_url = std::env::var("MEMAGENT_OBSERVER_API_URL")
            .unwrap_or_else(|_| OPENCODE_GO_API_URL.to_string());

        let model = std::env::var("MEMAGENT_OBSERVER_MODEL")
            .unwrap_or_else(|_| OPENCODE_GO_MODEL.to_string());

        Some(Self { api_url, api_key, model })
    }

    fn read_opencode_go_key() -> Result<String, std::io::Error> {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
        let auth_path = format!("{home}/.local/share/opencode/auth.json");
        let content = std::fs::read_to_string(&auth_path)?;
        let json: Value = serde_json::from_str(&content)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        json["opencode-go"]["key"]
            .as_str()
            .map(String::from)
            .ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "opencode-go key not found in auth.json",
                )
            })
    }

    pub fn classify_observation(
        &self,
        tool_name: &str,
        tool_input: &str,
        tool_output: &str,
        cwd: &str,
        timestamp: &str,
        user_prompt: Option<&str>,
    ) -> Option<ObservationOutput> {
        let prompt = build_observation_prompt(
            tool_name, tool_input, tool_output, cwd, timestamp, user_prompt,
        );

        let response_text = self.call_api(&prompt)?;
        crate::observer::classifier::parse_observation_xml(&response_text)
    }

    pub fn summarize_session(
        &self,
        session_title: &str,
        observation_count: usize,
    ) -> Option<String> {
        let prompt =
            crate::observer::prompt::build_summary_prompt(session_title, observation_count);

        self.call_api(&prompt)
    }

    fn call_api(&self, prompt: &str) -> Option<String> {
        let client = reqwest::blocking::Client::new();

        let body = json!({
            "model": self.model,
            "messages": [
                {
                    "role": "system",
                    "content": "You are a memory observer. Output only XML. No explanations outside tags."
                },
                {
                    "role": "user",
                    "content": prompt
                }
            ],
            "temperature": 0.1,
            "max_tokens": 2048,
            "stream": false
        });

        let response = client
            .post(&self.api_url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .ok()?;

        if !response.status().is_success() {
            eprintln!(
                "[observer] API error: {} {}",
                response.status(),
                response.text().unwrap_or_default()
            );
            return None;
        }

        let json: Value = response.json().ok()?;
        let content = json["choices"][0]["message"]["content"]
            .as_str()
            .or_else(|| json["choices"][0]["text"].as_str())?;

        Some(content.to_string())
    }
}
