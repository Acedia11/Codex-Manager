use keyring::{Entry, Error as KeyringError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::types::TokenSet;

const VAULT_SERVICE: &str = "com.codexmanager.vault";
const VAULT_USER: &str = "codexmanager";

const TOKEN_SERVICE: &str = "com.codexmanager.tokens";
const PASSWORD_SERVICE: &str = "com.codexmanager.passwords";

// Windows Credential Manager caps each blob at ~1280 UTF-16 chars.
// 1200 leaves headroom for encoding variance.
const WIN_CHUNK_MAX_CHARS: usize = 1200;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct Vault {
    #[serde(default)]
    Tokens: HashMap<String, TokenSet>,
    #[serde(default)]
    Passwords: HashMap<String, String>,
}

fn LoadVault() -> Vault {
    match LoadChunked(VAULT_SERVICE, VAULT_USER) {
        Ok(Some(Json)) => serde_json::from_str(&Json).unwrap_or_default(),
        _ => Vault::default(),
    }
}

fn SaveVault(V: &Vault) -> Result<(), String> {
    let Json = serde_json::to_string(V).map_err(|E| E.to_string())?;
    StoreChunked(VAULT_SERVICE, VAULT_USER, &Json)
}

fn ChunkKey(AccountId: &str, Idx: usize) -> String {
    format!("{}#chunk{}", AccountId, Idx)
}

fn StoreChunked(Service: &str, AccountId: &str, Value: &str) -> Result<(), String> {
    if !UsesPerEntryStorage() || Value.len() <= WIN_CHUNK_MAX_CHARS {
        DeleteChunked(Service, AccountId)?;
        let Ent = Entry::new(Service, AccountId).map_err(|E| E.to_string())?;
        return Ent.set_password(Value).map_err(|E| E.to_string());
    }

    // Clean up any previous chunks before writing new ones
    DeleteChunked(Service, AccountId)?;

    let mut Chunks: Vec<&str> = Vec::new();
    let mut Remaining = Value;
    while !Remaining.is_empty() {
        let End = Remaining.char_indices()
            .nth(WIN_CHUNK_MAX_CHARS)
            .map(|(I, _)| I)
            .unwrap_or(Remaining.len());
        Chunks.push(&Remaining[..End]);
        Remaining = &Remaining[End..];
    }

    let Header = format!("$$CHUNKED$$:{}", Chunks.len());
    let Ent = Entry::new(Service, AccountId).map_err(|E| E.to_string())?;
    Ent.set_password(&Header).map_err(|E| E.to_string())?;

    for (I, Chunk) in Chunks.iter().enumerate() {
        let Key = ChunkKey(AccountId, I);
        let Ent = Entry::new(Service, &Key).map_err(|E| E.to_string())?;
        Ent.set_password(Chunk).map_err(|E| E.to_string())?;
    }

    Ok(())
}

fn LoadChunked(Service: &str, AccountId: &str) -> Result<Option<String>, String> {
    let Ent = Entry::new(Service, AccountId).map_err(|E| E.to_string())?;
    let Header = match Ent.get_password() {
        Ok(V) => V,
        Err(KeyringError::NoEntry) => return Ok(None),
        Err(E) => return Err(E.to_string()),
    };

    if !Header.starts_with("$$CHUNKED$$:") {
        return Ok(Some(Header));
    }

    let Count: usize = Header["$$CHUNKED$$:".len()..]
        .parse()
        .map_err(|E: std::num::ParseIntError| E.to_string())?;

    let mut Result = String::new();
    for I in 0..Count {
        let Key = ChunkKey(AccountId, I);
        let Ent = Entry::new(Service, &Key).map_err(|E| E.to_string())?;
        match Ent.get_password() {
            Ok(V) => Result.push_str(&V),
            Err(E) => return Err(format!("missing chunk {}: {}", I, E)),
        }
    }
    Ok(Some(Result))
}

fn DeleteChunked(Service: &str, AccountId: &str) -> Result<(), String> {
    let Ent = Entry::new(Service, AccountId).map_err(|E| E.to_string())?;
    if let Ok(Header) = Ent.get_password() {
        if Header.starts_with("$$CHUNKED$$:") {
            if let Ok(Count) = Header["$$CHUNKED$$:".len()..].parse::<usize>() {
                for I in 0..Count {
                    let Key = ChunkKey(AccountId, I);
                    if let Ok(E) = Entry::new(Service, &Key) {
                        let _ = E.delete_credential();
                    }
                }
            }
        }
    }
    match Ent.delete_credential() {
        Ok(()) | Err(KeyringError::NoEntry) => Ok(()),
        Err(E) => Err(E.to_string()),
    }
}

fn StoreJson<T: Serialize>(Service: &str, AccountId: &str, Value: &T) -> Result<(), String> {
    let Json = serde_json::to_string(Value).map_err(|E| E.to_string())?;
    StoreChunked(Service, AccountId, &Json)
}

fn LoadJson<T: for<'de> Deserialize<'de>>(Service: &str, AccountId: &str) -> Result<Option<T>, String> {
    match LoadChunked(Service, AccountId)? {
        Some(Json) => serde_json::from_str(&Json)
            .map(Some)
            .map_err(|E| E.to_string()),
        None => Ok(None),
    }
}

fn StoreString(Service: &str, AccountId: &str, Value: &str) -> Result<(), String> {
    StoreChunked(Service, AccountId, Value)
}

fn LoadString(Service: &str, AccountId: &str) -> Result<Option<String>, String> {
    LoadChunked(Service, AccountId)
}

fn DeleteEntry(Service: &str, AccountId: &str) -> Result<(), String> {
    DeleteChunked(Service, AccountId)
}

fn DeleteFromVault(Update: impl FnOnce(&mut Vault) -> bool) -> Result<(), String> {
    let mut V = LoadVault();
    if !Update(&mut V) {
        return Ok(());
    }
    SaveVault(&V)
}

fn MergeDeleteResults(Results: &[Result<(), String>]) -> Result<(), String> {
    let Errors: Vec<String> = Results.iter()
        .filter_map(|R| R.as_ref().err().cloned())
        .collect();

    if Errors.is_empty() {
        Ok(())
    } else {
        Err(Errors.join("; "))
    }
}

fn UsesPerEntryStorage() -> bool {
    cfg!(target_os = "windows")
}

pub fn MigrateOldEntries(AccountIds: &[String]) {
    if UsesPerEntryStorage() {
        return;
    }

    let mut V = LoadVault();
    let mut Changed = false;

    for AcctId in AccountIds {
        // Migrate tokens
        if !V.Tokens.contains_key(AcctId) {
            if let Ok(Ent) = Entry::new(TOKEN_SERVICE, AcctId) {
                if let Ok(Json) = Ent.get_password() {
                    if let Ok(Tokens) = serde_json::from_str::<TokenSet>(&Json) {
                        V.Tokens.insert(AcctId.clone(), Tokens);
                        Changed = true;
                        let _ = Ent.delete_credential();
                    }
                }
            }
        }
        // Migrate passwords
        if !V.Passwords.contains_key(AcctId) {
            if let Ok(Ent) = Entry::new(PASSWORD_SERVICE, AcctId) {
                if let Ok(Pw) = Ent.get_password() {
                    V.Passwords.insert(AcctId.clone(), Pw);
                    Changed = true;
                    let _ = Ent.delete_credential();
                }
            }
        }
    }

    if Changed {
        let _ = SaveVault(&V);
        log::info!("Migrated {} old keychain entries to vault", AccountIds.len());
    }
}

pub fn StoreTokens(AccountId: &str, Tokens: &TokenSet) -> Result<(), String> {
    if UsesPerEntryStorage() {
        return StoreJson(TOKEN_SERVICE, AccountId, Tokens);
    }

    let mut V = LoadVault();
    V.Tokens.insert(AccountId.to_string(), Tokens.clone());
    SaveVault(&V)
}

pub fn LoadTokens(AccountId: &str) -> Result<Option<TokenSet>, String> {
    if UsesPerEntryStorage() {
        if let Some(Tokens) = LoadJson(TOKEN_SERVICE, AccountId)? {
            return Ok(Some(Tokens));
        }
        return Ok(LoadVault().Tokens.get(AccountId).cloned());
    }

    if let Some(Tokens) = LoadVault().Tokens.get(AccountId).cloned() {
        return Ok(Some(Tokens));
    }

    LoadJson(TOKEN_SERVICE, AccountId)
}

pub fn DeleteTokens(AccountId: &str) -> Result<(), String> {
    MergeDeleteResults(&[
        DeleteEntry(TOKEN_SERVICE, AccountId),
        DeleteFromVault(|V| V.Tokens.remove(AccountId).is_some()),
    ])
}

pub fn StorePassword(AccountId: &str, Password: &str) -> Result<(), String> {
    if UsesPerEntryStorage() {
        return StoreString(PASSWORD_SERVICE, AccountId, Password);
    }

    let mut V = LoadVault();
    V.Passwords.insert(AccountId.to_string(), Password.to_string());
    SaveVault(&V)
}

pub fn LoadPassword(AccountId: &str) -> Result<Option<String>, String> {
    if UsesPerEntryStorage() {
        if let Some(Password) = LoadString(PASSWORD_SERVICE, AccountId)? {
            return Ok(Some(Password));
        }
        return Ok(LoadVault().Passwords.get(AccountId).cloned());
    }

    if let Some(Password) = LoadVault().Passwords.get(AccountId).cloned() {
        return Ok(Some(Password));
    }

    LoadString(PASSWORD_SERVICE, AccountId)
}

pub fn DeletePassword(AccountId: &str) -> Result<(), String> {
    MergeDeleteResults(&[
        DeleteEntry(PASSWORD_SERVICE, AccountId),
        DeleteFromVault(|V| V.Passwords.remove(AccountId).is_some()),
    ])
}

pub fn HasPassword(AccountId: &str) -> bool {
    LoadPassword(AccountId).ok().flatten().is_some()
}

pub fn BatchCheckAccounts(AccountIds: &[&str]) -> (HashMap<String, bool>, HashMap<String, bool>) {
    let V = LoadVault();
    let mut HasTokens: HashMap<String, bool> = HashMap::new();
    let mut HasPw: HashMap<String, bool> = HashMap::new();

    for &AcctId in AccountIds {
        let TokenFound = if UsesPerEntryStorage() {
            LoadJson::<TokenSet>(TOKEN_SERVICE, AcctId).ok().flatten().is_some()
                || V.Tokens.contains_key(AcctId)
        } else {
            V.Tokens.contains_key(AcctId)
                || LoadJson::<TokenSet>(TOKEN_SERVICE, AcctId).ok().flatten().is_some()
        };

        let PwFound = if UsesPerEntryStorage() {
            LoadString(PASSWORD_SERVICE, AcctId).ok().flatten().is_some()
                || V.Passwords.contains_key(AcctId)
        } else {
            V.Passwords.contains_key(AcctId)
                || LoadString(PASSWORD_SERVICE, AcctId).ok().flatten().is_some()
        };

        HasTokens.insert(AcctId.to_string(), TokenFound);
        HasPw.insert(AcctId.to_string(), PwFound);
    }

    (HasTokens, HasPw)
}

