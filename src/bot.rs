use std::sync::atomic::{AtomicU64, Ordering};
use tokio::sync::{mpsc, oneshot};

use crate::utils::{create_user_token, shuffle};
use crate::Dictionary;

pub const NAME: &str = "kaoruko ✨";

struct Bot {
    receiver: mpsc::Receiver<BotMessage>,
    dictionary: Dictionary,
    // unique id of the bot inside room
    self_peer_id: AtomicU64,
    // the user who created the room
    room_creator: String,
    // unique bot token for each room
    token: String,
    used_words: Vec<String>,
    player_word: String,
    syllable: String,
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
    GetBotToken {
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
            token: create_user_token().unwrap_or("aaaaaaaaaaaaaaaa".to_string()),
            used_words: Vec::<String>::new(),
            player_word: String::default(),
            syllable: String::default(),
        }
    }
    async fn handle_message(&mut self, msg: BotMessage) {
        match msg {
            BotMessage::GetWords { query, respond_to } => {
                let Dictionary { dictionary, .. } = &self.dictionary;
                let result = dictionary.borrow();
                let result = result
                    .iter()
                    .filter(|word| word.contains(&query))
                    .collect::<Vec<_>>();
                let fifteen = if result.len() > 15 { 15 } else { result.len() };
                let fifteen = result[0..fifteen]
                    .iter()
                    .map(|&word| word.clone())
                    .collect::<Vec<_>>()
                    .join(", ");
                respond_to
                    .send(format!("results({}): {fifteen}", result.len()))
                    .unwrap();
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
            BotMessage::GetBotToken { respond_to } => {
                respond_to.send(self.token.clone()).unwrap();
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
                let index = dict.iter().position(|w| w == &word).unwrap();
                dict.remove(index);
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

    pub async fn get_word(&self, query: String) -> String {
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

    pub async fn get_bot_token(&self) -> String {
        let (send, recv) = oneshot::channel::<String>();
        let msg = BotMessage::GetBotToken { respond_to: send };

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
}
