#[derive(Debug)]
pub struct RssItem {
    pub title: String,
    pub description: String,
    pub link: String,
    pub date: String,
}

pub struct Article {
    pub title: String,
    pub content: String,
    pub source: String,
    pub date: String,
    pub description: String,
}
