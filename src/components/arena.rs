use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use web_sys::KeyboardEvent;
use yew::prelude::*;
use yew_agent::{Bridge, Bridged};

use crate::services::event_bus::EventBus;
use crate::{services::websocket::WebsocketService, User};

pub enum Msg {
    HandleMsg(String),
    StartGame,
    KeyPress(KeyboardEvent),
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum MsgTypes {
    Users,
    Register,
    Message,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WebSocketMessage {
    message_type: MsgTypes,
    data_array: Option<Vec<String>>,
    data: Option<String>,
}

// ---------------- TAMBAHAN BARU ----------------
// Struct untuk membaca bungkusan pesan dari Server Node.js
#[derive(Deserialize)]
struct MessageData {
    from: String,
    message: String,
}
// -----------------------------------------------

#[derive(Serialize, Deserialize, Debug)]
struct ArenaMessage {
    #[serde(default)]
    arena_action: String, 
    username: String,
    score: u32,
}

pub struct Arena {
    username: String,
    scores: HashMap<String, u32>,
    game_active: bool,
    winner: Option<String>,
    wss: WebsocketService,
    _producer: Box<dyn Bridge<EventBus>>,
}

impl Component for Arena {
    type Message = Msg;
    type Properties = ();

    fn create(ctx: &Context<Self>) -> Self {
        let (user, _) = ctx.link().context::<User>(Callback::noop()).expect("context to be set");
        let wss = WebsocketService::new();
        let username_cloned = user.username.borrow().clone();

        let reg_msg = WebSocketMessage {
            message_type: MsgTypes::Register,
            data: Some(username_cloned.clone()),
            data_array: None,
        };
        let _ = wss.tx.clone().try_send(serde_json::to_string(&reg_msg).unwrap());

        Self {
            username: username_cloned,
            scores: HashMap::new(),
            game_active: false,
            winner: None,
            wss,
            _producer: EventBus::bridge(ctx.link().callback(Msg::HandleMsg)),
        }
    }

    fn update(&mut self, _ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::HandleMsg(s) => {
                if let Ok(ws_msg) = serde_json::from_str::<WebSocketMessage>(&s) {
                    if ws_msg.message_type == MsgTypes::Message {
                        if let Some(data_str) = ws_msg.data {
                            
                            // PERBAIKAN: Kita parse dulu bungkusan (MessageData) dari server Node.js
                            if let Ok(msg_data) = serde_json::from_str::<MessageData>(&data_str) {
                                
                                // Setelah itu, baru kita ambil "message" aslinya dan parse menjadi ArenaMessage
                                if let Ok(arena_msg) = serde_json::from_str::<ArenaMessage>(&msg_data.message) {
                                    match arena_msg.arena_action.as_str() {
                                        "start" => {
                                            self.scores.clear();
                                            self.game_active = true;
                                            self.winner = None;
                                            return true;
                                        }
                                        "update" => {
                                            self.scores.insert(arena_msg.username, arena_msg.score);
                                            return true;
                                        }
                                        "win" => {
                                            self.game_active = false;
                                            self.winner = Some(arena_msg.username);
                                            return true;
                                        }
                                        _ => {}
                                    }
                                }
                            }
                        }
                    }
                }
                false 
            }
            Msg::StartGame => {
                let arena_msg = ArenaMessage { arena_action: "start".into(), username: self.username.clone(), score: 0 };
                
                let ws_msg = WebSocketMessage {
                    message_type: MsgTypes::Message,
                    data: Some(serde_json::to_string(&arena_msg).unwrap()),
                    data_array: None,
                };
                
                let _ = self.wss.tx.clone().try_send(serde_json::to_string(&ws_msg).unwrap());
                false
            }
            Msg::KeyPress(e) => {
                if e.key() == " " {
                    e.prevent_default();
                }

                if self.game_active && e.key() == " " { 
                    let current_score = self.scores.get(&self.username).copied().unwrap_or(0) + 1;
                    
                    let action = if current_score >= 50 { "win" } else { "update" };
                    let arena_msg = ArenaMessage { arena_action: action.into(), username: self.username.clone(), score: current_score };
                    
                    let ws_msg = WebSocketMessage {
                        message_type: MsgTypes::Message,
                        data: Some(serde_json::to_string(&arena_msg).unwrap()),
                        data_array: None,
                    };

                    let _ = self.wss.tx.clone().try_send(serde_json::to_string(&ws_msg).unwrap());
                }
                false
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let start_cb = ctx.link().callback(|_| Msg::StartGame);
        let keypress_cb = ctx.link().callback(Msg::KeyPress);

        let mut sorted_scores: Vec<_> = self.scores.iter().collect();
        sorted_scores.sort_by(|a, b| b.1.cmp(a.1)); 

        html! {
            <div class="flex flex-col items-center justify-center w-screen h-screen bg-gray-900 text-white outline-none" tabindex="0" onkeydown={keypress_cb}>
                <h1 class="text-5xl font-bold mb-8">{"🔥 GLOBAL SPAM ARENA 🔥"}</h1>
                
                if let Some(w) = &self.winner {
                    <div class="text-4xl text-green-400 mb-8 font-extrabold animate-bounce">{format!("👑 {} WINS! 👑", w)}</div>
                } else if self.game_active {
                    <div class="text-3xl text-yellow-400 mb-8 animate-pulse font-bold">{"SPAM SPACEBAR!! (First to 50)"}</div>
                } else {
                    <div class="text-2xl text-gray-400 mb-8">{"Waiting to start..."}</div>
                }

                <button onclick={start_cb} class="bg-red-600 hover:bg-red-700 text-white font-bold py-4 px-8 rounded-full mb-8 text-2xl shadow-lg shadow-red-500/50 transition transform hover:scale-110">
                    {"START ARENA"}
                </button>

                <div class="w-1/2 bg-gray-800 rounded-lg p-6 shadow-2xl">
                    <h2 class="text-2xl font-bold mb-4 border-b border-gray-700 pb-2">{"Leaderboard"}</h2>
                    {
                        for sorted_scores.iter().map(|(user, score)| {
                            html! {
                                <div class="flex justify-between text-xl my-3 p-2 bg-gray-700 rounded">
                                    <span class="font-bold">{user}</span>
                                    <span class="font-mono text-blue-400 font-bold">{score}</span>
                                </div>
                            }
                        })
                    }
                </div>
                <div class="mt-8 text-gray-500 italic">{"(Click anywhere on the black background to focus, then spam space!)"}</div>
            </div>
        }
    }
}