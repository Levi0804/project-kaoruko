use anyhow::anyhow;
use futures_util::FutureExt;
use rust_socketio::{
    asynchronous::{Client, ClientBuilder},
    Event, Payload, TransportType,
};
use serde_json::{from_value, json};
use std::sync::Arc;
use std::sync::LazyLock;
use std::{collections::HashMap, pin::Pin, time::Duration};
use tokio::sync::{Mutex, Notify};

use crate::command::CommandParserTrait;

pub mod bot;
pub mod command;
pub mod types;
pub mod utils;

use bot::BotHandle;
use command::Command;
use types::*;
use utils::*;

// TODO: do not use this
const TOKEN: &str = "aaaaaaaaaaaaaaaa";

// TODO: the on start room creator should be the bot, add this to the actor.
// TODO: configure a new feature for development.
// TODO: separate game socket and room socket.

// timeout to wait for socket to receive response (in seconds).
const TIMEOUT: u64 = 5;
// unique room for every connected socket.
type RoomCode = String;
// TODO: avoid using tokio mutex.
type Rooms = LazyLock<Arc<Mutex<HashMap<RoomCode, Arc<BotHandle>>>>>;
// every room with it's own unique state
static ROOMS: Rooms = LazyLock::new(|| Arc::new(Mutex::new(HashMap::new())));

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let subscriber = tracing_subscriber::fmt()
        .compact()
        .with_file(true)
        .with_line_number(true)
        .with_target(false)
        .finish();

    tracing::subscriber::set_global_default(subscriber)?;

    // a central room will be created on start
    tokio::spawn(async move {
        // let token = Arc::new(create_user_token().await?);
        // let token2 = Arc::clone(&token);

        let (host, room_code) = start_new_room(None, false, &TOKEN.to_string()).await?;

        let room_code = Arc::new(room_code);
        let room_code2 = Arc::clone(&room_code);

        let notifier = Arc::new(Notify::new());
        let notifier2 = Arc::clone(&notifier);

        let socket = ClientBuilder::new(host)
            .reconnect(false)
            .transport_type(TransportType::Websocket)
            .on(Event::Connect, move |payload, socket| {
                let room_code = room_code.to_string();
                tracing::info!("Playing at you know where with code: {room_code}");
                on_connect(payload, socket, room_code, TOKEN.to_string())
            })
            .on("chat", move |payload, socket| {
                on_chat(
                    payload,
                    socket,
                    room_code2.to_string(),
                    Arc::clone(&notifier),
                )
            })
            .on("chatterAdded", on_chatter_added)
            .connect()
            .await
            .expect("Connection failed");
        // keep the task alive, giving some time to the "connect" event
        notifier2.notified().await;
        // this line will be executed when user wants to exit
        socket.disconnect().await?;

        Ok::<(), anyhow::Error>(())
    })
    .await??;

    Ok(())
}

fn on_connect(
    _paylod: Payload,
    socket: Client,
    room_code: RoomCode,
    token: String,
) -> Pin<Box<dyn std::future::Future<Output = ()> + Send + 'static>> {
    async move {
        let bot = Arc::new(BotHandle::new());
        let bot2 = Arc::clone(&bot);
        let room_code = Arc::new(room_code);
        let room_code2 = Arc::clone(&room_code);
        socket
            .emit_with_ack(
                "joinRoom",
                json!({
                    "roomCode": room_code.to_string(),
                    "userToken": token,
                    "picture": "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",
                    "language": "en-US",
                    "nickname": "kaoruko ✨",
                }),
                Duration::from_secs(TIMEOUT),
                move |payload: Payload, socket: Client| {
                    let bot = Arc::clone(&bot);
                    on_connect_inner(payload, socket, bot, room_code2.to_string())
                },
            )
            .await
            .expect("server unreachable");
        // we don't care about errors
        ROOMS.lock().await.insert(room_code.to_string(), bot2);
    }
    .boxed()
}

fn on_connect_inner(
    payload: Payload,
    _socket: Client,
    bot: Arc<BotHandle>,
    room_code: RoomCode,
) -> Pin<Box<dyn std::future::Future<Output = ()> + Send + 'static>> {
    async move {
        let Payload::Text(values) = payload else {
            unreachable!("entered unreachable!?");
        };
        let RoomDetails { self_peer_id, .. } = RoomDetails::from(values);
        bot.set_peer_id(self_peer_id).await;

        let room_code = Arc::new(room_code);
        let room_code2 = Arc::clone(&room_code);
        let room_code3 = Arc::clone(&room_code);
        let room_code4 = Arc::clone(&room_code);
        let room_code5 = Arc::clone(&room_code);
        let room_code6 = Arc::clone(&room_code);

        let host = join_room(&room_code).await.unwrap();

        let bot = ROOMS.lock().await;
        let bot = bot
            .get(&room_code.to_string())
            .ok_or(anyhow!("cannot get bot handle"))
            .unwrap();
        // bombparty socket
        let game_socket = ClientBuilder::new(host)
            .reconnect(false)
            .transport_type(TransportType::Websocket)
            .on(Event::Connect, move |_: Payload, socket: Client| {
                let room_code = Arc::clone(&room_code);
                async move {
                    socket
                        .emit(
                            "joinGame",
                            vec![
                                json!("bombparty"),
                                json!(room_code.to_string()),
                                json!(TOKEN),
                            ],
                        )
                        .await
                        .expect("unable to join game");
                    // ignore the errors
                    let _ = socket.emit("joinRound", "").await;
                }
                .boxed()
            })
            .on("nextTurn", move |payload: Payload, socket: Client| {
                on_next_turn(payload, socket, room_code2.to_string())
            })
            .on("setPlayerWord", move |payload: Payload, socket: Client| {
                on_set_player_word(payload, socket, room_code6.to_string())
            })
            .on("correctWord", move |payload: Payload, socket: Client| {
                on_correct_word(payload, socket, room_code5.to_string())
            })
            .on("failWord", move |payload: Payload, socket: Client| {
                on_fail_word(payload, socket, room_code3.to_string())
            })
            .on("setMilestone", move |payload: Payload, socket: Client| {
                on_set_milestone(payload, socket, room_code4.to_string())
            })
            .on("setup", on_setup)
            .connect()
            .await
            .expect("unable to create game socket");
        bot.set_game_socket(game_socket).await;
    }
    .boxed()
}

fn on_set_milestone(
    payload: Payload,
    game_socket: Client,
    room_code: RoomCode,
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
            let bot = ROOMS.lock().await;
            let bot = bot
                .get(&room_code)
                .ok_or(anyhow!("cannot get bot handle"))
                .unwrap();
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

fn on_set_player_word(
    payload: Payload,
    _game_socket: Client,
    room_code: RoomCode,
) -> Pin<Box<dyn futures_util::Future<Output = ()> + Send + 'static>> {
    async move {
        let payload = text_payload(payload);
        let bot = ROOMS.lock().await;
        let bot = bot
            .get(&room_code)
            .ok_or(anyhow!("cannot get bot handle"))
            .unwrap();
        let bot2 = Arc::clone(&bot);
        let word = serde_json::from_value::<String>(payload[1].clone()).unwrap();
        bot2.set_player_word(word).await;
    }
    .boxed()
}

// TODO: pass bot inside these functions
fn on_next_turn(
    payload: Payload,
    game_socket: Client,
    room_code: RoomCode,
) -> Pin<Box<dyn futures_util::Future<Output = ()> + Send + 'static>> {
    async move {
        let bot = ROOMS.lock().await;
        let bot = bot
            .get(&room_code)
            .ok_or(anyhow!("cannot get bot handle"))
            .unwrap();
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

fn on_setup(
    _payload: Payload,
    _game_socket: Client,
) -> Pin<Box<dyn futures_util::Future<Output = ()> + Send + 'static>> {
    async move {}.boxed()
}

fn on_correct_word(
    payload: Payload,
    _socket: Client,
    room_code: RoomCode,
) -> Pin<Box<dyn futures_util::Future<Output = ()> + Send + 'static>> {
    async move {
        let _payload = text_payload(payload);
        let bot = ROOMS.lock().await;
        let bot = bot
            .get(&room_code)
            .ok_or(anyhow!("cannot get bot handle"))
            .unwrap();
        let correct_word = bot.get_player_word().await;
        bot.add_used_word(correct_word).await
    }
    .boxed()
}

fn on_fail_word(
    payload: Payload,
    game_socket: Client,
    room_code: RoomCode,
) -> Pin<Box<dyn futures_util::Future<Output = ()> + Send + 'static>> {
    async move {
        let bot = ROOMS.lock().await;
        let bot = bot
            .get(&room_code)
            .ok_or(anyhow!("cannot get bot handle"))
            .unwrap();
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

fn on_chatter_added(
    payload: Payload,
    socket: Client,
) -> Pin<Box<dyn futures_util::Future<Output = ()> + Send + 'static>> {
    async move {
        let payload = text_payload(payload);
        let NewChatter {
            auth: Auth { id, .. },
            peer_id,
            nickname,
        } = NewChatter::from(payload);
        // greet new chatter
        socket
            .emit("chat", format!("Hey, {nickname}!"))
            .await
            .unwrap();
        // mods me.
        if id == "988839581384323083" || id == "907638639314473060" {
            socket
                .emit_with_ack(
                    "setUserModerator",
                    vec![json!(peer_id), json!(true)],
                    Duration::from_secs(TIMEOUT),
                    move |_: Payload, _: Client| async move {}.boxed(),
                )
                .await
                .unwrap(); // will this unwrap panic if bot is not leader?
        }
    }
    .boxed()
}

fn on_chat(
    payload: Payload,
    socket: Client,
    room_code: RoomCode,
    notifier: Arc<Notify>,
) -> Pin<Box<dyn futures_util::Future<Output = ()> + Send + 'static>> {
    async move {
        let Payload::Text(values) = payload else {
            tracing::warn!("Entered unreachable!?");
            unreachable!();
        };
        tokio::spawn(async move {
            let chatter = serde_json::from_value::<Chatter>(values[0].clone())?;
            let message = serde_json::from_value::<String>(values[1].clone())?;

            let bot = ROOMS.lock().await;
            let bot = bot
                .get(&room_code)
                .ok_or(anyhow!("cannot get bot handle"))?;

            #[allow(clippy::redundant_closure_call)]
            (async || {
                // TODO: also move this parsing inside the function
                let bot_peer_id = bot.get_peer_id().await;
                let (cmd, query) = message.split_once(" ").unwrap_or((&message, ""));
                if !cmd.starts_with("!") && bot_peer_id == chatter.peer_id {
                    return Ok::<(), anyhow::Error>(());
                }
                let Some(cmd) = cmd.strip_prefix("!") else {
                    return Ok(());
                };
                match cmd.parse_command(
                    chatter.roles,
                    chatter.auth.as_ref(),
                    bot.get_room_creator().await,
                ) {
                    Ok(cmd) => match cmd {
                        Command::Search => {
                            if query == "" {
                                let syllable = bot.get_syllable().await;
                                let word = bot.get_word(syllable).await;
                                socket.emit("chat", word).await?;
                                return Ok(());
                            }
                            let word = bot.get_word(query.to_string()).await;
                            socket.emit("chat", word).await?;
                        }
                        Command::Exit => {
                            socket.emit("chat", "sayonara!").await?;
                            tokio::time::sleep(Duration::from_millis(500)).await;
                            notifier.notify_one();
                        }
                        Command::Help => match query.parse::<Command>() {
                            Ok(cmd) => socket.emit("chat", cmd.help()).await?,
                            Err(err) => socket.emit("chat", err.to_string()).await?,
                        },
                        Command::StartNow => {
                            bot.start_round_now().await;
                        }
                    },
                    Err(err) => {
                        socket.emit("chat", err.to_string()).await?;
                        return Ok(());
                    }
                };
                Ok(())
            })()
            .await
            .unwrap(); // is infallible

            Ok::<(), anyhow::Error>(())
        })
        .await
        .expect("error spawning task")
        .expect("error handling message");
    }
    .boxed()
}
