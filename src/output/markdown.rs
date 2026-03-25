use anyhow::Result;

pub fn html_to_markdown(html: &str) -> Result<String> {
    let md = htmd::HtmlToMarkdown::builder()
        .skip_tags(vec!["script", "style", "noscript", "svg"])
        .build()
        .convert(html)
        .map_err(|e| anyhow::anyhow!("markdown conversion failed: {}", e))?;
    Ok(md)
}
