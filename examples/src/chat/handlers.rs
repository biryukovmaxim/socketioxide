use std::sync::Arc;

use serde::Deserialize;
use socketioxide::{adapter::LocalAdapter, Socket};
use tracing::info;

#[derive(Deserialize, Clone, Debug)]
struct Nickname(String);

#[derive(Deserialize)]
struct Auth {
    pub nickname: Nickname,
}

pub async fn handler(socket: Arc<Socket<LocalAdapter>>) {
    info!("Socket connected on / with id: {}", socket.sid);
    if let Ok(data) = socket.handshake.data::<Auth>() {
        info!("Nickname: {:?}", data.nickname);
        socket.extensions.insert(data.nickname);
        socket.emit("message", "Welcome to the chat!").ok();
        socket.join("default");
    } else {
        info!("No nickname provided, disconnecting...");
        socket.disconnect().ok();
        return;
    }

    socket.on(
        "message",
        |socket, (room, message): (String, String), _, _| async move {
            let Nickname(ref nickname) = *socket.extensions.get().unwrap();
            info!("transfering message from {nickname} to {room}: {message}");
            info!("Sockets in room: {:?}", socket.local().sockets());
            if let Some(dest) = socket.to("default").sockets().iter().find(|s| {
                s.extensions
                    .get::<Nickname>()
                    .map(|n| n.0 == room)
                    .unwrap_or_default()
            }) {
                info!("Sending message to {}", room);
                dest.emit("message", format!("{}: {}", nickname, message))
                    .ok();
            }

            socket
                .to(room)
                .emit("message", format!("{}: {}", nickname, message))
                .ok();
        },
    );

    socket.on("join", |socket, room: String, _, _| async move {
        info!("Joining room {}", room);
        socket.join(room);
    });

    socket.on("leave", |socket, room: String, _, _| async move {
        info!("Leaving room {}", room);
        socket.leave(room);
    });

    socket.on("list", |socket, room: Option<String>, _, _| async move {
        if let Some(room) = room {
            info!("Listing sockets in room {}", room);
            let sockets = socket
                .within(room)
                .sockets()
                .iter()
                .filter_map(|s| s.extensions.get::<Nickname>())
                .fold("".to_string(), |a, b| a + &b.0 + ", ")
                .trim_end_matches(", ")
                .to_string();
            socket.emit("message", sockets).ok();
        } else {
            let rooms = socket.rooms();
            info!("Listing rooms: {:?}", &rooms);
            socket.emit("message", rooms).ok();
        }
    });

    socket.on("nickname", |socket, nickname: Nickname, _, _| async move {
        let previous = socket.extensions.insert(nickname.clone());
        info!("Nickname changed from {:?} to {:?}", &previous, &nickname);
        let msg = format!(
            "{} changed his nickname to {}",
            previous.map(|n| n.0).unwrap_or_default(),
            nickname.0
        );
        socket.to("default").emit("message", msg).ok();
    });
}
