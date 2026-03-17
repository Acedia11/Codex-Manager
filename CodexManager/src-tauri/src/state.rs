use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use crate::types::{AccountMeta, UsageData, TokenStatus, AccountDisplay, IsMsEmail};
use crate::keychain;

pub struct AppState {
    pub Accounts: HashMap<String, AccountMeta>,
    pub Usage: HashMap<String, UsageData>,
    pub TokenStatus: HashMap<String, TokenStatus>,
    pub LastRefreshed: HashMap<String, i64>,
    pub ProxyRunning: bool,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            Accounts: HashMap::new(),
            Usage: HashMap::new(),
            TokenStatus: HashMap::new(),
            LastRefreshed: HashMap::new(),
            ProxyRunning: false,
        }
    }
}

impl AppState {
    fn BuildDisplay(&self, Meta: &AccountMeta) -> AccountDisplay {
        AccountDisplay {
            Id: Meta.Id.clone(),
            Email: Meta.Email.clone(),
            PlanType: Meta.PlanType.clone(),
            HasPassword: keychain::HasPassword(&Meta.AccountId),
            HasMsLinked: keychain::HasMsTokens(&Meta.AccountId),
            IsMsEmail: IsMsEmail(&Meta.Email),
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

    fn AvailableAccounts(&self) -> impl Iterator<Item = &AccountMeta> {
        self.Accounts.values()
            .filter(|Meta| matches!(self.TokenStatus.get(&Meta.Id), Some(TokenStatus::Active)))
            .filter(|Meta| match self.Usage.get(&Meta.Id) {
                Some(U) => U.SecondaryUsedPercent < 97.0,
                None => true,
            })
    }

    pub fn SelectBestAccount(&self) -> Option<(String, String, String)> {
        self.AvailableAccounts()
            .min_by(|A, B| {
                let Pa = self.Usage.get(&A.Id).map(|U| U.PrimaryUsedPercent).unwrap_or(0.0);
                let Pb = self.Usage.get(&B.Id).map(|U| U.PrimaryUsedPercent).unwrap_or(0.0);
                Pa.partial_cmp(&Pb).unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|Meta| (Meta.Id.clone(), Meta.AccountId.clone(), Meta.Email.clone()))
    }

    pub fn AvailableAccountCount(&self) -> u32 {
        self.AvailableAccounts().count() as u32
    }
}

pub type SharedState = Arc<Mutex<AppState>>;
