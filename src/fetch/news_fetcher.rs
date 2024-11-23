use rss::Channel;
use scraper::{Html, Selector};
use crate::types::Article;
use futures::stream::{self, StreamExt};
use std::sync::Arc;
use tokio::sync::{Mutex, Semaphore};
use chrono::{DateTime, Utc, Duration};
use std::collections::HashSet;

pub async fn fetch_news(sources: &[&str]) -> Result<Vec<Article>, Box<dyn std::error::Error>> {
    let client: Arc<reqwest::Client> = Arc::new(
        reqwest::Client::builder()
            .user_agent("Mozilla/5.0")
            .pool_idle_timeout(std::time::Duration::from_secs(15))
            .build()?
    );

    let shared: Arc<(Semaphore, Mutex<HashSet<String>>)> = Arc::new((
        Semaphore::new(50),
        Mutex::new(HashSet::new())
    ));

    let results: Vec<Article> = stream::iter(sources)
        .map(|&source| {
            let client: Arc<reqwest::Client> = client.clone();
            let shared: Arc<(Semaphore, Mutex<HashSet<String>>)> = shared.clone();

            async move {
                let _permit: tokio::sync::SemaphorePermit<'_> = shared.0.acquire().await?;
                let articles: Vec<Article> = fetch_source(source, &client).await?;

                let mut titles: tokio::sync::MutexGuard<'_, HashSet<String>> = shared.1.lock().await;
                Ok::<Vec<Article>, Box<dyn std::error::Error>>(articles.into_iter()
                    .filter(|a: &Article| titles.insert(a.title.clone()))
                    .collect::<Vec<_>>())
            }
        })
        .buffered(50)
        .collect::<Vec<_>>()
        .await
        .into_iter()
        .filter_map(Result::ok)
        .flatten()
        .collect::<Vec<_>>();

    Ok(results)
}
async fn fetch_source(source: &str, client: &reqwest::Client) -> Result<Vec<Article>, Box<dyn std::error::Error>> {
    let channel: Channel = Channel::read_from(
        client.get(source)
            .header("Accept-Charset", "UTF-8")
            .send()
            .await?
            .text()
            .await?
            .as_bytes()
    )?;

    let now: DateTime<Utc> = Utc::now();

    Ok(stream::iter(channel.items())
        .filter_map(|item: &rss::Item| async move {
            let date: DateTime<Utc> = DateTime::parse_from_rfc2822(item.pub_date()?).ok()?
                .with_timezone(&Utc);

            if now.signed_duration_since(date) <= Duration::days(1) {
                Some(Article {
                    title: item.title().unwrap_or("Untitled").into(),
                    source: item.link().unwrap_or("").into(),
                    date: item.pub_date().unwrap_or("").into(),
                    description: item.description().unwrap_or("No description available").into(),
                    content: String::new(),
                })
            } else {
                None
            }
        })
        .collect()
        .await)
}

pub async fn fetch_article(title: &str, link: &str, description: &str, client: &reqwest::Client)
    -> Result<Article, Box<dyn std::error::Error>>
{
    let html_content: String = client.get(link)
        .header("Accept-Charset", "UTF-8")
        .send()
        .await?
        .text()
        .await?;

    let content: String = Html::parse_document(&html_content)
        .select(&Selector::parse("article p").unwrap())
        .map(|x: scraper::ElementRef<'_>| x.text().collect::<String>())
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(1024)
        .collect::<String>();

    Ok(Article {
        title: title.into(),
        content,
        source: link.into(),
        date: String::new(),
        description: description.into(),
    })
}
