use futures_util::FutureExt;
use rust_socketio::{asynchronous::Client, Payload};
use serde_json::{from_value, json};
use std::pin::Pin;
use std::sync::Arc;

use crate::{bot::BotHandle, text_payload};

pub fn on_set_milestone(
    payload: Payload,
    game_socket: Client,
    bot: Arc<BotHandle>,
) -> Pin<Box<dyn futures_util::Future<Output = ()> + Send + 'static>> {
    async move {
        let payload = text_payload(payload);
        let name = serde_json::from_value::<String>(payload[0]["name"].clone()).unwrap();
        if &name == "seating" {
            // TODO: only thing that needs to be changed is the dictionary not everything
            // so only change that don't take the entire bot as new.
            // let bot = Arc::new(BotHandle::new());
            // ROOMS.lock().await.insert(room_code.clone(), bot);
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
        if bot.get_peer_id().await == player_peer_id {
            let syllable = from_value::<String>(values[1].clone()).unwrap();
            bot.set_syllable(syllable.clone()).await;
            let word = bot.get_single_word(syllable).await;
            let _ = game_socket
                .emit("setWord", vec![json!(word), json!(true)])
                .await;
        }
    }
    .boxed()
}

// Do we really need this?
pub fn on_setup(
    _payload: Payload,
    _game_socket: Client,
) -> Pin<Box<dyn futures_util::Future<Output = ()> + Send + 'static>> {
    async move {}.boxed()
}

pub fn on_correct_word(
    _payload: Payload,
    _socket: Client,
    bot: Arc<BotHandle>,
) -> Pin<Box<dyn futures_util::Future<Output = ()> + Send + 'static>> {
    async move {
        let correct_word = bot.get_player_word().await;
        bot.add_used_word(correct_word).await
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
