use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use serde::Deserialize;

use crate::keychain;
use crate::types::{AccountMeta, TokenSet};

#[derive(Deserialize)]
struct MigrationVault {
    #[serde(default)]
    Tokens: HashMap<String, TokenSet>,
    #[serde(default)]
    Passwords: HashMap<String, String>,
}

#[derive(Deserialize)]
struct MigrationData {
    Accounts: Vec<AccountMeta>,
    Vault: MigrationVault,
}

fn FindMigrationFile() -> Option<PathBuf> {
    let Exe = std::env::current_exe().ok()?;
    let ExeDir = Exe.parent()?;

    // Check next to the exe first
    let P = ExeDir.join("migration.json");
    if P.exists() { return Some(P); }

    // Check project root (two levels up from exe on macOS: target/debug/exe)
    for Ancestor in ExeDir.ancestors().skip(1) {
        let P = Ancestor.join("migration.json");
        if P.exists() { return Some(P); }
    }

    // Check current working directory
    if let Ok(Cwd) = std::env::current_dir() {
        let P = Cwd.join("migration.json");
        if P.exists() { return Some(P); }
    }

    None
}

pub fn TryImport() -> Vec<AccountMeta> {
    let Path = match FindMigrationFile() {
        Some(P) => P,
        None => return Vec::new(),
    };

    log::info!("Found migration.json at {:?}, importing...", Path);

    let Content = match fs::read_to_string(&Path) {
        Ok(C) => C,
        Err(E) => {
            log::error!("Failed to read migration.json: {}", E);
            return Vec::new();
        }
    };

    let Data: MigrationData = match serde_json::from_str(&Content) {
        Ok(D) => D,
        Err(E) => {
            log::error!("Failed to parse migration.json: {}", E);
            return Vec::new();
        }
    };

    let mut Errors = Vec::new();

    for (AcctId, Tokens) in &Data.Vault.Tokens {
        if let Err(E) = keychain::StoreTokens(AcctId, Tokens) {
            Errors.push(format!("tokens for {}: {}", AcctId, E));
        }
    }

    for (AcctId, Password) in &Data.Vault.Passwords {
        if let Err(E) = keychain::StorePassword(AcctId, Password) {
            Errors.push(format!("password for {}: {}", AcctId, E));
        }
    }

    if !Errors.is_empty() {
        log::error!(
            "Migration import incomplete; leaving migration.json in place. Errors: {}",
            Errors.join("; ")
        );
        return Vec::new();
    }

    if let Err(E) = fs::remove_file(&Path) {
        log::warn!("Failed to delete migration.json after import: {}", E);
    } else {
        log::info!("migration.json imported and deleted successfully");
    }

    Data.Accounts
}
