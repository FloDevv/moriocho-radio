// news_fetcher.rs

use rss::Channel;
use scraper::{Html, Selector};
use crate::types::Article;
use futures::stream::{self, StreamExt};
use std::sync::Arc;
use tokio::sync::Semaphore;
use chrono::{DateTime, Utc, Duration};

pub async fn fetch_news(sources: &[&str]) -> Result<Vec<Article>, Box<dyn std::error::Error>> {
    // Shared HTTP client
    let client: Arc<reqwest::Client> = Arc::new(
        reqwest::Client::builder()
            .user_agent("Mozilla/5.0")
            .pool_idle_timeout(std::time::Duration::from_secs(15))
            .build()?
    );

    // Limit of simultaneous connections
    let semaphore: Arc<Semaphore> = Arc::new(Semaphore::new(50));

    // Parallel processing of sources
    let articles: Vec<Article> = stream::iter(sources)
        .map(|source: &&str| {
            let client: Arc<reqwest::Client> = client.clone();
            let semaphore: Arc<Semaphore> = semaphore.clone();

            async move {
                let _permit: tokio::sync::SemaphorePermit<'_> = semaphore.acquire().await?;
                fetch_source(source, &client).await
            }
        })
        .buffered(50)
        .collect::<Vec<_>>()
        .await
        .into_iter()
        .filter_map(Result::ok)
        .flatten()
        .collect();

    Ok(articles)
}

fn truncate_to_char_boundary(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        return s.to_string();
    }

    s.chars()
        .take(max_chars)
        .collect::<String>() + "..."
}

async fn fetch_source(source: &str, client: &reqwest::Client) -> Result<Vec<Article>, Box<dyn std::error::Error>> {
    let content: String = client.get(source)
        .header("Accept-Charset", "UTF-8")
        .send()
        .await?
        .text()
        .await?;

    let channel: Channel = Channel::read_from(content.as_bytes())?;
    let now: DateTime<Utc> = Utc::now();

    // Filter articles by date before processing
    let articles: Vec<Article> = stream::iter(channel.items())
        .filter_map(|item: &rss::Item| async move {
            let pub_date: &str = item.pub_date()?;
            let article_date: DateTime<Utc> = DateTime::parse_from_rfc2822(pub_date)
                .ok()?
                .with_timezone(&Utc);

            if now.signed_duration_since(article_date) <= Duration::days(1) {
                Some(Article {
                    title: item.title().unwrap_or("Untitled").to_string(),
                    source: item.link().unwrap_or("").to_string(),
                    date: item.pub_date().unwrap_or("").to_string(),
                    description: item.description().unwrap_or("No description available").to_string(),
                    content: "".to_string(),
                })
            } else {
                None
            }
        })
        .collect::<Vec<_>>()
        .await;

    Ok(articles)
}

pub async fn fetch_article(
    title: &str,
    link: &str,
    description: &str,
    client: &reqwest::Client,
) -> Result<Article, Box<dyn std::error::Error>> {
    let article_content: String = client.get(link)
        .header("Accept-Charset", "UTF-8")
        .send()
        .await?
        .text()
        .await?;

    let document: Html = Html::parse_document(&article_content);
    let article_selector: Selector = Selector::parse("article p").unwrap();

    let content: String = document.select(&article_selector)
        .map(|x: scraper::ElementRef<'_>| x.text().collect::<String>())
        .collect::<Vec<_>>()
        .join(" ");

    let truncated_content: String = truncate_to_char_boundary(&content, 1024);

    Ok(Article {
        title: title.to_string(),
        content: truncated_content,
        source: link.to_string(),
        date: "".to_string(),
        description: description.to_string(),
    })
}
