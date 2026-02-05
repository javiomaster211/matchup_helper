//! Matchup data structures and logic

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A single version of matchup notes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchupVersion {
    pub version: u32,
    pub date: DateTime<Utc>,
    pub notes: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub runes: Vec<String>,
    #[serde(default)]
    pub summoner_spells: Vec<String>,
    #[serde(default)]
    pub items: Vec<String>,
}

/// A matchup between two champions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Matchup {
    pub id: String,
    pub my_champion: String,
    pub enemy_champion: String,
    pub role: String,
    pub versions: Vec<MatchupVersion>,
    pub current_version: u32,
}

impl Matchup {
    /// Create a new matchup with initial empty version
    pub fn new(my_champion: String, enemy_champion: String, role: String) -> Self {
        let id = Uuid::new_v4().to_string();
        let initial_version = MatchupVersion {
            version: 1,
            date: Utc::now(),
            notes: String::new(),
            tags: Vec::new(),
            runes: Vec::new(),
            summoner_spells: Vec::new(),
            items: Vec::new(),
        };

        Self {
            id,
            my_champion,
            enemy_champion,
            role,
            versions: vec![initial_version],
            current_version: 1,
        }
    }

    /// Add a new version with updated data
    pub fn add_version(&mut self, update: MatchupUpdate) {
        let new_version_num = self.versions.len() as u32 + 1;
        let new_version = MatchupVersion {
            version: new_version_num,
            date: Utc::now(),
            notes: update.notes,
            tags: update.tags,
            runes: update.runes,
            summoner_spells: update.summoner_spells,
            items: update.items,
        };

        self.versions.push(new_version);
        self.current_version = new_version_num;
    }

    /// Get the current version
    pub fn current(&self) -> Option<&MatchupVersion> {
        self.versions.get(self.current_version as usize - 1)
    }
}

/// Data for creating a new matchup
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewMatchup {
    pub my_champion: String,
    pub enemy_champion: String,
    pub role: String,
}

/// Data for updating a matchup (creates new version)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchupUpdate {
    pub notes: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub runes: Vec<String>,
    #[serde(default)]
    pub summoner_spells: Vec<String>,
    #[serde(default)]
    pub items: Vec<String>,
}

/// Filter options for querying matchups
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MatchupFilter {
    pub my_champion: Option<String>,
    pub enemy_champion: Option<String>,
    pub role: Option<String>,
    pub tags: Option<Vec<String>>,
    pub search: Option<String>,
}

impl Matchup {
    /// Check if matchup matches the filter
    pub fn matches_filter(&self, filter: &MatchupFilter) -> bool {
        // Filter by my champion
        if let Some(ref champ) = filter.my_champion {
            if !self.my_champion.eq_ignore_ascii_case(champ) {
                return false;
            }
        }

        // Filter by enemy champion
        if let Some(ref champ) = filter.enemy_champion {
            if !self.enemy_champion.eq_ignore_ascii_case(champ) {
                return false;
            }
        }

        // Filter by role
        if let Some(ref role) = filter.role {
            if !self.role.eq_ignore_ascii_case(role) {
                return false;
            }
        }

        // Filter by tags (must have all specified tags)
        if let Some(ref filter_tags) = filter.tags {
            if let Some(current) = self.current() {
                for tag in filter_tags {
                    if !current.tags.iter().any(|t| t.eq_ignore_ascii_case(tag)) {
                        return false;
                    }
                }
            } else {
                return false;
            }
        }

        // Search in notes and champion names
        if let Some(ref search) = filter.search {
            let search_lower = search.to_lowercase();
            let my_champ_match = self.my_champion.to_lowercase().contains(&search_lower);
            let enemy_champ_match = self.enemy_champion.to_lowercase().contains(&search_lower);
            let notes_match = self
                .current()
                .map(|v| v.notes.to_lowercase().contains(&search_lower))
                .unwrap_or(false);

            if !my_champ_match && !enemy_champ_match && !notes_match {
                return false;
            }
        }

        true
    }
}

/// A single match from game history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Match {
    pub id: String,
    pub game_id: Option<String>,
    pub date: DateTime<Utc>,
    pub my_champion: String,
    pub enemy_champion: String,
    pub role: String,
    pub result: MatchResult,
    pub notes: String,
    pub linked_matchup: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum MatchResult {
    Win,
    Loss,
}

impl Match {
    pub fn new(
        my_champion: String,
        enemy_champion: String,
        role: String,
        result: MatchResult,
        game_id: Option<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            game_id,
            date: Utc::now(),
            my_champion,
            enemy_champion,
            role,
            result,
            notes: String::new(),
            linked_matchup: None,
        }
    }
}

/// Update data for a match
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchUpdate {
    pub notes: Option<String>,
    pub linked_matchup: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_matchup() {
        let matchup = Matchup::new(
            "Darius".to_string(),
            "Garen".to_string(),
            "top".to_string(),
        );

        assert_eq!(matchup.my_champion, "Darius");
        assert_eq!(matchup.enemy_champion, "Garen");
        assert_eq!(matchup.role, "top");
        assert_eq!(matchup.versions.len(), 1);
        assert_eq!(matchup.current_version, 1);
    }

    #[test]
    fn test_add_version() {
        let mut matchup = Matchup::new(
            "Darius".to_string(),
            "Garen".to_string(),
            "top".to_string(),
        );

        matchup.add_version(MatchupUpdate {
            notes: "Test notes".to_string(),
            tags: vec!["easy".to_string()],
            runes: vec![],
            summoner_spells: vec![],
            items: vec![],
        });

        assert_eq!(matchup.versions.len(), 2);
        assert_eq!(matchup.current_version, 2);
        assert_eq!(matchup.current().unwrap().notes, "Test notes");
    }

    #[test]
    fn test_filter() {
        let matchup = Matchup::new(
            "Darius".to_string(),
            "Garen".to_string(),
            "top".to_string(),
        );

        let filter = MatchupFilter {
            my_champion: Some("Darius".to_string()),
            ..Default::default()
        };

        assert!(matchup.matches_filter(&filter));

        let filter2 = MatchupFilter {
            my_champion: Some("Garen".to_string()),
            ..Default::default()
        };

        assert!(!matchup.matches_filter(&filter2));
    }
}
