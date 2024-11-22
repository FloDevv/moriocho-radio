use std::error::Error;
use dotenv::dotenv;
use ai::{ai_filter, ai_resume};
use fetch::{news_fetcher, meteo};
pub mod types;
mod ai;
mod fetch;
mod config;
use futures::stream::{self, StreamExt};
use reqwest::Client;
use indicatif::{ProgressBar, ProgressStyle};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Initialize shared resources
    dotenv().ok();
    let client: Client = reqwest::Client::new();
    let config: config::Config = config::load_config()?;

    // Create progress style once
    let progress_style: ProgressStyle = ProgressStyle::default_bar()
        .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({percent}%)")
        .unwrap();

    // Fetch weather
    println!("Fetching weather for {}...", config.city);
    let weather: meteo::WeatherResponse = meteo::fetch_weather(&config.city).await?;

    // Fetch and filter articles
    let news_sources: Vec<&str> = config.news_sources
        .iter()
        .map(|source: &config::Source| source.url.as_str())
        .collect();

    let articles: Vec<types::Article> = news_fetcher::fetch_news(&news_sources).await?;

    println!("Filtering articles...");
    let filter_pb: ProgressBar = ProgressBar::new(articles.len() as u64).with_style(progress_style.clone());

    let filtered_articles: Vec<&types::Article> = stream::iter(articles.iter())
        .filter_map(|article: &types::Article| {
            let client: &Client = &client;
            let filter_pb: &ProgressBar = &filter_pb;
            let filter_clone: config::FilterConfig = config.filter.clone();
            async move {
                let is_relevant = ai_filter::filter(
                    &article.title,
                    &article.description,
                    &filter_clone,
                    client
                ).await.unwrap_or(false);
                filter_pb.inc(1);
                if is_relevant { Some(article) } else { None }
            }
        })
        .collect()
        .await;


    filter_pb.finish_with_message("Filtering done");

    // Fetch content for filtered articles
    println!("Fetching article content...");
    let fetch_pb: ProgressBar = ProgressBar::new(filtered_articles.len() as u64).with_style(progress_style);

    let articles_with_content: Vec<types::Article> = stream::iter(filtered_articles)
        .map(|article: &types::Article| {
            let client: &Client = &client;
            let fetch_pb: &ProgressBar = &fetch_pb;
            async move {
                if !article.source.is_empty() {
                    let result: Result<types::Article, Box<dyn Error>> = news_fetcher::fetch_article(
                        &article.title,
                        &article.source,
                        &article.description,
                        client
                    ).await;
                    fetch_pb.inc(1);
                    result.ok()
                } else {
                    fetch_pb.inc(1);
                    None
                }
            }
        })
        .buffer_unordered(10)
        .filter_map(|x: Option<types::Article>| async move { x })
        .collect()
        .await;

    fetch_pb.finish_with_message("Content fetched");

    // Generate summary
    let articles_text: String = articles_with_content.iter()
        .map(|a: &types::Article| format!("Title: {}\nDescription: {}\nContent: {}\n---\n",
            a.title, a.description, a.content))
        .collect::<String>();

    println!("Generating summary...");
    let ai_summary: String = ai_resume::summarize_articles(&weather, &articles_text, &client).await?;
    println!("\nSummary:\n{}", ai_summary);

    Ok(())
}
