use std::{
    fmt,
    net::SocketAddr,
    path::{Path, PathBuf},
};

use cobalt_sdk::{Difficulty, Dimension, Gamemode};
use figment::providers::{Env, Format, Toml};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct ServerConfig {
    pub network: NetworkConfig,
    pub profile: ProfileConfig,
    pub auth: AuthentificationConfig,
}

impl ServerConfig {
    pub fn load<P: AsRef<Path>>(config_file: P) -> Result<Self, figment::Error> {
        figment::Figment::new()
            .merge(Toml::file(config_file))
            .merge(Env::prefixed("SERVER_"))
            .extract()
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            network: NetworkConfig::default(),
            profile: ProfileConfig::default(),
            auth: AuthentificationConfig::default(),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct NetworkConfig {
    pub addr: SocketAddr,
    pub protocol_version: u32,
    pub threshold: u32,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            addr: "127.0.0.1:25565".parse().unwrap(),
            threshold: 256,
            protocol_version: 47,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct ProfileConfig {
    pub name: String,
    pub description: String,
    pub max_players: u32,
    pub icon: Option<PathBuf>,

    pub gamemode: Gamemode,
    pub dimension: Dimension,
    pub difficulty: Difficulty,
}

impl Default for ProfileConfig {
    fn default() -> Self {
        Self {
            name: "Cobalt Server".into(),
            max_players: 20,
            gamemode: Gamemode::Survival,
            dimension: Dimension::Overworld,
            difficulty: Difficulty::Normal,
            description: "".to_string(),
            icon: None,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct AuthentificationConfig {
    pub enabled: bool,
    pub private_key: Option<PathBuf>,
    pub public_key: Option<PathBuf>,
}

impl Default for AuthentificationConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            private_key: None,
            public_key: None,
        }
    }
}

impl fmt::Display for ServerConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Network
        writeln!(f, "  Network")?;
        writeln!(f, "    Address          : {}", self.network.addr)?;
        writeln!(
            f,
            "    Protocol Version : {}",
            self.network.protocol_version
        )?;
        writeln!(f, "    Threshold        : {}", self.network.threshold)?;

        // Profile
        writeln!(f, "  Profile")?;
        writeln!(f, "    Name             : {}", self.profile.name)?;
        writeln!(f, "    Description      : {}", self.profile.description)?;
        writeln!(f, "    Max Players      : {}", self.profile.max_players)?;
        writeln!(f, "    Gamemode         : {:?}", self.profile.gamemode)?;
        writeln!(f, "    Dimension        : {:?}", self.profile.dimension)?;
        writeln!(f, "    Difficulty       : {:?}", self.profile.difficulty)?;
        writeln!(
            f,
            "    Icon             : {}",
            self.profile
                .icon
                .as_ref()
                .and_then(|p| p.file_name())
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| "None".into())
        )?;

        // Authentication
        writeln!(f, "  Authentication")?;
        writeln!(f, "    Enabled          : {}", self.auth.enabled)?;
        writeln!(
            f,
            "    Private Key      : {}",
            self.auth
                .private_key
                .as_ref()
                .and_then(|p| p.file_name())
                .map(|n| format!("***{}", n.to_string_lossy()))
                .unwrap_or_else(|| "None".into())
        )?;
        write!(
            f,
            "    Public Key       : {}",
            self.auth
                .public_key
                .as_ref()
                .and_then(|p| p.file_name())
                .map(|n| format!("***{}", n.to_string_lossy()))
                .unwrap_or_else(|| "None".into())
        )
    }
}
