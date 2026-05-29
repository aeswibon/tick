use serde_json::{json, Value};

/// Jira Cloud REST v3 comment body (Atlassian Document Format).
pub fn plain_text_body(text: &str) -> Value {
    json!({
        "type": "doc",
        "version": 1,
        "content": [{
            "type": "paragraph",
            "content": [{ "type": "text", "text": text }]
        }]
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
}
