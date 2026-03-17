use tauri::{AppHandle, Manager};
use crate::config;
use crate::keychain;
use crate::microsoft;
use crate::oauth;
use crate::polling;
use crate::proxy;
use crate::state::SharedState;
use crate::types::{AccountDisplay, AccountMeta, ProxyStatus, TokenStatus};

fn SaveAllConfig(AppHandle: &AppHandle) -> Result<(), String> {
    let Metas: Vec<AccountMeta> = {
        let State = AppHandle.state::<SharedState>();
        let Guard = State.lock().unwrap();
        Guard.CollectMetas()
    };
    config::SaveConfig(AppHandle, &Metas)
}

#[tauri::command]
pub fn GetAccounts(StateHandle: tauri::State<'_, SharedState>) -> Vec<AccountDisplay> {
    let Guard = StateHandle.lock().unwrap();
    Guard.ToDisplayList()
}

#[tauri::command]
pub async fn StartLogin(AppHandle: AppHandle) -> Result<AccountDisplay, String> {
    let (Tokens, AccountId, Email, PlanType) = oauth::Login().await?;

    let Id = uuid::Uuid::new_v4().to_string();

    keychain::StoreTokens(&AccountId, &Tokens)?;

    let Meta = AccountMeta {
        Id: Id.clone(),
        Email,
        AccountId: AccountId.clone(),
        PlanType,
        EmailLink: None,
    };

    {
        let State = AppHandle.state::<SharedState>();
        let mut Guard = State.lock().unwrap();
        Guard.Accounts.insert(Id.clone(), Meta.clone());
        Guard.TokenStatus.insert(Id.clone(), TokenStatus::Active);
    }

    SaveAllConfig(&AppHandle)?;

    let _ = polling::RefreshSingleAccount(&AppHandle, &Id, &AccountId).await;

    let _ = SaveAllConfig(&AppHandle);

    let State = AppHandle.state::<SharedState>();
    let Guard = State.lock().unwrap();
    Guard.ToDisplay(&Id).ok_or_else(|| "Account not found after creation".to_string())
}

#[tauri::command]
pub async fn RemoveAccount(AppHandle: AppHandle, Id: String) -> Result<(), String> {
    let AccountId = {
        let State = AppHandle.state::<SharedState>();
        let mut Guard = State.lock().unwrap();
        let Meta = Guard.Accounts.remove(&Id)
            .ok_or_else(|| "Account not found".to_string())?;
        Guard.Usage.remove(&Id);
        Guard.TokenStatus.remove(&Id);
        Guard.LastRefreshed.remove(&Id);
        Meta.AccountId
    };

    let _ = keychain::DeleteTokens(&AccountId);
    let _ = keychain::DeletePassword(&AccountId);
    let _ = keychain::DeleteMsTokens(&AccountId);

    SaveAllConfig(&AppHandle)?;

    Ok(())
}

#[tauri::command]
pub fn SetPassword(StateHandle: tauri::State<'_, SharedState>, Id: String, Password: String) -> Result<(), String> {
    let Guard = StateHandle.lock().unwrap();
    let AccountId = Guard.GetAccountId(&Id)?;
    keychain::StorePassword(&AccountId, &Password)
}

#[tauri::command]
pub fn GetPassword(StateHandle: tauri::State<'_, SharedState>, Id: String) -> Result<Option<String>, String> {
    let Guard = StateHandle.lock().unwrap();
    let AccountId = Guard.GetAccountId(&Id)?;
    keychain::LoadPassword(&AccountId)
}

#[tauri::command]
pub async fn RefreshAccount(AppHandle: AppHandle, Id: String) -> Result<AccountDisplay, String> {
    let AccountId = {
        let State = AppHandle.state::<SharedState>();
        let Guard = State.lock().unwrap();
        Guard.GetAccountId(&Id)?
    };

    if let Err(E) = polling::RefreshSingleAccount(&AppHandle, &Id, &AccountId).await {
        log::warn!("Refresh failed for {}: {}", Id, E);
    }

    let State = AppHandle.state::<SharedState>();
    let Guard = State.lock().unwrap();
    Guard.ToDisplay(&Id).ok_or_else(|| "Account not found".to_string())
}

#[tauri::command]
pub async fn RefreshAll(AppHandle: AppHandle) -> Result<Vec<AccountDisplay>, String> {
    polling::RefreshAllAccounts(&AppHandle).await;

    let _ = SaveAllConfig(&AppHandle);

    let State = AppHandle.state::<SharedState>();
    let Guard = State.lock().unwrap();
    Ok(Guard.ToDisplayList())
}

#[tauri::command]
pub fn SetEmailLink(AppHandle: AppHandle, Id: String, Link: String) -> Result<(), String> {
    let State = AppHandle.state::<SharedState>();
    let mut Guard = State.lock().unwrap();
    let Meta = Guard.Accounts.get_mut(&Id)
        .ok_or_else(|| "Account not found".to_string())?;
    Meta.EmailLink = Some(Link);
    let Metas: Vec<_> = Guard.CollectMetas();
    drop(Guard);
    config::SaveConfig(&AppHandle, &Metas)
}

#[tauri::command]
pub async fn LinkHotmail(AppHandle: AppHandle, Id: String) -> Result<AccountDisplay, String> {
    let AccountId = {
        let State = AppHandle.state::<SharedState>();
        let Guard = State.lock().unwrap();
        Guard.GetAccountId(&Id)?
    };

    let Tokens = microsoft::LinkAccount().await?;
    keychain::StoreMsTokens(&AccountId, &Tokens)?;

    let State = AppHandle.state::<SharedState>();
    let Guard = State.lock().unwrap();
    Guard.ToDisplay(&Id).ok_or_else(|| "Account not found".to_string())
}

#[tauri::command]
pub async fn FetchVerificationCode(AppHandle: AppHandle, Id: String) -> Result<Option<String>, String> {
    let AccountId = {
        let State = AppHandle.state::<SharedState>();
        let Guard = State.lock().unwrap();
        Guard.GetAccountId(&Id)?
    };

    let Result = microsoft::FetchVerificationCode(&AccountId).await?;
    if let Some(ref Code) = Result {
        if let Ok(mut Clipboard) = arboard::Clipboard::new() {
            let _ = Clipboard.set_text(Code);
        }
    }
    Ok(Result)
}

#[tauri::command]
pub fn GetProxyStatus(StateHandle: tauri::State<'_, SharedState>) -> ProxyStatus {
    let Guard = StateHandle.lock().unwrap();
    ProxyStatus {
        Running: Guard.ProxyRunning,
        Port: proxy::PROXY_PORT,
        AvailableAccounts: Guard.AvailableAccountCount(),
    }
}
