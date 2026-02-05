//! League Client Update (LCU) API integration
//! Connects to the local League of Legends client to fetch match history

use base64::{engine::general_purpose::STANDARD, Engine};
use serde::{Deserialize, Serialize};
use std::process::Command;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum LcuError {
    #[error("League client not running")]
    ClientNotRunning,
    #[error("Failed to parse client info: {0}")]
    ParseError(String),
    #[error("HTTP request failed: {0}")]
    RequestError(#[from] reqwest::Error),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

/// LCU connection credentials
#[derive(Debug, Clone)]
pub struct LcuCredentials {
    pub port: u16,
    pub token: String,
}

/// Connection status response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LcuConnectionStatus {
    pub connected: bool,
    pub summoner_name: Option<String>,
}

/// Match history entry from LCU
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LcuMatchHistoryEntry {
    #[serde(rename = "gameId")]
    pub game_id: i64,
    #[serde(rename = "gameCreation")]
    pub game_creation: i64,
    #[serde(rename = "gameDuration")]
    pub game_duration: i64,
    pub participants: Vec<LcuParticipant>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LcuParticipant {
    #[serde(rename = "championId")]
    pub champion_id: i32,
    pub stats: LcuParticipantStats,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LcuParticipantStats {
    pub win: bool,
}

/// LCU API client
pub struct LcuClient {
    credentials: Option<LcuCredentials>,
    http_client: reqwest::blocking::Client,
}

impl LcuClient {
    pub fn new() -> Self {
        // Create HTTP client that accepts self-signed certificates
        let http_client = reqwest::blocking::Client::builder()
            .danger_accept_invalid_certs(true)
            .build()
            .expect("Failed to create HTTP client");

        Self {
            credentials: None,
            http_client,
        }
    }

    /// Try to connect to the League client
    pub fn connect(&mut self) -> Result<LcuConnectionStatus, LcuError> {
        let credentials = self.get_credentials()?;
        self.credentials = Some(credentials);

        // Test connection by getting current summoner
        match self.get_current_summoner() {
            Ok(summoner) => Ok(LcuConnectionStatus {
                connected: true,
                summoner_name: Some(summoner.display_name),
            }),
            Err(e) => {
                self.credentials = None;
                Err(e)
            }
        }
    }

    /// Get LCU credentials from the running League process
    fn get_credentials(&self) -> Result<LcuCredentials, LcuError> {
        // On Windows, we need to get the command line of LeagueClientUx.exe
        #[cfg(target_os = "windows")]
        {
            let output = Command::new("wmic")
                .args([
                    "process",
                    "where",
                    "name='LeagueClientUx.exe'",
                    "get",
                    "commandline",
                ])
                .output()?;

            let stdout = String::from_utf8_lossy(&output.stdout);

            if stdout.is_empty() || !stdout.contains("LeagueClientUx") {
                return Err(LcuError::ClientNotRunning);
            }

            // Parse port from --app-port=
            let port = Self::extract_value(&stdout, "--app-port=")
                .and_then(|s| s.parse::<u16>().ok())
                .ok_or_else(|| LcuError::ParseError("Could not find port".to_string()))?;

            // Parse auth token from --remoting-auth-token=
            let token = Self::extract_value(&stdout, "--remoting-auth-token=")
                .ok_or_else(|| LcuError::ParseError("Could not find auth token".to_string()))?;

            Ok(LcuCredentials { port, token })
        }

        #[cfg(not(target_os = "windows"))]
        {
            // On macOS/Linux, read from lockfile
            let lockfile_path = dirs::home_dir()
                .map(|p| p.join(".config/riot-games/league-of-legends/lockfile"))
                .ok_or(LcuError::ClientNotRunning)?;

            if !lockfile_path.exists() {
                return Err(LcuError::ClientNotRunning);
            }

            let contents = std::fs::read_to_string(lockfile_path)?;
            let parts: Vec<&str> = contents.split(':').collect();

            if parts.len() < 4 {
                return Err(LcuError::ParseError("Invalid lockfile format".to_string()));
            }

            let port = parts[2]
                .parse::<u16>()
                .map_err(|_| LcuError::ParseError("Invalid port".to_string()))?;
            let token = parts[3].to_string();

            Ok(LcuCredentials { port, token })
        }
    }

    /// Extract a value from command line arguments
    fn extract_value(text: &str, prefix: &str) -> Option<String> {
        text.find(prefix).map(|start| {
            let value_start = start + prefix.len();
            let end = text[value_start..]
                .find(|c: char| c.is_whitespace() || c == '"')
                .map(|i| value_start + i)
                .unwrap_or(text.len());
            text[value_start..end].to_string()
        })
    }

    /// Make an authenticated request to the LCU API
    fn request(&self, endpoint: &str) -> Result<String, LcuError> {
        let creds = self
            .credentials
            .as_ref()
            .ok_or(LcuError::ClientNotRunning)?;

        let url = format!("https://127.0.0.1:{}{}", creds.port, endpoint);
        let auth = STANDARD.encode(format!("riot:{}", creds.token));

        let response = self
            .http_client
            .get(&url)
            .header("Authorization", format!("Basic {}", auth))
            .send()?;

        Ok(response.text()?)
    }

    /// Get current summoner info
    fn get_current_summoner(&self) -> Result<CurrentSummoner, LcuError> {
        let response = self.request("/lol-summoner/v1/current-summoner")?;
        serde_json::from_str(&response)
            .map_err(|e| LcuError::ParseError(format!("Failed to parse summoner: {}", e)))
    }

    /// Get match history
    pub fn get_match_history(&self, count: u32) -> Result<Vec<LcuMatchHistoryEntry>, LcuError> {
        let endpoint = format!(
            "/lol-match-history/v1/products/lol/current-summoner/matches?begIndex=0&endIndex={}",
            count
        );
        let response = self.request(&endpoint)?;

        #[derive(Deserialize)]
        struct MatchHistoryResponse {
            games: MatchHistoryGames,
        }

        #[derive(Deserialize)]
        struct MatchHistoryGames {
            games: Vec<LcuMatchHistoryEntry>,
        }

        let parsed: MatchHistoryResponse = serde_json::from_str(&response)
            .map_err(|e| LcuError::ParseError(format!("Failed to parse match history: {}", e)))?;

        Ok(parsed.games.games)
    }

    /// Check if connected
    pub fn is_connected(&self) -> bool {
        self.credentials.is_some()
    }
}

#[derive(Debug, Clone, Deserialize)]
struct CurrentSummoner {
    #[serde(rename = "displayName")]
    display_name: String,
}

impl Default for LcuClient {
    fn default() -> Self {
        Self::new()
    }
}
