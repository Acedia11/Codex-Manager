use crate::types::UsageData;
use crate::utils;

const USAGE_URL: &str = "https://chatgpt.com/backend-api/wham/usage";

pub async fn FetchUsage(AccessToken: &str, AccountId: &str) -> Result<UsageData, String> {
    let Resp = utils::HTTP
        .get(USAGE_URL)
        .header("Authorization", format!("Bearer {}", AccessToken))
        .header("ChatGPT-Account-Id", AccountId)
        .send()
        .await
        .map_err(|E| format!("Usage request failed: {}", E))?;

    if !Resp.status().is_success() {
        let Status = Resp.status();
        let Body = Resp.text().await.unwrap_or_default();
        return Err(format!("Usage fetch failed ({}): {}", Status, Body));
    }

    let Body: serde_json::Value = Resp.json().await.map_err(|E| E.to_string())?;

    let PlanType = Body["plan_type"].as_str().unwrap_or("unknown").to_string();

    let Primary = &Body["rate_limit"]["primary_window"];
    let Secondary = &Body["rate_limit"]["secondary_window"];

    let PrimaryUsedPercent = Primary["used_percent"].as_f64().unwrap_or(0.0);
    let PrimaryResetAt = Primary["reset_at"].as_i64().unwrap_or(0);
    let PrimaryWindowSeconds = Primary["limit_window_seconds"].as_i64().unwrap_or(18000);

    let SecondaryUsedPercent = Secondary["used_percent"].as_f64().unwrap_or(0.0);
    let SecondaryResetAt = Secondary["reset_at"].as_i64().unwrap_or(0);
    let SecondaryWindowSeconds = Secondary["limit_window_seconds"].as_i64().unwrap_or(604800);

    let Credits = &Body["credits"];
    let HasCredits = Credits["has_credits"].as_bool().unwrap_or(false);
    let CreditBalance = Credits["balance"].as_f64().unwrap_or(0.0);
    let Unlimited = Credits["unlimited"].as_bool().unwrap_or(false);

    Ok(UsageData {
        PlanType,
        PrimaryUsedPercent,
        PrimaryResetAt,
        PrimaryWindowSeconds,
        SecondaryUsedPercent,
        SecondaryResetAt,
        SecondaryWindowSeconds,
        HasCredits,
        CreditBalance,
        Unlimited,
    })
}
