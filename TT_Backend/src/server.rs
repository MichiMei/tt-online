//!
//! This is a async backend server.
//! It listens on one port for incoming websocket connections from clients using the WebApp and on
//! another port for incoming tcp connections by host(s) using the HostApp.
//! An arbitrary number of clients can connect to the server but only one host. If a new one tries
//! to connect, the old one gets disconnected (to prevent waiting for its timeout)
//!

use std::borrow::{Borrow, BorrowMut};
use std::collections::HashMap;
use std::net::SocketAddr;
use futures_util::stream::{SplitSink, SplitStream};
use log::{error, info, warn};
use tokio::net::TcpStream;
use tokio::net::tcp::OwnedWriteHalf;
use tokio::sync::mpsc;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::WebSocketStream;
use crate::server::messages::BackendMessage;
use crate::server::networking::tcp_sockets::{create_host_listener, host_close_connection, host_send_message, host_socket_reader};
use crate::server::networking::websockets::{client_socket_reader, client_close_connection, create_client_listener, client_send_message};

pub mod networking;
pub mod messages;

const CHANNEL_SIZE: usize = 16;

#[derive(Debug)]
pub enum InternalMessage {
    ClientConnected{write: SplitSink<WebSocketStream<TcpStream>, Message>, read: SplitStream<WebSocketStream<TcpStream>>, address: SocketAddr, name: String},
    ClientCloseConnection {address: SocketAddr, reason: &'static str},
    HostConnected{stream: TcpStream, address: SocketAddr},
    HostCloseConnection {address: SocketAddr, reason: &'static str},
    ClientInput{address: SocketAddr, content: String},
    HostUpdate{address : SocketAddr, content: String},
    HostChangeState{address : SocketAddr, content: String},
}

struct Client {
    name: String,
    write: SplitSink<WebSocketStream<TcpStream>, Message>,
}

pub async fn run(listen_ip: &str, web_socket_port: u16, tcp_port: u16) {
    // init logger
    let _ = env_logger::try_init();

    // create MPSC channel for internal communication
    let (tx, rx) = mpsc::channel(CHANNEL_SIZE);

    // create Websocket listener
    create_client_listener(tx.clone(), listen_ip, web_socket_port).await;

    // create TCP listener
    create_host_listener(tx.clone(), listen_ip, tcp_port).await;

    // create (and run) main task
    main_task(tx, rx).await;
}

async fn main_task(mut channel_tx: Sender<InternalMessage>, mut channel_rx: Receiver<InternalMessage>) {

    let mut clients = HashMap::new();
    let mut state = None;
    let mut host: Option<(SocketAddr, OwnedWriteHalf)> = None;

    loop {
        // receive next internal message
        let message = match channel_rx.recv().await {
            Some(v) => v,
            None => break
        };

        // TODO handle internal message
        match message {
            InternalMessage::ClientConnected { write, read, address, name} =>
                handle_client_connected(&mut channel_tx, &mut clients, state.clone(), write, read, address, name).await,
            InternalMessage::ClientCloseConnection { address, reason } =>
                handle_client_close_connection(&mut clients, address, reason).await,
            InternalMessage::HostConnected { stream, address} =>
                handle_host_connected(channel_tx.clone(), &mut host, stream, address).await,
            InternalMessage::HostCloseConnection { address, reason } =>
                handle_host_close_connection(&mut host, address, reason).await,
            InternalMessage::ClientInput{ address, content } =>
                handle_client_input(channel_tx.clone(), &mut host, clients.get_mut(&address), address, content).await,
            InternalMessage::HostUpdate{ address, content } =>
                handle_host_update(channel_tx.clone(), &host, &mut clients, address, content).await,
            InternalMessage::HostChangeState{ address, content } =>
                handle_host_change_state(channel_tx.clone(), &host, &mut clients, &mut state, address, content).await,
        }

    }

    // TODO cleanup
}

/// Handles the ClientConnected event
/// Tries to send current state to the client (closes connection if this fails)
/// Adds the client to the client container and starts its listener
async fn handle_client_connected(channel_tx: &mut Sender<InternalMessage>, clients: &mut HashMap<SocketAddr, Client>, state: Option<BackendMessage>, mut write: SplitSink<WebSocketStream<TcpStream>, Message>, read: SplitStream<WebSocketStream<TcpStream>>, address: SocketAddr, name: String) {
    info!("handle_client_connected(..): Client {} connected, name: {}", address, &name);

    // Send initial state
    if state.is_some() {
        match client_send_message(&mut write, state.unwrap()).await {
            Ok(_) => {}
            Err(e) => {
                error!("handle_client_connected(..): Sending initial state to client {} failed, closing connection!\nError: {:?}", address, e);
                client_close_connection(write, address, networking::DISCONNECT_REASON_SEND_FAILED).await;
                return
            }
        };
    }

    // Insert write to clients
    clients.insert(address, Client{ name, write });

    // Create listener
    tokio::spawn(client_socket_reader(channel_tx.clone(), read, address));

    // TODO Notify host about new client
    // send to host ClientConnected
}

/// Handles the ClientCloseConnection event
/// Removes the client from the container
/// Closes the connection to the client
async fn handle_client_close_connection(clients: &mut HashMap<SocketAddr, Client>, address: SocketAddr, reason: &str) {
    info!("handle_client_close_connection(..): Closing connection to client {}", address);

    // Remove client from the container
    let client = match clients.remove(&address) {
        Some(v) => v,
        None => {
            warn!("handle_client_close_connection(..): Client {} not in the list", address);
            return;
        }
    };

    // Close connection to the client
    client_close_connection(client.write, address, reason).await;

    // TODO Notify host about disconnected client
    // send to host ClientDisconnected
}

/// Handles the HostConnected event
/// Disconnects previous host (if existent)
/// Starts the host listener
async fn handle_host_connected(channel_tx: Sender<InternalMessage>, host: &mut Option<(SocketAddr, OwnedWriteHalf)>, stream: TcpStream, address: SocketAddr) {
    info!("handle_host_connected(..): Host {} connected", address);

    let (read, write) = stream.into_split();

    // Check if a host is already connected -> disconnect
    if host.is_some() {
        info!("handle_host_connected(..): Old host {} still connected. Disconnecting.", host.as_ref().unwrap().0);

        let prev_host = host.take().unwrap();

        host_close_connection(prev_host.1, prev_host.0, networking::DISCONNECT_REASON_HOST_OTHER).await;
    }
    assert!(host.is_none(), "handle_host_connected(..): Host should have been disconnected");

    // Spawn host listener
    tokio::spawn(host_socket_reader(channel_tx, read, address));

    // Set host
    *host = Some((address, write));
}

/// Handles the HostCloseConnection event
/// Closes the connection to the host
async fn handle_host_close_connection(host: &mut Option<(SocketAddr, OwnedWriteHalf)>, address: SocketAddr, reason: &str) {
    info!("handle_host_closed(..): Disconnecting host {}", address);
    if host.is_some() {
        let current_address = host.as_ref().unwrap().0;
        if address == current_address {
            info!("handle_host_closed(..): Is current host -> disconnecting");

            let old_host = host.take().unwrap();
            host_close_connection(old_host.1, address, reason).await;
            *host = None;
        }
    }
}

/// Forwards the clients Input to the host
/// Checks if the client and host are connected
async fn handle_client_input(channel_tx: Sender<InternalMessage>, host: &mut Option<(SocketAddr, OwnedWriteHalf)>, /*&mut clients: &HashMap<SocketAddr, Client>*/mut client: Option<&mut Client>, address: SocketAddr, content: String) {
    info!("handle_client_input(..): Client {} send input\nContent: {}", address, content);

    // Check if client is still connected
   /* if !clients.contains_key(&address) {
        warn!("handle_client_input(..): Client {} not contained in list -> ignoring input", address);
        return;
    }*/
    if client.is_none() {
        warn!("handle_client_input(..): Client {} not contained in list -> ignoring input", address);
        return;
    }

    // Check if host is connected
    if host.is_none() {
        warn!("handle_client_input(..): No host connected -> ignoring input");
        return;
    }

    // Send input to host
    let client = client.as_mut().unwrap();
    let name = &client.name;
    let write = host.as_mut().unwrap().1.borrow_mut();
    let result = host_send_message(write, BackendMessage::Input { input: content, name: String::from(name), address: address.to_string() }).await;
    let host_address = host.as_ref().unwrap().0.borrow();
    match result {
        Ok(_) => {}
        Err(e) => {
            error!("handle_client_input(..): Sending to host {} failed. Disconnecting\nError: {:?}", host_address, e);
            channel_tx.send(InternalMessage::HostCloseConnection { address: host_address.clone(), reason: networking::DISCONNECT_REASON_SEND_FAILED }).await.expect("handle_client_input(..): Sending internal message failed");
            return;
        }
    };
    assert!(host.is_some(), "Host must not be consumed");
}

/// Forwards the hosts Update to all connected clients
/// Checks if the Update was sent by the current host
async fn handle_host_update(channel_tx: Sender<InternalMessage>, host: &Option<(SocketAddr, OwnedWriteHalf)>, clients: &mut HashMap<SocketAddr, Client>, address: SocketAddr, content: String) {
    info!("handle_host_update(..): Host send update\nContent: {}", content);
    if host.is_none() || host.as_ref().unwrap().0 != address {
        warn!("handle_host_update(..): is not current Host -> ignoring update");
        return;
    }

    write_to_all_clients(channel_tx, clients, BackendMessage::Update { content }).await;
}

/// Forwards the hosts ChangeState to all connected clients
/// Checks if the ChangeState was sent by the current host
async fn handle_host_change_state(channel_tx: Sender<InternalMessage>, host: &Option<(SocketAddr, OwnedWriteHalf)>, clients: &mut HashMap<SocketAddr, Client>, state: &mut Option<BackendMessage>, address: SocketAddr, content: String) {
    info!("handle_host_change_state(..): Host send state change\nmsg: {}", content);
    if host.is_none() || host.as_ref().unwrap().0 != address {
        warn!("handle_host_change_state(..): is not current Host -> ignoring state change");
        return;
    }

    let backend_message = BackendMessage::ChangeState { content };

    *state = Some(backend_message.clone());

    write_to_all_clients(channel_tx, clients, backend_message).await;
}

/// Sends the given BackendMessage to all connected clients
/// If sending to a client fails, the client is disconnected
async fn write_to_all_clients(channel_tx: Sender<InternalMessage>, clients: &mut HashMap<SocketAddr, Client>, msg: BackendMessage) {
    for (cli_addr, client) in clients.iter_mut() {
        let write = client.write.borrow_mut();
        match client_send_message(write, msg.clone()).await {
            Ok(_) => {}
            Err(e) => {
                error!("write_to_all_clients(..): Sending update to client {} failed. Disconnecting\nError: {:?}", cli_addr, e);
                channel_tx.send(InternalMessage::ClientCloseConnection { address: cli_addr.clone(), reason: networking::DISCONNECT_REASON_SEND_FAILED }).await.expect(" write_to_all_clients(..): Sending internal message failed.")
            }
        };
    }
}