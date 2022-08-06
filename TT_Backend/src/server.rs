//!
//! This is a async backend server.
//! It listens on one port for incoming websocket connections from clients using the WebApp and on
//! another port for incoming tcp connections by host(s) using the HostApp.
//! An arbitrary number of clients can connect to the server but only one host. If a new one tries
//! to connect, the old one gets disconnected (to prevent waiting for its timeout)
//!

use std::collections::HashMap;
use std::net::SocketAddr;
use log::{info, warn};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio::sync::mpsc::{Receiver, Sender};
use crate::server::messages::BackendMessage;
use crate::server::networking::{ClientConnection, HostConnection};
use crate::server::networking::tcp_sockets::{create_host_listener, host_socket_reader};
use crate::server::networking::websockets::{client_socket_reader, create_client_listener, WsReadHalve};

pub mod networking;
pub mod messages;

pub struct Server {
    clients: HashMap<SocketAddr, ClientConnection>,
    host: Option<HostConnection>,
    state: Option<BackendMessage>,
    channel_rcv: Receiver<InternalMessage>,
    channel_snd: Sender<InternalMessage>,
}

impl Server {
    /// Creates a new Server
    pub fn new() -> Self {
        let _ = env_logger::try_init();
        let (tx, rx) = mpsc::channel(CHANNEL_SIZE);

        Server{
            clients: Default::default(),
            host: None,
            state: None,
            channel_rcv: rx,
            channel_snd: tx,
        }
    }

    /// Starts listening for incoming connections and handling internal messages
    pub async fn run(&mut self, listen_ip: &str, web_socket_port: u16, tcp_port: u16) {
        create_client_listener(self.get_channel_sender(), listen_ip, web_socket_port).await;
        create_host_listener(self.get_channel_sender(), listen_ip, tcp_port).await;
        self.run_main_handler().await;
    }

    /// Returns a (cloned) sending channel for internal messages
    /// Is used to enqueue tasks for the main handler
    pub fn get_channel_sender(&self) -> Sender<InternalMessage> {
        self.channel_snd.clone()
    }
}

impl Server {
    async fn run_main_handler(&mut self) {
        info!("run_main_handler(..): Started");
        while let Some(message) = self.channel_rcv.recv().await {
            self.handle_message(message).await;
        }
        info!("run_main_handler(..): All sending channel ends closed -> shutting down")
    }

    async fn handle_message(&mut self, message: InternalMessage) {
        match message {
            InternalMessage::ClientConnected {client, read} =>
                self.handle_client_connected(read, client).await,
            InternalMessage::ClientCloseConnection {address, reason} =>
                self.handle_client_close_connection(address, reason).await,
            InternalMessage::HostConnected {stream, address} =>
                self.handle_host_connected(stream, address).await,
            InternalMessage::HostCloseConnection {address, reason} =>
                self.handle_host_close_connection(address, reason).await,
            InternalMessage::ClientInput {address, content} =>
                self.handle_client_input(address, content).await,
            InternalMessage::HostUpdate {address, content} =>
                self.handle_host_update(address, content).await,
            InternalMessage::HostChangeState {address, content} =>
                self.handle_host_change_state(address, content).await,
        }

    }

    async fn handle_client_connected(&mut self, read: WsReadHalve, mut client: ClientConnection) {
        info!("handle_client_connected(..): Client {} connected, name: {}", client.get_address_as_str(), client.get_name());

        if self.state.is_some() {
            client.send_message(self.state.as_ref().unwrap().clone()).await;
        }

        self.notify_host_client_connected(&client).await;

        tokio::spawn(client_socket_reader(self.get_channel_sender(), read, client.get_address()));

        self.clients.insert(client.get_address(), client);
    }

    async fn notify_host_client_connected(&mut self, client: &ClientConnection) {
        if let Some(host) = self.host.as_mut() {
            let msg = BackendMessage::ClientConnected {
                name: String::from(client.get_name()),
                address: client.get_address_as_str()
            };
            host.send_message(msg).await;
        }
    }

    async fn handle_client_close_connection(&mut self, address: SocketAddr, reason: &str) {
        if let Some(client) = self.clients.remove(&address) {
            info!("handle_client_close_connection(..): Closing connection to client {} ({})\nReason: {}", client.get_name(), address, reason);

            self.notify_host_client_disconnected(&client, reason).await;

            client.close(reason).await;
        }
    }

    async fn notify_host_client_disconnected(&mut self, client: &ClientConnection, reason: &str) {
        if let Some(host) = self.host.as_mut() {
            let msg = BackendMessage::ClientDisconnected {
                name: String::from(client.get_name()),
                address: client.get_address_as_str(),
                reason: String::from(reason)
            };
            host.send_message(msg).await;
        }
    }

    async fn handle_host_connected(&mut self, stream: TcpStream, address: SocketAddr) {
        info!("handle_host_connected(..): Host {} connected", address);

        let (read_half, write_half) = stream.into_split();

        if let Some(host) = self.host.take() {
            info!("handle_host_connected(..): Old host {} still connected. Disconnecting.", host.get_address());
            host.close(networking::DISCONNECT_REASON_HOST_OTHER).await;
        }
        assert!(self.host.is_none(), "handle_host_connected(..): Host should have been consumed");

        tokio::spawn(host_socket_reader(self.get_channel_sender(), read_half, address));

        self.host = Some(HostConnection::new(address, write_half, self.get_channel_sender()));
    }

    async fn handle_host_close_connection(&mut self, address: SocketAddr, reason: &str) {
        if self.host.is_some() {
            if self.host.as_ref().unwrap().get_address() == address {
                info!("handle_host_closed(..): Disconnecting host {}\nReason: {}", address, reason);

                self.host.take().unwrap().close(reason).await;

                assert!(self.host.is_none(), "handle_host_closed(..): Host should have been consumed");
            }
        }
    }

    async fn handle_client_input(&mut self, address: SocketAddr, content: String) {
        if let Some(client) = self.clients.get(&address) {
            if let Some(host) = self.host.as_mut() {
                info!("handle_client_input(..): Client {} ({}) send input\nContent: {}", client.get_name(), address, content);

                let msg = BackendMessage::Input {
                    input: content,
                    name: String::from(client.get_name()),
                    address: address.to_string()
                };
                host.send_message(msg).await;
            }
        }
    }

    async fn handle_host_update(&mut self, address: SocketAddr, content: String) {
        if let Some(host) = self.host.as_ref() {
            if host.get_address() == address {
                if self.clients.is_empty() {
                    warn!("handle_host_update(..): No clients connected");
                } else {
                    info!("handle_host_update(..): Host {} send update\nContent: {}", host.get_address(), content);
                    let msg = BackendMessage::Update {content};
                    self.write_to_all_clients(msg).await;
                }
            }
        }
    }

    async fn handle_host_change_state(&mut self, address: SocketAddr, content: String) {
        if let Some(host) = self.host.as_ref() {
            if host.get_address() == address {
                info!("handle_host_change_state(..): Host {} send change state\nContent: {}", host.get_address(), content);
                let msg = BackendMessage::ChangeState {content};

                self.state = Some(msg.clone());

                if self.clients.is_empty() {
                    warn!("handle_host_change_state(..): No clients connected");
                } else {
                    self.write_to_all_clients(msg).await;
                }
            }
        }
    }

    async fn write_to_all_clients(&mut self, msg: BackendMessage) {
        for (_, client) in self.clients.iter_mut() {
            client.send_message(msg.clone()).await;
        }
    }

}


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
