use anyhow::{anyhow, Result};
use serde_json::Value;
use std::path::Path;

pub struct CaptureParams {
    pub url: String,
    pub viewport: String,
    pub full_page: bool,
    pub wait_strategy: String,
    pub format: String,
    pub output_path: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
}

const VALID_VIEWPORTS: &[&str] = &[
    "desktop",
    "laptop",
    "tablet",
    "mobile",
    "mobile_landscape",
    "custom",
];
const VALID_FORMATS: &[&str] = &["png", "jpeg"];
const VALID_WAIT_STRATEGIES: &[&str] = &["load", "networkidle", "domcontentloaded"];

pub fn validate_capture_page(args: &Value) -> Result<CaptureParams> {
    let url = require_str(args, "url")?;
    validate_url(url)?;

    let viewport = args
        .get("viewport")
        .and_then(|v| v.as_str())
        .unwrap_or("desktop");
    if !VALID_VIEWPORTS.contains(&viewport) {
        return Err(anyhow!(
            "viewport must be one of: {}",
            VALID_VIEWPORTS.join(", ")
        ));
    }

    let full_page = args
        .get("full_page")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);

    let wait_strategy = args
        .get("wait_strategy")
        .and_then(|v| v.as_str())
        .unwrap_or("networkidle");
    validate_wait_strategy(wait_strategy)?;

    let format = args.get("format").and_then(|v| v.as_str()).unwrap_or("png");
    if !VALID_FORMATS.contains(&format) {
        return Err(anyhow!(
            "format must be one of: {}",
            VALID_FORMATS.join(", ")
        ));
    }

    let output_path = args.get("output_path").and_then(|v| v.as_str());
    if let Some(p) = output_path {
        validate_output_path(p)?;
    }

    let width = args.get("width").and_then(|v| v.as_u64()).map(|v| v as u32);
    let height = args
        .get("height")
        .and_then(|v| v.as_u64())
        .map(|v| v as u32);

    if viewport == "custom" && (width.is_none() || height.is_none()) {
        return Err(anyhow!(
            "width and height are required when viewport is 'custom'"
        ));
    }

    Ok(CaptureParams {
        url: url.to_string(),
        viewport: viewport.to_string(),
        full_page,
        wait_strategy: wait_strategy.to_string(),
        format: format.to_string(),
        output_path: output_path.map(String::from),
        width,
        height,
    })
}

fn require_str<'a>(args: &'a Value, key: &str) -> Result<&'a str> {
    args.get(key)
        .and_then(|v| v.as_str())
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| anyhow!("missing required parameter: {key}"))
}

fn validate_url(url: &str) -> Result<()> {
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Err(anyhow!("url must start with http:// or https://"));
    }
    Ok(())
}

fn validate_wait_strategy(s: &str) -> Result<()> {
    if VALID_WAIT_STRATEGIES.contains(&s) {
        return Ok(());
    }
    if let Some(rest) = s.strip_prefix("delay:") {
        rest.parse::<u64>()
            .map_err(|_| anyhow!("delay:N — N must be a positive integer"))?;
        return Ok(());
    }
    Err(anyhow!(
        "wait_strategy must be one of: {}, or delay:N",
        VALID_WAIT_STRATEGIES.join(", ")
    ))
}

fn validate_output_path(p: &str) -> Result<()> {
    if p.contains("..") {
        return Err(anyhow!("output_path must not contain '..'"));
    }
    let path = Path::new(p);
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            return Err(anyhow!(
                "output_path parent directory does not exist: {}",
                parent.display()
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn rejects_missing_url() {
        assert!(validate_capture_page(&json!({})).is_err());
    }

    #[test]
    fn rejects_non_http_url() {
        assert!(validate_capture_page(&json!({"url": "ftp://example.com"})).is_err());
    }

    #[test]
    fn accepts_https_url() {
        assert!(validate_capture_page(&json!({"url": "https://example.com"})).is_ok());
    }

    #[test]
    fn rejects_path_traversal() {
        assert!(validate_output_path("/tmp/../etc/passwd").is_err());
    }

    #[test]
    fn rejects_custom_without_dimensions() {
        assert!(
            validate_capture_page(&json!({"url": "https://x.com", "viewport": "custom"})).is_err()
        );
    }

    #[test]
    fn accepts_delay_strategy() {
        assert!(validate_wait_strategy("delay:500").is_ok());
    }

    #[test]
    fn rejects_invalid_delay() {
        assert!(validate_wait_strategy("delay:abc").is_err());
    }
}
