// ai_resume.rs
use std::env;
use reqwest::Client;
use serde_json::{json, Value};

pub async fn summarize_articles(articles_text: &str) -> Result<String, Box<dyn std::error::Error>> {
    let api_key: String = env::var("API_KEY").expect("API_KEY not set");
    let api_url: String = env::var("API_URL").expect("API_URL not set");
    let language: String = env::var("LANGUAGE").expect("LANGUAGE not set");
    let payload: Value = json!({
        "model": "llama-3.1-70b-versatile",
        "messages": [
            {
                "role": "system",

            "content": format!("You are a skilled journalist who creates concise news summaries. Focus on key events, dates, and developments. You speak and write too in {}", language)
            },
            {
                "role": "user",
                "content": format!("Please provide a comprehensive summary of these news articles, highlighting the most important developments and their dates:\n\n{}", articles_text)
            }
        ],
        "temperature": 0.3,
        "max_tokens": 8000,
        "top_p": 1,
        "stream": false
    });

    let client: Client = Client::new();
    let response: reqwest::Response = client
        .post(api_url)
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&payload)
        .send()
        .await?;

    if !response.status().is_success() {
        let status: reqwest::StatusCode = response.status();
        let error_text: String = response.text().await?;
        return Err(format!("API error: {} - {}", status, error_text).into());
    }

    let body: Value = response.json().await?;
    let content: String = body["choices"][0]["message"]["content"]
        .as_str()
        .ok_or("Failed to extract content")?
        .to_string();

    Ok(content)
}
