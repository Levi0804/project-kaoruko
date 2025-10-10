use anyhow::anyhow;
use rand::Rng;
use reqwest::{header::CONTENT_TYPE, Client};
use rust_socketio::Payload;
use serde_json::{json, Value};
use std::cell::RefCell;

pub fn create_user_token() -> anyhow::Result<String> {
    let mut rng = rand::rng();
    let token = (0..16)
        .map(|_| rng.sample(rand::distr::Alphabetic) as u8)
        .collect::<Vec<_>>();
    Ok(String::from_utf8(token)?.to_lowercase())
}

pub async fn start_new_room(
    room_name: Option<&str>,
    is_public: bool,
    bot_token: &str,
) -> anyhow::Result<(String, String)> {
    let response = Client::new()
        .post(env!("START_ROOM"))
        .header(CONTENT_TYPE, "application/json")
        .json(&json!({
            "name": room_name.unwrap_or("kaoruko ✨"),
            "isPublic": is_public,
            "gameId": "bombparty",
            "creatorUserToken": bot_token,
        }))
        .send()
        .await?;
    let url = serde_json::from_str::<serde_json::Value>(&response.text().await?)?;
    let Value::String(code) = url["roomCode"].clone() else {
        return Err(anyhow!("Unable to get room code"));
    };
    Ok((join_room(code.as_str()).await?, code))
}

pub async fn join_room(room_code: &str) -> anyhow::Result<String> {
    let response = Client::new()
        .post(env!("JOIN_ROOM"))
        .header(CONTENT_TYPE, "application/json")
        .json(&json!( { "roomCode": room_code } ))
        .send()
        .await
        .map_err(|err| anyhow!("request failed: {err}"))?
        .text()
        .await
        .map_err(|err| anyhow!("no response from jklm: {err}"))?;
    let url = serde_json::from_str::<Value>(&response)
        .map_err(|err| anyhow!("failed to deserialize value: {err}"))?;
    let Value::String(url) = url["url"].clone() else {
        return Err(anyhow!("Unable to join with the provided room code"));
    };
    Ok(url)
}

pub fn text_payload(payload: Payload) -> Vec<Value> {
    if let Payload::Text(values) = payload {
        values
    } else {
        unimplemented!("unhandled payload");
    }
}

pub fn shuffle(words: &mut RefCell<Vec<String>>) {
    let mut words = words.borrow_mut();
    let mut current_index = words.len();

    while current_index != 0 {
        let random_index = (rand::rng().random_range(0.0..1.0) * current_index as f64) as usize;
        current_index -= 1;

        [words[current_index], words[random_index]] =
            [words[random_index].clone(), words[current_index].clone()];
    }
}
