#![allow(non_snake_case, non_upper_case_globals)]

mod commands;
mod config;
mod keychain;
mod migration;
mod oauth;
mod polling;
mod state;
mod types;
mod utils;
mod usage;

use state::{AppState, SharedState};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_log::Builder::default()
            .level(log::LevelFilter::Info)
            .build())
        .plugin(tauri_plugin_oauth::init())
        .plugin(tauri_plugin_opener::init())
        .manage(Arc::new(Mutex::new(AppState::default())) as SharedState)
        .setup(|App| {
            let Handle = App.handle().clone();

            let mut Metas = config::LoadConfig(&Handle);

            let Imported = migration::TryImport();
            if !Imported.is_empty() {
                let ImportedCount = Imported.len();
                let mut ByAccountId: HashMap<String, types::AccountMeta> = Metas
                    .into_iter()
                    .map(|Meta| (Meta.AccountId.clone(), Meta))
                    .collect();

                for Meta in Imported {
                    ByAccountId.insert(Meta.AccountId.clone(), Meta);
                }

                Metas = ByAccountId.into_values().collect();
                let _ = config::SaveConfig(&Handle, &Metas);
                log::info!("Imported {} accounts from migration.json", ImportedCount);
            }

            let OldAccountIds: Vec<String> = Metas.iter().map(|M| M.AccountId.clone()).collect();
            keychain::MigrateOldEntries(&OldAccountIds);

            {
                let AccountIds: Vec<&str> = Metas.iter().map(|M| M.AccountId.as_str()).collect();
                let (TokenMap, PwMap) = keychain::BatchCheckAccounts(&AccountIds);

                let State = Handle.state::<SharedState>();
                let mut Guard = State.lock().unwrap();
                for Meta in Metas {
                    let HasTokens = TokenMap.get(&Meta.AccountId).copied().unwrap_or(false);
                    let Status = if HasTokens {
                        types::TokenStatus::Active
                    } else {
                        types::TokenStatus::Expired
                    };

                    Guard.TokenStatus.insert(Meta.Id.clone(), Status);
                    Guard.HasPassword.insert(
                        Meta.Id.clone(),
                        PwMap.get(&Meta.AccountId).copied().unwrap_or(false),
                    );
                    Guard.Accounts.insert(Meta.Id.clone(), Meta);
                }
            }

            let PollHandle = Handle.clone();
            tauri::async_runtime::spawn(async move {
                polling::RefreshAllAccounts(&PollHandle).await;
            });

            polling::StartPollingTask(Handle.clone());

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::GetAccounts,
            commands::StartLogin,
            commands::RemoveAccount,
            commands::SetPassword,
            commands::GetPassword,
            commands::RefreshAccount,
            commands::RefreshAll,
            commands::SetEmailLink,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
