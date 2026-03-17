use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountMeta {
    pub Id: String,
    pub Email: String,
    pub AccountId: String,
    pub PlanType: String,
    #[serde(default)]
    pub EmailLink: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenSet {
    pub AccessToken: String,
    pub RefreshToken: String,
    pub ExpiresAt: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageData {
    pub PlanType: String,
    pub PrimaryUsedPercent: f64,
    pub PrimaryResetAt: i64,
    pub PrimaryWindowSeconds: i64,
    pub SecondaryUsedPercent: f64,
    pub SecondaryResetAt: i64,
    pub SecondaryWindowSeconds: i64,
    pub HasCredits: bool,
    pub CreditBalance: f64,
    pub Unlimited: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TokenStatus {
    Active,
    Expired,
    Refreshing,
    Error(String),
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountDisplay {
    pub Id: String,
    pub Email: String,
    pub PlanType: String,
    pub HasPassword: bool,
    pub EmailLink: Option<String>,
    pub Usage: Option<UsageData>,
    pub TokenStatus: TokenStatus,
    pub LastRefreshed: Option<i64>,
}

