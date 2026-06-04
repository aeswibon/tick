//! Issue attachments and media upload for rich comments.

use std::path::{Path, PathBuf};

use reqwest::multipart;
use serde_json::Value;

use super::JiraClient;

#[derive(Debug, Clone)]
pub struct UploadedAttachment {
    pub filename: String,
    pub content_url: String,
}

#[derive(Debug, Clone)]
pub struct UploadedMedia {
    pub id: String,
    pub collection: String,
    pub filename: String,
}

impl JiraClient {
    /// Upload a file to the issue attachments list (non-inline).
    pub async fn upload_issue_attachment(
        &self,
        base_url: &str,
        issue_key: &str,
        path: &Path,
    ) -> Result<UploadedAttachment, String> {
        let bytes = std::fs::read(path).map_err(|e| format!("Read {}: {e}", path.display()))?;
        let filename = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("attachment")
            .to_string();
        let mime = mime_guess(path);
        let url = format!(
            "{}/rest/api/3/issue/{issue_key}/attachments",
            base_url.trim_end_matches('/')
        );
        let resp = self
            .send(|| {
                let part = multipart::Part::bytes(bytes.clone())
                    .file_name(filename.clone())
                    .mime_str(&mime)
                    .expect("mime_guess returns valid types");
                let form = multipart::Form::new().part("file", part);
                self.post(&url)
                    .header("X-Atlassian-Token", "no-check")
                    .multipart(form)
                    .send()
            })
            .await?;
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(format!("Attachment upload {status}: {body}"));
        }
        let data: Value =
            serde_json::from_str(&body).map_err(|e| format!("Parse attachment response: {e}"))?;
        let entry = data.as_array().and_then(|a| a.first()).unwrap_or(&data);
        let content_url = entry
            .get("content")
            .and_then(|c| c.as_str())
            .unwrap_or("")
            .to_string();
        if content_url.is_empty() {
            return Err("Attachment upload succeeded but no content URL returned".into());
        }
        Ok(UploadedAttachment {
            filename,
            content_url,
        })
    }

    /// Upload binary for inline ADF media in a comment (images).
    pub async fn upload_comment_media(
        &self,
        base_url: &str,
        issue_key: &str,
        path: &Path,
    ) -> Result<UploadedMedia, String> {
        let bytes = std::fs::read(path).map_err(|e| format!("Read {}: {e}", path.display()))?;
        let filename = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("image")
            .to_string();
        let mime = mime_guess(path);
        let url = format!(
            "{}/rest/api/3/media/upload?IssueIdOrKey={issue_key}",
            base_url.trim_end_matches('/')
        );
        let resp = self
            .send(|| {
                let part = multipart::Part::bytes(bytes.clone())
                    .file_name(filename.clone())
                    .mime_str(&mime)
                    .expect("mime_guess returns valid types");
                let form = multipart::Form::new().part("file", part);
                self.post(&url)
                    .header("X-Atlassian-Token", "no-check")
                    .multipart(form)
                    .send()
            })
            .await?;
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(format!("Media upload {status}: {body}"));
        }
        parse_media_upload_response(&body, &filename)
    }

    /// Upload paths and merge attachment/media blocks into an ADF comment document.
    pub async fn build_comment_body(
        &self,
        base_url: &str,
        issue_key: &str,
        markdown: &str,
        mentions: &[(String, String)],
        attach_paths: &[PathBuf],
    ) -> Result<Value, String> {
        let mut doc = super::markdown::to_adf(markdown, mentions);
        let mut extra: Vec<Value> = Vec::new();
        for path in attach_paths {
            if is_image_path(path) {
                let media = self.upload_comment_media(base_url, issue_key, path).await?;
                extra.push(media_adf_block(&media));
            } else {
                let att = self
                    .upload_issue_attachment(base_url, issue_key, path)
                    .await?;
                extra.push(attachment_link_paragraph(&att));
            }
        }
        if let Some(content) = doc.get_mut("content").and_then(|c| c.as_array_mut()) {
            content.extend(extra);
        }
        Ok(doc)
    }
}

pub fn media_adf_block(media: &UploadedMedia) -> Value {
    serde_json::json!({
        "type": "mediaSingle",
        "attrs": { "layout": "center" },
        "content": [{
            "type": "media",
            "attrs": {
                "type": "file",
                "id": media.id,
                "collection": media.collection,
                "alt": media.filename,
            }
        }]
    })
}

pub fn attachment_link_paragraph(att: &UploadedAttachment) -> Value {
    serde_json::json!({
        "type": "paragraph",
        "content": [{
            "type": "text",
            "text": att.filename,
            "marks": [{ "type": "link", "attrs": { "href": att.content_url } }]
        }]
    })
}

pub fn is_image_path(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| {
            matches!(
                e.to_ascii_lowercase().as_str(),
                "png" | "jpg" | "jpeg" | "gif" | "webp" | "bmp" | "svg"
            )
        })
        .unwrap_or(false)
}

fn mime_guess(path: &Path) -> String {
    match path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
    {
        Some(ext) if ext == "png" => "image/png".into(),
        Some(ext) if ext == "jpg" || ext == "jpeg" => "image/jpeg".into(),
        Some(ext) if ext == "gif" => "image/gif".into(),
        Some(ext) if ext == "webp" => "image/webp".into(),
        Some(ext) if ext == "svg" => "image/svg+xml".into(),
        Some(ext) if ext == "pdf" => "application/pdf".into(),
        Some(ext) if ext == "txt" => "text/plain".into(),
        _ => "application/octet-stream".into(),
    }
}

fn parse_media_upload_response(body: &str, filename: &str) -> Result<UploadedMedia, String> {
    let data: Value =
        serde_json::from_str(body).map_err(|e| format!("Parse media response: {e}"))?;
    let entry = data.as_array().and_then(|a| a.first()).unwrap_or(&data);
    let id = entry
        .get("fileId")
        .or_else(|| entry.get("mediaFileId"))
        .or_else(|| entry.get("id"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| format!("Media upload missing id: {body}"))?;
    let collection = entry
        .get("collection")
        .or_else(|| entry.get("collectionName"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    Ok(UploadedMedia {
        id: id.to_string(),
        collection,
        filename: filename.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_image_extensions() {
        assert!(is_image_path(Path::new("/tmp/x.PNG")));
        assert!(!is_image_path(Path::new("/tmp/x.pdf")));
    }

    #[test]
    fn media_block_has_file_attrs() {
        let block = media_adf_block(&UploadedMedia {
            id: "abc".into(),
            collection: "col".into(),
            filename: "x.png".into(),
        });
        assert_eq!(block["type"], "mediaSingle");
        assert_eq!(block["content"][0]["attrs"]["type"], "file");
    }
}
