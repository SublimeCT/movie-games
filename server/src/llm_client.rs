use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

const DEFAULT_TIMEOUT_SECS: u64 = 600;

#[derive(Serialize)]
struct Message {
    role: String,
    content: String,
}

#[derive(Serialize)]
struct ResponseFormat {
    r#type: String,
}

// DeepSeek Request Structure
#[derive(Serialize)]
struct DeepSeekRequest {
    model: String,
    messages: Vec<Message>,
    #[serde(skip_serializing_if = "Option::is_none")]
    response_format: Option<ResponseFormat>,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
}

// GLM Request Structure
#[derive(Serialize)]
struct GlmRequest {
    model: String,
    messages: Vec<Message>,
    #[serde(skip_serializing_if = "Option::is_none")]
    response_format: Option<ResponseFormat>,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
}

// Generic Request Structure (for Other/OpenAI)
#[derive(Serialize)]
struct GenericRequest {
    model: String,
    messages: Vec<Message>,
    #[serde(skip_serializing_if = "Option::is_none")]
    response_format: Option<ResponseFormat>,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
}

#[derive(Deserialize, Debug)]
struct ChatResponse {
    choices: Vec<Choice>,
    usage: Option<Usage>,
    error: Option<serde_json::Value>,
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
    #[allow(dead_code)]
    reasoning_content: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Provider {
    DeepSeek,
    Glm,
    Other,
}

pub struct LlmConfig {
    pub api_key: String,
    pub endpoint: String,
    pub model: String,
    pub provider: Provider,
}

pub fn resolve_config() -> Result<LlmConfig, String> {
    // Determine Provider from Environment Variable
    let provider_env = std::env::var("LLM_PROVIDER").unwrap_or_else(|_| "deepseek".to_string());
    let provider = match provider_env.to_lowercase().as_str() {
        "glm" => Provider::Glm,
        "other" => Provider::Other,
        _ => Provider::DeepSeek,
    };

    // Resolve details based on provider
    let (api_key, endpoint, model) = match provider {
        Provider::DeepSeek => {
            let key = std::env::var("DEEPSEEK_API_KEY")
                .map_err(|_| "Missing DEEPSEEK_API_KEY")?;
            
            let url = std::env::var("DEEPSEEK_BASE_URL")
                .unwrap_or_else(|_| "https://api.deepseek.com/chat/completions".to_string());
            
            let m = std::env::var("DEEPSEEK_MODEL")
                .unwrap_or_else(|_| "deepseek-chat".to_string());
            
            (key, url, m)
        },
        Provider::Glm => {
            let key = std::env::var("GLM_API_KEY")
                .or_else(|_| std::env::var("BIGMODEL_API_KEY"))
                .map_err(|_| "Missing GLM_API_KEY")?;
            
            let url = std::env::var("GLM_BASE_URL")
                .unwrap_or_else(|_| "https://open.bigmodel.cn/api/paas/v4/chat/completions".to_string());
            
            let m = std::env::var("GLM_MODEL")
                .unwrap_or_else(|_| "glm-4-flash".to_string());
            
            (key, url, m)
        },
        Provider::Other => {
             let key = std::env::var("OPENAI_API_KEY")
                .map_err(|_| "Missing API Key for custom provider")?;
            
            let url = std::env::var("OPENAI_BASE_URL")
                .map_err(|_| "Missing Endpoint for custom provider")?;
            let m = std::env::var("OPENAI_MODEL")
                .unwrap_or_else(|_| "gpt-4o".to_string());
            
            (key, url, m)
        }
    };

    // Normalize endpoint
    let final_endpoint = if endpoint.contains("chat/completions") {
        endpoint
    } else {
        let mut s = endpoint;
        if !s.ends_with('/') {
            s.push('/');
        }
        format!("{}chat/completions", s)
    };

    Ok(LlmConfig {
        api_key,
        endpoint: final_endpoint,
        model,
        provider,
    })
}

pub async fn call_llm(
    prompt: String,
    json_mode: bool,
) -> Result<String, String> {
    println!("Init LLM Client with {}s timeout...", DEFAULT_TIMEOUT_SECS);
    let client = Client::builder()
        .timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECS))
        .build()
        .map_err(|e| format!("Failed to build client: {}", e))?;

    let config = resolve_config()?;

    println!("Sending request to LLM (Provider: {:?}, Model: {}, Endpoint: {})...", config.provider, config.model, config.endpoint);

    let system_content = if json_mode {
        "You are a professional interactive movie scriptwriter and game designer. Output strictly valid JSON."
    } else {
        "You are a professional interactive movie scriptwriter and game designer."
    };

    let messages = vec![
        Message {
            role: "system".to_string(),
            content: system_content.to_string(),
        },
        Message {
            role: "user".to_string(),
            content: prompt,
        },
    ];

    let response = match config.provider {
        Provider::DeepSeek => {
            let body = DeepSeekRequest {
                model: config.model,
                messages,
                response_format: if json_mode {
                    Some(ResponseFormat { r#type: "json_object".to_string() })
                } else { None },
                stream: false,
                max_tokens: Some(8192),
                temperature: Some(1.5),
                top_p: Some(0.95),
            };
            client.post(&config.endpoint)
                .header("Authorization", format!("Bearer {}", config.api_key))
                .json(&body)
                .send()
                .await
        },
        Provider::Glm => {
            let body = GlmRequest {
                model: config.model,
                messages,
                response_format: if json_mode {
                    Some(ResponseFormat { r#type: "json_object".to_string() })
                } else { None },
                stream: false,
                max_tokens: Some(4096),
                temperature: Some(1.0),
                top_p: Some(0.7),
            };
            client.post(&config.endpoint)
                .header("Authorization", format!("Bearer {}", config.api_key))
                .json(&body)
                .send()
                .await
        },
        Provider::Other => {
             let body = GenericRequest {
                model: config.model,
                messages,
                response_format: if json_mode {
                    Some(ResponseFormat { r#type: "json_object".to_string() })
                } else { None },
                stream: false,
                max_tokens: Some(4096),
                temperature: Some(1.0),
                top_p: Some(1.0),
            };
            client.post(&config.endpoint)
                .header("Authorization", format!("Bearer {}", config.api_key))
                .json(&body)
                .send()
                .await
        }
    };

    let response = response.map_err(|e| format!("Request failed: {}", e))?;

    if !response.status().is_success() {
        let text = response.text().await.unwrap_or_default();
        println!("LLM Error Body: {}", text);
        return Err(text);
    }

    let text_response = response
        .text()
        .await
        .map_err(|e| format!("Failed to read response text: {}", e))?;

    // Try to parse as generic JSON first to check for "error" field
    if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(&text_response) {
        if json_value.get("error").is_some() {
            println!("LLM returned 200 OK but with error body: {}", text_response);
            return Err(text_response);
        }
    }

    let chat_response: ChatResponse = serde_json::from_str(&text_response)
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    if let Some(usage) = chat_response.usage {
        println!("Token Usage: {}", usage.total_tokens);
    }

    if let Some(choice) = chat_response.choices.first() {
        Ok(choice.message.content.clone())
    } else {
        Err("No choices in response".to_string())
    }
}

pub fn resolve_api_key(_api_key: Option<String>) -> Result<String, String> {
    resolve_config().map(|c| c.api_key)
}
