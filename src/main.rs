mod users;
use axum::{
    extract::{
        ws::{Message, WebSocket},
        State, WebSocketUpgrade,
    },
    response::IntoResponse,
    routing::get,
    Router,
};
use axum_extra::{headers, TypedHeader};
use futures::{sink::SinkExt, stream::StreamExt};
use serde::{Deserialize, Serialize};
use std::{
    net::SocketAddr,
    sync::{Arc, Mutex},
};
use tower_http::{services::ServeDir, trace::TraceLayer};

#[tokio::main]
async fn main() {
    let room = Arc::new(Mutex::new(Room::new()));

    let addr = SocketAddr::from(([127, 0, 0, 1], 8080));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();

    let app = Router::new()
        .fallback_service(ServeDir::new("public"))
        .route("/ws", get(ws_handler))
        .with_state(room);

    axum::serve(listener, app.layer(TraceLayer::new_for_http()))
        .await
        .unwrap();
}

async fn ws_handler(
    State(room): State<Arc<Mutex<Room>>>,
    ws: WebSocketUpgrade,
    user_agent: Option<TypedHeader<headers::UserAgent>>,
) -> impl IntoResponse {
    let user_agent = if let Some(TypedHeader(user_agent)) = user_agent {
        user_agent.to_string()
    } else {
        String::from("Unknown browser")
    };
    println!("`{user_agent}` ");
    // finalize the upgrade process by returning upgrade callback.
    // we can customize the callback by sending additional info such as address.
    ws.on_upgrade(|socket| handle_socket(socket, room))
}

async fn handle_socket(socket: WebSocket, room: Arc<Mutex<Room>>) {
    let (user_send, mut user_recv) = tokio::sync::mpsc::unbounded_channel::<MyMessage>();
    let (mut socket_send, mut socket_recv) = socket.split();
    tokio::spawn(async move {
        while let Some(msg) = user_recv.recv().await {
            match msg {
                MyMessage::NewMsg(msg) => {
                    socket_send
                        .send(Message::Text(
                            serde_json::to_string(&MyMessage::NewMsg(msg))
                                .expect("Failed to serialize MyMessage"),
                        ))
                        .await
                        .expect("Failed to send");
                }
                _ => todo!(),
            }
        }
    });
    let user = User::new(user_send);
    room.lock()
        .expect("other thread panicked while holding lock")
        .join_user(user);
    while let Some(msg) = socket_recv.next().await {
        println!("{:?}", msg);
        let msg = msg.expect("deserialize message");
        match msg {
            Message::Text(msg_txt) => {
                println!("{:?}", &msg_txt);
                match serde_json::from_str(&msg_txt).expect("deserialze MyMessage") {
                    MyMessage::NewMsg(new_msg_txt) => {
                        println!("{:?}", new_msg_txt);
                        if let Ok(mut room) = room.as_ref().lock() {
                            room.send(&MyMessage::NewMsg(new_msg_txt))
                        }
                    }
                    MyMessage::UpdateUserList(s) => {
                        println!("{:?}", s);
                    }
                    MyMessage::Unknown(msg) => {
                        println!("{:?}", msg);
                    }
                    MyMessage::NewUser(user_info) => {
                        println!("{:?}", user_info)
                    }
                }
            }
            Message::Close(_) => {}
            _ => {}
        }
    }
}

struct Room {
    users: Vec<User>,
    messages: Vec<MyMessage>,
}

impl Room {
    fn new() -> Self {
        Self {
            users: Default::default(),
            messages: vec![],
        }
    }

    fn join_user(&mut self, user: User) {
        self.users.push(user);
    }

    fn send(&mut self, msg: &MyMessage) {
        self.messages.push(msg.clone());
        self.users.iter().for_each(|user| user.send_msg(msg));
    }
}

struct User {
    send_ch: tokio::sync::mpsc::UnboundedSender<MyMessage>,
    user_info: UserInfo,
}
impl User {
    fn new(chan: tokio::sync::mpsc::UnboundedSender<MyMessage>) -> Self {
        Self {
            send_ch: chan,
            user_info: UserInfo {
                name: String::from("test"),
                room: String::from("test"),
            },
        }
    }
    fn send_msg(&self, msg: &MyMessage) {
        self.send_ch.send(msg.clone()).expect("send failed");
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase", content = "content")]
enum MyMessage {
    NewMsg(String),
    NewUser(UserInfo),
    UpdateUserList(Vec<String>),
    Unknown(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct UserInfo {
    name: String,
    room: String,
}
