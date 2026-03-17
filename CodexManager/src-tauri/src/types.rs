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
    pub HasMsLinked: bool,
    pub IsMsEmail: bool,
    pub EmailLink: Option<String>,
    pub Usage: Option<UsageData>,
    pub TokenStatus: TokenStatus,
    pub LastRefreshed: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyAccountInfo {
    pub Email: String,
    pub PlanType: String,
    pub PrimaryUsedPercent: f64,
    pub SecondaryUsedPercent: f64,
    pub Active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyStatus {
    pub Running: bool,
    pub Port: u16,
    pub AvailableAccounts: u32,
}

const MS_DOMAINS: &[&str] = &["@hotmail.com", "@outlook.com", "@live.com", "@msn.com"];

pub fn IsMsEmail(Email: &str) -> bool {
    let Lower = Email.to_lowercase();
    MS_DOMAINS.iter().any(|D| Lower.ends_with(D))
}
