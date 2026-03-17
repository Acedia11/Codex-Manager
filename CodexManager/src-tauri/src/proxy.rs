use axum::{
    Router,
    extract::State,
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use futures_util::StreamExt;
use serde_json::{json, Value};

use crate::keychain;
use crate::oauth;
use crate::state::SharedState;
use crate::types::ProxyAccountInfo;
use crate::utils;

const CODEX_RESPONSES_URL: &str = "https://chatgpt.com/backend-api/codex/responses";
pub const PROXY_PORT: u16 = 18080;

const STRIPPED_FIELDS: &[&str] = &[
    "max_output_tokens",
    "max_completion_tokens",
    "metadata",
    "previous_response_id",
    "prompt_cache_retention",
];

const UNSUPPORTED_TOOL_TYPES: &[&str] = &[
    "shell",
    "file_search",
    "code_interpreter",
    "computer_use",
];

fn ErrorResponse(Code: StatusCode, Message: &str, Kind: &str) -> Response {
    let Body = json!({
        "error": {
            "message": Message,
            "type": Kind,
        }
    });
    (Code, [(header::CONTENT_TYPE, "application/json")], Body.to_string()).into_response()
}

pub async fn StartProxyServer(AppState: SharedState) {
    let App = Router::new()
        .route("/v1/responses", post(HandleResponses))
        .route("/v1/models", get(HandleModels))
        .route("/v1/accounts", get(HandleAccounts))
        .route("/v1/health", get(HandleHealth))
        .with_state(AppState.clone());

    let Addr = std::net::SocketAddr::from(([127, 0, 0, 1], PROXY_PORT));
    log::info!("Proxy server starting on http://{}", Addr);

    let Listener = match tokio::net::TcpListener::bind(Addr).await {
        Ok(L) => L,
        Err(E) => {
            log::error!("Failed to bind proxy server to {}: {}", Addr, E);
            return;
        }
    };

    {
        let mut Guard = AppState.lock().unwrap();
        Guard.ProxyRunning = true;
    }

    if let Err(E) = axum::serve(Listener, App).await {
        log::error!("Proxy server error: {}", E);
    }
}

async fn HandleResponses(
    State(AppState): State<SharedState>,
    RawBody: String,
) -> Response {
    let mut Body: Value = match serde_json::from_str(&RawBody) {
        Ok(V) => V,
        Err(_) => return ErrorResponse(
            StatusCode::BAD_REQUEST,
            "Invalid request body",
            "invalid_request_error",
        ),
    };

    if Body.get("model").and_then(|V| V.as_str()).is_none() {
        return ErrorResponse(
            StatusCode::BAD_REQUEST,
            "model is required",
            "invalid_request_error",
        );
    }

    let ClientWantsStream = Body.get("stream")
        .and_then(|V| V.as_bool())
        .unwrap_or(true);

    let (AccountId, Email) = {
        let Guard = AppState.lock().unwrap();
        match Guard.SelectBestAccount() {
            Some((_Id, AccId, Eml)) => (AccId, Eml),
            None => return ErrorResponse(
                StatusCode::SERVICE_UNAVAILABLE,
                "No available accounts — all rate-limited or expired",
                "server_error",
            ),
        }
    };

    let AccessToken = match GetValidToken(&AccountId).await {
        Ok(T) => T,
        Err(E) => {
            log::warn!("Token acquisition failed for {}: {}", Email, E);
            return ErrorResponse(
                StatusCode::BAD_GATEWAY,
                &format!("Token acquisition failed: {}", E),
                "server_error",
            );
        }
    };

    SanitizeRequest(&mut Body);

    let Resp = match utils::HTTP
        .post(CODEX_RESPONSES_URL)
        .header("Authorization", format!("Bearer {}", AccessToken))
        .header("ChatGPT-Account-Id", &AccountId)
        .header("Content-Type", "application/json")
        .body(Body.to_string())
        .send()
        .await
    {
        Ok(R) => R,
        Err(E) => return ErrorResponse(
            StatusCode::BAD_GATEWAY,
            &format!("Codex backend request failed: {}", E),
            "server_error",
        ),
    };

    let UpstreamStatus = Resp.status();

    if !UpstreamStatus.is_success() {
        let Code = StatusCode::from_u16(UpstreamStatus.as_u16())
            .unwrap_or(StatusCode::BAD_GATEWAY);
        let ErrBody = Resp.text().await.unwrap_or_default();
        return (Code, [(header::CONTENT_TYPE, "application/json")], ErrBody).into_response();
    }

    let AccountHeader = Email.parse().unwrap_or_else(|_| "unknown".parse().unwrap());

    if ClientWantsStream {
        let ByteStream = Resp.bytes_stream().map(|Chunk| {
            Chunk.map_err(|E| std::io::Error::new(std::io::ErrorKind::Other, E))
        });
        let StreamBody = axum::body::Body::from_stream(ByteStream);

        let mut Headers = HeaderMap::new();
        Headers.insert(header::CONTENT_TYPE, "text/event-stream".parse().unwrap());
        Headers.insert(header::CACHE_CONTROL, "no-cache".parse().unwrap());
        Headers.insert(header::CONNECTION, "keep-alive".parse().unwrap());
        Headers.insert("X-CodexManager-Account", AccountHeader);

        (Headers, StreamBody).into_response()
    } else {
        let mut FullText = String::new();
        let mut FinalResponse: Option<Value> = None;
        let RawBody = Resp.text().await.unwrap_or_default();

        for Line in RawBody.lines() {
            let Line = Line.trim();
            if !Line.starts_with("data: ") {
                continue;
            }
            let Data = &Line[6..];
            if Data == "[DONE]" {
                break;
            }
            if let Ok(Event) = serde_json::from_str::<Value>(Data) {
                let EventType = Event.get("type").and_then(|V| V.as_str()).unwrap_or("");
                match EventType {
                    "response.output_text.delta" => {
                        if let Some(Delta) = Event.get("delta").and_then(|V| V.as_str()) {
                            FullText.push_str(Delta);
                        }
                    }
                    "response.completed" => {
                        FinalResponse = Event.get("response").cloned();
                    }
                    _ => {}
                }
            }
        }

        let ResponseBody = FinalResponse.unwrap_or_else(|| {
            json!({
                "object": "response",
                "output": [{
                    "type": "message",
                    "role": "assistant",
                    "content": [{"type": "output_text", "text": FullText}],
                }],
            })
        });

        let mut Headers = HeaderMap::new();
        Headers.insert(header::CONTENT_TYPE, "application/json".parse().unwrap());
        Headers.insert("X-CodexManager-Account", AccountHeader);

        (Headers, ResponseBody.to_string()).into_response()
    }
}

async fn HandleModels(State(_AppState): State<SharedState>) -> Response {
    let Body = json!({
        "object": "list",
        "data": [
            {"id": "gpt-5-codex-mini", "object": "model", "owned_by": "openai"},
            {"id": "gpt-5.1-codex-mini", "object": "model", "owned_by": "openai"},
            {"id": "gpt-5.3-codex", "object": "model", "owned_by": "openai"},
            {"id": "gpt-5.4", "object": "model", "owned_by": "openai"},
        ]
    });
    ([(header::CONTENT_TYPE, "application/json")], Body.to_string()).into_response()
}

async fn HandleAccounts(State(AppState): State<SharedState>) -> Response {
    let Accounts: Vec<ProxyAccountInfo> = {
        let Guard = AppState.lock().unwrap();
        Guard.Accounts.values().map(|Meta| {
            let Usage = Guard.Usage.get(&Meta.Id);
            let Status = Guard.TokenStatus.get(&Meta.Id);
            ProxyAccountInfo {
                Email: Meta.Email.clone(),
                PlanType: Meta.PlanType.clone(),
                PrimaryUsedPercent: Usage.map(|U| U.PrimaryUsedPercent).unwrap_or(0.0),
                SecondaryUsedPercent: Usage.map(|U| U.SecondaryUsedPercent).unwrap_or(0.0),
                Active: matches!(Status, Some(crate::types::TokenStatus::Active)),
            }
        }).collect()
    };

    let Body = serde_json::to_string(&Accounts).unwrap_or_else(|_| "[]".to_string());
    ([(header::CONTENT_TYPE, "application/json")], Body).into_response()
}

async fn HandleHealth(State(AppState): State<SharedState>) -> Response {
    let Count = {
        let Guard = AppState.lock().unwrap();
        Guard.AvailableAccountCount()
    };

    let Body = json!({
        "status": "ok",
        "available_accounts": Count,
    });
    ([(header::CONTENT_TYPE, "application/json")], Body.to_string()).into_response()
}

fn SanitizeRequest(Body: &mut Value) {
    if let Some(Obj) = Body.as_object_mut() {
        for Field in STRIPPED_FIELDS {
            Obj.remove(*Field);
        }

        Obj.insert("store".to_string(), json!(false));
        Obj.insert("stream".to_string(), json!(true));

        if !Obj.contains_key("instructions") {
            Obj.insert("instructions".to_string(), json!("You are a helpful assistant."));
        }
    }

    if let Some(Input) = Body.get_mut("input") {
        if let Some(Items) = Input.as_array_mut() {
            FixContentTypes(Items);
            Items.retain(|Item| {
                Item.get("type").and_then(|V| V.as_str()) != Some("reasoning")
            });
        }
    }

    if let Some(Tools) = Body.get_mut("tools").and_then(|V| V.as_array_mut()) {
        Tools.retain(|Tool| {
            let ToolType = Tool.get("type").and_then(|V| V.as_str()).unwrap_or("");
            !UNSUPPORTED_TOOL_TYPES.contains(&ToolType)
        });

        for Tool in Tools.iter_mut() {
            if let Some(TypeVal) = Tool.get_mut("type") {
                if TypeVal.as_str() == Some("web_search_preview") {
                    *TypeVal = json!("web_search");
                }
            }
        }
    } else if let Some(Obj) = Body.as_object_mut() {
        Obj.insert("tools".to_string(), json!([{"type": "web_search"}]));
    }
}

fn FixContentTypes(Items: &mut Vec<Value>) {
    for Item in Items.iter_mut() {
        if let Some(Content) = Item.get_mut("content") {
            if let Some(Parts) = Content.as_array_mut() {
                for Part in Parts.iter_mut() {
                    if let Some(TypeVal) = Part.get_mut("type") {
                        if TypeVal.as_str() == Some("text") {
                            *TypeVal = json!("input_text");
                        }
                    }
                }
            }
        }
    }
}

async fn GetValidToken(AccountId: &str) -> Result<String, String> {
    let Id = AccountId.to_string();
    let IdClone = Id.clone();
    let Tokens = tokio::task::spawn_blocking(move || keychain::LoadTokens(&IdClone))
        .await
        .map_err(|E| format!("Keychain task failed: {}", E))?
        .map_err(|E| format!("Keychain error: {}", E))?
        .ok_or_else(|| "No tokens found".to_string())?;

    if oauth::NeedsRefresh(Tokens.ExpiresAt) {
        let NewTokens = oauth::RefreshAccessToken(&Tokens.RefreshToken).await?;
        let Token = NewTokens.AccessToken.clone();
        let _ = tokio::task::spawn_blocking(move || keychain::StoreTokens(&Id, &NewTokens)).await;
        Ok(Token)
    } else {
        Ok(Tokens.AccessToken)
    }
}
