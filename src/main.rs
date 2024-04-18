mod users;
use axum::{
    debug_handler,
    extract::{
        ws::{Message, WebSocket},
        Query, State, WebSocketUpgrade,
    },
    response::IntoResponse,
    routing::get,
    Router,
};
use axum_extra::{headers, TypedHeader};
use core::panic;
use futures::{sink::SinkExt, stream::StreamExt};
use serde::{Deserialize, Serialize};
use std::{net::SocketAddr, sync::Arc};
use tokio::sync::broadcast;
use tower_http::{services::ServeDir, trace::TraceLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() {
    // let room = Arc::new(Mutex::new(Room::new()));
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "example_websockets=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();
    let addr = SocketAddr::from(([127, 0, 0, 1], 8080));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();

    let broadcast_chan = Arc::new(broadcast::channel::<ChatMessage>(10));
    let app = Router::new()
        .fallback_service(ServeDir::new("public"))
        .route("/ws", get(ws_handler))
        .with_state(broadcast_chan);

    axum::serve(listener, app.layer(TraceLayer::new_for_http()))
        .await
        .unwrap();
}

type RoomBroadcast = (
    broadcast::Sender<ChatMessage>,
    broadcast::Receiver<ChatMessage>,
);

#[debug_handler]
async fn ws_handler(
    State(room_broadcast): State<Arc<RoomBroadcast>>,
    ws: WebSocketUpgrade,
    Query(user_info): Query<UserInfo>,
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
    let sender = room_broadcast.0.clone();
    let receiver = room_broadcast.0.subscribe();

    ws.on_upgrade(|socket| handle_socket(socket, (sender, receiver), user_info))
}

async fn handle_socket(socket: WebSocket, broadcast: RoomBroadcast, user_info: UserInfo) {
    let (bc_send, mut bc_recv) = broadcast;
    let (mut socket_send, mut socket_recv) = socket.split();

    tokio::spawn(async move {
        while let Ok(msg) = bc_recv.recv().await {
            let _ = socket_send
                .send(Message::Text(
                    serde_json::to_string(&msg).expect("Failed to serialize MyMessage"),
                ))
                .await;
        }
    });

    while let Some(msg) = socket_recv.next().await {
        println!("{:?}", msg);
        let msg = msg.expect("deserialize message");
        match msg {
            Message::Text(msg_txt) => {
                println!("{:?}", msg_txt);
                let _ = bc_send.send(ChatMessage {
                    text: msg_txt,
                    from: String::from(&user_info.name),
                });
            }
            Message::Close(_) => {}
            _ => {}
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct UserInfo {
    name: String,
    room: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "content")]
enum BroadcastMessage {
    ChatMessage,
    UpdateUserList,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ChatMessage {
    text: String,
    from: String,
}
