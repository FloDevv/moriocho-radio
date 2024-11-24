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
    const MAX_RETRIES: u32 = 10;
    const TIMEOUT_SECS: u64 = 10;

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

    for attempt in 0..MAX_RETRIES {
        match tokio::time::timeout(
            std::time::Duration::from_secs(TIMEOUT_SECS),
            client.post(&api_url)
                .header("Authorization", format!("Bearer {}", api_key))
                .json(&payload)
                .send()
        ).await {
            Ok(response_result) => {
                match response_result {
                    Ok(response) => {
                        if !response.status().is_success() {
                            let status: reqwest::StatusCode = response.status();
                            let error_text: String = response.text().await?;
                            eprintln!("API error on attempt {}: {} - {}", attempt + 1, status, error_text);
                            if attempt == MAX_RETRIES - 1 {
                                return Ok(false);
                            }
                            tokio::time::sleep(std::time::Duration::from_secs(2 * (attempt + 1) as u64)).await;
                            continue;
                        }

                        match response.json::<Value>().await {
                            Ok(body) => {
                                let content = body["choices"][0]["message"]["content"]
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

                                return Ok(is_relevant);
                            }
                            Err(e) => {
                                eprintln!("JSON parse error on attempt {}: {}", attempt + 1, e);
                                if attempt == MAX_RETRIES - 1 {
                                    return Ok(false);
                                }
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Request error on attempt {}: {}", attempt + 1, e);
                        if attempt == MAX_RETRIES - 1 {
                            return Ok(false);
                        }
                    }
                }
            }
            Err(_) => {
                eprintln!("Timeout on attempt {}", attempt + 1);
                if attempt == MAX_RETRIES - 1 {
                    return Ok(false);
                }
            }
        }

        tokio::time::sleep(std::time::Duration::from_secs(2 * (attempt + 1) as u64)).await;
    }

    Ok(false)
}
