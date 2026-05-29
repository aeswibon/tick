//! Minimal markdown → ADF for descriptions and comments.

use super::adf::mention_node;
use serde_json::{json, Value};

/// Convert markdown text to an ADF document (headings, bullets, inline styles, @mentions).
pub fn to_adf(text: &str, mentions: &[(String, String)]) -> Value {
    let mut blocks: Vec<Value> = Vec::new();
    let mut list_items: Vec<Value> = Vec::new();

    for line in text.split('\n') {
        if line.trim().is_empty() {
            flush_bullet_list(&mut blocks, &mut list_items);
            blocks.push(json!({ "type": "paragraph", "content": [] }));
            continue;
        }

        if let Some((level, rest)) = parse_heading(line) {
            flush_bullet_list(&mut blocks, &mut list_items);
            blocks.push(json!({
                "type": "heading",
                "attrs": { "level": level },
                "content": parse_inline_markdown(rest, mentions),
            }));
            continue;
        }

        if let Some(rest) = line.strip_prefix("- ").or_else(|| line.strip_prefix("* ")) {
            list_items.push(json!({
                "type": "listItem",
                "content": [{
                    "type": "paragraph",
                    "content": parse_inline_markdown(rest, mentions),
                }],
            }));
            continue;
        }

        flush_bullet_list(&mut blocks, &mut list_items);
        blocks.push(json!({
            "type": "paragraph",
            "content": parse_inline_markdown(line, mentions),
        }));
    }

    flush_bullet_list(&mut blocks, &mut list_items);

    if blocks.is_empty() {
        blocks.push(json!({ "type": "paragraph", "content": [] }));
    }

    json!({
        "type": "doc",
        "version": 1,
        "content": blocks,
    })
}

fn flush_bullet_list(blocks: &mut Vec<Value>, items: &mut Vec<Value>) {
    if items.is_empty() {
        return;
    }
    blocks.push(json!({
        "type": "bulletList",
        "content": std::mem::take(items),
    }));
}

fn parse_heading(line: &str) -> Option<(u64, &str)> {
    let trimmed = line.trim_start();
    if trimmed.starts_with("### ") {
        return Some((3, trimmed.trim_start_matches("### ")));
    }
    if trimmed.starts_with("## ") {
        return Some((2, trimmed.trim_start_matches("## ")));
    }
    if trimmed.starts_with("# ") {
        return Some((1, trimmed.trim_start_matches("# ")));
    }
    None
}

fn parse_inline_markdown(line: &str, mentions: &[(String, String)]) -> Vec<Value> {
    if mentions.is_empty() {
        return parse_styled_text(line);
    }
    let mut nodes = Vec::new();
    let mut rest = line;
    while !rest.is_empty() {
        let mut best: Option<(usize, &str, &str)> = None;
        for (label, account_id) in mentions {
            if let Some(pos) = rest.find(label.as_str()) {
                if best.map(|(p, _, _)| pos < p).unwrap_or(true) {
                    best = Some((pos, label.as_str(), account_id.as_str()));
                }
            }
        }
        match best {
            Some((0, label, account_id)) => {
                nodes.push(mention_node(account_id, label));
                rest = &rest[label.len()..];
            }
            Some((pos, label, account_id)) => {
                nodes.extend(parse_styled_text(&rest[..pos]));
                nodes.push(mention_node(account_id, label));
                rest = &rest[pos + label.len()..];
            }
            None => {
                nodes.extend(parse_styled_text(rest));
                break;
            }
        }
    }
    nodes
}

fn parse_styled_text(input: &str) -> Vec<Value> {
    let mut nodes = Vec::new();
    let mut rest = input;
    while !rest.is_empty() {
        let mut earliest: Option<(usize, InlineToken)> = None;
        for (pat, token) in [
            ("**", InlineToken::Bold),
            ("__", InlineToken::Bold),
            ("*", InlineToken::Italic),
            ("_", InlineToken::Italic),
            ("`", InlineToken::Code),
            ("[", InlineToken::Link),
        ] {
            if let Some(pos) = rest.find(pat) {
                if earliest.map(|(p, _)| pos < p).unwrap_or(true) {
                    earliest = Some((pos, token));
                }
            }
        }
        let Some((pos, token)) = earliest else {
            if !rest.is_empty() {
                nodes.push(text_node(rest, None));
            }
            break;
        };
        if pos > 0 {
            nodes.push(text_node(&rest[..pos], None));
        }
        rest = &rest[pos..];
        match token {
            InlineToken::Bold => {
                if let Some((inner, tail)) =
                    take_wrapped(rest, "**").or_else(|| take_wrapped(rest, "__"))
                {
                    nodes.push(text_node(inner, Some("strong")));
                    rest = tail;
                } else {
                    nodes.push(text_node("**", None));
                    rest = &rest[2..];
                }
            }
            InlineToken::Italic => {
                if let Some((inner, tail)) =
                    take_wrapped(rest, "*").or_else(|| take_wrapped(rest, "_"))
                {
                    nodes.push(text_node(inner, Some("em")));
                    rest = tail;
                } else {
                    nodes.push(text_node("*", None));
                    rest = &rest[1..];
                }
            }
            InlineToken::Code => {
                if let Some((inner, tail)) = take_wrapped(rest, "`") {
                    nodes.push(text_node(inner, Some("code")));
                    rest = tail;
                } else {
                    nodes.push(text_node("`", None));
                    rest = &rest[1..];
                }
            }
            InlineToken::Link => {
                if let Some((label, href, tail)) = parse_link(rest) {
                    nodes.push(link_node(label, href));
                    rest = tail;
                } else {
                    nodes.push(text_node("[", None));
                    rest = &rest[1..];
                }
            }
        }
    }
    nodes
}

#[derive(Clone, Copy)]
enum InlineToken {
    Bold,
    Italic,
    Code,
    Link,
}

fn take_wrapped<'a>(s: &'a str, delim: &str) -> Option<(&'a str, &'a str)> {
    if !s.starts_with(delim) {
        return None;
    }
    let inner = &s[delim.len()..];
    let end = inner.find(delim)?;
    let content = &inner[..end];
    let tail = &inner[end + delim.len()..];
    Some((content, tail))
}

fn parse_link(s: &str) -> Option<(&str, &str, &str)> {
    if !s.starts_with('[') {
        return None;
    }
    let rest = &s[1..];
    let label_end = rest.find(']')?;
    let label = &rest[..label_end];
    let after = &rest[label_end + 1..];
    if !after.starts_with('(') {
        return None;
    }
    let url_rest = &after[1..];
    let url_end = url_rest.find(')')?;
    let href = &url_rest[..url_end];
    let tail = &url_rest[url_end + 1..];
    Some((label, href, tail))
}

fn text_node(text: &str, mark: Option<&str>) -> Value {
    if text.is_empty() {
        return json!({ "type": "text", "text": "" });
    }
    let mut node = json!({ "type": "text", "text": text });
    if let Some(m) = mark {
        node["marks"] = json!([{ "type": m }]);
    }
    node
}

fn link_node(label: &str, href: &str) -> Value {
    json!({
        "type": "text",
        "text": label,
        "marks": [{ "type": "link", "attrs": { "href": href } }],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heading_and_bullet_list() {
        let doc = to_adf("# Title\n\n- one\n- two", &[]);
        let content = doc["content"].as_array().unwrap();
        assert_eq!(content[0]["type"], "heading");
        assert_eq!(content[2]["type"], "bulletList");
    }

    #[test]
    fn bold_and_mention() {
        let mentions = vec![("@Ada".into(), "acc-1".into())];
        let doc = to_adf("**Hi** @Ada", &mentions);
        let inline = &doc["content"][0]["content"];
        assert!(inline
            .as_array()
            .unwrap()
            .iter()
            .any(|n| n["type"] == "mention"));
    }

    #[test]
    fn link_inline() {
        let doc = to_adf("see [docs](https://example.com)", &[]);
        let inline = doc["content"][0]["content"].as_array().unwrap();
        assert!(inline.iter().any(|n| {
            n["marks"]
                .as_array()
                .is_some_and(|m| m[0]["type"] == "link")
        }));
    }
}
