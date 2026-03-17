use std::fs;
use std::path::PathBuf;
use crate::types::AccountMeta;

fn ConfigPath(AppHandle: &tauri::AppHandle) -> Result<PathBuf, String> {
    let Dir = AppHandle.path().app_data_dir().map_err(|E| E.to_string())?;
    fs::create_dir_all(&Dir).map_err(|E| E.to_string())?;
    Ok(Dir.join("accounts.json"))
}

pub fn LoadConfig(AppHandle: &tauri::AppHandle) -> Vec<AccountMeta> {
    let Path = match ConfigPath(AppHandle) {
        Ok(P) => P,
        Err(_) => return Vec::new(),
    };
    if !Path.exists() {
        return Vec::new();
    }
    let Data = match fs::read_to_string(&Path) {
        Ok(D) => D,
        Err(_) => return Vec::new(),
    };
    serde_json::from_str(&Data).unwrap_or_default()
}

pub fn SaveConfig(AppHandle: &tauri::AppHandle, Accounts: &[AccountMeta]) -> Result<(), String> {
    let Path = ConfigPath(AppHandle)?;
    let Json = serde_json::to_string_pretty(Accounts).map_err(|E| E.to_string())?;
    fs::write(&Path, Json).map_err(|E| E.to_string())
}

use tauri::Manager;
