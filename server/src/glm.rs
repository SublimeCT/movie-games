use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use url::Url;

const API_URL: &str = "https://open.bigmodel.cn/api/paas/v4/chat/completions";
const DEFAULT_MODEL: &str = "glm-4.6v-flash";

pub const GLM_LIMIT_FRIENDLY_MESSAGE: &str =
    "GLM 已达最大调用频率, 请填写自己的 API Key 并再次尝试";

/// Error code 1305 from GLM API: "当前API请求过多，请稍后重试。"
/// This indicates rate limiting, user should use their own API key.
pub const GLM_RATE_LIMIT_CODE: &str = "1305";

/// Parses GLM error response to extract error code
/// Returns Some(code) if the response contains an error code, None otherwise
pub fn extract_glm_error_code(text: &str) -> Option<String> {
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(text) {
        if let Some(error_obj) = value.get("error") {
            if let Some(code) = error_obj.get("code") {
                if let Some(code_str) = code.as_str() {
                    return Some(code_str.to_string());
                }
                if let Some(code_i64) = code.as_i64() {
                    return Some(code_i64.to_string());
                }
                if let Some(code_u64) = code.as_u64() {
                    return Some(code_u64.to_string());
                }
            }
        }
    }
    None
}

pub fn contains_limit(text: &str) -> bool {
    text.to_ascii_lowercase().contains("limit")
}

pub fn is_rate_limit_error(text: &str) -> bool {
    extract_glm_error_code(text).as_deref() == Some(GLM_RATE_LIMIT_CODE)
}

fn glm_api_key() -> Result<String, String> {
    std::env::var("GLM_API_KEY")
        .or_else(|_| std::env::var("BIGMODEL_API_KEY"))
        .map_err(|_| "Missing GLM_API_KEY".to_string())
}

fn resolve_glm_api_key(override_key: Option<String>) -> Result<String, String> {
    let from_req = override_key.unwrap_or_default().trim().to_string();

    if !from_req.is_empty() {
        return Ok(from_req);
    }

    glm_api_key()
}

fn resolve_glm_endpoint(base_url: Option<String>) -> Result<String, String> {
    let raw = base_url.unwrap_or_default();
    let raw = raw.trim();
    if raw.is_empty() {
        return Ok(API_URL.to_string());
    }

    if raw.contains("chat/completions") {
        let u = Url::parse(raw).map_err(|_| "Invalid baseUrl".to_string())?;
        let scheme = u.scheme();
        if scheme != "http" && scheme != "https" {
            return Err("Invalid baseUrl".to_string());
        }
        return Ok(u.to_string());
    }

    let mut s = raw.to_string();
    if !s.ends_with('/') {
        s.push('/');
    }

    let base = Url::parse(&s).map_err(|_| "Invalid baseUrl".to_string())?;
    let scheme = base.scheme();
    if scheme != "http" && scheme != "https" {
        return Err("Invalid baseUrl".to_string());
    }

    base.join("chat/completions")
        .map(|u| u.to_string())
        .map_err(|_| "Invalid baseUrl".to_string())
}

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<Message>,
    response_format: Option<ResponseFormat>,
    stream: bool,
    max_tokens: u32,
}

#[derive(Serialize)]
struct Message {
    role: String,
    content: String,
}

#[derive(Serialize)]
struct ResponseFormat {
    r#type: String,
}

#[derive(Deserialize, Debug)]
struct ChatResponse {
    choices: Vec<Choice>,
    usage: Option<Usage>,
}

#[derive(Deserialize, Debug)]
struct Usage {
    total_tokens: u32,
}

#[derive(Deserialize, Debug)]
struct Choice {
    message: MessageContent,
}

#[derive(Deserialize, Debug)]
struct MessageContent {
    content: String,
}

#[allow(dead_code)]
pub async fn call_glm_with_api_key(
    prompt: String,
    json_mode: bool,
    api_key: Option<String>,
    base_url: Option<String>,
    model: Option<String>,
) -> Result<String, String> {
    println!("Init GLM Client with 300s timeout...");
    let client = Client::builder()
        .timeout(Duration::from_secs(300))
        .build()
        .map_err(|e| format!("Failed to build client: {}", e))?;

    let _using_override_key = api_key.as_ref().is_some_and(|k| !k.trim().is_empty());

    let api_key = resolve_glm_api_key(api_key)?;
    let endpoint = resolve_glm_endpoint(base_url)?;
    let model = model.unwrap_or_else(|| DEFAULT_MODEL.to_string());

    let request_body = ChatRequest {
        model,
        messages: vec![
            Message {
                role: "system".to_string(),
                content: if json_mode {
                    "You are a professional interactive movie game designer. Output strictly valid JSON."
                } else {
                    "You are a professional interactive movie game designer."
                }.to_string(),
            },
            Message {
                role: "user".to_string(),
                content: prompt.clone(),
            },
        ],
        max_tokens: 8192,
        response_format: if json_mode {
            Some(ResponseFormat { r#type: "json_object".to_string() })
        } else {
            None
        },
        stream: false,
    };

    println!("Sending request to GLM (Prompt len: {})...", prompt.len());
    let start = std::time::Instant::now();

    let response = client
        .post(endpoint)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&request_body)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    let duration = start.elapsed();
    println!("GLM Request took: {:?}", duration);

    if !response.status().is_success() {
        let text = response.text().await.unwrap_or_default();
        println!("GLM Error Body: {}", text);

        if is_rate_limit_error(&text) {
            return Err(format!(
                "GLM API 返回错误码 {}: {}",
                GLM_RATE_LIMIT_CODE, text
            ));
        }

        if contains_limit(&text) {
            return Err(GLM_LIMIT_FRIENDLY_MESSAGE.to_string());
        }

        return Err(text);
    }

    let text_response = response
        .text()
        .await
        .map_err(|e| format!("Failed to read response text: {}", e))?;

    // Try to parse as generic JSON first to check for "error" field
    // (GLM sometimes returns 200 OK with "error" in body)
    if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(&text_response) {
        if json_value.get("error").is_some() {
            println!("GLM returned 200 OK but with error body: {}", text_response);

            // Check for rate limit in this body
            if is_rate_limit_error(&text_response) {
                return Err(format!(
                    "GLM API 返回错误码 {}: {}",
                    GLM_RATE_LIMIT_CODE, text_response
                ));
            }

            return Err(text_response);
        }
    }

    let chat_response: ChatResponse = serde_json::from_str(&text_response)
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    if let Some(usage) = chat_response.usage {
        println!("Token Usage: {}", usage.total_tokens);
    }

    if let Some(choice) = chat_response.choices.first() {
        println!(
            "GLM Response Content Length: {}",
            choice.message.content.len()
        );
        Ok(choice.message.content.clone())
    } else {
        Err("No choices in response".to_string())
    }
}
