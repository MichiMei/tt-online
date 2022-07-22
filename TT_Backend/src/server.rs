//!
//! This is a async backend server.
//! It listens on one port for incoming websocket connections from clients using the WebApp and on
//! another port for incoming tcp connections by host(s) using the HostApp.
//! An arbitrary number of clients can connect to the server but only one host. If a new one tries
//! to connect, the old one gets disconnected (to prevent waiting for its timeout)
//!

use std::collections::HashMap;
use std::io::Error;
use std::io::ErrorKind::ConnectionReset;
use std::net::SocketAddr;
use futures_util::{SinkExt, StreamExt};
use futures_util::stream::{SplitSink, SplitStream};
use log::{error, info, warn};
use serde_json::Value;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::sync::mpsc;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::WebSocketStream;

const CHANNEL_SIZE: usize = 16;

enum InternalMessage {
    ClientConnected{stream: TcpStream, address: SocketAddr},
    ClientClosed{address: SocketAddr},
    HostConnected{stream: TcpStream, address: SocketAddr},
    HostClosed{address: SocketAddr},
    ClientInput{address: SocketAddr, msg: Message},
    HostUpdate{address : SocketAddr, msg: String},
    HostChangeState{address : SocketAddr, msg: String},
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
    let mut state = Some(String::from("DUMMY"));
    let mut host: Option<(SocketAddr, OwnedWriteHalf)> = None;

    loop {
        // receive next internal message
        let message = match channel_rx.recv().await {
            Some(v) => v,
            None => break
        };

        // TODO handle internal message
        match message {
            InternalMessage::ClientConnected { stream, address} => handle_client_connected(&mut channel_tx, &mut clients, &state, stream, address).await,
            InternalMessage::ClientClosed{ address } => handle_client_closed(&mut clients, address).await,
            InternalMessage::HostConnected { stream, address} => handle_host_connected(channel_tx.clone(), &state, &mut host, stream, address).await,
            InternalMessage::HostClosed{ address } => handle_host_closed(&mut host, address).await,
            InternalMessage::ClientInput{ address, msg } => handle_client_input(&mut host, &clients, address, msg).await,
            InternalMessage::HostUpdate{ address, msg } => handle_host_update(&host, &mut clients, address, msg).await,
            InternalMessage::HostChangeState{ address, msg } => handle_host_change_state(&host, &mut clients, &mut state, address, msg).await,
        }

    }

    // TODO cleanup
}

async fn write_to_socket(write: &mut OwnedWriteHalf, msg: &str) -> Result<(), Error> {
    let bytes = msg.as_bytes();
    let length = bytes.len() as u32;
    match write.write_u32(length).await {
        Ok(_) => {}
        Err(e) => return Err(e)
    };
    match write.write_all(bytes).await{
        Ok(_) => {}
        Err(e) => return Err(e)
    };
    Ok(())
}

async fn handle_client_connected(channel_tx: &mut Sender<InternalMessage>, clients: &mut HashMap<SocketAddr, SplitSink<WebSocketStream<TcpStream>, Message>>, state: &Option<String>, stream: TcpStream, address: SocketAddr) {
    info!("handle_client_connected(..): Client connected: {}", address);
    // upgrade stream to websocket
    let ws_stream = match tokio_tungstenite::accept_async(stream).await {
        Ok(v) => v,
        Err(e) => {
            warn!("main_task(..): websocket handshake failed\nclient: {}\nmsg: {:?}", address, e);
            return
        }
    };
    let (mut ws_write, ws_read) = ws_stream.split();

    // send initial state
    if state.is_some() {
        match ws_write.send(Message::Text(state.as_ref().unwrap().clone())).await {
            Ok(_) => {}
            Err(e) => {
                warn!("main_task(..): sending initial state failed, closing websocket\nclient: {}\nmsg: {:?}", address, e);
                match ws_write.close().await {
                    Ok(_) => {}
                    Err(e) => {
                        error!("main_task(..): closing websocket failed\nclient: {}\nmsg: {:?}", address, e);
                    }
                };
                return
            }
        };
    }

    // insert write to clients
    clients.insert(address, ws_write);

    // create listener
    tokio::spawn(websocket_listen(channel_tx.clone(), ws_read, address));
}

async fn handle_client_closed(clients: &mut HashMap<SocketAddr, SplitSink<WebSocketStream<TcpStream>, Message>>, address: SocketAddr) {
    info!("handle_client_closed(..): client {} disconnected", address);
    let mut write = match clients.remove(&address) {
        Some(v) => v,
        None => {
            warn!("handle_client_closed(..): client not in the list");
            return;
        }
    };

    match write.close().await{
        Ok(_) => {}
        Err(e) => {
            error!("handle_client_closed(..): closing connection failed\nclient: {}\nmsg: {:?}", address, e);
        }
    };
}

async fn handle_host_connected(channel_tx: Sender<InternalMessage>, state: &Option<String>, host: &mut Option<(SocketAddr, OwnedWriteHalf)>, stream: TcpStream, address: SocketAddr) {
    info!("handle_host_connected(..): Host connected: {}", address);
    let (read, mut write) = stream.into_split();
    // Send state to new Host
    if state.is_some() {
        info!("handle_host_connected(..): Sending initial state {}", state.as_ref().unwrap());
        match write_to_socket(&mut write, state.as_ref().unwrap()).await {
            Ok(_) => {}
            Err(e) => {
                warn!("handle_host_connected(..): sending initial state failed, closing connection\nhost: {}\nmsg: {:?}", address, e);
                match write.shutdown().await {
                    Ok(_) => {}
                    Err(e) => {
                        error!("handle_host_connected(..): closing connection failed\nhost: {}\nmsg: {:?}", address, e);
                    }
                };
                return
            }
        };
    }

    // check if a host is already connected -> disconnect
    if host.is_some() {
        info!("handle_host_connected(..): Old host still connected. Disconnecting: {}", host.as_ref().unwrap().0);
        match host.as_mut().unwrap().1.shutdown().await {
            Ok(_) => {}
            Err(e) => {
                error!("handle_host_connected(..): closing connection to old host failed\nhost: {}\nmsg: {:?}", host.as_ref().unwrap().0, e);
            }
        };
        *host = None;
    }
    assert!(host.is_none(), "handle_host_connected(..): Host should have been disconnected");

    tokio::spawn(tcp_listen(channel_tx, read, address));

    *host = Some((address, write));
}

async fn handle_host_closed(host: &mut Option<(SocketAddr, OwnedWriteHalf)>, address: SocketAddr) {
    info!("handle_host_closed(..): host {} disconnected", address);
    if host.is_some() {
        let current_address = host.as_ref().unwrap().0;
        if address == current_address {
            info!("handle_host_closed(..): is current host -> disconnecting");
            match host.as_mut().unwrap().1.shutdown().await {
                Ok(_) => {}
                Err(e) => {
                    error!("handle_host_closed(..): closing connection failed\nhost: {}\nmsg: {:?}", address, e);
                }
            }
            *host = None;
        }
    }
}

async fn handle_client_input(host: &mut Option<(SocketAddr, OwnedWriteHalf)>, clients: &HashMap<SocketAddr, SplitSink<WebSocketStream<TcpStream>, Message>>, address: SocketAddr, msg: Message) {
    // TODO add client address to json

    info!("handle_client_input(..): Client {} send input\nmsg: {}", address, msg);
    if !clients.contains_key(&address) {
        warn!("handle_client_input(..): client {} not contained in list -> ignoring input", address);
        return;
    }

    if !msg.is_text() {
        warn!("handle_client_input(..): input from client {} is not text -> ignoring input\nmsg: {}", address, msg);
        return;
    }

    if host.is_none() {
        warn!("handle_client_input(..): no host connected -> ignoring input");
        return;
    }

    let msg_str = msg.to_string();
    match write_to_socket(&mut host.as_mut().unwrap().1, &msg_str).await {
        Ok(_) => {}
        Err(e) => {
            error!("handle_client_input(..): sending input to host failed\nmsg: {}", e);
        }
    };
}

async fn handle_host_update(host: &Option<(SocketAddr, OwnedWriteHalf)>, clients: &mut HashMap<SocketAddr, SplitSink<WebSocketStream<TcpStream>, Message>>, address: SocketAddr, msg: String) {
    info!("handle_host_update(..): Host send update\nmsg: {}", msg);
    if host.is_none() || host.as_ref().unwrap().0 != address {
        warn!("handle_host_update(..): is not current Host -> ignoring update");
        return;
    }

    write_to_all_clients(clients, &msg).await;
}

async fn handle_host_change_state(host: &Option<(SocketAddr, OwnedWriteHalf)>, clients: &mut HashMap<SocketAddr, SplitSink<WebSocketStream<TcpStream>, Message>>, state: &mut Option<String>, address: SocketAddr, msg: String) {
    info!("handle_host_change_state(..): Host send state change\nmsg: {}", msg);
    if host.is_none() || host.as_ref().unwrap().0 != address {
        warn!("handle_host_change_state(..): is not current Host -> ignoring state change");
        return;
    }

    *state = Some(msg.clone());

    write_to_all_clients(clients, &msg).await;
}

async fn write_to_all_clients(clients: &mut HashMap<SocketAddr, SplitSink<WebSocketStream<TcpStream>, Message>>, msg: &String) {
    for (cli_addr, write) in clients {
        match write.send(Message::Text(msg.clone())).await {
            Ok(_) => {}
            Err(e) => {
                error!("write_to_all_clients(..): sending update to client {} failed\nmsg: {:?}", cli_addr, e);
            }
        };
    }
}

async fn create_client_listener(channel: Sender<InternalMessage>, ip: &str, port: u16) {
    // address
    let addr = (ip.to_owned()+":"+ &*port.to_string()).to_string();

    // Create the event loop and TCP listener we'll accept connections on.
    let listener = TcpListener::bind(&addr).await.expect("create_client_listener(..) failed");
    info!("Listening for clients on: {}", addr);

    // create listener task
    tokio::spawn(client_listen(channel, listener));
}

async fn client_listen(channel: Sender<InternalMessage>, listener: TcpListener) {
    // TODO nice terminate
    loop {
        let (stream, address) = match listener.accept().await {
            Ok(v) => v,
            Err(e) => {
                warn!("client_listen(..): Could not accept connection: {}", e);
                continue
            },
        };
        match channel.send(InternalMessage::ClientConnected {stream, address}).await {
            Ok(_) => {}
            Err(e) => error!("client_listen(..): Could not send internal message: {}", e),
        }
    }

}

async fn create_host_listener(channel: Sender<InternalMessage>, ip: &str, port: u16) {
    // address
    let addr = (ip.to_owned()+":"+ &*port.to_string()).to_string();

    // Create the event loop and TCP listener we'll accept connections on.
    let listener = TcpListener::bind(&addr).await.expect("create_host_listener(..) failed");
    info!("Listening for host(s) on: {}", addr);

    // create listener task
    tokio::spawn(host_listen(channel, listener));
}

async fn host_listen(channel: Sender<InternalMessage>, listener: TcpListener) {
    // TODO nice terminate
    loop {
        let (stream, address) = match listener.accept().await {
            Ok(v) => v,
            Err(e) => {
                warn!("host_listen(..): Could not accept connection: {}", e);
                continue
            },
        };
        match channel.send(InternalMessage::HostConnected{stream, address}).await {
            Ok(_) => {}
            Err(e) => error!("host_listen(..): Could not send internal message: {}", e),
        }
    }
}

async fn websocket_listen(channel: Sender<InternalMessage>, mut reader: SplitStream<WebSocketStream<TcpStream>>, address: SocketAddr) {
    // TODO nice terminate... None or Err???
    loop {
        let result = match reader.next().await{
            Some(res) => res,
            None => {
                error!("websocket_listen(..): returned None\nclient: {}\nprobably closed?", address);
                continue
            }
        };
        match result {
            Ok(msg) => {
                info!("websocket_listen(..): message received\nclient: {}\nmsg: {}", address, msg);
                match channel.send(InternalMessage::ClientInput{address, msg}).await {
                    Ok(_) => {}
                    Err(e) => error!("websocket_listen(..): Could not send internal message: {}", e),
                }
            }
            Err(e) => {
                error!("websocket_listen(..): returned Err\nclient: {}\nmsg: {:?}", address, e);
                continue
            }
        }
    }
}

async fn tcp_listen(channel: Sender<InternalMessage>, mut reader: OwnedReadHalf, address: SocketAddr) {
    loop {
        // read length
        let length = match reader.read_u32().await {
            Ok(v) => v,
            Err(e) => {
                if e.kind() == ConnectionReset {
                    info!("tcp_listen(..): host {} closed connection", address);
                    match channel.send(InternalMessage::HostClosed{address}).await {
                        Ok(_) => {}
                        Err(e) => error!("tcp_listen(..): Could not send internal message: {}", e),
                    };
                    break;
                }
                error!("tcp_listen(..): read int returned Err\nhost: {}\nmsg: {}", address, e);
                continue;
            }
        };

        // read json
        let mut buf = vec![0; length as usize];

        match reader.read_exact(&mut buf).await{
            Ok(_) => {}
            Err(e) => {
                if e.kind() == ConnectionReset {
                    info!("tcp_listen(..): host {} closed connection", address);
                    match channel.send(InternalMessage::HostClosed{address}).await {
                        Ok(_) => {}
                        Err(e) => error!("tcp_listen(..): Could not send internal message: {}", e),
                    };
                    break;
                }
                error!("tcp_listen(..): read buf returned Err\nhost: {}\nmsg: {}", address, e);
                continue;
            }
        }

        let json_str = match String::from_utf8(buf) {
            Ok(v) => v,
            Err(e) => {
                error!("tcp_listen(..): decoding failed\nmsg: {}", e);
                continue;
            }
        };

        let json_obj: Value = match serde_json::from_str(&json_str) {
            Ok(v) => v,
            Err(e) => {
                error!("tcp_listen(..): parsing json failed\nmsg: {}", e);
                continue;
            }
        };

        let message_type = json_obj["type"].to_string();

        match message_type.as_str() {
            "\"Update\"" => {
                info!("tcp_listen(..): received HostUpdate from {}\nmsg: {}", address, json_str);
                match channel.send(InternalMessage::HostUpdate{address, msg: json_str}).await {
                    Ok(_) => {}
                    Err(e) => error!("tcp_listen(..): Could not send internal message: {}", e),
                };
            }
            "\"ChangeState\"" => {
                info!("tcp_listen(..): received HostChangeState from {}\nmsg: {}", address, json_str);
                match channel.send(InternalMessage::HostChangeState{address, msg: json_str}).await {
                    Ok(_) => {}
                    Err(e) => error!("tcp_listen(..): Could not send internal message: {}", e),
                };
            }
            _ => {
                error!("tcp_listen(..): received a message with wrong type from {}\n{}", address, json_str);
            }
        }

    }
}