use reqwest::Client;

#[derive(serde::Deserialize)]
pub struct MojangProfile {
    pub id: String,
    pub name: String,
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
