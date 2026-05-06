use cobalt_net::packet::server::Property;
use reqwest::Client;
use uuid::Uuid;

#[derive(serde::Deserialize)]
pub struct MojangProfile {
    pub id: Uuid,
    pub name: String,
    pub properties: Vec<Property>,
}

pub async fn verify_auth(username: &str, hash: &str) -> Option<MojangProfile> {
    let url = format!(
        "https://sessionserver.mojang.com/session/minecraft/hasJoined?username={}&serverId={}",
        username, hash
    );

    let resp = Client::new().get(&url).send().await.ok()?;
    match resp.status().as_u16() {
        200 => resp.json::<MojangProfile>().await.ok(),
        _ => None,
    }
}

pub async fn fetch_player_properties(uuid: &str) -> Result<Vec<Property>, reqwest::Error> {
    let client = Client::new();
    let url = format!(
        "https://sessionserver.mojang.com/session/minecraft/profile/{}?unsigned=false",
        uuid
    );

    let profile: MojangProfile = client.get(&url).send().await?.json().await?;
    Ok(profile.properties)
}
