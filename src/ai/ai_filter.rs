// src/ai/ai_filter.rs
use std::env;
use reqwest::Client;
use serde_json::{json, Value};
// use tokio::time::{sleep, Duration};
use crate::config::FilterConfig;

fn validate_ai_response(content: &str) -> bool {
    let normalized: String = content.trim().to_lowercase();
    match normalized.as_str() {
        "true" => true,
        "false" => false,
        _ => {
            println!("⚠️ Warning: AI response format incorrect: '{}'", content);
            false
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
                    "You are a news filter. Respond with 'true' if the title and description match any of these categories: {}. Otherwise, respond with 'false'.",
                    categories
                )
            },
            {
                "role": "user",
                "content": format!("Title: {}\nDescription: {}", title, description)
            }
        ],
        "temperature": 0.2,
        "max_tokens": 7,
        "top_p": 0.3,
    });

    let response: reqwest::Response = client
        .post(&api_url)
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&payload)
        .send()
        .await?;

    let body: Value = response.json().await?;
    let content: &str = body["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("false")
        .trim();

    let is_relevant: bool = validate_ai_response(content);
    // println!("Title: '{}'\n Description: '{}'\n  AI Decision: {}",
    //     title,
    //     description,
    //     if is_relevant { "✅True" } else { "❌False" },
    // );

    // sleep(Duration::from_secs(2)).await;

    Ok(is_relevant)
}


