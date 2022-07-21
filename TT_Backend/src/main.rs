use std::{env, io::Error};

use futures_util::{future, StreamExt, TryStreamExt};
use log::info;
use tokio::net::{TcpListener, TcpStream};

const WS_IP: &str = "127.0.0.1";
const WS_PORT: &str = "8080";

const TCP_IP: &str = "127.0.0.1";
const TCP_PORT: &str = "8080";

mod server;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Error> {

    //let server = server::Server::run(WS_IP, WS_PORT).await;



    let listener = init(WS_IP, WS_PORT).await;

    while let Ok((stream, _)) = listener.accept().await {
        tokio::spawn(accept_connection(stream));
    }

    Ok(())
}

async fn init(ip: &str, port: &str) -> TcpListener {
    // Initialize logger
    let _ = env_logger::try_init();

    // Create socket address
    let addr = env::args().nth(1).unwrap_or_else(|| (ip.to_owned()+":"+ port).to_string());

    // Create the event loop and TCP listener we'll accept connections on.
    let try_socket = TcpListener::bind(&addr).await;
    let listener = try_socket.expect("Failed to bind");
    info!("Listening on: {}", addr);
    println!("Listening on: {}", addr);

    listener
}

async fn accept_connection(stream: TcpStream) {
    let addr = stream.peer_addr().expect("connected streams should have a peer address");
    info!("Peer address: {}", addr);
    println!("Peer address: {}", addr);

    let ws_stream = tokio_tungstenite::accept_async(stream)
        .await
        .expect("Error during the websocket handshake occurred");

    info!("New WebSocket connection: {}", addr);
    println!("New WebSocket connection: {}", addr);

    let (write, read) = ws_stream.split();

    // TODO do something useful, for now: just read and send back
    // We should not forward messages other than text or binary.
    read.try_filter(|msg| future::ready(msg.is_text() || msg.is_binary()))
        .forward(write)
        .await
        .expect("Failed to forward messages")
}