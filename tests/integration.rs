use clawbrowser::dom::{query_selector, query_selector_all, serialize_inner_html, serialize_to_html, NodeType};
use clawbrowser::html::parse_html;
use clawbrowser::output::{cleanup_intrusive_overlays, html_to_markdown};

// --- DOM Parsing Tests ---

#[test]
fn parse_basic_html() {
    let html = r#"<!DOCTYPE html><html><head><title>Test</title></head><body><p>Hello</p></body></html>"#;
    let doc = parse_html(html, None);

    assert_eq!(doc.title(), Some("Test".to_string()));
    assert!(doc.body().is_some());
    assert!(doc.document_element().is_some());
}

#[test]
fn parse_nested_elements() {
    let html = r#"<html><body>
        <div id="main">
            <h1>Title</h1>
            <ul><li>A</li><li>B</li><li>C</li></ul>
        </div>
    </body></html>"#;
    let doc = parse_html(html, None);

    let main = doc.get_element_by_id("main");
    assert!(main.is_some());

    let by_tag = doc.get_elements_by_tag_name("li");
    assert_eq!(by_tag.len(), 3);
}

#[test]
fn parse_malformed_html() {
    let html = r#"<p>unclosed paragraph<div>nested<span>deep"#;
    let doc = parse_html(html, None);
    assert!(doc.body().is_some());
}

#[test]
fn get_element_by_id() {
    let html = r#"<html><body><div id="target">found</div></body></html>"#;
    let doc = parse_html(html, None);

    let node = doc.get_element_by_id("target").expect("should find element");
    let data = doc.arena.get(node);
    if let NodeType::Element(ref e) = data.node_type {
        assert_eq!(e.tag_name(), "div");
    } else {
        panic!("expected Element");
    }
}

#[test]
fn query_selector_basic() {
    let html = r#"<html><body>
        <div class="a"><p class="b">text</p></div>
    </body></html>"#;
    let doc = parse_html(html, None);

    let result = query_selector(&doc.arena, doc.document_node, "div.a p.b");
    assert!(result.is_some());
}

#[test]
fn query_selector_all_test() {
    let html = r#"<html><body>
        <span class="x">1</span>
        <span class="x">2</span>
        <span>3</span>
    </body></html>"#;
    let doc = parse_html(html, None);

    let results = query_selector_all(&doc.arena, doc.document_node, "span.x");
    assert_eq!(results.len(), 2);
}

// --- Serialization Tests ---

#[test]
fn serialize_roundtrip() {
    let html = r#"<html><head><title>RT</title></head><body><div id="box"><p>hello</p></div></body></html>"#;
    let doc = parse_html(html, None);

    let output = serialize_to_html(&doc.arena, doc.document_node);
    assert!(output.contains("<title>RT</title>"));
    assert!(output.contains(r#"<div id="box">"#));
    assert!(output.contains("<p>hello</p>"));
}

#[test]
fn serialize_inner_html_test() {
    let html = r#"<html><body><div id="c"><b>bold</b> text</div></body></html>"#;
    let doc = parse_html(html, None);

    let div = doc.get_element_by_id("c").unwrap();
    let inner = serialize_inner_html(&doc.arena, div);
    assert!(inner.contains("<b>bold</b>"));
    assert!(inner.contains("text"));
}

// --- DOM Manipulation Tests ---

#[test]
fn create_and_append_element() {
    let html = r#"<html><body><div id="root"></div></body></html>"#;
    let mut doc = parse_html(html, None);

    let root = doc.get_element_by_id("root").unwrap();
    let new_p = doc.create_element("p");
    let text = doc.create_text_node("dynamic");
    doc.arena.append_child(new_p, text);
    doc.arena.append_child(root, new_p);

    let inner = serialize_inner_html(&doc.arena, root);
    assert!(inner.contains("<p>dynamic</p>"));
}

#[test]
fn set_and_get_attribute() {
    let html = r#"<html><body><a id="link">click</a></body></html>"#;
    let mut doc = parse_html(html, None);

    let link = doc.get_element_by_id("link").unwrap();
    if let NodeType::Element(ref mut e) = doc.arena.get_mut(link).node_type {
        e.set_attribute("href", "https://example.com".to_string());
        e.set_attribute("target", "_blank".to_string());
    }

    if let NodeType::Element(ref e) = doc.arena.get(link).node_type {
        assert_eq!(e.get_attribute("href"), Some("https://example.com"));
        assert_eq!(e.get_attribute("target"), Some("_blank"));
    } else {
        panic!("expected Element");
    }
}

#[test]
fn remove_child() {
    let html = r#"<html><body><ul id="list"><li>A</li><li>B</li></ul></body></html>"#;
    let mut doc = parse_html(html, None);

    let list = doc.get_element_by_id("list").unwrap();
    let items = doc.arena.children(list);
    assert_eq!(items.len(), 2);

    doc.arena.detach(items[0]);
    let after = doc.arena.children(list);
    assert_eq!(after.len(), 1);
}

#[test]
fn clone_node_shallow() {
    let html = r#"<html><body><div id="src"><p>child</p></div></body></html>"#;
    let mut doc = parse_html(html, None);

    let src = doc.get_element_by_id("src").unwrap();
    let cloned = doc.arena.clone_node(src, false);

    let children = doc.arena.children(cloned);
    assert!(children.is_empty(), "shallow clone should have no children");
}

#[test]
fn clone_node_deep() {
    let html = r#"<html><body><div id="src"><p>child</p></div></body></html>"#;
    let mut doc = parse_html(html, None);

    let src = doc.get_element_by_id("src").unwrap();
    let cloned = doc.arena.clone_node(src, true);

    let children = doc.arena.children(cloned);
    assert_eq!(children.len(), 1, "deep clone should copy children");
}

// --- Markdown Output Tests ---

#[test]
fn html_to_markdown_basic() {
    let html = r#"<html><body><h1>Title</h1><p>Paragraph</p></body></html>"#;
    let md = html_to_markdown(html).unwrap();
    assert!(md.contains("Title"));
    assert!(md.contains("Paragraph"));
}

#[test]
fn html_to_markdown_links() {
    let html = r#"<html><body><a href="https://example.com">link</a></body></html>"#;
    let md = html_to_markdown(html).unwrap();
    assert!(md.contains("[link]"));
    assert!(md.contains("https://example.com"));
}

#[test]
fn html_to_markdown_strips_script() {
    let html = r#"<html><body><script>alert(1)</script><p>visible</p></body></html>"#;
    let md = html_to_markdown(html).unwrap();
    assert!(!md.contains("alert"));
    assert!(md.contains("visible"));
}

#[test]
fn cleanup_removes_intrusive_login_modal() {
    let html = r#"
        <html style="overflow: hidden">
            <body class="modal-open" style="overflow: hidden; background: #fff">
                <main id="feed">
                    <article>
                        <h1>精选内容</h1>
                        <p>这里是页面正文，应该被保留下来。</p>
                    </article>
                </main>
                <div class="login-mask" style="position: fixed; inset: 0; background: rgba(0,0,0,.6); z-index: 999;"></div>
                <div class="login-modal" role="dialog" aria-modal="true" style="position: fixed; z-index: 1000;">
                    <p>登录后查看更多内容</p>
                    <button>立即登录</button>
                </div>
            </body>
        </html>
    "#;
    let mut doc = parse_html(html, None);

    let removed = cleanup_intrusive_overlays(&mut doc);
    let output = serialize_to_html(&doc.arena, doc.document_node);

    assert_eq!(removed, 2);
    assert!(output.contains("精选内容"));
    assert!(!output.contains("立即登录"));
    assert!(!output.contains("login-mask"));
    assert!(!output.contains("overflow: hidden"));
    assert!(!output.contains("modal-open"));
}

#[test]
fn cleanup_keeps_fixed_non_modal_content() {
    let html = r#"
        <html>
            <body>
                <header class="site-header" style="position: fixed; top: 0; z-index: 10;">导航</header>
                <main>
                    <article>
                        <h1>正文标题</h1>
                        <p>正常内容。</p>
                    </article>
                </main>
            </body>
        </html>
    "#;
    let mut doc = parse_html(html, None);

    let removed = cleanup_intrusive_overlays(&mut doc);
    let output = serialize_to_html(&doc.arena, doc.document_node);

    assert_eq!(removed, 0);
    assert!(output.contains("site-header"));
    assert!(output.contains("正文标题"));
}

// --- Network Integration Test (requires internet) ---

#[tokio::test]
async fn fetch_example_com_no_js() {
    let config = clawbrowser::config::FetchConfig {
        no_js: true,
        timeout_secs: 10,
        wait_ms: 0,
        ..Default::default()
    };
    let page = clawbrowser::engine::Page::navigate("https://example.com", &config)
        .await
        .expect("should fetch example.com");

    assert_eq!(page.title(), Some("Example Domain".to_string()));

    let md = page.to_markdown().unwrap();
    assert!(md.contains("Example Domain"));
}

#[test]
fn page_to_markdown_handles_full_html_document() {
    let html = r#"
        <html>
            <head>
                <title>Heavy</title>
                <script>var blob = "xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx";</script>
                <style>body { overflow: hidden; }</style>
            </head>
            <body>
                <main>
                    <h1>正文标题</h1>
                    <p>这是正文内容。</p>
                </main>
            </body>
        </html>
    "#;
    let page = clawbrowser::engine::Page {
        document: parse_html(html, None),
        url: "https://example.com".to_string(),
    };

    let md = page.to_markdown().unwrap();
    assert!(md.contains("正文标题"));
    assert!(md.contains("这是正文内容"));
    assert!(!md.contains("var blob"));
    assert!(!md.contains("overflow: hidden"));
}
