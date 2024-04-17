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
    let (ch_send, mut ch_recv) = tokio::sync::mpsc::unbounded_channel::<MyMessage>();
    let (mut socket_send, mut socket_recv) = socket.split();
    tokio::spawn(async move {
        while let Some(msg) = ch_recv.recv().await {
            match msg {
                MyMessage::NewMsg(msg) => {
                    socket_send
                        .send(Message::Text(format!("newMsg, {msg}")))
                        .await
                        .expect("Failed to send");
                }
                _ => todo!(),
            }
        }
    });
    let user = User::new(ch_send);
    room.lock()
        .expect("other thread panicked while holding loc")
        .join_user(user);
    loop {
        match socket_recv
            .next()
            .await
            .expect("empty message")
            .expect("deserialize message")
        {
            Message::Text(msg_txt) => match MyMessage::from(msg_txt) {
                MyMessage::NewMsg(new_msg_txt) => {
                    println!("{:?}", new_msg_txt.clone());
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
            },
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
}
impl User {
    fn new(chan: tokio::sync::mpsc::UnboundedSender<MyMessage>) -> Self {
        Self { send_ch: chan }
    }
    fn send_msg(&self, msg: &MyMessage) {
        if (self.send_ch.send(msg.clone())).is_ok() {
            println!("sent msg: {:?}", msg.clone());
        }
    }
}

#[derive(Debug, Clone)]
enum MyMessage {
    NewMsg(String),
    UpdateUserList(Vec<String>),
    Unknown(String),
}

impl From<String> for MyMessage {
    fn from(value: String) -> Self {
        if value.starts_with("msg,") {
            return Self::NewMsg(
                value
                    .split_once("msg,")
                    .expect("message starts with `msg`")
                    .1
                    .to_string(),
            );
        }
        Self::Unknown(value)
    }
}
