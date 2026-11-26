//! Render Markdown sang HTML an toàn, loại bỏ mọi payload XSS (script, onerror, javascript:).

use pulldown_cmark::{Options, Parser, html};

/// Chuyển nội dung Markdown sang HTML đã qua bộ lọc `ammonia` chống XSS.
#[must_use]
pub fn render_safe_markdown(markdown: &str) -> String {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);

    let parser = Parser::new_ext(markdown, options);
    let mut raw_html = String::with_capacity(markdown.len().saturating_mul(2));
    html::push_html(&mut raw_html, parser);

    ammonia::clean(&raw_html)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_xss_payloads_while_preserving_markdown() {
        let input =
            "# Hello\n\n<script>alert('xss')</script>\n\n[Click](javascript:alert(1))\n\n**Bold**";
        let output = render_safe_markdown(input);
        assert!(output.contains("<h1>Hello</h1>"));
        assert!(output.contains("<strong>Bold</strong>"));
        assert!(!output.contains("<script"));
        assert!(!output.contains("javascript:"));
    }
}
