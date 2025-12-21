use axum::http::StatusCode;
use base64::Engine;
use reqwest::Client;
use serde::Deserialize;
use serde_json::json;

use crate::api_types::{CharacterInput, GenerateRequest};
use crate::types::MovieTemplate;

pub(crate) fn pick_background_prompt(req: &GenerateRequest, template: &MovieTemplate) -> String {
    let from_template = template.meta.synopsis.trim();
    if !from_template.is_empty() {
        return from_template.to_string();
    }

    let from_req = req.synopsis.as_deref().unwrap_or("").trim();
    if !from_req.is_empty() {
        return from_req.to_string();
    }

    let from_theme = req.theme.as_deref().unwrap_or("").trim();
    if !from_theme.is_empty() {
        return from_theme.to_string();
    }

    let from_free = req.free_input.as_deref().unwrap_or("").trim();
    if !from_free.is_empty() {
        return from_free.to_string();
    }

    template.title.trim().to_string()
}

fn simple_hash_u32(s: &str) -> u32 {
    let mut h: u32 = 2166136261;
    for b in s.as_bytes() {
        h ^= *b as u32;
        h = h.wrapping_mul(16777619);
    }
    h
}

fn svg_to_data_uri(svg: &str) -> String {
    let b64 = base64::engine::general_purpose::STANDARD.encode(svg.as_bytes());
    format!("data:image/svg+xml;base64,{}", b64)
}

pub(crate) fn fallback_background_data_uri(title: &str, synopsis: &str) -> String {
    let seed = simple_hash_u32(&format!("{}::{}", title.trim(), synopsis.trim()));
    let h1 = (seed % 360) as i32;
    let h2 = ((seed.wrapping_mul(3) % 360) as i32 + 360) % 360;
    let h3 = ((seed.wrapping_mul(7) % 360) as i32 + 360) % 360;
    let svg = format!(
        r#"<svg xmlns='http://www.w3.org/2000/svg' width='1024' height='1024' viewBox='0 0 1024 1024'>
  <defs>
    <linearGradient id='g' x1='0' y1='0' x2='1' y2='1'>
      <stop offset='0%' stop-color='hsl({h1} 85% 55%)' stop-opacity='0.95'/>
      <stop offset='55%' stop-color='hsl({h2} 85% 55%)' stop-opacity='0.75'/>
      <stop offset='100%' stop-color='hsl({h3} 85% 50%)' stop-opacity='0.95'/>
    </linearGradient>
    <filter id='blur' x='-20%' y='-20%' width='140%' height='140%'>
      <feGaussianBlur stdDeviation='38'/>
    </filter>
  </defs>
  <rect width='1024' height='1024' fill='url(#g)'/>
  <g filter='url(#blur)'>
    <circle cx='260' cy='280' r='240' fill='white' opacity='0.14'/>
    <circle cx='780' cy='360' r='280' fill='white' opacity='0.10'/>
    <circle cx='520' cy='820' r='320' fill='black' opacity='0.10'/>
  </g>
  <rect width='1024' height='1024' fill='black' opacity='0.22'/>
</svg>"#
    );
    svg_to_data_uri(&svg)
}

pub(crate) fn fallback_avatar_data_uri(name: &str) -> String {
    let seed = simple_hash_u32(name.trim());
    let h1 = (seed % 360) as i32;
    let h2 = ((seed.wrapping_mul(5) % 360) as i32 + 360) % 360;
    let svg = format!(
        r#"<svg xmlns='http://www.w3.org/2000/svg' width='512' height='512' viewBox='0 0 512 512'>
  <defs>
    <radialGradient id='rg' cx='35%' cy='30%' r='80%'>
      <stop offset='0%' stop-color='hsl({h1} 90% 65%)' stop-opacity='1'/>
      <stop offset='65%' stop-color='hsl({h2} 90% 55%)' stop-opacity='1'/>
      <stop offset='100%' stop-color='hsl({h2} 90% 35%)' stop-opacity='1'/>
    </radialGradient>
  </defs>
  <rect width='512' height='512' rx='256' fill='url(#rg)'/>
  <g opacity='0.92'>
    <circle cx='256' cy='210' r='86' fill='rgba(255,255,255,0.92)'/>
    <path d='M128 446c18-86 78-134 128-134s110 48 128 134' fill='rgba(255,255,255,0.92)'/>
  </g>
  <rect width='512' height='512' rx='256' fill='rgba(0,0,0,0.18)'/>
</svg>"#
    );
    svg_to_data_uri(&svg)
}

pub(crate) fn attach_avatar_to_template(
    template: &mut MovieTemplate,
    protagonist_name: &str,
    avatar_data_uri: String,
) {
    let protagonist_name = protagonist_name.trim();
    if protagonist_name.is_empty() {
        return;
    }

    if let Some((_k, c)) = template
        .characters
        .iter_mut()
        .find(|(_k, c)| c.name.trim() == protagonist_name)
    {
        if c.avatar_path.as_deref().unwrap_or("").trim().is_empty() {
            c.avatar_path = Some(avatar_data_uri);
        }
    }
}

fn attach_avatar_to_first_character(template: &mut MovieTemplate, avatar_data_uri: String) {
    if let Some((_k, c)) = template.characters.iter_mut().next() {
        if c.avatar_path.as_deref().unwrap_or("").trim().is_empty() {
            c.avatar_path = Some(avatar_data_uri);
        }
    }
}

pub(crate) fn ensure_avatar_fallbacks(
    template: &mut MovieTemplate,
    req_chars: Option<&Vec<CharacterInput>>,
) {
    let mut expected_names: Vec<String> = vec![];
    if let Some(req_chars) = req_chars {
        let mut mains: Vec<&CharacterInput> = req_chars.iter().filter(|c| c.is_main).collect();
        mains.sort_by(|a, b| a.name.cmp(&b.name));
        expected_names.extend(mains.into_iter().take(2).map(|c| c.name.trim().to_string()));
        if expected_names.is_empty() {
            expected_names.extend(req_chars.iter().take(2).map(|c| c.name.trim().to_string()));
        }
    }

    expected_names.retain(|n| !n.trim().is_empty());

    if expected_names.is_empty() {
        let any_name = template
            .characters
            .values()
            .next()
            .map(|c| c.name.clone())
            .unwrap_or_else(|| "Protagonist".to_string());
        attach_avatar_to_first_character(template, fallback_avatar_data_uri(&any_name));
        return;
    }

    for name in expected_names {
        let uri = fallback_avatar_data_uri(&name);
        attach_avatar_to_template(template, &name, uri.clone());
    }

    if template
        .characters
        .values()
        .all(|c| c.avatar_path.as_deref().unwrap_or("").trim().is_empty())
    {
        attach_avatar_to_first_character(template, fallback_avatar_data_uri("Protagonist"));
    }
}

#[derive(Clone, Debug)]
pub(crate) struct ProtagonistSpec {
    name: String,
    description: String,
    gender: String,
}

fn select_protagonists(req_chars: Option<&Vec<CharacterInput>>) -> Vec<ProtagonistSpec> {
    let Some(req_chars) = req_chars else {
        return vec![];
    };

    let mut mains: Vec<&CharacterInput> = req_chars.iter().filter(|c| c.is_main).collect();
    mains.sort_by(|a, b| a.name.cmp(&b.name));

    let mut picked: Vec<&CharacterInput> = vec![];
    if !mains.is_empty() {
        picked.extend(mains.into_iter().take(2));
    } else {
        picked.extend(req_chars.iter().take(2));
    }

    picked
        .into_iter()
        .map(|c| ProtagonistSpec {
            name: c.name.trim().to_string(),
            description: c.description.trim().to_string(),
            gender: c.gender.as_deref().unwrap_or("").trim().to_string(),
        })
        .filter(|c| !c.name.is_empty() && !c.description.is_empty())
        .collect()
}

pub(crate) fn normalize_cogview_size(raw: Option<&str>) -> String {
    match raw.unwrap_or("").trim() {
        "1024x1024" => "1024x1024".to_string(),
        "864x1152" => "864x1152".to_string(),
        "1152x864" => "1152x864".to_string(),
        _ => "1024x1024".to_string(),
    }
}

pub(crate) async fn generate_scene_background_base64(
    client: &Client,
    synopsis: &str,
    language_tag: &str,
    size: &str,
    api_key: &str,
) -> Result<String, StatusCode> {
    #[derive(Deserialize)]
    struct CogViewImageResponse {
        created: u64,
        data: Vec<CogViewImageData>,
    }

    #[derive(Deserialize)]
    struct CogViewImageData {
        url: String,
    }

    let language_hint = if language_tag.to_lowercase().starts_with("zh") {
        "简体中文"
    } else {
        "English"
    };

    let prompt = format!(
        "Create a cinematic environment / scene image for an interactive movie game.\n\
Language: {}\n\
Story synopsis: {}\n\
Hard constraints (must follow):\n\
- DO NOT generate any people, characters, faces, portraits, hands, or human silhouettes.\n\
- Scene / environment ONLY: locations, lighting, atmosphere, props, architecture, weather.\n\
- No text, no logos, no watermarks, no UI elements.\n\
- Keep mood consistent with the synopsis.",
        language_hint,
        synopsis.trim()
    );

    let request_body = json!({
        "model": "cogview-3-flash",
        "prompt": prompt,
        "quality": "hd",
        "size": size,
        "watermark_enabled": false
    });

    let resp = client
        .post("https://open.bigmodel.cn/api/paas/v4/images/generations")
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&request_body)
        .send()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if !resp.status().is_success() {
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    let json_resp: CogViewImageResponse = resp
        .json()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let _ = json_resp.created;

    let url = json_resp
        .data
        .get(0)
        .map(|d| d.url.trim().to_string())
        .filter(|u| !u.is_empty())
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

    let img_resp = client
        .get(url)
        .send()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if !img_resp.status().is_success() {
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    let content_type = img_resp
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("image/png")
        .to_string();

    let bytes = img_resp
        .bytes()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let b64 = base64::engine::general_purpose::STANDARD.encode(bytes);
    Ok(format!("data:{};base64,{}", content_type, b64))
}

pub(crate) async fn generate_protagonist_avatar_base64(
    client: &Client,
    template: &MovieTemplate,
    protagonist: &ProtagonistSpec,
    language_tag: &str,
    api_key: &str,
) -> Result<String, StatusCode> {
    #[derive(Deserialize)]
    struct CogViewImageResponse {
        created: u64,
        data: Vec<CogViewImageData>,
    }

    #[derive(Deserialize)]
    struct CogViewImageData {
        url: String,
    }

    let language_hint = if language_tag.to_lowercase().starts_with("zh") {
        "简体中文"
    } else {
        "English"
    };

    let extra = template
        .characters
        .values()
        .find(|c| c.name.trim() == protagonist.name.trim())
        .map(|c| {
            format!(
                "Background: {}\nRole: {}",
                c.background.trim(),
                c.role.trim()
            )
        })
        .unwrap_or_default();

    let prompt = format!(
        "Create a high-quality protagonist portrait avatar for an interactive movie game.\n\
Language: {}\n\
Character name: {}\n\
Character gender: {}\n\
Character introduction: {}\n\
Additional character details: {}\n\
Hard constraints (must follow):\n\
- Single person ONLY.\n\
- Front-facing portrait / headshot, centered, shoulders-up.\n\
- Transparent background (alpha).\n\
- No text, no logos, no watermark, no UI.\n\
- No extra people, no hands, no full body.\n\
- Cinematic realistic style, clean lighting, sharp focus.",
        language_hint,
        protagonist.name.trim(),
        protagonist.gender.trim(),
        protagonist.description.trim(),
        extra.trim()
    );

    let request_body = json!({
        "model": "cogview-3-flash",
        "prompt": prompt,
        "quality": "hd",
        "size": "1024x1024",
        "watermark_enabled": false
    });

    let resp = client
        .post("https://open.bigmodel.cn/api/paas/v4/images/generations")
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&request_body)
        .send()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if !resp.status().is_success() {
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    let json_resp: CogViewImageResponse = resp
        .json()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let _ = json_resp.created;

    let url = json_resp
        .data
        .get(0)
        .map(|d| d.url.trim().to_string())
        .filter(|u| !u.is_empty())
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

    let img_resp = client
        .get(url)
        .send()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if !img_resp.status().is_success() {
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    let content_type = img_resp
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("image/png")
        .to_string();

    let bytes = img_resp
        .bytes()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let b64 = base64::engine::general_purpose::STANDARD.encode(bytes);
    Ok(format!("data:{};base64,{}", content_type, b64))
}

pub(crate) async fn maybe_attach_generated_avatars(
    client: &Client,
    template: &mut MovieTemplate,
    req_chars: Option<&Vec<CharacterInput>>,
    language_tag: &str,
    api_key: &str,
) {
    let protagonists = select_protagonists(req_chars);
    if protagonists.len() == 1 {
        if let Some(spec) = protagonists.get(0) {
            if let Ok(img) =
                generate_protagonist_avatar_base64(client, template, spec, language_tag, api_key)
                    .await
            {
                attach_avatar_to_template(template, &spec.name, img);
            }
        }
    } else if protagonists.len() >= 2 {
        let a = protagonists[0].clone();
        let b = protagonists[1].clone();
        let (ra, rb) = tokio::join!(
            generate_protagonist_avatar_base64(client, template, &a, language_tag, api_key),
            generate_protagonist_avatar_base64(client, template, &b, language_tag, api_key)
        );
        if let Ok(img) = ra {
            attach_avatar_to_template(template, &a.name, img);
        }
        if let Ok(img) = rb {
            attach_avatar_to_template(template, &b.name, img);
        }
    }
}
