//! Markdown ↔ ADF for descriptions and comments.

use super::adf::mention_node;
use serde_json::{json, Value};

const ADF_JSON_FENCE: &str = "adf-json";

/// Convert markdown text to an ADF document (headings, lists, inline styles, @mentions).
pub fn to_adf(text: &str, mentions: &[(String, String)]) -> Value {
    let mut parser = MdParser::new(mentions);
    for line in text.split('\n') {
        parser.line(line);
    }
    parser.finish()
}

struct MdParser<'a> {
    mentions: &'a [(String, String)],
    blocks: Vec<Value>,
    bullet_items: Vec<Value>,
    ordered_items: Vec<Value>,
    task_items: Vec<Value>,
    fence: Option<FenceState>,
    blockquote_lines: Vec<String>,
}

enum FenceState {
    Code { lang: String, lines: Vec<String> },
    AdfJson { lines: Vec<String> },
}

impl<'a> MdParser<'a> {
    fn new(mentions: &'a [(String, String)]) -> Self {
        Self {
            mentions,
            blocks: Vec::new(),
            bullet_items: Vec::new(),
            ordered_items: Vec::new(),
            task_items: Vec::new(),
            fence: None,
            blockquote_lines: Vec::new(),
        }
    }

    fn line(&mut self, line: &str) {
        if let Some(state) = self.fence.take() {
            self.continue_fence(state, line);
            return;
        }

        if self.handle_fence_start(line) {
            return;
        }

        self.flush_blockquote();

        if line.trim().is_empty() {
            self.flush_lists();
            return;
        }

        if line.trim() == "---" {
            self.flush_lists();
            self.blocks.push(json!({ "type": "rule" }));
            return;
        }

        if let Some(rest) = line
            .strip_prefix('>')
            .map(|s| s.strip_prefix(' ').unwrap_or(s))
        {
            self.flush_lists();
            self.blockquote_lines.push(rest.to_string());
            return;
        }

        if let Some((level, rest)) = parse_heading(line) {
            self.flush_lists();
            self.blocks.push(json!({
                "type": "heading",
                "attrs": { "level": level },
                "content": parse_inline_markdown(rest, self.mentions),
            }));
            return;
        }

        if let Some(rest) = parse_task_item(line) {
            self.flush_bullet_and_ordered();
            self.task_items
                .push(task_item_node(rest.0, rest.1, self.mentions));
            return;
        }

        if let Some((n, rest)) = parse_ordered_item(line) {
            self.flush_bullet_and_task();
            self.ordered_items
                .push(list_item_from_inline(n, rest, self.mentions));
            return;
        }

        if let Some(rest) = line.strip_prefix("- ").or_else(|| line.strip_prefix("* ")) {
            self.flush_ordered_and_task();
            self.bullet_items
                .push(list_item_from_inline(0, rest, self.mentions));
            return;
        }

        self.flush_lists();
        self.blocks.push(json!({
            "type": "paragraph",
            "content": parse_inline_markdown(line, self.mentions),
        }));
    }

    fn handle_fence_start(&mut self, line: &str) -> bool {
        let trimmed = line.trim();
        if trimmed == "```" {
            self.flush_lists();
            self.fence = Some(FenceState::Code {
                lang: String::new(),
                lines: Vec::new(),
            });
            return true;
        }
        if let Some(lang) = trimmed.strip_prefix("```") {
            self.flush_lists();
            if lang == ADF_JSON_FENCE {
                self.fence = Some(FenceState::AdfJson { lines: Vec::new() });
            } else {
                self.fence = Some(FenceState::Code {
                    lang: lang.to_string(),
                    lines: Vec::new(),
                });
            }
            return true;
        }
        false
    }

    fn continue_fence(&mut self, state: FenceState, line: &str) {
        if line.trim() == "```" {
            self.finish_fence(state);
            return;
        }
        match state {
            FenceState::Code { lang, mut lines } => {
                lines.push(line.to_string());
                self.fence = Some(FenceState::Code { lang, lines });
            }
            FenceState::AdfJson { mut lines } => {
                lines.push(line.to_string());
                self.fence = Some(FenceState::AdfJson { lines });
            }
        }
    }

    fn finish_fence(&mut self, state: FenceState) {
        match state {
            FenceState::Code { lang, lines } => {
                let text = lines.join("\n");
                let mut block = json!({
                    "type": "codeBlock",
                    "content": [{ "type": "text", "text": text }],
                });
                if !lang.is_empty() {
                    block["attrs"] = json!({ "language": lang });
                }
                self.blocks.push(block);
            }
            FenceState::AdfJson { lines } => {
                let raw = lines.join("\n");
                if let Ok(node) = serde_json::from_str::<Value>(&raw) {
                    self.blocks.push(node);
                }
            }
        }
    }

    fn flush_blockquote(&mut self) {
        if self.blockquote_lines.is_empty() {
            return;
        }
        let inner: Vec<Value> = self
            .blockquote_lines
            .drain(..)
            .map(|l| {
                json!({
                    "type": "paragraph",
                    "content": parse_inline_markdown(&l, self.mentions),
                })
            })
            .collect();
        self.blocks.push(json!({
            "type": "blockquote",
            "content": inner,
        }));
    }

    fn flush_bullet_and_ordered(&mut self) {
        flush_bullet_list(&mut self.blocks, &mut self.bullet_items);
        flush_ordered_list(&mut self.blocks, &mut self.ordered_items);
    }

    fn flush_bullet_and_task(&mut self) {
        flush_bullet_list(&mut self.blocks, &mut self.bullet_items);
        flush_task_list(&mut self.blocks, &mut self.task_items);
    }

    fn flush_ordered_and_task(&mut self) {
        flush_ordered_list(&mut self.blocks, &mut self.ordered_items);
        flush_task_list(&mut self.blocks, &mut self.task_items);
    }

    fn flush_lists(&mut self) {
        self.flush_blockquote();
        flush_bullet_list(&mut self.blocks, &mut self.bullet_items);
        flush_ordered_list(&mut self.blocks, &mut self.ordered_items);
        flush_task_list(&mut self.blocks, &mut self.task_items);
    }

    fn finish(mut self) -> Value {
        self.flush_lists();
        if let Some(state) = self.fence.take() {
            self.finish_fence(state);
        }
        if self.blocks.is_empty() {
            self.blocks
                .push(json!({ "type": "paragraph", "content": [] }));
        }
        json!({
            "type": "doc",
            "version": 1,
            "content": self.blocks,
        })
    }
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

fn flush_ordered_list(blocks: &mut Vec<Value>, items: &mut Vec<Value>) {
    if items.is_empty() {
        return;
    }
    blocks.push(json!({
        "type": "orderedList",
        "content": std::mem::take(items),
    }));
}

fn flush_task_list(blocks: &mut Vec<Value>, items: &mut Vec<Value>) {
    if items.is_empty() {
        return;
    }
    blocks.push(json!({
        "type": "taskList",
        "content": std::mem::take(items),
    }));
}

fn list_item_from_inline(_n: u32, rest: &str, mentions: &[(String, String)]) -> Value {
    json!({
        "type": "listItem",
        "content": [{
            "type": "paragraph",
            "content": parse_inline_markdown(rest, mentions),
        }],
    })
}

fn task_item_node(checked: bool, rest: &str, mentions: &[(String, String)]) -> Value {
    let state = if checked { "DONE" } else { "TODO" };
    json!({
        "type": "taskItem",
        "attrs": { "localId": "", "state": state },
        "content": [{
            "type": "paragraph",
            "content": parse_inline_markdown(rest, mentions),
        }],
    })
}

fn parse_task_item(line: &str) -> Option<(bool, &str)> {
    let trimmed = line.trim_start();
    if let Some(rest) = trimmed
        .strip_prefix("- [x] ")
        .or_else(|| trimmed.strip_prefix("- [X] "))
    {
        return Some((true, rest));
    }
    if let Some(rest) = trimmed.strip_prefix("- [ ] ") {
        return Some((false, rest));
    }
    None
}

fn parse_ordered_item(line: &str) -> Option<(u32, &str)> {
    let trimmed = line.trim_start();
    let dot = trimmed.find(". ")?;
    let num: u32 = trimmed[..dot].trim().parse().ok()?;
    Some((num, &trimmed[dot + 2..]))
}

fn parse_heading(line: &str) -> Option<(u64, &str)> {
    let trimmed = line.trim_start();
    for level in (1..=6).rev() {
        let prefix = "#".repeat(level) + " ";
        if let Some(rest) = trimmed.strip_prefix(&prefix) {
            return Some((level as u64, rest));
        }
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
            ("~~", InlineToken::Strike),
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
            InlineToken::Strike => {
                if let Some((inner, tail)) = take_wrapped(rest, "~~") {
                    nodes.push(text_node(inner, Some("strike")));
                    rest = tail;
                } else {
                    nodes.push(text_node("~~", None));
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
    Strike,
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
        assert_eq!(content[1]["type"], "bulletList");
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

    #[test]
    fn strike_and_rule() {
        let doc = to_adf("~~x~~\n\n---", &[]);
        let content = doc["content"].as_array().unwrap();
        assert_eq!(content[0]["content"][0]["marks"][0]["type"], "strike");
        assert_eq!(content[1]["type"], "rule");
    }

    #[test]
    fn code_fence_and_blockquote() {
        let doc = to_adf("> quote\n\n```rs\nlet x = 1;\n```", &[]);
        let content = doc["content"].as_array().unwrap();
        assert_eq!(content[0]["type"], "blockquote");
        assert_eq!(content[1]["type"], "codeBlock");
        assert_eq!(content[1]["attrs"]["language"], "rs");
    }

    #[test]
    fn ordered_and_task_lists() {
        let doc = to_adf("1. first\n2. second\n\n- [ ] todo\n- [x] done", &[]);
        let content = doc["content"].as_array().unwrap();
        assert_eq!(content[0]["type"], "orderedList");
        assert_eq!(content[1]["type"], "taskList");
        assert_eq!(content[1]["content"][1]["attrs"]["state"], "DONE");
    }

    #[test]
    fn adf_json_fence_round_trip() {
        let exotic = serde_json::json!({
            "type": "layoutSection",
            "content": []
        });
        let md = format!(
            "intro\n\n```adf-json\n{}\n```",
            serde_json::to_string(&exotic).unwrap()
        );
        let doc = to_adf(&md, &[]);
        assert_eq!(doc["content"][0]["type"], "paragraph");
        assert_eq!(doc["content"][1], exotic);
    }
}
