use scraper::{Html, Selector};

pub struct SearchResult {
    pub title: String,
    pub snippet: String,
    pub full_text: Option<String>,
}

pub fn web_search(query: &str) -> Vec<SearchResult> {
    let encoded = url::form_urlencoded::byte_serialize(query.as_bytes()).collect::<String>();
    let target = format!("https://html.duckduckgo.com/html/?q={encoded}");
    let body = match ureq::get(&target)
        .set("User-Agent", "Mozilla/5.0 (compatible; MD1RoBoT/1.0)")
        .timeout(std::time::Duration::from_secs(20))
        .call()
    {
        Ok(resp) => resp.into_string().unwrap_or_default(),
        Err(_) => return vec![],
    };
    let document = Html::parse_document(&body);
    let result_sel = Selector::parse(".result").unwrap();
    let title_sel = Selector::parse(".result__title").unwrap();
    let snippet_sel = Selector::parse(".result__snippet").unwrap();
    let link_sel = Selector::parse("a.result__a").unwrap();
    let mut results = vec![];
    for el in document.select(&result_sel).take(3) {
        let title = el.select(&title_sel).next().map(|t| t.text().collect::<String>().trim().to_string()).unwrap_or_default();
        let snippet = el.select(&snippet_sel).next().map(|s| s.text().collect::<String>().trim().to_string()).unwrap_or_default();
        let link = el.select(&link_sel).next().and_then(|a| a.value().attr("href")).unwrap_or("");
        let full_text = if !link.is_empty() { fetch_article_text(link) } else { None };
        if !title.is_empty() {
            results.push(SearchResult { title, snippet, full_text });
        }
    }
    results
}

fn fetch_article_text(url_str: &str) -> Option<String> {
    use url::Url;
    let url = Url::parse(url_str).ok()?;
    if url.scheme() != "https" && url.scheme() != "http" { return None; }
    let resp = ureq::get(url.as_str())
        .set("User-Agent", "Mozilla/5.0")
        .timeout(std::time::Duration::from_secs(10))
        .call().ok()?.into_string().ok()?;
    let doc = Html::parse_document(&resp);
    let p_sel = Selector::parse("p").unwrap();
    let text: String = doc.select(&p_sel).flat_map(|p| p.text()).collect::<Vec<_>>().join(" ");
    if text.len() > 100 { Some(text.chars().take(2000).collect()) } else { None }
}
