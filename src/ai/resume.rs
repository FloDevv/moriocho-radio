use reqwest::Client;
use serde_json::{ json, Value };
use crate::{ config::Config, types::WeatherResponse };

pub async fn ai_resume(
    weather: &WeatherResponse,
    articles_text: &str,
    client: &Client,
    config: &Config
) -> Result<String, Box<dyn std::error::Error>> {
    let weather_info: String = format!(
        "Current weather at {} :\nTime: {}\nTemperature: {}°C\nConditions: {}\n{}",
        weather.city,
        weather.current_weather.time,
        weather.current_weather.temperature,
        weather.current_weather.get_weather_description(),
        weather.get_day_forecast()
    );

    let payload: Value =
        json!({
        "model": "llama-3.3-70b-versatile",
        "messages": [
            {
                "role": "system",
                "content": format!(
                    "This is your host from Morioh-cho Radio, bringing you the latest news! You are a skilled journalist working for Morioh-cho Radio's morning news segment. Start with a good morning greeting, then present today's weather, followed by the news summary. End with 'Have a great day!'. If no articles are provided, mention there is no information today. You speak and write in {}.",
                    &config.language
                )
            },
            {
                "role": "user",
                "content": format!(
                    "Format this summary as a radio show presentation with the following weather information:\n\nWeather Info:\n{}\n\nNews Summary:\n{}",
                    weather_info,
                    articles_text
                )
            }
        ],
        "temperature": 0.1,
        "max_tokens": 8000,
        "top_p": 0.3,
        "stream": false
    });

    let response: reqwest::Response = client
        .post(&config.api_url)
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", &config.api_key))
        .json(&payload)
        .send().await?;

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

pub async fn ai_resume_aggregate(
    partials: &str,
    client: &Client,
    config: &Config
) -> Result<String, Box<dyn std::error::Error>> {
    let payload: Value =
        json!({
        "model": "llama-3.3-70b-versatile",
        "messages": [
            {
                "role": "system",
                "content": "You are a master summarizer. Combine and condense these text  into one coherent summaries. Keep important details and remove redundancies. Write in plain text, no markdown."
            },
            {
                "role": "user",
                "content": format!("Partial text:\n\n{}", partials)
            }
        ],
        "temperature": 0.1,
        "max_tokens": 8000,
        "top_p": 0.3,
        "stream": false
    });

    let response: reqwest::Response = client
        .post(&config.api_url)
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", &config.api_key))
        .json(&payload)
        .send().await?;

    if !response.status().is_success() {
        return Err(format!("API error: {} - {}", response.status(), response.text().await?).into());
    }

    let body: Value = response.json::<serde_json::Value>().await?;
    let content: String = body["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("")
        .to_string();

    Ok(content)
}
