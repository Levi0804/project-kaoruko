use anyhow::anyhow;
use futures_util::FutureExt;
use regex::Regex;
use rust_socketio::{asynchronous::Client, Payload};
use serde_json::{from_value, json};
use std::pin::Pin;
use std::sync::Arc;

use crate::{
    bot::BotHandle,
    text_payload,
    types::{DataOnNextTurn, Player, PlayerStats},
};

pub fn on_set_milestone(
    payload: Payload,
    game_socket: Client,
    bot: Arc<BotHandle>,
) -> Pin<Box<dyn futures_util::Future<Output = anyhow::Result<()>> + Send + 'static>> {
    async move {
        let payload = text_payload(payload);
        let name = serde_json::from_value::<String>(payload[0]["name"].clone())?;
        if name.as_str() == "seating" {
            let _ = game_socket.emit("joinRound", "").await;
        }
        if let Ok(current_player_peer_id) =
            serde_json::from_value::<u64>(payload[0]["currentPlayerPeerId"].clone())
        {
            if bot.get_peer_id().await == current_player_peer_id {
                let syllable = serde_json::from_value::<String>(payload[0]["syllable"].clone())?;
                bot.set_syllable(syllable.clone()).await;
                let word = bot.get_single_word(syllable).await;
                let _ = game_socket
                    .emit("setWord", vec![json!(word.clone()), json!(true)])
                    .await;
            }
        }
        Ok(())
    }
    .boxed()
}

pub fn on_set_player_word(
    payload: Payload,
    _game_socket: Client,
    bot: Arc<BotHandle>,
) -> Pin<Box<dyn futures_util::Future<Output = anyhow::Result<()>> + Send + 'static>> {
    async move {
        let word = serde_json::from_value::<String>(text_payload(payload)[1].clone())?;
        bot.set_player_word(word).await;
        Ok(())
    }
    .boxed()
}

pub fn on_next_turn(
    payload: Payload,
    game_socket: Client,
    bot: Arc<BotHandle>,
) -> Pin<Box<dyn futures_util::Future<Output = anyhow::Result<()>> + Send + 'static>> {
    async move {
        let details = DataOnNextTurn::try_from(text_payload(payload))?;
        bot.set_syllable(details.syllable.clone()).await;
        if bot.get_peer_id().await == details.player_peer_id {
            let word = bot.get_single_word(details.syllable).await;
            let _ = game_socket
                .emit("setWord", vec![json!(word), json!(true)])
                .await;
        }
        Ok(())
    }
    .boxed()
}

pub fn on_add_player(
    payload: Payload,
    _socket: Client,
    bot: Arc<BotHandle>,
) -> Pin<Box<dyn futures_util::Future<Output = anyhow::Result<()>> + Send + 'static>> {
    async move {
        let player = Player::try_from(text_payload(payload))?;
        if player.peer_id != bot.get_peer_id().await {
            bot.add_player(player.nickname, player.peer_id, player.roles)
                .await;
        }
        Ok(())
    }
    .boxed()
}

pub fn on_correct_word(
    payload: Payload,
    _socket: Client,
    bot: Arc<BotHandle>,
) -> Pin<Box<dyn futures_util::Future<Output = anyhow::Result<()>> + Send + 'static>> {
    async move {
        let re = Regex::new(r"[^a-z-' ]")?;
        let correct_word = re.replace_all(&bot.get_player_word().await, "").to_string();

        bot.add_used_word(correct_word.clone()).await;

        let player_peer_id = text_payload(payload)[0]["playerPeerId"]
            .as_u64()
            .ok_or_else(|| anyhow!("failed to extract peer id"))?;
        if player_peer_id != bot.get_peer_id().await {
            if let Some(PlayerStats { nickname, .. }) = bot.get_player(player_peer_id).await {
                bot.is_condierable_word(nickname, player_peer_id, correct_word)
                    .await;
            }
        }
        Ok(())
    }
    .boxed()
}

pub fn on_fail_word(
    payload: Payload,
    game_socket: Client,
    bot_handle: Arc<BotHandle>,
) -> Pin<Box<dyn futures_util::Future<Output = anyhow::Result<()>> + Send + 'static>> {
    async move {
        let payload = text_payload(payload);
        let player_peer_id = payload[0]
            .as_u64()
            .ok_or_else(|| anyhow!("failed to extract peer id"))?;
        let reason = from_value::<String>(payload[1].clone())?;

        let bot_handle2 = Arc::clone(&bot_handle);

        if player_peer_id == bot_handle.get_peer_id().await && reason == "notInDictionary" {
            tokio::spawn(async move {
                let incorrect_word = bot_handle2.get_player_word().await;
                bot_handle2.remove_word(incorrect_word).await;
            });
            let word = bot_handle
                .get_single_word(bot_handle.get_syllable().await)
                .await;
            let _ = game_socket
                .emit("setWord", vec![json!(word), json!(true)])
                .await;
        }
        Ok(())
    }
    .boxed()
}

pub fn on_lives_lost(
    payload: Payload,
    _: Client,
    bot: Arc<BotHandle>,
) -> Pin<Box<dyn futures_util::Future<Output = anyhow::Result<()>> + Send + 'static>> {
    async move {
        let payload = text_payload(payload);
        let peer_id = payload[0]
            .as_u64()
            .ok_or_else(|| anyhow!("failed to extract peer id"))?;
        let lives = payload[1]
            .as_u64()
            .ok_or_else(|| anyhow!("failed to extract lives"))?;
        if lives == 0 {
            if let Some(PlayerStats {
                nickname,
                words,
                subs,
                longs,
                hyphens,
                multi,
                lives,
                ..
            }) = bot.get_player(peer_id).await
            {
                bot.set_chat(format!(
                    "Well played {nickname}! \
                lives: {lives} — words: {words} — subs: {subs} \
                — longs: {longs} — hyphens: {hyphens} — multi: {multi}"
                ))
                .await;
            }
        }
        Ok(())
    }
    .boxed()
}

pub fn on_bonus_alphabet_completed(
    payload: Payload,
    _: Client,
    bot: Arc<BotHandle>,
) -> Pin<Box<dyn futures_util::Future<Output = anyhow::Result<()>> + Send + 'static>> {
    async move {
        // let _lives = text_payload(payload)[1].as_u64();
        let peer_id = text_payload(payload)[0]
            .as_u64()
            .ok_or_else(|| anyhow!("failed to extract peer id"))?;
        if let Some(PlayerStats { nickname, .. }) = bot.get_player(peer_id).await {
            let lives_count = bot.increment_lives(peer_id).await;
            bot.set_chat(format!("{nickname} has gained a life ({lives_count})"))
                .await;
        }
        Ok(())
    }
    .boxed()
}
