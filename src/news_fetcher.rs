use rss::Channel;
use scraper::{Html, Selector};
use crate::types::Article;
use futures::stream::{self, StreamExt};
use std::sync::Arc;
use tokio::sync::Semaphore;

pub async fn fetch_news(sources: &[&str]) -> Result<Vec<Article>, Box<dyn std::error::Error>> {
    // Shared HTTP client
    let client = Arc::new(
        reqwest::Client::builder()
            .user_agent("Mozilla/5.0")
            .pool_idle_timeout(std::time::Duration::from_secs(15))
            .build()?
    );

    // Limit of simultaneous connections
    let semaphore = Arc::new(Semaphore::new(5));

    // Parallel processing of sources
    let articles = stream::iter(sources)
        .map(|source| {
            let client = client.clone();
            let semaphore = semaphore.clone();

            async move {
                let _permit = semaphore.acquire().await?;
                fetch_source(source, &client).await
            }
        })
        .buffered(10) // Limit the number of simultaneous tasks
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
    let content = client.get(source)
        .header("Accept-Charset", "UTF-8")
        .send()
        .await?
        .text()
        .await?;

    let channel = Channel::read_from(content.as_bytes())?;

    // Parallel processing of articles
    let articles = stream::iter(channel.items())
        .filter_map(|item| async move {
            let link = item.link()?;
            Some((item, link))
        })
        .map(|(item, link)| {
            let client = client.clone();

            async move {
                match fetch_article(&item, link, client).await {
                    Ok(article) => Some(article),
                    Err(_) => None,
                }
            }
        })
        .buffered(5)
        .collect::<Vec<_>>()
        .await
        .into_iter()
        .flatten()
        .collect();

    Ok(articles)
}


async fn fetch_article(
    item: &rss::Item,
    link: &str,
    client: reqwest::Client,
) -> Result<Article, Box<dyn std::error::Error>> {
    let article_content = client.get(link)
        .header("Accept-Charset", "UTF-8")
        .send()
        .await?
        .text()
        .await?;

    let document = Html::parse_document(&article_content);
    let article_selector = Selector::parse("article p").unwrap();

    let content = document.select(&article_selector)
        .map(|x| x.text().collect::<String>())
        .collect::<Vec<_>>()
        .join(" ");

    // Use safe truncation
    let truncated_content = truncate_to_char_boundary(&content, 500);

    Ok(Article {
        title: item.title().unwrap_or("Untitled").to_string(),
        content: truncated_content,
        source: link.to_string(),
        date: item.pub_date().unwrap_or("").to_string(),
    })
}
