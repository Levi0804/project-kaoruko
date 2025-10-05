use serde::{Deserialize, Deserializer};
use serde_json::Value;
use std::cell::RefCell;
use std::collections::HashMap;

// the defalut implementation is used inside kaoruko_derive
#[derive(Debug, Deserialize, Default)]
pub struct Auth {
    pub id: String,
    pub service: String,
    pub username: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Chatter {
    pub nickname: String,
    #[serde(default)]
    pub auth: Option<Auth>,
    pub peer_id: u64,
    #[serde(skip)]
    pub picture: String,
    pub roles: Vec<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Dictionary {
    pub dictionary: RefCell<Vec<String>>,
    #[serde(skip)]
    pub syllables: HashMap<String, u32>,
    pub sn: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RoomEntry {
    pub beta: serde_json::Value,
    pub chat_mode: String,
    pub game_id: String,
    pub is_public: bool,
    pub name: String,
    pub player_count: u64,
    pub room_code: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RoomDetails {
    pub room_entry: RoomEntry,
    pub scripts: serde_json::Value,
    pub self_peer_id: u64,
    pub self_roles: Vec<String>,
}

impl From<Vec<Value>> for RoomDetails {
    fn from(values: Vec<Value>) -> Self {
        serde_json::from_value::<Self>(values[0][0].clone())
            .expect("failed to deserialize room details")
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewChatter {
    #[serde(deserialize_with = "handle_null")]
    pub auth: Auth,
    pub nickname: String,
    pub peer_id: u64,
}

fn handle_null<'de, D>(deserializer: D) -> Result<Auth, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(Deserialize::deserialize(deserializer).unwrap_or(Auth::default()))
}

impl From<Vec<Value>> for NewChatter {
    fn from(value: Vec<Value>) -> Self {
        serde_json::from_value::<Self>(value[0].clone()).expect("failed to deserialize new chatter")
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Setup {
    pub constants: Constants,
    pub leader_peer_id: u64,
    pub milestone: Milestone,
    // pub players: Vec<idk>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Constants {
    pub max_bomb_duration: u64,
    pub max_players: u64,
    pub max_word_length: u64,
    pub min_bomb_duration: u64,
    pub min_players: u64,
    pub start_timer_duration: u64,
    pub submit_rate_limit: SubmitRateLimit,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubmitRateLimit {
    pub interval: u64,
    pub max: u64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Milestone {
    pub name: String,
    pub rules_locked: bool,
}

#[derive(Debug, Clone)]
pub struct PlayerStats {
    pub nickname: String,
    pub roles: Vec<String>,
    pub words: u64,
    pub subs: u64,
    pub longs: u64,
    pub hyphens: u64,
    pub multi: u64,
    pub lives: u64,
    pub streak: u64,
}

impl PlayerStats {
    pub fn new(nickname: String, roles: Vec<String>) -> Self {
        Self {
            nickname,
            roles,
            words: u64::default(),
            subs: u64::default(),
            longs: u64::default(),
            hyphens: u64::default(),
            multi: u64::default(),
            lives: u64::default(),
            streak: u64::default(),
        }
    }
}
