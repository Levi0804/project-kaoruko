use regex::Regex;
use rust_socketio::asynchronous::Client;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::sync::{mpsc, oneshot};

use crate::types::PlayerStats;
use crate::utils::shuffle;
use crate::Dictionary;

struct Bot {
    // for receiving values from the associated sender
    receiver: mpsc::Receiver<BotMessage>,
    // english dictionary
    dictionary: Dictionary,
    // unique id of bot inside room
    self_peer_id: AtomicU64,
    // the user who created the room
    room_creator: String,
    // a list of words used in the game
    used_words: Vec<String>,
    // dynamically changing words as per typing
    player_word: String,
    // current active syllable in game
    syllable: String,
    // bombparty game socket
    game_socket: Option<Client>,
    // all the players in game
    players: HashMap<u64, PlayerStats>,
    // room socket
    room_socket: Option<Client>,
}

enum BotMessage {
    GetWords {
        query: String,
        respond_to: oneshot::Sender<String>,
    },
    GetWord {
        syllable: String,
        respond_to: oneshot::Sender<String>,
    },
    SetPeerId {
        peer_id: u64,
    },
    GetPeerId {
        respond_to: oneshot::Sender<u64>,
    },
    SetRoomCreator {
        creator: String,
    },
    GetRoomCreator {
        respond_to: oneshot::Sender<String>,
    },
    AddWord {
        word: String,
    },
    IsUsedWord {
        word: String,
        respond_to: oneshot::Sender<bool>,
    },
    SetPlayerWord {
        word: String,
    },
    GetPlayerWord {
        respond_to: oneshot::Sender<String>,
    },
    SetSyllable {
        syllable: String,
    },
    GetSyllable {
        respond_to: oneshot::Sender<String>,
    },
    RemoveWord {
        word: String,
    },
    SetGameSocket {
        socket: Client,
    },
    StartRoundNow,
    IsConsiderableWord {
        nickname: String,
        peer_id: u64,
        word: String,
    },
    AddPlayer {
        nickname: String,
        peer_id: u64,
        roles: Vec<String>,
    },
    GetPlayer {
        peer_id: u64,
        respond_to: oneshot::Sender<Option<PlayerStats>>,
    },
    SetRoomSocket {
        socket: Client,
    },
    SetChat {
        message: String,
    },
    UpdateLives {
        peer_id: u64,
        respond_to: oneshot::Sender<u64>,
    },
}

impl Bot {
    fn new(receiver: mpsc::Receiver<BotMessage>) -> Self {
        let dictionary = std::fs::read_to_string("src/dictionaries/english.json").unwrap();
        let mut dictionary = serde_json::from_str::<Dictionary>(&dictionary).unwrap();
        shuffle(&mut dictionary.dictionary);
        Self {
            receiver,
            dictionary,
            self_peer_id: AtomicU64::default(),
            room_creator: String::default(),
            used_words: Vec::<String>::new(),
            player_word: String::default(),
            syllable: String::default(),
            game_socket: None,
            players: HashMap::default(),
            room_socket: None,
        }
    }
    async fn handle_message(&mut self, msg: BotMessage) {
        match msg {
            BotMessage::GetWords { query, respond_to } => {
                let words = self.dictionary.dictionary.get_mut();
                if let Ok(re) = Regex::new(&query) {
                    let result = words.iter().filter(|w| re.is_match(w)).collect::<Vec<_>>();
                    let fifteen = if result.len() > 15 { 15 } else { result.len() };
                    let fifteen = result[0..fifteen]
                        .iter()
                        .map(|&word| word.clone())
                        .collect::<Vec<_>>()
                        .join(", ");
                    if fifteen.is_empty() {
                        respond_to
                            .send(format!("No result found for: {query}"))
                            .unwrap();
                    } else {
                        respond_to
                            .send(format!("results({}): {fifteen}", result.len()))
                            .unwrap();
                        shuffle(&mut self.dictionary.dictionary);
                    }
                } else {
                    respond_to.send(format!("too expensive regex")).unwrap();
                }
            }
            BotMessage::SetPeerId { peer_id } => {
                self.self_peer_id.swap(peer_id, Ordering::Relaxed);
            }
            BotMessage::GetPeerId { respond_to } => {
                respond_to
                    .send(self.self_peer_id.load(Ordering::Relaxed))
                    .unwrap();
            }
            BotMessage::SetRoomCreator { creator } => {
                self.room_creator = creator;
            }
            BotMessage::GetRoomCreator { respond_to } => {
                respond_to.send(self.room_creator.clone()).unwrap();
            }
            BotMessage::GetWord {
                syllable,
                respond_to,
            } => {
                let Dictionary { dictionary, .. } = &self.dictionary;
                for word in dictionary.borrow().iter() {
                    if word.contains(&syllable) && !self.used_words.contains(word) {
                        respond_to.send(word.clone()).unwrap();
                        break;
                    }
                }
            }
            BotMessage::AddWord { word } => {
                self.used_words.push(word);
            }
            BotMessage::IsUsedWord { word, respond_to } => {
                respond_to.send(self.used_words.contains(&word)).unwrap();
            }
            BotMessage::SetPlayerWord { word } => {
                self.player_word = word;
            }
            BotMessage::GetPlayerWord { respond_to } => {
                respond_to.send(self.player_word.clone()).unwrap();
            }
            BotMessage::SetSyllable { syllable } => {
                self.syllable = syllable;
            }
            BotMessage::GetSyllable { respond_to } => {
                respond_to.send(self.syllable.clone()).unwrap();
            }
            BotMessage::RemoveWord { word } => {
                let mut dict = self.dictionary.dictionary.borrow_mut();
                if let Some(index) = dict.iter().position(|w| w == &word) {
                    dict.remove(index);
                }
            }
            BotMessage::SetGameSocket { socket } => {
                self.game_socket = Some(socket);
            }
            BotMessage::StartRoundNow => {
                self.game_socket
                    .as_ref()
                    .unwrap()
                    .emit("startRoundNow", "")
                    .await
                    .unwrap();
            }
            BotMessage::IsConsiderableWord {
                nickname,
                peer_id,
                word,
            } => {
                let player = self.players.get_mut(&peer_id).unwrap();
                let client = self.room_socket.as_ref().unwrap();
                let mut perk = format!("{nickname} has placed");
                let mut considerable = false;
                if word.len() >= 20 {
                    player.longs += 1;
                    considerable = true;
                    perk.push_str(format!(" a long ({}) —", player.longs).as_str());
                }
                if word.contains("-") {
                    player.hyphens += 1;
                    considerable = true;
                    perk.push_str(format!(" a hyphen ({}) —", player.hyphens).as_str());
                }
                if self.dictionary.sn.contains(&word) {
                    player.subs += 1;
                    considerable = true;
                    perk.push_str(format!(" a sn ({}) —", player.subs).as_str());
                }
                if word.contains(" ") {
                    let word = word.split(" ").collect::<Vec<_>>();
                    let words = self.dictionary.dictionary.get_mut();
                    let mut contains = true;
                    for w in word {
                        if !words.contains(&w.to_string()) {
                            contains = false;
                            break;
                        }
                    }
                    if contains {
                        player.multi += 1;
                        considerable = true;
                        perk.push_str(format!(" a multi ({}) —", player.multi).as_str());
                    }
                }
                player.words += 1;
                if perk.ends_with("—") {
                    perk = perk.replace(" —", "");
                }
                perk.push_str(format!(": {word}").as_str());
                if considerable {
                    client.emit("chat", perk).await.unwrap();
                }
            }
            BotMessage::AddPlayer {
                nickname,
                peer_id,
                roles,
            } => {
                self.players
                    .insert(peer_id, PlayerStats::new(nickname, roles));
            }
            BotMessage::GetPlayer {
                peer_id,
                respond_to,
            } => {
                let player = self.players.get(&peer_id).map(|p| p.clone());
                respond_to.send(player).unwrap();
            }
            BotMessage::SetRoomSocket { socket } => {
                self.room_socket = Some(socket);
            }
            BotMessage::SetChat { message } => {
                let _ = self
                    .room_socket
                    .as_ref()
                    .unwrap()
                    .emit("chat", message)
                    .await;
            }
            BotMessage::UpdateLives {
                peer_id,
                respond_to,
            } => {
                let player = self.players.get_mut(&peer_id).unwrap();
                player.lives += 1;
                respond_to.send(player.lives).unwrap();
            }
        }
    }
}

async fn run_my_bot(mut bot: Bot) {
    while let Some(msg) = bot.receiver.recv().await {
        bot.handle_message(msg).await;
    }
}

#[allow(clippy::new_without_default)]
#[derive(Clone)]
pub struct BotHandle {
    sender: mpsc::Sender<BotMessage>,
}

impl BotHandle {
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::channel::<BotMessage>(512);
        let bot = Bot::new(receiver);
        tokio::spawn(run_my_bot(bot));

        Self { sender }
    }

    pub async fn get_words(&self, query: String) -> String {
        let (send, recv) = oneshot::channel::<String>();
        let msg = BotMessage::GetWords {
            query,
            respond_to: send,
        };

        self.sender.send(msg).await.unwrap();
        recv.await.expect("Bot has been killed")
    }

    pub async fn set_peer_id(&self, peer_id: u64) {
        let msg = BotMessage::SetPeerId { peer_id };
        self.sender.send(msg).await.unwrap();
    }

    pub async fn get_peer_id(&self) -> u64 {
        let (send, recv) = oneshot::channel::<u64>();
        let msg = BotMessage::GetPeerId { respond_to: send };

        self.sender.send(msg).await.unwrap();
        recv.await.expect("Bot has been killed")
    }

    pub async fn set_room_creator(&self, creator: String) {
        let msg = BotMessage::SetRoomCreator { creator };
        self.sender.send(msg).await.unwrap();
    }

    pub async fn get_room_creator(&self) -> String {
        let (send, recv) = oneshot::channel::<String>();
        let msg = BotMessage::GetRoomCreator { respond_to: send };

        self.sender.send(msg).await.unwrap();
        recv.await.expect("Bot has been killed")
    }

    pub async fn get_single_word(&self, syllable: String) -> String {
        let (send, recv) = oneshot::channel::<String>();
        let msg = BotMessage::GetWord {
            syllable,
            respond_to: send,
        };
        self.sender.send(msg).await.unwrap();
        recv.await.expect("Bot has been killed")
    }

    pub async fn add_used_word(&self, word: String) {
        let msg = BotMessage::AddWord { word };
        self.sender.send(msg).await.unwrap();
    }

    pub async fn is_used_word(&self, word: String) -> bool {
        let (send, recv) = oneshot::channel::<bool>();
        let msg = BotMessage::IsUsedWord {
            word,
            respond_to: send,
        };
        self.sender.send(msg).await.unwrap();
        recv.await.expect("Bot has been killed")
    }

    pub async fn set_player_word(&self, word: String) {
        let msg = BotMessage::SetPlayerWord { word };
        self.sender.send(msg).await.unwrap();
    }

    pub async fn get_player_word(&self) -> String {
        let (send, recv) = oneshot::channel::<String>();
        let msg = BotMessage::GetPlayerWord { respond_to: send };
        self.sender.send(msg).await.unwrap();
        recv.await.expect("Bot has been killed")
    }

    pub async fn set_syllable(&self, syllable: String) {
        let msg = BotMessage::SetSyllable { syllable };
        self.sender.send(msg).await.unwrap();
    }

    pub async fn get_syllable(&self) -> String {
        let (send, recv) = oneshot::channel::<String>();
        let msg = BotMessage::GetSyllable { respond_to: send };
        self.sender.send(msg).await.unwrap();
        recv.await.expect("Bot has been killed")
    }

    pub async fn remove_word(&self, word: String) {
        let msg = BotMessage::RemoveWord { word };
        self.sender.send(msg).await.unwrap();
    }

    pub async fn set_game_socket(&self, socket: Client) {
        let msg = BotMessage::SetGameSocket { socket };
        self.sender.send(msg).await.unwrap();
    }

    pub async fn start_round_now(&self) {
        let msg = BotMessage::StartRoundNow;
        self.sender.send(msg).await.unwrap();
    }

    pub async fn is_condierable_word(&self, nickname: String, peer_id: u64, word: String) {
        let msg = BotMessage::IsConsiderableWord {
            nickname,
            peer_id,
            word,
        };
        self.sender.send(msg).await.unwrap();
    }

    pub async fn add_player(&self, nickname: String, peer_id: u64, roles: Vec<String>) {
        let msg = BotMessage::AddPlayer {
            nickname,
            peer_id,
            roles,
        };
        self.sender.send(msg).await.unwrap();
    }

    pub async fn get_player(&self, peer_id: u64) -> Option<PlayerStats> {
        let (send, recv) = oneshot::channel::<Option<PlayerStats>>();
        let msg = BotMessage::GetPlayer {
            peer_id,
            respond_to: send,
        };
        self.sender.send(msg).await.unwrap();
        recv.await.expect("Bot has been killed")
    }

    pub async fn set_room_socket(&self, socket: Client) {
        let msg = BotMessage::SetRoomSocket { socket };
        self.sender.send(msg).await.unwrap();
    }

    pub async fn set_chat(&self, message: String) {
        let msg = BotMessage::SetChat { message };
        self.sender.send(msg).await.unwrap();
    }

    pub async fn increment_lives(&self, peer_id: u64) -> u64 {
        let (send, recv) = oneshot::channel::<u64>();
        let msg = BotMessage::UpdateLives {
            peer_id,
            respond_to: send,
        };
        self.sender.send(msg).await.unwrap();
        recv.await.expect("Bot has been killed")
    }
}
