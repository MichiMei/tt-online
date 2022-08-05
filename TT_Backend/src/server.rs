//!
//! This is a async backend server.
//! It listens on one port for incoming websocket connections from clients using the WebApp and on
//! another port for incoming tcp connections by host(s) using the HostApp.
//! An arbitrary number of clients can connect to the server but only one host. If a new one tries
//! to connect, the old one gets disconnected (to prevent waiting for its timeout)
//!

use std::collections::HashMap;
use std::net::SocketAddr;
use futures_util::stream::SplitStream;
use log::{error, info, warn};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio_tungstenite::WebSocketStream;
use crate::server::messages::BackendMessage;
use crate::server::networking::{ClientConnection, DISCONNECT_REASON_SEND_FAILED, HostConnection};
use crate::server::networking::tcp_sockets::{create_host_listener, host_socket_reader};
use crate::server::networking::websockets::{client_socket_reader, create_client_listener};
use crate::server::networking::websockets::WsReadHalve;

pub mod networking;
pub mod messages;

const CHANNEL_SIZE: usize = 16;

#[derive(Debug)]
pub enum InternalMessage {
    ClientConnected{read: WsReadHalve, client: ClientConnection},
    ClientCloseConnection {address: SocketAddr, reason: &'static str},
    HostConnected{stream: TcpStream, address: SocketAddr},
    HostCloseConnection {address: SocketAddr, reason: &'static str},
    ClientInput{address: SocketAddr, content: String},
    HostUpdate{address : SocketAddr, content: String},
    HostChangeState{address : SocketAddr, content: String},
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
    let mut host: Option<HostConnection> = None;

    loop {
        // receive next internal message
        let message = match channel_rx.recv().await {
            Some(v) => v,
            None => break
        };

        match message {
            InternalMessage::ClientConnected { read, client } =>
                handle_client_connected(&mut host, &mut channel_tx, &mut clients, state.clone(), read, client).await,
            InternalMessage::ClientCloseConnection { address, reason } =>
                handle_client_close_connection(&mut host, &mut channel_tx, &mut clients, address, reason).await,
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
async fn handle_client_connected(host: &mut Option<HostConnection>, channel_tx: &mut Sender<InternalMessage>, clients: &mut HashMap<SocketAddr, ClientConnection>, state: Option<BackendMessage>, read: WsReadHalve, mut client: ClientConnection) {
     info!("handle_client_connected(..): Client {} connected, name: {}", client.get_address_as_str(), client.get_name());

    // Send initial state
    if state.is_some() {
        match client.send_message(state.unwrap()).await {
            Ok(_) => {}
            Err(e) => {
                error!("handle_client_connected(..): Sending initial state to client {} failed, closing connection!\nError: {:?}", client.get_address_as_str(), e);
                client.close(DISCONNECT_REASON_SEND_FAILED).await;
                return
            }
        };
    }

    // Notify host about new client
    if host.is_some() {
        let host = host.as_mut().unwrap();
        let msg = BackendMessage::ClientConnected { name: String::from(client.get_name()), address: client.get_address_as_str() };
        match host.send_message(msg).await {
            Ok(_) => {}
            Err(e) => {
                error!("handle_client_connected(..): Sending message to host {} failed, Disconnecting!\nError: {}", host.get_address(), e);
                channel_tx.send(InternalMessage::HostCloseConnection {address: host.get_address(), reason: DISCONNECT_REASON_SEND_FAILED}).await.expect("handle_client_connected(..): Sending internal message failed");
            }
        };
    }

    // Create listener
    tokio::spawn(client_socket_reader(channel_tx.clone(), read, client.get_address()));

    // Insert client to clients collection
    clients.insert(client.get_address(), client);
}

/// Handles the ClientCloseConnection event
/// Removes the client from the container
/// Closes the connection to the client
async fn handle_client_close_connection(host: &mut Option<HostConnection>, channel_tx: &mut Sender<InternalMessage>, clients: &mut HashMap<SocketAddr, ClientConnection>, address: SocketAddr, reason: &str) {
    info!("handle_client_close_connection(..): Closing connection to client {}", address);

    // Remove client from the container
    let client = match clients.remove(&address) {
        Some(v) => v,
        None => {
            warn!("handle_client_close_connection(..): Client {} not in the list", address);
            return;
        }
    };

    // Notify host about disconnected client
    if host.is_some() {
        let host = host.as_mut().unwrap();
        let msg = BackendMessage::ClientDisconnected { name: String::from(client.get_name()), address: address.to_string(), reason: String::from(reason) };
        match host.send_message(msg).await {
            Ok(_) => {}
            Err(e) => {
                error!("handle_client_connected(..): Sending message to host {} failed, Disconnecting!\nError: {}", host.get_address(), e);
                channel_tx.send(InternalMessage::HostCloseConnection {address: host.get_address(), reason: DISCONNECT_REASON_SEND_FAILED}).await.expect("handle_client_connected(..): Sending internal message failed");
            }
        };
    }

    // Close connection to the client
    client.close(reason).await;
}

/// Handles the HostConnected event
/// Disconnects previous host (if existent)
/// Starts the host listener
async fn handle_host_connected(channel_tx: Sender<InternalMessage>, host: &mut Option<HostConnection>, stream: TcpStream, address: SocketAddr) {
    info!("handle_host_connected(..): Host {} connected", address);

    let (read, write) = stream.into_split();

    // Check if a host is already connected -> disconnect
    if host.is_some() {
        info!("handle_host_connected(..): Old host {} still connected. Disconnecting.", host.as_ref().unwrap().get_address());

        let prev_host = host.take().unwrap();

        //host_close_connection(prev_host.1, prev_host.0, networking::DISCONNECT_REASON_HOST_OTHER).await;
        prev_host.close(networking::DISCONNECT_REASON_HOST_OTHER).await;
    }
    assert!(host.is_none(), "handle_host_connected(..): Host should have been disconnected");

    // Spawn host listener
    tokio::spawn(host_socket_reader(channel_tx, read, address));

    // Set host
    *host = Some(HostConnection::new(address, write));
}

/// Handles the HostCloseConnection event
/// Closes the connection to the host
async fn handle_host_close_connection(host: &mut Option<HostConnection>, address: SocketAddr, reason: &str) {
    info!("handle_host_closed(..): Disconnecting host {}", address);
    if host.is_some() {
        let current_address = host.as_ref().unwrap().get_address();
        if address == current_address {
            info!("handle_host_closed(..): Is current host -> disconnecting");

            let old_host = host.take().unwrap();
            old_host.close(reason).await;
            assert!(host.is_none(), "handle_host_closed(..): Host should have been consumed")
        }
    }
}

/// Forwards the clients Input to the host
/// Checks if the client and host are connected
async fn handle_client_input(channel_tx: Sender<InternalMessage>, host: &mut Option<HostConnection>, mut client: Option<&mut ClientConnection>, address: SocketAddr, content: String) {
    info!("handle_client_input(..): Client {} send input\nContent: {}", address, content);

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
    let name = client.get_name();
    let msg = BackendMessage::Input { input: content, name: String::from(name), address: address.to_string() };
    let result = host.as_mut().unwrap().send_message(msg).await;
    let host_address = host.as_ref().unwrap().get_address();
    match result {
        Ok(_) => {}
        Err(e) => {
            error!("handle_client_input(..): Sending to host {} failed. Disconnecting\nError: {:?}", host_address, e);
            channel_tx.send(InternalMessage::HostCloseConnection { address: host_address.clone(), reason: DISCONNECT_REASON_SEND_FAILED }).await.expect("handle_client_input(..): Sending internal message failed");
            return;
        }
    };
    assert!(host.is_some(), "Host must not be consumed");
}

/// Forwards the hosts Update to all connected clients
/// Checks if the Update was sent by the current host
async fn handle_host_update(channel_tx: Sender<InternalMessage>, host: &Option<HostConnection>, clients: &mut HashMap<SocketAddr, ClientConnection>, address: SocketAddr, content: String) {
    info!("handle_host_update(..): Host send update\nContent: {}", content);
    if host.is_none() || host.as_ref().unwrap().get_address() != address {
        warn!("handle_host_update(..): is not current Host -> ignoring update");
        return;
    }

    write_to_all_clients(channel_tx, clients, BackendMessage::Update { content }).await;
}

/// Forwards the hosts ChangeState to all connected clients
/// Checks if the ChangeState was sent by the current host
async fn handle_host_change_state(channel_tx: Sender<InternalMessage>, host: &Option<HostConnection>, clients: &mut HashMap<SocketAddr, ClientConnection>, state: &mut Option<BackendMessage>, address: SocketAddr, content: String) {
    info!("handle_host_change_state(..): Host send state change\nmsg: {}", content);
    if host.is_none() || host.as_ref().unwrap().get_address() != address {
        warn!("handle_host_change_state(..): is not current Host -> ignoring state change");
        return;
    }

    let backend_message = BackendMessage::ChangeState { content };

    *state = Some(backend_message.clone());

    write_to_all_clients(channel_tx, clients, backend_message).await;
}

/// Sends the given BackendMessage to all connected clients
/// If sending to a client fails, the client is disconnected
async fn write_to_all_clients(channel_tx: Sender<InternalMessage>, clients: &mut HashMap<SocketAddr, ClientConnection>, msg: BackendMessage) {
    for (cli_addr, client) in clients.iter_mut() {
        match client.send_message(msg.clone()).await {
            Ok(_) => {}
            Err(e) => {
                error!("write_to_all_clients(..): Sending update to client {} failed. Disconnecting\nError: {:?}", cli_addr, e);
                channel_tx.send(InternalMessage::ClientCloseConnection { address: cli_addr.clone(), reason: DISCONNECT_REASON_SEND_FAILED }).await.expect(" write_to_all_clients(..): Sending internal message failed.")
            }
        };
    }
}