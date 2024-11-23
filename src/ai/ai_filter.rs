// src/ai/ai_filter.rs
use std::env;
use reqwest::Client;
use serde_json::{json, Value};
use crate::config::FilterConfig;

fn validate_ai_response(content: &str) -> bool {
    let normalized: String = content
        .trim()
        .to_lowercase()
        .chars()
        .filter(|c: &char| !c.is_whitespace())
        .collect::<String>();

    match normalized.as_str() {
        "true" | "yes" | "1" | "correct" => true,
        "false" | "no" | "0" | "incorrect" => false,
        other => {
            eprintln!("⚠️ Warning: Unexpected AI response format: '{}' (normalized: '{}')", content, other);
            if other.contains("true") || other.contains("yes") {
                true
            } else {
                false
            }
        }
    }
}

pub async fn filter(
    title: &str,
    description: &str,
    filter_config: &FilterConfig,
    client: &Client,
) -> Result<bool, Box<dyn std::error::Error>> {
    let api_key: String = env::var("API_KEY")?;
    let api_url: String = env::var("API_URL")?;
    let categories: String = filter_config.categories.join(", ");

    let payload: Value = json!({
        "model": "gemma2-9b-it",
        "messages": [
            {
                "role": "system",
                "content": format!(
                    "You are a news filter. You MUST respond with ONLY 'true' or 'false'.\n\
                    RULES:\n\
                    1. Answer 'true' if content matches any category: {}\n\
                    2. Answer 'false' if no match\n\
                    3. ONLY respond with the single word 'true' or 'false'",
                    categories
                )
            },
            {
                "role": "user",
                "content": format!(
                    "Evaluate if this content matches any category:\nTitle: {}\nDescription: {}",
                    title, description
                )
            }
        ],
        "temperature": 0.1,
        "max_tokens": 5,
        "top_p": 0.1,
    });

    let response: reqwest::Response = client
        .post(&api_url)
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&payload)
        .send()
        .await?;

    let status: reqwest::StatusCode = response.status();
    if !status.is_success() {
        let error_text: String = response.text().await?;
        return Err(format!("API error: {} - {}", status, error_text).into());
    }

    let body: Value = response.json().await?;
    let content: &str = body["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("false")
        .trim();

    let is_relevant: bool = validate_ai_response(content);

    #[cfg(debug_assertions)]
    println!(
        "Filter: '{}'\nResponse: '{}' -> {}",
        title,
        content,
        if is_relevant { "✅" } else { "❌" }
    );

    Ok(is_relevant)
}
