use once_cell::sync::Lazy;
use regex::Regex;
use std::sync::mpsc;
use tauri_plugin_oauth::{start_with_config, OauthConfig, cancel};
use url::Url;

use crate::types::TokenSet;
use crate::utils;

const MS_AUTH_ENDPOINT: &str = "https://login.microsoftonline.com/consumers/oauth2/v2.0/authorize";
const MS_TOKEN_ENDPOINT: &str = "https://login.microsoftonline.com/consumers/oauth2/v2.0/token";
const MS_CLIENT_ID: &str = "9d18d0e9-e166-4400-945d-1d3840238937";
const MS_REDIRECT_PORT: u16 = 1456;
const MS_REDIRECT_URI: &str = "http://localhost:1456/auth/ms-callback";
const MS_SCOPES: &str = "openid email Mail.Read offline_access";

pub async fn LinkAccount() -> Result<TokenSet, String> {
    let (Verifier, Challenge) = utils::GeneratePkce();
    let State = utils::GenerateState();
    let ExpectedState = State.clone();

    let AuthUrl = format!(
        "{}?response_type=code&client_id={}&redirect_uri={}&scope={}&code_challenge={}&code_challenge_method=S256&state={}",
        MS_AUTH_ENDPOINT,
        urlencoding::encode(MS_CLIENT_ID),
        urlencoding::encode(MS_REDIRECT_URI),
        urlencoding::encode(MS_SCOPES),
        urlencoding::encode(&Challenge),
        urlencoding::encode(&State),
    );

    let (Tx, Rx) = mpsc::channel::<String>();

    let Config = OauthConfig {
        ports: Some(vec![MS_REDIRECT_PORT]),
        response: Some("Microsoft account linked! You can close this tab.".into()),
    };

    let Port = start_with_config(Config, move |UrlStr| {
        let _ = Tx.send(UrlStr);
    }).map_err(|E| format!("Failed to start MS OAuth server: {}", E))?;

    open::that(&AuthUrl).map_err(|E| format!("Failed to open browser: {}", E))?;

    let CallbackUrl = Rx.recv_timeout(std::time::Duration::from_secs(300))
        .map_err(|_| "Microsoft login timed out after 5 minutes".to_string())?;

    let _ = cancel(Port);

    let Parsed = Url::parse(&CallbackUrl)
        .or_else(|_| Url::parse(&format!("http://localhost{}", CallbackUrl)))
        .map_err(|E| format!("Failed to parse MS callback URL: {}", E))?;

    let Code = Parsed.query_pairs()
        .find(|(K, _)| K == "code")
        .map(|(_, V)| V.to_string())
        .ok_or_else(|| {
            let ErrMsg = Parsed.query_pairs()
                .find(|(K, _)| K == "error_description")
                .or_else(|| Parsed.query_pairs().find(|(K, _)| K == "error"))
                .map(|(_, V)| V.to_string())
                .unwrap_or_else(|| "unknown error".to_string());
            format!("MS OAuth failed: {}", ErrMsg)
        })?;

    let CallbackState = Parsed.query_pairs()
        .find(|(K, _)| K == "state")
        .map(|(_, V)| V.to_string());

    if CallbackState.as_deref() != Some(&ExpectedState) {
        return Err("MS OAuth state mismatch".to_string());
    }

    ExchangeCode(&Code, &Verifier).await
}

async fn ExchangeCode(Code: &str, Verifier: &str) -> Result<TokenSet, String> {
    let Client = &*utils::HTTP;
    let Resp = Client
        .post(MS_TOKEN_ENDPOINT)
        .form(&[
            ("grant_type", "authorization_code"),
            ("client_id", MS_CLIENT_ID),
            ("code", Code),
            ("redirect_uri", MS_REDIRECT_URI),
            ("code_verifier", Verifier),
        ])
        .send()
        .await
        .map_err(|E| format!("MS token exchange failed: {}", E))?;

    if !Resp.status().is_success() {
        let Status = Resp.status();
        let Body = Resp.text().await.unwrap_or_default();
        return Err(format!("MS token exchange failed ({}): {}", Status, Body));
    }

    let Body: serde_json::Value = Resp.json().await.map_err(|E| E.to_string())?;

    let AccessToken = Body["access_token"].as_str()
        .ok_or("No access_token in MS response")?
        .to_string();
    let RefreshToken = Body["refresh_token"].as_str()
        .unwrap_or("")
        .to_string();
    let ExpiresIn = Body["expires_in"].as_i64().unwrap_or(3600);
    let ExpiresAt = utils::UnixNow() + ExpiresIn;

    Ok(TokenSet { AccessToken, RefreshToken, ExpiresAt })
}

pub async fn RefreshMsToken(CurrentRefreshToken: &str) -> Result<TokenSet, String> {
    let Client = &*utils::HTTP;
    let Resp = Client
        .post(MS_TOKEN_ENDPOINT)
        .form(&[
            ("grant_type", "refresh_token"),
            ("client_id", MS_CLIENT_ID),
            ("refresh_token", CurrentRefreshToken),
            ("scope", MS_SCOPES),
        ])
        .send()
        .await
        .map_err(|E| format!("MS token refresh failed: {}", E))?;

    if !Resp.status().is_success() {
        let Status = Resp.status();
        let Body = Resp.text().await.unwrap_or_default();
        return Err(format!("MS token refresh failed ({}): {}", Status, Body));
    }

    let Body: serde_json::Value = Resp.json().await.map_err(|E| E.to_string())?;

    let AccessToken = Body["access_token"].as_str()
        .ok_or("No access_token in MS refresh response")?
        .to_string();
    let RefreshToken = Body["refresh_token"].as_str()
        .unwrap_or(CurrentRefreshToken)
        .to_string();
    let ExpiresIn = Body["expires_in"].as_i64().unwrap_or(3600);
    let ExpiresAt = utils::UnixNow() + ExpiresIn;

    Ok(TokenSet { AccessToken, RefreshToken, ExpiresAt })
}

async fn GetValidMsToken(AccountId: &str) -> Result<String, String> {
    let Tokens = crate::keychain::LoadMsTokens(AccountId)?
        .ok_or_else(|| "No Microsoft account linked".to_string())?;

    if utils::UnixNow() >= Tokens.ExpiresAt - 300 {
        let Refreshed = RefreshMsToken(&Tokens.RefreshToken).await?;
        crate::keychain::StoreMsTokens(AccountId, &Refreshed)?;
        Ok(Refreshed.AccessToken)
    } else {
        Ok(Tokens.AccessToken)
    }
}

pub async fn FetchVerificationCode(AccountId: &str) -> Result<Option<String>, String> {
    let Token = GetValidMsToken(AccountId).await?;
    let Client = &*utils::HTTP;

    // Filters to try in order — eq can return 0 on personal accounts (known MS bug),
    // startsWith is more reliable, and broadening to all openai.com is the last resort
    let Filters: &[&str] = &[
        "receivedDateTime ge 1900-01-01T00:00:00Z and from/emailAddress/address eq 'noreply@tm.openai.com'",
        "receivedDateTime ge 1900-01-01T00:00:00Z and startsWith(from/emailAddress/address, 'noreply@tm.openai.com')",
        "receivedDateTime ge 1900-01-01T00:00:00Z and startsWith(from/emailAddress/address, 'noreply')",
    ];

    for Folder in &["junkemail", "inbox"] {
        let BaseUrl = format!(
            "https://graph.microsoft.com/v1.0/me/mailFolders/{}/messages",
            Folder
        );

        for Filter in Filters {
            let Resp = Client.get(&BaseUrl)
                .header("Authorization", format!("Bearer {}", Token))
                .query(&[
                    ("$filter", *Filter),
                    ("$orderby", "receivedDateTime desc"),
                    ("$top", "5"),
                    ("$select", "subject,body,receivedDateTime,from"),
                ])
                .send()
                .await
                .map_err(|E| format!("Graph API request failed: {}", E))?;

            if !Resp.status().is_success() {
                continue;
            }

            let Body: serde_json::Value = Resp.json().await.map_err(|E| E.to_string())?;
            if let Some(Code) = ExtractCodeFromMessages(&Body) {
                return Ok(Some(Code));
            }
        }

        // Last resort: fetch recent messages unfiltered, check sender client-side
        let Resp = Client.get(&BaseUrl)
            .header("Authorization", format!("Bearer {}", Token))
            .query(&[
                ("$orderby", "receivedDateTime desc"),
                ("$top", "25"),
                ("$select", "subject,body,receivedDateTime,from"),
            ])
            .send()
            .await
            .map_err(|E| format!("Graph API request failed: {}", E))?;

        if Resp.status().is_success() {
            let Body: serde_json::Value = Resp.json().await.map_err(|E| E.to_string())?;
            if let Some(Code) = ExtractCodeFromOpenAiMessages(&Body) {
                return Ok(Some(Code));
            }
        }
    }

    Ok(None)
}

fn ExtractCodeFromMessages(Body: &serde_json::Value) -> Option<String> {
    let Messages = Body["value"].as_array()?;
    for Msg in Messages {
        let Content = Msg["body"]["content"].as_str().unwrap_or("");
        if let Some(Code) = ExtractCode(Content) {
            return Some(Code);
        }
    }
    None
}

fn ExtractCodeFromOpenAiMessages(Body: &serde_json::Value) -> Option<String> {
    let Messages = Body["value"].as_array()?;
    for Msg in Messages {
        let Sender = Msg["from"]["emailAddress"]["address"].as_str().unwrap_or("");
        if !Sender.contains("openai.com") {
            continue;
        }
        let Content = Msg["body"]["content"].as_str().unwrap_or("");
        if let Some(Code) = ExtractCode(Content) {
            return Some(Code);
        }
    }
    None
}

static TAG_RE: Lazy<Regex> = Lazy::new(||
    Regex::new(r"<style[^>]*>[\s\S]*?</style>|<script[^>]*>[\s\S]*?</script>|<[^>]+>").unwrap()
);
static SPACE_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"\s+").unwrap());
static CONTEXT_RE: Lazy<Regex> = Lazy::new(||
    Regex::new(r"(?i)(?:code|verify|verification)[^0-9]{0,30}(\d{6})\b").unwrap()
);
static CODE_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"\b(\d{6})\b").unwrap());

fn StripHtmlTags(Html: &str) -> String {
    let Stripped = TAG_RE.replace_all(Html, " ");
    SPACE_RE.replace_all(&Stripped, " ").to_string()
}

fn ExtractCode(Html: &str) -> Option<String> {
    let Text = StripHtmlTags(Html);
    if let Some(Cap) = CONTEXT_RE.captures(&Text) {
        return Some(Cap[1].to_string());
    }
    CODE_RE.captures(&Text).map(|C| C[1].to_string())
}

