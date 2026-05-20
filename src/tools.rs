use anyhow::Result;
use serde_json::{json, Value};

use crate::browser;
use crate::tools_validation;

pub fn tools_list() -> Value {
    json!({
        "tools": [{
            "name": "capture_page",
            "description": "Capture a screenshot of a web page. Waits for JavaScript rendering before capturing. Returns a PNG or JPEG image.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "url": {
                        "type": "string",
                        "description": "The URL to capture. Must start with http:// or https://."
                    },
                    "viewport": {
                        "type": "string",
                        "enum": ["desktop", "laptop", "tablet", "mobile", "mobile_landscape", "custom"],
                        "description": "Viewport preset. desktop=1440x900, laptop=1280x800, tablet=768x1024, mobile=390x844 (with mobile UA), custom requires width+height.",
                        "default": "desktop"
                    },
                    "full_page": {
                        "type": "boolean",
                        "description": "Capture the full scrollable page height. Default true.",
                        "default": true
                    },
                    "wait_strategy": {
                        "type": "string",
                        "description": "When to consider the page ready. 'networkidle' (default, most resilient), 'load', 'domcontentloaded', or 'delay:N' (wait extra N ms after networkidle).",
                        "default": "networkidle"
                    },
                    "format": {
                        "type": "string",
                        "enum": ["png", "jpeg"],
                        "description": "Output image format. Default 'png'.",
                        "default": "png"
                    },
                    "output_path": {
                        "type": "string",
                        "description": "Absolute file path to save the screenshot. If omitted, the image is returned as base64 in the response."
                    },
                    "width": {
                        "type": "integer",
                        "description": "Viewport width in pixels. Required when viewport is 'custom'."
                    },
                    "height": {
                        "type": "integer",
                        "description": "Viewport height in pixels. Required when viewport is 'custom'."
                    }
                },
                "required": ["url"],
                "additionalProperties": false
            }
        }]
    })
}

pub async fn call(name: &str, arguments: Value) -> Result<Value> {
    match name {
        "capture_page" => capture_page(&arguments).await,
        other => Err(anyhow::anyhow!("unknown tool: {other}")),
    }
}

async fn capture_page(args: &Value) -> Result<Value> {
    let params = tools_validation::validate_capture_page(args)?;
    let result = browser::capture(&params).await?;

    if let Some(path) = &params.output_path {
        browser::save_to_file(&result.data, path)?;
        let size_bytes = result.data.len();
        return Ok(json!({
            "content": [{
                "type": "text",
                "text": serde_json::to_string(&json!({
                    "saved_to": path,
                    "size_bytes": size_bytes,
                    "width": result.width,
                    "height": result.height,
                    "format": result.format
                }))?
            }],
            "isError": false
        }));
    }

    let mime = if result.format == "jpeg" {
        "image/jpeg"
    } else {
        "image/png"
    };
    let b64 = browser::encode_base64(&result.data);

    Ok(json!({
        "content": [{
            "type": "image",
            "data": b64,
            "mimeType": mime
        }],
        "isError": false
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tools_list_has_one_tool() {
        let v = tools_list();
        let arr = v["tools"].as_array().unwrap();
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["name"], "capture_page");
    }

    #[test]
    fn tools_list_has_additional_properties_false() {
        let v = tools_list();
        let schema = &v["tools"][0]["inputSchema"];
        assert_eq!(schema["additionalProperties"], false);
    }

    #[test]
    fn tools_list_url_is_required() {
        let v = tools_list();
        let required = v["tools"][0]["inputSchema"]["required"]
            .as_array()
            .unwrap();
        assert!(required.iter().any(|r| r == "url"));
    }
}
