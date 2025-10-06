use futures_util::FutureExt;
use regex::Regex;
use rust_socketio::{asynchronous::Client, Payload};
use serde_json::{from_value, json};
use std::pin::Pin;
use std::sync::Arc;

use crate::{bot::BotHandle, text_payload, types::PlayerStats};

pub fn on_set_milestone(
    payload: Payload,
    game_socket: Client,
    bot: Arc<BotHandle>,
) -> Pin<Box<dyn futures_util::Future<Output = ()> + Send + 'static>> {
    async move {
        let payload = text_payload(payload);
        let name = serde_json::from_value::<String>(payload[0]["name"].clone()).unwrap();
        if &name == "seating" {
            let _ = game_socket.emit("joinRound", "").await;
        }
        if let Ok(current_player_peer_id) =
            serde_json::from_value::<u64>(payload[0]["currentPlayerPeerId"].clone())
        {
            if bot.get_peer_id().await == current_player_peer_id {
                let syllable =
                    serde_json::from_value::<String>(payload[0]["syllable"].clone()).unwrap();
                bot.set_syllable(syllable.clone()).await;
                let word = bot.get_single_word(syllable).await;
                let _ = game_socket
                    .emit("setWord", vec![json!(word.clone()), json!(true)])
                    .await;
            }
        }
    }
    .boxed()
}

pub fn on_set_player_word(
    payload: Payload,
    _game_socket: Client,
    bot: Arc<BotHandle>,
) -> Pin<Box<dyn futures_util::Future<Output = ()> + Send + 'static>> {
    async move {
        let payload = text_payload(payload);
        let word = serde_json::from_value::<String>(payload[1].clone()).unwrap();
        bot.set_player_word(word).await;
    }
    .boxed()
}

// TODO: pass bot inside these functions
pub fn on_next_turn(
    payload: Payload,
    game_socket: Client,
    bot: Arc<BotHandle>,
) -> Pin<Box<dyn futures_util::Future<Output = ()> + Send + 'static>> {
    async move {
        let values = text_payload(payload);
        let player_peer_id = values[0].as_u64().unwrap();
        let _prompt_age = values[2].as_u64().unwrap();
        let syllable = from_value::<String>(values[1].clone()).unwrap();
        bot.set_syllable(syllable.clone()).await;
        if bot.get_peer_id().await == player_peer_id {
            let word = bot.get_single_word(syllable).await;
            let _ = game_socket
                .emit("setWord", vec![json!(word), json!(true)])
                .await;
        }
    }
    .boxed()
}

pub fn on_add_player(
    payload: Payload,
    _socket: Client,
    bot: Arc<BotHandle>,
) -> Pin<Box<dyn futures_util::Future<Output = ()> + Send + 'static>> {
    async move {
        let payload = text_payload(payload);
        // TODO: instead of this, deserialize everthing.
        let nickname =
            serde_json::from_value::<String>(payload[0]["profile"]["nickname"].clone()).unwrap();
        let peer_id = payload[0]["profile"]["peerId"].clone().as_u64().unwrap();
        let roles =
            serde_json::from_value::<Vec<String>>(payload[0]["profile"]["roles"].clone()).unwrap();

        if peer_id != bot.get_peer_id().await {
            bot.add_player(nickname, peer_id, roles).await;
        }
    }
    .boxed()
}

pub fn on_correct_word(
    payload: Payload,
    _socket: Client,
    bot: Arc<BotHandle>,
) -> Pin<Box<dyn futures_util::Future<Output = ()> + Send + 'static>> {
    async move {
        let correct_word = bot.get_player_word().await;
        let re = Regex::new(r"[^a-z-' ]").unwrap();
        let correct_word = re.replace_all(&correct_word, "").to_string();
        bot.add_used_word(correct_word.clone()).await;
        let payload = text_payload(payload);
        let peer_id = payload[0]["playerPeerId"].clone().as_u64().unwrap();
        // tokio::task::spawn(async move {
        if peer_id != bot.get_peer_id().await {
            let player_stats = bot.get_player(peer_id).await;
            if let Some(p) = player_stats {
                bot.is_condierable_word(p.nickname.clone(), peer_id, correct_word)
                    .await;
            }
        }
        // });
    }
    .boxed()
}

pub fn on_fail_word(
    payload: Payload,
    game_socket: Client,
    bot: Arc<BotHandle>,
) -> Pin<Box<dyn futures_util::Future<Output = ()> + Send + 'static>> {
    async move {
        let bot2 = Arc::clone(&bot);
        let values = text_payload(payload);
        let player_peer_id = values[0].as_u64().unwrap();
        let reason = from_value::<String>(values[1].clone()).unwrap();
        if player_peer_id == bot.get_peer_id().await && reason == "notInDictionary" {
            tokio::spawn(async move {
                let incorrect_word = bot2.get_player_word().await;
                bot2.remove_word(incorrect_word).await;
            });
            let word = bot.get_single_word(bot.get_syllable().await).await;
            let _ = game_socket
                .emit("setWord", vec![json!(word), json!(true)])
                .await;
        }
    }
    .boxed()
}

pub fn on_lives_lost(
    payload: Payload,
    _: Client,
    bot: Arc<BotHandle>,
) -> Pin<Box<dyn futures_util::Future<Output = ()> + Send + 'static>> {
    async move {
        let payload = text_payload(payload);
        let lives = payload[1].as_u64().unwrap();
        if lives == 0 {
            let peer_id = payload[0].as_u64().unwrap();
            if let Some(p) = bot.get_player(peer_id).await {
                let PlayerStats {
                    nickname,
                    words,
                    subs,
                    longs,
                    hyphens,
                    multi,
                    lives,
                    ..
                } = p;
                let message = format!("Well played {nickname}! lives: {lives} — words: {words} — subs: {subs} — longs: {longs} — hyphens: {hyphens} — multi: {multi}");
                bot.set_chat(message).await;
            } 
        }
    }
    .boxed()
}

pub fn on_bonus_alphabet_completed(
    payload: Payload,
    _: Client,
    bot: Arc<BotHandle>,
) -> Pin<Box<dyn futures_util::Future<Output = ()> + Send + 'static>> {
    async move {
        let payload = text_payload(payload);
        let peer_id = payload[0].as_u64().unwrap();
        let _lives = payload[1].as_u64();
        if let Some(p) = bot.get_player(peer_id).await {
            let nickname = p.nickname;
            let count = bot.increment_lives(peer_id).await;
            bot.set_chat(format!("{nickname} has gained a life ({count})")).await;
        }
    }
    .boxed()
}
