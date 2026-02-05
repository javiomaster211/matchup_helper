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
    #[error("API error: {0}")]
    ApiError(String),
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

/// Processed match data from LCU
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LcuMatchData {
    pub game_id: i64,
    pub game_creation: i64,
    pub my_champion_id: i32,
    pub my_champion_name: String,
    pub enemy_champion_id: Option<i32>,
    pub enemy_champion_name: Option<String>,
    pub role: String,
    pub lane: String,
    pub win: bool,
    pub queue_id: i32,
}

/// LCU API client
pub struct LcuClient {
    credentials: Option<LcuCredentials>,
    http_client: reqwest::blocking::Client,
    summoner_puuid: Option<String>,
}

impl LcuClient {
    pub fn new() -> Self {
        let http_client = reqwest::blocking::Client::builder()
            .danger_accept_invalid_certs(true)
            .build()
            .expect("Failed to create HTTP client");

        Self {
            credentials: None,
            http_client,
            summoner_puuid: None,
        }
    }

    /// Try to connect to the League client
    pub fn connect(&mut self) -> Result<LcuConnectionStatus, LcuError> {
        let credentials = self.get_credentials()?;
        self.credentials = Some(credentials);

        match self.get_current_summoner() {
            Ok(summoner) => {
                self.summoner_puuid = Some(summoner.puuid.clone());
                Ok(LcuConnectionStatus {
                    connected: true,
                    summoner_name: Some(summoner.display_name),
                })
            }
            Err(e) => {
                self.credentials = None;
                Err(e)
            }
        }
    }

    /// Get LCU credentials from the running League process
    fn get_credentials(&self) -> Result<LcuCredentials, LcuError> {
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

            let port = Self::extract_value(&stdout, "--app-port=")
                .and_then(|s| s.parse::<u16>().ok())
                .ok_or_else(|| LcuError::ParseError("Could not find port".to_string()))?;

            let token = Self::extract_value(&stdout, "--remoting-auth-token=")
                .ok_or_else(|| LcuError::ParseError("Could not find auth token".to_string()))?;

            Ok(LcuCredentials { port, token })
        }

        #[cfg(not(target_os = "windows"))]
        {
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

        let status = response.status();
        let text = response.text()?;

        if !status.is_success() {
            return Err(LcuError::ApiError(format!(
                "HTTP {}: {}",
                status,
                text.chars().take(200).collect::<String>()
            )));
        }

        Ok(text)
    }

    /// Get current summoner info
    fn get_current_summoner(&self) -> Result<CurrentSummoner, LcuError> {
        let response = self.request("/lol-summoner/v1/current-summoner")?;
        serde_json::from_str(&response)
            .map_err(|e| LcuError::ParseError(format!("Failed to parse summoner: {} - Response: {}", e, &response[..200.min(response.len())])))
    }

    /// Get match history with proper parsing
    pub fn get_match_history(&self, count: u32) -> Result<Vec<LcuMatchData>, LcuError> {
        let endpoint = format!(
            "/lol-match-history/v1/products/lol/current-summoner/matches?begIndex=0&endIndex={}",
            count
        );
        let response = self.request(&endpoint)?;

        // Parse the response - LCU returns nested structure
        let parsed: serde_json::Value = serde_json::from_str(&response)
            .map_err(|e| LcuError::ParseError(format!("JSON parse error: {}", e)))?;

        let games = parsed
            .get("games")
            .and_then(|g| g.get("games"))
            .and_then(|g| g.as_array())
            .ok_or_else(|| LcuError::ParseError(format!(
                "Unexpected response structure. Keys: {:?}",
                parsed.as_object().map(|o| o.keys().collect::<Vec<_>>())
            )))?;

        let puuid = self.summoner_puuid.as_ref()
            .ok_or_else(|| LcuError::ParseError("No summoner PUUID".to_string()))?;

        let mut matches = Vec::new();

        for game in games {
            if let Some(match_data) = self.parse_game(game, puuid) {
                matches.push(match_data);
            }
        }

        Ok(matches)
    }

    /// Parse a single game from match history
    fn parse_game(&self, game: &serde_json::Value, puuid: &str) -> Option<LcuMatchData> {
        let game_id = game.get("gameId")?.as_i64()?;
        let game_creation = game.get("gameCreation")?.as_i64()?;
        let queue_id = game.get("queueId")?.as_i64()? as i32;

        // Find our participant
        let participants = game.get("participants")?.as_array()?;
        let participant_identities = game.get("participantIdentities")?.as_array()?;

        // Find our participant ID
        let mut my_participant_id = None;
        for identity in participant_identities {
            let player = identity.get("player")?;
            let player_puuid = player.get("puuid").and_then(|p| p.as_str());
            if player_puuid == Some(puuid) {
                my_participant_id = identity.get("participantId")?.as_i64();
                break;
            }
        }

        let my_participant_id = my_participant_id?;

        // Find our participant data
        let my_participant = participants.iter().find(|p| {
            p.get("participantId").and_then(|id| id.as_i64()) == Some(my_participant_id)
        })?;

        let my_champion_id = my_participant.get("championId")?.as_i64()? as i32;
        let my_team_id = my_participant.get("teamId")?.as_i64()?;
        let stats = my_participant.get("stats")?;
        let win = stats.get("win")?.as_bool()?;

        let timeline = my_participant.get("timeline");
        let role = timeline
            .and_then(|t| t.get("role"))
            .and_then(|r| r.as_str())
            .unwrap_or("NONE")
            .to_string();
        let lane = timeline
            .and_then(|t| t.get("lane"))
            .and_then(|l| l.as_str())
            .unwrap_or("NONE")
            .to_string();

        // Find enemy laner (same lane, different team)
        let mut enemy_champion_id = None;
        for participant in participants {
            let team_id = participant.get("teamId").and_then(|t| t.as_i64());
            if team_id != Some(my_team_id) {
                let enemy_timeline = participant.get("timeline");
                let enemy_lane = enemy_timeline
                    .and_then(|t| t.get("lane"))
                    .and_then(|l| l.as_str());

                if enemy_lane == Some(&lane) || lane == "NONE" {
                    enemy_champion_id = participant.get("championId").and_then(|c| c.as_i64());
                    break;
                }
            }
        }

        Some(LcuMatchData {
            game_id,
            game_creation,
            my_champion_id,
            my_champion_name: champion_id_to_name(my_champion_id),
            enemy_champion_id: enemy_champion_id.map(|id| id as i32),
            enemy_champion_name: enemy_champion_id.map(|id| champion_id_to_name(id as i32)),
            role: normalize_role(&role, &lane),
            lane,
            win,
            queue_id,
        })
    }

    /// Debug: get raw API response
    pub fn debug_endpoint(&self, endpoint: &str) -> Result<String, LcuError> {
        self.request(endpoint)
    }

    pub fn is_connected(&self) -> bool {
        self.credentials.is_some()
    }
}

#[derive(Debug, Clone, Deserialize)]
struct CurrentSummoner {
    #[serde(rename = "displayName")]
    display_name: String,
    puuid: String,
}

impl Default for LcuClient {
    fn default() -> Self {
        Self::new()
    }
}

/// Convert champion ID to name (basic mapping for common champions)
fn champion_id_to_name(id: i32) -> String {
    match id {
        1 => "Annie",
        2 => "Olaf",
        3 => "Galio",
        4 => "TwistedFate",
        5 => "XinZhao",
        6 => "Urgot",
        7 => "LeBlanc",
        8 => "Vladimir",
        9 => "Fiddlesticks",
        10 => "Kayle",
        11 => "MasterYi",
        12 => "Alistar",
        13 => "Ryze",
        14 => "Sion",
        15 => "Sivir",
        16 => "Soraka",
        17 => "Teemo",
        18 => "Tristana",
        19 => "Warwick",
        20 => "Nunu",
        21 => "MissFortune",
        22 => "Ashe",
        23 => "Tryndamere",
        24 => "Jax",
        25 => "Morgana",
        26 => "Zilean",
        27 => "Singed",
        28 => "Evelynn",
        29 => "Twitch",
        30 => "Karthus",
        31 => "Chogath",
        32 => "Amumu",
        33 => "Rammus",
        34 => "Anivia",
        35 => "Shaco",
        36 => "DrMundo",
        37 => "Sona",
        38 => "Kassadin",
        39 => "Irelia",
        40 => "Janna",
        41 => "Gangplank",
        42 => "Corki",
        43 => "Karma",
        44 => "Taric",
        45 => "Veigar",
        48 => "Trundle",
        50 => "Swain",
        51 => "Caitlyn",
        53 => "Blitzcrank",
        54 => "Malphite",
        55 => "Katarina",
        56 => "Nocturne",
        57 => "Maokai",
        58 => "Renekton",
        59 => "JarvanIV",
        60 => "Elise",
        61 => "Orianna",
        62 => "Wukong",
        63 => "Brand",
        64 => "LeeSin",
        67 => "Vayne",
        68 => "Rumble",
        69 => "Cassiopeia",
        72 => "Skarner",
        74 => "Heimerdinger",
        75 => "Nasus",
        76 => "Nidalee",
        77 => "Udyr",
        78 => "Poppy",
        79 => "Gragas",
        80 => "Pantheon",
        81 => "Ezreal",
        82 => "Mordekaiser",
        83 => "Yorick",
        84 => "Akali",
        85 => "Kennen",
        86 => "Garen",
        89 => "Leona",
        90 => "Malzahar",
        91 => "Talon",
        92 => "Riven",
        96 => "KogMaw",
        98 => "Shen",
        99 => "Lux",
        101 => "Xerath",
        102 => "Shyvana",
        103 => "Ahri",
        104 => "Graves",
        105 => "Fizz",
        106 => "Volibear",
        107 => "Rengar",
        110 => "Varus",
        111 => "Nautilus",
        112 => "Viktor",
        113 => "Sejuani",
        114 => "Fiora",
        115 => "Ziggs",
        117 => "Lulu",
        119 => "Draven",
        120 => "Hecarim",
        121 => "Khazix",
        122 => "Darius",
        126 => "Jayce",
        127 => "Lissandra",
        131 => "Diana",
        133 => "Quinn",
        134 => "Syndra",
        136 => "AurelionSol",
        141 => "Kayn",
        142 => "Zoe",
        143 => "Zyra",
        145 => "Kaisa",
        147 => "Seraphine",
        150 => "Gnar",
        154 => "Zac",
        157 => "Yasuo",
        161 => "Velkoz",
        163 => "Taliyah",
        164 => "Camille",
        166 => "Akshan",
        200 => "Belveth",
        201 => "Braum",
        202 => "Jhin",
        203 => "Kindred",
        221 => "Zeri",
        222 => "Jinx",
        223 => "TahmKench",
        233 => "Briar",
        234 => "Viego",
        235 => "Senna",
        236 => "Lucian",
        238 => "Zed",
        240 => "Kled",
        245 => "Ekko",
        246 => "Qiyana",
        254 => "Vi",
        266 => "Aatrox",
        267 => "Nami",
        268 => "Azir",
        350 => "Yuumi",
        360 => "Samira",
        412 => "Thresh",
        420 => "Illaoi",
        421 => "RekSai",
        427 => "Ivern",
        429 => "Kalista",
        432 => "Bard",
        497 => "Rakan",
        498 => "Xayah",
        516 => "Ornn",
        517 => "Sylas",
        518 => "Neeko",
        523 => "Aphelios",
        526 => "Rell",
        555 => "Pyke",
        711 => "Vex",
        777 => "Yone",
        799 => "Ambessa",
        875 => "Sett",
        876 => "Lillia",
        887 => "Gwen",
        888 => "Renata",
        893 => "Aurora",
        895 => "Nilah",
        897 => "KSante",
        901 => "Smolder",
        902 => "Milio",
        910 => "Hwei",
        950 => "Naafiri",
        _ => return format!("Champion{}", id),
    }
    .to_string()
}

/// Normalize role from LCU format to our format
fn normalize_role(role: &str, lane: &str) -> String {
    match lane.to_uppercase().as_str() {
        "TOP" => "top".to_string(),
        "JUNGLE" => "jungle".to_string(),
        "MIDDLE" | "MID" => "mid".to_string(),
        "BOTTOM" | "BOT" => {
            if role.to_uppercase() == "CARRY" || role.to_uppercase() == "DUO_CARRY" {
                "adc".to_string()
            } else {
                "support".to_string()
            }
        }
        _ => lane.to_lowercase(),
    }
}
