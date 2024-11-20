// main.rs
use std::error::Error;
use dotenv::dotenv;
pub mod types;
mod news_fetcher;
mod ai_resume;
mod config;
// main.rs
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenv().ok();

   let config = config::load_config()?;
    let news_sources: Vec<&str> = config.news_sources
        .iter()
        .map(|source| source.url.as_str())
        .collect();
    println!("Fetching news articles...");
    let articles: Vec<types::Article> = news_fetcher::fetch_news(&news_sources).await?;

    // Take only last 5 articles and format them concisely

    let articles_text: String = articles.iter()
        .take(5)
        .map(|article: &types::Article| format!(
            "Date: {}\nHeadline: {}\nSummary: {}\n---\n",
            article.date,
            article.title,
            article.content // Content is already truncated safely
        ))
        .collect::<String>();

    let ai_summary: String = ai_resume::summarize_articles(&articles_text).await?;
    println!("AI Summary:\n{}", ai_summary);

    Ok(())
}
