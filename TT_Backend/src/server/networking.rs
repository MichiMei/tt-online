#![allow(dead_code)]

use std::io::Error;
use std::net::SocketAddr;
use futures_util::stream::SplitSink;
use tokio::net::tcp::OwnedWriteHalf;
use tokio::net::TcpStream;
use tokio_native_tls::native_tls::TlsStream;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::WebSocketStream;
use crate::server::messages::BackendMessage;
use crate::server::networking::tcp_sockets::{host_close_connection, host_send_message};
use crate::server::networking::websockets::{client_close_connection, client_send_message, WsWriteHalve};

pub const DISCONNECT_REASON_CLI_CLOSED_GRACEFULLY: &str = "Connection closed gracefully by client";
pub const DISCONNECT_REASON_CLI_CLOSED_FORCEFULLY: &str = "Connection closed forcefully by client";
pub const DISCONNECT_REASON_HOST_CLOSED_GRACEFULLY: &str = "Connection closed gracefully by host";
pub const DISCONNECT_REASON_HOST_CLOSED_FORCEFULLY: &str = "Connection closed forcefully by host";
pub const DISCONNECT_REASON_HOST_OTHER: &str = "Another host connected";
pub const DISCONNECT_REASON_VIOLATION: &str = "Protocol violation";
pub const DISCONNECT_REASON_SEND_FAILED: &str = "Sending failed";

#[derive(Debug)]
pub struct HostConnection {
    address: SocketAddr,
    write: OwnedWriteHalf,
}

impl HostConnection {
    pub fn get_address(&self) -> SocketAddr {
        self.address.clone()
    }

    pub fn get_address_as_str(&self) -> String {
        self.address.to_string()
    }

    pub async fn send_message(&mut self, msg: BackendMessage) -> Result<(), Error> {
        host_send_message(&mut self.write, msg).await
    }

    pub async fn close(self, reason: &str) {
        host_close_connection(self.write, self.address, reason).await
    }

    pub fn new(address: SocketAddr, write: OwnedWriteHalf) -> Self {
        HostConnection{address, write }
    }
}


#[derive(Debug)]
pub struct ClientConnection {
    name: String,
    address: SocketAddr,
    write: WsWriteHalve,
}

impl ClientConnection {
    pub fn get_address(&self) -> SocketAddr {
        self.address.clone()
    }

    pub fn get_address_as_str(&self) -> String {
        self.address.to_string()
    }

    pub fn get_name(&self) -> &str {
        &self.name
    }

    pub async fn send_message(&mut self, msg: BackendMessage) -> Result<(), tokio_tungstenite::tungstenite::Error> {
        client_send_message(&mut self.write, msg).await
    }

    pub async fn close(self, reason: &str) {
        client_close_connection(self.write, self.address, reason).await
    }

    pub fn new(name: String, address: SocketAddr, write: WsWriteHalve) -> Self {
        ClientConnection{ name, address, write }
    }
}

/// Useful functions to interact with clients connected via websocket
pub mod websockets {
    use std::net::SocketAddr;
    use std::sync::Arc;
    use futures_util::stream::{SplitSink, SplitStream};
    use futures_util::{SinkExt, StreamExt};
    use log::{error, info, warn};
    use tokio::fs::File;
    use tokio::io::AsyncReadExt;
    use tokio::net::{TcpListener, TcpStream};
    use tokio::sync::mpsc::Sender;
    use tokio_native_tls::native_tls::{Identity, TlsAcceptor, TlsStream};
    use tokio_tungstenite::tungstenite::{Error, Message};
    use tokio_tungstenite::WebSocketStream;
    use crate::server::InternalMessage;
    use crate::server::messages::{BackendMessage, ClientMessage, encode_backend_msg, parse_client_msg};
    use crate::server::networking::{ClientConnection, DISCONNECT_REASON_CLI_CLOSED_FORCEFULLY, DISCONNECT_REASON_CLI_CLOSED_GRACEFULLY, DISCONNECT_REASON_VIOLATION};

    // #[cfg(not(feature = "insecure_ws"))]
    pub type TcpOrTlsStream = tokio_native_tls::TlsStream<TcpStream>;
    // #[cfg(feature = "insecure_ws")]
    // pub type TcpOrTlsStream = TcpStream;
    pub type WsReadHalve = SplitStream<WebSocketStream<TcpOrTlsStream>>;
    pub type WsWriteHalve = SplitSink<WebSocketStream<TcpOrTlsStream>, Message>;


    /// Create a listener on the websocket port waiting for client connections
    pub async fn create_client_listener(channel: Sender<InternalMessage>, ip: &str, port: u16) {
        // Websocket address
        let addr = (ip.to_owned()+":"+ &*port.to_string()).to_string();

        // TCP listener
        let listener = TcpListener::bind(&addr).await.expect("create_client_listener(..): Creating tcp listener failed");
        info!("create_client_listener(..): Listening for clients on {}", addr);

        // Spawn listener
        tokio::spawn(listen(channel, listener));
    }

    // #[cfg(not(feature = "insecure_ws"))]
    async fn create_tls_acceptor() -> Arc<tokio_native_tls::TlsAcceptor> {
        // TODO error handling
        let mut cert_file = File::open("res/cert/cert.pem").await.unwrap();
        let mut cert_data = vec![];
        let x = cert_file.read_to_end(&mut cert_data).await.unwrap();
        info!("create_tls_acceptor(..): reading cert successful, {} bytes", x);

        let mut key_file = File::open("res/cert/key.pem").await.unwrap();
        let mut key_data = vec![];
        let x = key_file.read_to_end(&mut key_data).await.unwrap();
        info!("create_tls_acceptor(..): reading key successful, {} bytes", x);

        let identity = Identity::from_pkcs8(&cert_data, &key_data).unwrap();

        //let acceptor = TlsAcceptor::new(identity).unwrap();
        let acceptor = tokio_native_tls::TlsAcceptor::from(native_tls::TlsAcceptor::builder(identity).build().unwrap());

        info!("worked!");

        Arc::new(acceptor)
    }

    // #[cfg(not(feature = "insecure_ws"))]
    async fn listen(channel: Sender<InternalMessage>, listener: TcpListener) {
        let tls_acceptor = create_tls_acceptor().await;

        // Listen forever
        loop {
            let (stream, address) = match listener.accept().await {
                Ok(v) => v,
                Err(e) => {
                    warn!("listen(..): Could not accept connection\nError: {}", e);
                    continue
                },
            };

            let tls_acceptor = tls_acceptor.clone();
            let x = match tls_acceptor.accept(stream).await {
                Ok(v) => v,
                Err(e) => {
                    warn!("listen(..): Could not accept TLS connection\nError: {}", e);
                    continue
                },
            };

            client_connecting(channel.clone(), x, address).await;
        }
    }

    /// Waiting for incoming connections
    /// Incoming connections are forwarded to upgrade and login the client
    // #[cfg(feature = "insecure_ws")]
    /*async fn listen(channel: Sender<InternalMessage>, listener: TcpListener) {
        // TODO nice terminate

        if cfg!(not(feature = "insecure_ws")) {
            let x = create_tls_acceptor().await;
        }

        // Listen forever
        loop {
            // Get next client
            let (stream, address) = match listener.accept().await {
                Ok(v) => v,
                Err(e) => {
                    warn!("listen(..): Could not accept connection\nError: {}", e);
                    continue
                },
            };

            // Forward client for socket upgrade and login
            info!("listen(..): Client {} accepted", address);
            client_connecting(channel.clone(), stream, address).await;
        }
    }*/

    /// Upgrade client connection and login
    /// First upgrades the connection to websocket
    /// Then waits for a 'ClientLogin' message, all messages before will be dropped (except Disconnect)
    /// Once the login is successful triggers the 'ClientConnected' event
    async fn client_connecting(channel: Sender<InternalMessage>, stream: TcpOrTlsStream, address: SocketAddr) {
        info!("client_connecting(..): Client {} connected", address);

        // Upgrade to websocket
        let ws_stream = match tokio_tungstenite::accept_async(stream).await {
            Ok(v) => v,
            Err(e) => {
                error!("client_connecting(..): Websocket handshake failed\nclient: {}\nmsg: {:?}", address, e);
                return
            }
        };
        let (ws_write, mut ws_read) = ws_stream.split();
        info!("client_connecting(..): Client {} upgraded to websocket", address);

        // Waiting for login
        loop {
            // Get next message
            let tmp_msg = match client_get_next_json(&mut ws_read, address).await {
                None => {
                    error!("client_connecting(..): Client {} closed connection. Closing connection.", address);
                    client_close_connection(ws_write, address, DISCONNECT_REASON_CLI_CLOSED_FORCEFULLY).await;
                    return
                }
                Some(v) => v
            };

            match tmp_msg {
                ClientMessage::ClientLogin {name} => {
                    info!("client_connecting(..): Client {} sent 'ClientLogin'.", address);
                    let client = ClientConnection::new(name, address, ws_write);
                    channel.send(InternalMessage::ClientConnected{read: ws_read, client}).await.expect("client_connecting(..): Sending internal message failed!");
                    return
                }
                ClientMessage::Disconnect {reason} => {
                    info!("client_connecting(..): Client {} send 'Disconnecting'. Closing connection!\nReason: {}", address, reason);
                    client_close_connection(ws_write, address, DISCONNECT_REASON_CLI_CLOSED_GRACEFULLY).await;
                    return
                }
                _ => {
                    warn!("client_connecting(..): Client {} send wrong message, expecting 'ClientLogin'.\nMessage: {}", address, tmp_msg);
                }
            }
        }
    }

    /// Returns the next parsable json message
    /// Will drop non-text or malformed messages
    pub async fn client_get_next_json(reader: &mut WsReadHalve, address: SocketAddr) -> Option<ClientMessage> {
        // TODO find out how closed behaviour and return None
        loop {
            // Get next message
            let msg = match reader.next().await {
                None => {
                    error!("client_get_next_json(..): Reader returned None. Probably closed?\nClient: {}", address);
                    return None
                }
                Some(Ok(v)) => v,
                Some(Err(e)) => {
                    error!("client_get_next_json(..): Reader returned Err. Client: {}\nError: {:?}", address, e);
                    continue
                }
            };

            // Check if message is text
            if !msg.is_text() {
                error!("client_get_next_json(..): Message by client {} is not text. Dropping!\nMessage: {}", address, msg);
                continue
            }

            // Parse message
            let parsed = match parse_client_msg(&msg.clone().into_text().unwrap()) {
                None => {
                    error!("client_get_next_json(..): Message by client {} is no valid json. Dropping!\nMessage: {}", address, msg);
                    continue
                }
                Some(v) => v
            };

            return Some(parsed)
        }
    }

    /// Closes the connection, ignoring possible errors
    pub async fn client_close_connection(mut writer: WsWriteHalve, address: SocketAddr, reason: &str) {
        let reason = String::from(reason);
        match client_send_message(&mut writer, BackendMessage::Disconnect {reason}).await {
            Ok(_) => {}
            Err(e) => {
                warn!("client_close_connection(..): Sending 'Disconnecting' to client {} failed!\nError: {:?}", address, e);
            }
        };
        match writer.close().await {
            Ok(_) => {}
            Err(e) => {
                error!("client_close_connection(..): Closing connection to client {} failed!\nError: {:?}", address, e);
            }
        }
    }

    /// Send the BackendMessage to the client (connected to the given websocket)
    /// Transforms the BackendMessage to the correct format.
    /// Forwards any sending errors
    pub async fn client_send_message(writer: &mut WsWriteHalve, msg_enum: BackendMessage) -> Result<(), Error> {
        let msg_str = encode_backend_msg(msg_enum);
        let msg = Message::from(msg_str);
        writer.send(msg).await
    }

    /// Reads all messages from the given socket
    /// Each valid message triggers the according event
    pub async fn client_socket_reader(channel: Sender<InternalMessage>, mut reader: WsReadHalve, address: SocketAddr) {
        // Read forever (until closed by client)
        loop {
            // Get next message
            let msg = match client_get_next_json(&mut reader, address).await {
                None => {
                    warn!("client_socket_reader(..): Client {} closed the connection. Closing connection.", address);
                    channel.send(InternalMessage::ClientCloseConnection {address, reason: DISCONNECT_REASON_CLI_CLOSED_FORCEFULLY}).await.expect("websocket_listen(..): Sending internal message failed!");
                    return
                }
                Some(v) => v
            };

            match msg {
                ClientMessage::ClientLogin { .. } => {
                    error!("client_socket_reader(..): Received unexpected 'ClientLogin' from {}. Closing connection!", address);
                    channel.send(InternalMessage::ClientCloseConnection {address, reason: DISCONNECT_REASON_VIOLATION }).await.expect("client_socket_reader(..): Sending internal message failed!");
                    return;
                }
                ClientMessage::Disconnect {reason} => {
                    info!("client_socket_reader(..): Client {} closed the connection. Closing connection.\nReason: {}", address, reason);
                    channel.send(InternalMessage::ClientCloseConnection {address, reason: DISCONNECT_REASON_CLI_CLOSED_GRACEFULLY}).await.expect("websocket_listen(..): Sending internal message failed!");
                    return;
                }
                ClientMessage::Input {content} => {
                    channel.send(InternalMessage::ClientInput {address, content}).await.expect("client_socket_reader(..): Sending internal message failed");
                }
            }
        }
    }

}

/// Useful functions to interact with hosts connected via tcp socket
pub mod tcp_sockets {
    use std::io::Error;
    use std::io::ErrorKind::ConnectionReset;
    use std::net::SocketAddr;
    use log::{error, info, warn};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
    use tokio::net::TcpListener;
    use tokio::sync::mpsc::Sender;
    use crate::server::InternalMessage;
    use crate::server::messages::{BackendMessage, encode_backend_msg, HostMessage, parse_host_msg};
    use crate::server::networking::{DISCONNECT_REASON_HOST_CLOSED_FORCEFULLY, DISCONNECT_REASON_HOST_CLOSED_GRACEFULLY};

    /// Create a listener on the tcp port waiting for host(s) connection(s)
    pub async fn create_host_listener(channel: Sender<InternalMessage>, ip: &str, port: u16) {
        // TCP address
        let addr = (ip.to_owned()+":"+ &*port.to_string()).to_string();

        // TCP listener
        let listener = TcpListener::bind(&addr).await.expect("create_host_listener(..): Creating tcp listener failed");
        info!("create_host_listener(..): Listening for host(s) on {}", addr);

        // Spawn listener
        tokio::spawn(listen(channel, listener));
    }

    /// Waiting for incoming connections
    /// Incoming connections trigger the 'HostConnected' event
    async fn listen(channel: Sender<InternalMessage>, listener: TcpListener) {
        // TODO nice terminate

        // Listen forever
        loop {
            // Get next host
            let (stream, address) = match listener.accept().await {
                Ok(v) => v,
                Err(e) => {
                    warn!("listen(..): Could not accept connection\nError: {}", e);
                    continue
                },
            };

            // Trigger HostConnected Event
            channel.send(InternalMessage::HostConnected{stream, address}).await.expect("listen(..): Sending internal message failed!");
        }
    }

    /// Returns the next parsable json message
    /// Will drop malformed messages
    pub async fn host_get_next_json(reader: &mut OwnedReadHalf, address: SocketAddr) -> Option<HostMessage> {
        loop {
            // Read length
            let length = match reader.read_u32().await {
                Ok(v) => v,
                Err(e) => {
                    if e.kind() == ConnectionReset {
                        info!("host_get_next_json(..): host {} closed connection", address);
                        return None
                    }
                    error!("host_get_next_json(..): read_u32 returned Err.\nHost: {}\nError: {}", address, e);
                    continue;
                }
            };

            // Read json
            let mut buf = vec![0; length as usize];
            match reader.read_exact(&mut buf).await {
                Ok(v) => assert_eq!(v, length as usize),
                Err(e) => {
                    if e.kind() == ConnectionReset {
                        info!("host_get_next_json(..): host {} closed connection", address);
                        return None
                    }
                    error!("host_get_next_json(..): read_exact returned Err.\nHost: {}\nError: {}", address, e);
                    continue;
                }
            };

            // Decoding bytes to utf-8 string
            let msg_str = match String::from_utf8(buf) {
                Ok(v) => v,
                Err(e) => {
                    error!("host_get_next_json(..): Decoding bytes to utf-8 string failed.\nHost: {}\nError: {}", address, e);
                    continue;
                }
            };

            // Parse string to HostMessage
            let host_message = match parse_host_msg(&msg_str) {
                None => {
                    error!("host_get_next_json(..): Message by client {} is no valid json. Dropping!\nMessage: {}", address, msg_str);
                    continue
                }
                Some(v) => v
            };

            return Some(host_message)
        }
    }

    /// Send the BackendMessage to the host (connected to the given tcp socket)
    /// Transforms the BackendMessage to the correct format.
    /// Forwards any sending errors
    pub async fn host_send_message(write: &mut OwnedWriteHalf, msg: BackendMessage) -> Result<(), Error> {
        // Encode BackendMessage to string
        let str_msg = encode_backend_msg(msg);

        // Encode string message to utf-8 encoded bytes
        let bytes = str_msg.as_bytes();
        let length = bytes.len() as u32;

        // Send length
        match write.write_u32(length).await {
            Ok(_) => {}
            Err(e) => return Err(e)
        };

        // Send bytes
        match write.write_all(bytes).await{
            Ok(_) => {}
            Err(e) => return Err(e)
        };
        Ok(())
    }

    /// Reads all messages from the given socket
    /// Each valid message triggers the according event
    pub async fn host_socket_reader(channel: Sender<InternalMessage>, mut reader: OwnedReadHalf, address: SocketAddr) {
        // Read forever (until closed by host)
        loop {
            let msg = match host_get_next_json(&mut reader, address).await {
                None => {
                    warn!("host_socket_reader(..): Host {} closed the connection. Closing connection", address);
                    channel.send(InternalMessage::HostCloseConnection {address, reason: DISCONNECT_REASON_HOST_CLOSED_FORCEFULLY}).await.expect("host_socket_reader(..): Sending internal message failed");
                    break;
                }
                Some(v) => v
            };

            // Handle HostMessage (send according event)
            match msg {
                HostMessage::Disconnect { reason } => {
                    info!("host_socket_reader(..): Host {} closed the connection. Closing connection\nReason: {}", address, reason);
                    channel.send(InternalMessage::HostCloseConnection {address, reason: DISCONNECT_REASON_HOST_CLOSED_GRACEFULLY}).await.expect("host_socket_reader(..): Sending internal message failed");
                    break;
                }
                HostMessage::Update { content } => {
                    info!("host_socket_reader(..): Host {} send Update {}", address, content);
                    channel.send(InternalMessage::HostUpdate { address, content }).await.expect("host_socket_reader(..): Sending internal message failed");
                }
                HostMessage::ChangeState { content } => {
                    info!("host_socket_reader(..): Host {} send ChangeState {}", address, content);
                    channel.send(InternalMessage::HostChangeState { address, content }).await.expect("host_socket_reader(..): Sending internal message failed");
                }
            }
        }
    }

    /// Closes the connection, ignoring possible errors
    pub async fn host_close_connection(mut write: OwnedWriteHalf, address: SocketAddr, reason: &str) {
        let reason = String::from(reason);
        match host_send_message(&mut write, BackendMessage::Disconnect {reason}).await {
            Ok(_) => {}
            Err(e) => {
                warn!("host_close_connection(..): Sending 'Disconnecting' to host {} failed!\nError: {}", address, e);
            }
        };
        match write.shutdown().await {
            Ok(_) => {}
            Err(e) => {
                error!("host_close_connection(..): Closing connection to host {} failed!\nError: {}", address, e);
            }
        }
    }
}