use std::path::Path;

use figment::providers::{Format, Toml};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct WhitelistedPlayer {
    pub uuid: String,
    pub name: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct BannedPlayer {
    pub uuid: String,
    pub name: String,
    pub reason: String,
    pub banned_by: String,
    pub date: String, // ou chrono::DateTime<Utc> si tu utilises chrono
}

#[derive(Debug, Deserialize, Serialize)]
pub struct BannedIp {
    pub ip: String,
    pub reason: String,
    pub banned_by: String,
    pub date: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Whitelist {
    pub enabled: bool,
    pub players: Vec<WhitelistedPlayer>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct BannedPlayers {
    pub players: Vec<BannedPlayer>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct BannedIps {
    pub ips: Vec<BannedIp>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AccessControlData {
    pub whitelist: Whitelist,
    pub banned_players: BannedPlayers,
    pub banned_ips: BannedIps,
}

impl AccessControlData {
    pub fn load(path: impl AsRef<Path>) -> figment::Result<Self> {
        figment::Figment::new().merge(Toml::file(path)).extract()
    }
}
