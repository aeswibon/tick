//! ADF JSON for **writing** to Jira (comments). Rendering lives in `ui::adf`.

use serde_json::{json, Value};

/// Jira Cloud REST v3 comment body (Atlassian Document Format).
pub fn plain_text_body(text: &str) -> Value {
    plain_text_to_description(text)
}

/// Multi-paragraph ADF description (blank lines → empty paragraphs).
pub fn plain_text_to_description(text: &str) -> Value {
    let content: Vec<Value> = text
        .split('\n')
        .map(|line| {
            let para_content = if line.is_empty() {
                vec![]
            } else {
                vec![json!({ "type": "text", "text": line })]
            };
            json!({
                "type": "paragraph",
                "content": para_content
            })
        })
        .collect();
    json!({
        "type": "doc",
        "version": 1,
        "content": content
    })
}

/// Comment body with `@Display Name` segments resolved to ADF mention nodes.
/// `mentions`: (`@Display Name`, `accountId`) in order of insertion.
pub fn comment_body_with_mentions(text: &str, mentions: &[(String, String)]) -> Value {
    let content: Vec<Value> = text
        .split('\n')
        .map(|line| {
            json!({
                "type": "paragraph",
                "content": inline_content_with_mentions(line, mentions)
            })
        })
        .collect();
    json!({
        "type": "doc",
        "version": 1,
        "content": content
    })
}

fn inline_content_with_mentions(line: &str, mentions: &[(String, String)]) -> Vec<Value> {
    if line.is_empty() {
        return vec![];
    }
    if mentions.is_empty() {
        return vec![json!({ "type": "text", "text": line })];
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
                nodes.push(json!({ "type": "text", "text": &rest[..pos] }));
                nodes.push(mention_node(account_id, label));
                rest = &rest[pos + label.len()..];
            }
            None => {
                nodes.push(json!({ "type": "text", "text": rest }));
                break;
            }
        }
    }
    nodes
}

pub fn mention_node(account_id: &str, text: &str) -> Value {
    json!({
        "type": "mention",
        "attrs": {
            "id": account_id,
            "text": text,
            "accessLevel": ""
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_text_body_is_adf_doc() {
        let body = plain_text_body("hello");
        assert_eq!(body["type"], "doc");
        assert_eq!(body["content"][0]["content"][0]["text"], "hello");
    }

    #[test]
    fn plain_text_to_description_splits_paragraphs() {
        let body = plain_text_to_description("line one\n\nline two");
        assert_eq!(body["content"].as_array().unwrap().len(), 3);
    }

    #[test]
    fn comment_body_multiline_with_mention() {
        let mentions = vec![("@Ada".into(), "acc-1".into())];
        let body = comment_body_with_mentions("line1\n@Ada review", &mentions);
        assert_eq!(body["content"].as_array().unwrap().len(), 2);
        assert_eq!(body["content"][1]["content"][0]["type"], "mention");
    }

    #[test]
    fn comment_body_embeds_mention_nodes() {
        let mentions = vec![
            ("@Alice".into(), "acc-alice".into()),
            ("@Bob".into(), "acc-bob".into()),
        ];
        let body = comment_body_with_mentions("hey @Alice and @Bob please", &mentions);
        let inline = &body["content"][0]["content"];
        assert_eq!(inline[0]["type"], "text");
        assert_eq!(inline[1]["type"], "mention");
        assert_eq!(inline[1]["attrs"]["id"], "acc-alice");
        assert_eq!(inline[1]["attrs"]["text"], "@Alice");
    }
}
