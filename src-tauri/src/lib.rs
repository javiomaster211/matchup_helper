//! MatchupHelper - Tauri commands and application logic

mod lcu;
mod matchup;
mod storage;

use lcu::{LcuClient, LcuConnectionStatus};
use matchup::{Match, MatchResult, MatchUpdate, Matchup, MatchupFilter, MatchupUpdate, NewMatchup};
use std::sync::Mutex;
use storage::Storage;
use tauri::State;

/// Application state
pub struct AppState {
    storage: Mutex<Storage>,
    lcu_client: Mutex<LcuClient>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            storage: Mutex::new(Storage::new().expect("Failed to initialize storage")),
            lcu_client: Mutex::new(LcuClient::new()),
        }
    }
}

// ==================== Matchup Commands ====================

/// Get all matchups, optionally filtered
#[tauri::command]
fn get_matchups(
    filter: Option<MatchupFilter>,
    state: State<AppState>,
) -> Result<Vec<Matchup>, String> {
    let storage = state.storage.lock().map_err(|e| e.to_string())?;
    let data = storage.load().map_err(|e| e.to_string())?;

    let matchups: Vec<Matchup> = if let Some(filter) = filter {
        data.matchups
            .values()
            .filter(|m| m.matches_filter(&filter))
            .cloned()
            .collect()
    } else {
        data.matchups.values().cloned().collect()
    };

    Ok(matchups)
}

/// Get a single matchup by ID
#[tauri::command]
fn get_matchup(id: String, state: State<AppState>) -> Result<Matchup, String> {
    let storage = state.storage.lock().map_err(|e| e.to_string())?;
    let data = storage.load().map_err(|e| e.to_string())?;

    data.matchups
        .get(&id)
        .cloned()
        .ok_or_else(|| "Matchup not found".to_string())
}

/// Create a new matchup
#[tauri::command]
fn create_matchup(matchup: NewMatchup, state: State<AppState>) -> Result<Matchup, String> {
    let storage = state.storage.lock().map_err(|e| e.to_string())?;
    let mut data = storage.load().map_err(|e| e.to_string())?;

    let new_matchup = Matchup::new(matchup.my_champion, matchup.enemy_champion, matchup.role);

    data.matchups.insert(new_matchup.id.clone(), new_matchup.clone());
    storage.save(&data).map_err(|e| e.to_string())?;

    Ok(new_matchup)
}

/// Update a matchup (creates a new version)
#[tauri::command]
fn update_matchup(
    id: String,
    update: MatchupUpdate,
    state: State<AppState>,
) -> Result<Matchup, String> {
    let storage = state.storage.lock().map_err(|e| e.to_string())?;
    let mut data = storage.load().map_err(|e| e.to_string())?;

    let matchup = data
        .matchups
        .get_mut(&id)
        .ok_or_else(|| "Matchup not found".to_string())?;

    matchup.add_version(update);

    let updated = matchup.clone();
    storage.save(&data).map_err(|e| e.to_string())?;

    Ok(updated)
}

/// Delete a matchup
#[tauri::command]
fn delete_matchup(id: String, state: State<AppState>) -> Result<(), String> {
    let storage = state.storage.lock().map_err(|e| e.to_string())?;
    let mut data = storage.load().map_err(|e| e.to_string())?;

    data.matchups
        .remove(&id)
        .ok_or_else(|| "Matchup not found".to_string())?;

    storage.save(&data).map_err(|e| e.to_string())?;

    Ok(())
}

/// Search matchups by query string
#[tauri::command]
fn search_matchups(query: String, state: State<AppState>) -> Result<Vec<Matchup>, String> {
    let filter = MatchupFilter {
        search: Some(query),
        ..Default::default()
    };

    get_matchups(Some(filter), state)
}

// ==================== Match History Commands ====================

/// Get all matches
#[tauri::command]
fn get_matches(state: State<AppState>) -> Result<Vec<Match>, String> {
    let storage = state.storage.lock().map_err(|e| e.to_string())?;
    let data = storage.load().map_err(|e| e.to_string())?;

    let mut matches: Vec<Match> = data.matches.values().cloned().collect();
    matches.sort_by(|a, b| b.date.cmp(&a.date));

    Ok(matches)
}

/// Update a match
#[tauri::command]
fn update_match(id: String, update: MatchUpdate, state: State<AppState>) -> Result<Match, String> {
    let storage = state.storage.lock().map_err(|e| e.to_string())?;
    let mut data = storage.load().map_err(|e| e.to_string())?;

    let match_entry = data
        .matches
        .get_mut(&id)
        .ok_or_else(|| "Match not found".to_string())?;

    if let Some(notes) = update.notes {
        match_entry.notes = notes;
    }
    if let Some(linked) = update.linked_matchup {
        match_entry.linked_matchup = if linked.is_empty() {
            None
        } else {
            Some(linked)
        };
    }

    let updated = match_entry.clone();
    storage.save(&data).map_err(|e| e.to_string())?;

    Ok(updated)
}

// ==================== LCU Commands ====================

/// Connect to the League Client
#[tauri::command]
fn connect_lcu(state: State<AppState>) -> Result<LcuConnectionStatus, String> {
    let mut client = state.lcu_client.lock().map_err(|e| e.to_string())?;
    client.connect().map_err(|e| e.to_string())
}

/// Import recent matches from the League Client
#[tauri::command]
fn import_matches(count: Option<u32>, state: State<AppState>) -> Result<Vec<Match>, String> {
    let client = state.lcu_client.lock().map_err(|e| e.to_string())?;

    if !client.is_connected() {
        return Err("Not connected to League client".to_string());
    }

    let lcu_matches = client
        .get_match_history(count.unwrap_or(20))
        .map_err(|e| e.to_string())?;

    let storage = state.storage.lock().map_err(|e| e.to_string())?;
    let mut data = storage.load().map_err(|e| e.to_string())?;

    let mut imported = Vec::new();

    for lcu_match in lcu_matches {
        let game_id = lcu_match.game_id.to_string();

        // Skip if already imported
        if data
            .matches
            .values()
            .any(|m| m.game_id.as_ref() == Some(&game_id))
        {
            continue;
        }

        // Get participant info (simplified - in real implementation would need more logic)
        if let Some(participant) = lcu_match.participants.first() {
            let result = if participant.stats.win {
                MatchResult::Win
            } else {
                MatchResult::Loss
            };

            // Note: Champion ID to name mapping would need Data Dragon
            let new_match = Match::new(
                format!("Champion{}", participant.champion_id),
                "Unknown".to_string(),
                "unknown".to_string(),
                result,
                Some(game_id),
            );

            data.matches.insert(new_match.id.clone(), new_match.clone());
            imported.push(new_match);
        }
    }

    storage.save(&data).map_err(|e| e.to_string())?;

    Ok(imported)
}

// ==================== Application Entry Point ====================

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            get_matchups,
            get_matchup,
            create_matchup,
            update_matchup,
            delete_matchup,
            search_matchups,
            get_matches,
            update_match,
            connect_lcu,
            import_matches,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
