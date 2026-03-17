use base64::Engine;
use std::sync::mpsc;
use tauri_plugin_oauth::{start_with_config, OauthConfig, cancel};
use url::Url;

use crate::types::TokenSet;
use crate::utils;

const AUTH_ENDPOINT: &str = "https://auth.openai.com/oauth/authorize";
const TOKEN_ENDPOINT: &str = "https://auth.openai.com/oauth/token";
const CLIENT_ID: &str = "app_EMoamEEZ73f0CkXaXp7hrann";
const REDIRECT_PORT: u16 = 1455;
const REDIRECT_URI: &str = "http://localhost:1455/auth/callback";
const SCOPES: &str = "openid profile email offline_access api.connectors.read api.connectors.invoke";
const REFRESH_MARGIN_SEC: i64 = 300;

pub fn ParseJwt(Token: &str) -> serde_json::Value {
    let Parts: Vec<&str> = Token.split('.').collect();
    if Parts.len() < 2 {
        return serde_json::Value::Null;
    }
    let Payload = Parts[1];
    let PadLen = (4 - Payload.len() % 4) % 4;
    let Padded = format!("{}{}", Payload, "=".repeat(PadLen));
    match base64::engine::general_purpose::URL_SAFE.decode(&Padded) {
        Ok(Decoded) => serde_json::from_slice(&Decoded).unwrap_or(serde_json::Value::Null),
        Err(_) => serde_json::Value::Null,
    }
}

pub fn ExtractAccountInfo(Claims: &serde_json::Value) -> (String, String, String) {
    let AuthClaims = &Claims["https://api.openai.com/auth"];
    let ProfileClaims = &Claims["https://api.openai.com/profile"];

    let AccountId = AuthClaims["chatgpt_account_id"]
        .as_str()
        .or_else(|| Claims["chatgpt_account_id"].as_str())
        .unwrap_or("")
        .to_string();

    let PlanType = AuthClaims["chatgpt_plan_type"]
        .as_str()
        .unwrap_or("unknown")
        .to_string();

    let Email = ProfileClaims["email"]
        .as_str()
        .unwrap_or("")
        .to_string();

    (AccountId, Email, PlanType)
}

pub async fn Login() -> Result<(TokenSet, String, String, String), String> {
    let (Verifier, Challenge) = utils::GeneratePkce();
    let State = utils::GenerateState();
    let ExpectedState = State.clone();

    let AuthUrl = format!(
        "{}?response_type=code&client_id={}&redirect_uri={}&scope={}&code_challenge={}&code_challenge_method=S256&state={}&id_token_add_organizations=true&codex_cli_simplified_flow=true",
        AUTH_ENDPOINT,
        urlencoding::encode(CLIENT_ID),
        urlencoding::encode(REDIRECT_URI),
        urlencoding::encode(SCOPES),
        urlencoding::encode(&Challenge),
        urlencoding::encode(&State),
    );

    let (Tx, Rx) = mpsc::channel::<String>();

    let Config = OauthConfig {
        ports: Some(vec![REDIRECT_PORT]),
        response: Some("Authentication successful! You can close this tab and return to CodexManager.".into()),
    };

    let Port = start_with_config(Config, move |UrlStr| {
        let _ = Tx.send(UrlStr);
    }).map_err(|E| format!("Failed to start OAuth server: {}", E))?;

    open::that(&AuthUrl).map_err(|E| format!("Failed to open browser: {}", E))?;

    let CallbackUrl = Rx.recv_timeout(std::time::Duration::from_secs(300))
        .map_err(|_| "OAuth login timed out after 5 minutes".to_string())?;

    let _ = cancel(Port);

    let Parsed = Url::parse(&CallbackUrl)
        .or_else(|_| Url::parse(&format!("http://localhost{}", CallbackUrl)))
        .map_err(|E| format!("Failed to parse callback URL: {}", E))?;

    let Code = Parsed.query_pairs()
        .find(|(K, _)| K == "code")
        .map(|(_, V)| V.to_string())
        .ok_or_else(|| {
            let ErrMsg = Parsed.query_pairs()
                .find(|(K, _)| K == "error")
                .map(|(_, V)| V.to_string())
                .unwrap_or_else(|| "unknown error".to_string());
            format!("OAuth failed: {}", ErrMsg)
        })?;

    let CallbackState = Parsed.query_pairs()
        .find(|(K, _)| K == "state")
        .map(|(_, V)| V.to_string());

    if CallbackState.as_deref() != Some(&ExpectedState) {
        return Err("OAuth state mismatch (possible CSRF attack)".to_string());
    }

    let Tokens = ExchangeCode(&Code, &Verifier).await?;
    let Claims = ParseJwt(&Tokens.AccessToken);
    let (AccountId, Email, PlanType) = ExtractAccountInfo(&Claims);

    if AccountId.is_empty() {
        return Err("Could not extract account ID from token".to_string());
    }

    Ok((Tokens, AccountId, Email, PlanType))
}

async fn ExchangeCode(Code: &str, Verifier: &str) -> Result<TokenSet, String> {
    let Client = &*utils::HTTP;
    let Resp = Client
        .post(TOKEN_ENDPOINT)
        .form(&[
            ("grant_type", "authorization_code"),
            ("client_id", CLIENT_ID),
            ("code", Code),
            ("redirect_uri", REDIRECT_URI),
            ("code_verifier", Verifier),
        ])
        .send()
        .await
        .map_err(|E| format!("Token exchange request failed: {}", E))?;

    if !Resp.status().is_success() {
        let Status = Resp.status();
        let Body = Resp.text().await.unwrap_or_default();
        return Err(format!("Token exchange failed ({}): {}", Status, Body));
    }

    let Body: serde_json::Value = Resp.json().await.map_err(|E| E.to_string())?;

    let AccessToken = Body["access_token"].as_str()
        .ok_or("No access_token in response")?
        .to_string();
    let RefreshToken = Body["refresh_token"].as_str()
        .unwrap_or("")
        .to_string();

    let Claims = ParseJwt(&AccessToken);
    let ExpiresAt = Claims["exp"].as_i64()
        .unwrap_or_else(|| utils::UnixNow() + 3600);

    Ok(TokenSet { AccessToken, RefreshToken, ExpiresAt })
}

pub async fn RefreshAccessToken(CurrentRefreshToken: &str) -> Result<TokenSet, String> {
    let Client = &*utils::HTTP;
    let Resp = Client
        .post(TOKEN_ENDPOINT)
        .form(&[
            ("grant_type", "refresh_token"),
            ("client_id", CLIENT_ID),
            ("refresh_token", CurrentRefreshToken),
        ])
        .send()
        .await
        .map_err(|E| format!("Token refresh request failed: {}", E))?;

    if !Resp.status().is_success() {
        let Status = Resp.status();
        let Body = Resp.text().await.unwrap_or_default();
        return Err(format!("Token refresh failed ({}): {}", Status, Body));
    }

    let Body: serde_json::Value = Resp.json().await.map_err(|E| E.to_string())?;

    let AccessToken = Body["access_token"].as_str()
        .ok_or("No access_token in refresh response")?
        .to_string();
    let RefreshToken = Body["refresh_token"].as_str()
        .unwrap_or(CurrentRefreshToken)
        .to_string();

    let Claims = ParseJwt(&AccessToken);
    let ExpiresAt = Claims["exp"].as_i64()
        .unwrap_or_else(|| utils::UnixNow() + 3600);

    Ok(TokenSet { AccessToken, RefreshToken, ExpiresAt })
}

pub fn NeedsRefresh(ExpiresAt: i64) -> bool {
    utils::UnixNow() >= ExpiresAt - REFRESH_MARGIN_SEC
}

