// main.rs

use std::error::Error;
use dotenv::dotenv;
use ai::{ai_filter, ai_resume};
use fetch::news_fetcher;
pub mod types;
mod ai;
mod fetch;
mod config;
use futures::stream::{self, StreamExt};
use reqwest::Client;
use indicatif::{ProgressBar, ProgressStyle};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenv().ok();
    let client: Client = reqwest::Client::new();
    let config: config::Config = config::load_config()?;
    let news_sources: Vec<&str> = config.news_sources
        .iter()
        .map(|source: &config::Source| source.url.as_str())
        .collect();

    let articles: Vec<types::Article> = news_fetcher::fetch_news(&news_sources).await?;
    // Initialize Progress Bar for Filtering
    println!("Filtering articles...");
    let filter_pb = ProgressBar::new(200);
    filter_pb.set_style(ProgressStyle::default_bar()
        .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({percent}%)")
        .unwrap());

    // Filter articles by title and description
    let mut filtered_articles: Vec<&types::Article> = Vec::new();
    for article in articles.iter().take(200) {
        if ai_filter::filter(&article.title, &article.description, &config.filter, &client).await? {
            filtered_articles.push(article);
        }
        filter_pb.inc(1);
    }
    filter_pb.finish_with_message("Filtering done");


    println!("Prepare Summarize with more content.");
    // Initialize Progress Bar for Fetching Content
    let fetch_pb = ProgressBar::new(filtered_articles.len() as u64);
    fetch_pb.set_style(ProgressStyle::default_bar()
        .template("{spinner:.blue} [{elapsed_precise}] [{bar:40.yellow/blue}] {pos}/{len} ({percent}%)")
        .unwrap());

    // Retrieve the content of the filtered articles with HTML scraping
    let articles_with_content: Vec<types::Article> = stream::iter(filtered_articles)
        .map(|article: &types::Article| {
            let client = &client;
            let title = &article.title;
            let link = &article.source;
            let description = &article.description;
            let fetch_pb = fetch_pb.clone();
            async move {
                if !link.is_empty() {
                    let result = news_fetcher::fetch_article(title, link, description, client).await;
                    fetch_pb.inc(1);
                    result
                } else {
                    fetch_pb.inc(1);
                    Err("No link available".into())
                }
            }
        })
        .buffer_unordered(10) // Adjust concurrency as needed
        .filter_map(|result: Result<types::Article, Box<dyn Error>>| async move { result.ok() })
        .collect()
        .await;

    fetch_pb.finish_with_message("Fetching content done");

    // Generate a summary of the filtered articles
    let articles_text: String = articles_with_content.iter()
        .map(|article: &types::Article| format!(
            "Title: {}\nDescription: {}\nContent: {}\n---\n",
            article.title,
            article.description,
            article.content
        ))
        .collect::<String>();

    // Initialize Progress Bar for Summarizing
    let summarize_pb = ProgressBar::new(1);
    summarize_pb.set_style(ProgressStyle::default_spinner()
        .template("{spinner:.green} {msg}")
        .unwrap());

    summarize_pb.set_message("Summarizing articles...");
    let ai_summary: String = ai_resume::summarize_articles(&articles_text, &client).await?;
    summarize_pb.finish_with_message("Summarization done \n Filtered articles summary:");

    println!("\n{}", ai_summary);

    Ok(())
}
