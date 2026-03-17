use std::time::Duration;
use futures_util::stream::{self, StreamExt};
use tauri::{AppHandle, Emitter, Manager};
use crate::config;
use crate::keychain;
use crate::oauth;
use crate::state::SharedState;
use crate::types::TokenStatus;
use crate::usage;
use crate::utils;

pub fn StartPollingTask(Handle: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let mut Interval = tokio::time::interval(Duration::from_secs(600));
        Interval.tick().await;
        loop {
            Interval.tick().await;
            log::info!("Polling: refreshing all accounts");
            RefreshAllAccounts(&Handle).await;
        }
    });
}

pub async fn RefreshAllAccounts(Handle: &AppHandle) {
    let Ids: Vec<(String, String)> = {
        let State = Handle.state::<SharedState>();
        let Guard = State.lock().unwrap();
        Guard.Accounts.values()
            .map(|M| (M.Id.clone(), M.AccountId.clone()))
            .collect()
    };

    stream::iter(Ids.iter())
        .for_each_concurrent(4, |(Id, AccountId)| async {
            let _ = RefreshSingleAccount(Handle, Id, AccountId).await;
        })
        .await;

    SaveState(Handle);

    let DisplayList = {
        let State = Handle.state::<SharedState>();
        let Guard = State.lock().unwrap();
        Guard.ToDisplayList()
    };

    let _ = Handle.emit("accounts-updated", &DisplayList);
}

pub async fn RefreshSingleAccount(Handle: &AppHandle, Id: &str, AccountId: &str) -> Result<(), String> {
    {
        let State = Handle.state::<SharedState>();
        let mut Guard = State.lock().unwrap();
        Guard.TokenStatus.insert(Id.to_string(), TokenStatus::Refreshing);
    }

    let Tokens = match keychain::LoadTokens(AccountId) {
        Ok(Some(T)) => T,
        Ok(None) => {
            SetStatus(Handle, Id, TokenStatus::Error("No tokens found".to_string()));
            return Err("No tokens found in secure storage".to_string());
        }
        Err(E) => {
            SetStatus(Handle, Id, TokenStatus::Error(E.clone()));
            return Err(format!("Secure storage error: {}", E));
        }
    };

    let AccessToken = if oauth::NeedsRefresh(Tokens.ExpiresAt) {
        match oauth::RefreshAccessToken(&Tokens.RefreshToken).await {
            Ok(NewTokens) => {
                let Token = NewTokens.AccessToken.clone();
                let _ = keychain::StoreTokens(AccountId, &NewTokens);
                Token
            }
            Err(E) => {
                log::warn!("Token refresh failed for {}: {}", AccountId, E);
                SetStatus(Handle, Id, TokenStatus::Expired);
                return Err(format!("Token refresh failed: {}", E));
            }
        }
    } else {
        Tokens.AccessToken
    };

    match usage::FetchUsage(&AccessToken, AccountId).await {
        Ok(Data) => {
            let Now = utils::UnixNow();
            let State = Handle.state::<SharedState>();
            let mut Guard = State.lock().unwrap();

            if Data.PlanType != "unknown" {
                if let Some(Meta) = Guard.Accounts.get_mut(Id) {
                    Meta.PlanType = Data.PlanType.clone();
                }
            }

            Guard.Usage.insert(Id.to_string(), Data);
            Guard.TokenStatus.insert(Id.to_string(), TokenStatus::Active);
            Guard.LastRefreshed.insert(Id.to_string(), Now);
            Ok(())
        }
        Err(E) => {
            log::warn!("Usage fetch failed for {}: {}", AccountId, E);
            SetStatus(Handle, Id, TokenStatus::Error(E.clone()));
            Err(format!("Usage fetch failed: {}", E))
        }
    }
}

fn SetStatus(Handle: &AppHandle, Id: &str, Status: TokenStatus) {
    let State = Handle.state::<SharedState>();
    let mut Guard = State.lock().unwrap();
    Guard.TokenStatus.insert(Id.to_string(), Status);
}

fn SaveState(Handle: &AppHandle) {
    let Metas: Vec<crate::types::AccountMeta> = {
        let State = Handle.state::<SharedState>();
        let Guard = State.lock().unwrap();
        Guard.CollectMetas()
    };
    if let Err(E) = config::SaveConfig(Handle, &Metas) {
        log::warn!("Failed to save config: {}", E);
    }
}
