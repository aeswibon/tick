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
}
