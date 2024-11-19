// main.rs
use std::error::Error;
pub mod types;
mod news_fetcher;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let news_sources = vec![
        "https://www.lemonde.fr/politique/rss_full.xml",
    ];

    let articles = news_fetcher::fetch_news(&news_sources).await?;

    // test
    for article in articles {
        println!("Titre: {}", article.title);
        println!("Contenu: {}\n", article.content);
    }

    Ok(())
}
