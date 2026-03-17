use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use crate::types::{AccountMeta, UsageData, TokenStatus, AccountDisplay};

pub struct AppState {
    pub Accounts: HashMap<String, AccountMeta>,
    pub Usage: HashMap<String, UsageData>,
    pub TokenStatus: HashMap<String, TokenStatus>,
    pub LastRefreshed: HashMap<String, i64>,
    pub HasPassword: HashMap<String, bool>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            Accounts: HashMap::new(),
            Usage: HashMap::new(),
            TokenStatus: HashMap::new(),
            LastRefreshed: HashMap::new(),
            HasPassword: HashMap::new(),
        }
    }
}

impl AppState {
    fn BuildDisplay(&self, Meta: &AccountMeta) -> AccountDisplay {
        AccountDisplay {
            Id: Meta.Id.clone(),
            Email: Meta.Email.clone(),
            PlanType: Meta.PlanType.clone(),
            HasPassword: self.HasPassword.get(&Meta.Id).copied().unwrap_or(false),
            EmailLink: Meta.EmailLink.clone(),
            Usage: self.Usage.get(&Meta.Id).cloned(),
            TokenStatus: self.TokenStatus.get(&Meta.Id).cloned().unwrap_or(TokenStatus::Active),
            LastRefreshed: self.LastRefreshed.get(&Meta.Id).copied(),
        }
    }

    pub fn ToDisplayList(&self) -> Vec<AccountDisplay> {
        let mut List: Vec<AccountDisplay> = self.Accounts.values()
            .map(|Meta| self.BuildDisplay(Meta))
            .collect();
        List.sort_by(|A, B| A.Email.cmp(&B.Email));
        List
    }

    pub fn ToDisplay(&self, Id: &str) -> Option<AccountDisplay> {
        self.Accounts.get(Id).map(|Meta| self.BuildDisplay(Meta))
    }

    pub fn GetAccountId(&self, Id: &str) -> Result<String, String> {
        self.Accounts.get(Id)
            .map(|Meta| Meta.AccountId.clone())
            .ok_or_else(|| "Account not found".to_string())
    }

    pub fn CollectMetas(&self) -> Vec<AccountMeta> {
        self.Accounts.values().cloned().collect()
    }

}

pub type SharedState = Arc<Mutex<AppState>>;
