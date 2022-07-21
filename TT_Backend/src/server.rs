use std::collections::HashMap;
use std::env;
use std::io::Error;
use std::net::SocketAddr;
use futures_util::{SinkExt, StreamExt};
use futures_util::stream::{SplitSink, SplitStream};
use log::{error, info, warn};
use tokio::io::AsyncWriteExt;
use tokio::net::{TcpListener, TcpStream};
use tokio::net::tcp::WriteHalf;
use tokio::sync::mpsc;
use tokio::sync::mpsc::error::SendError;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::WebSocketStream;

const CHANNEL_SIZE: usize = 16;

enum InternalMessage {
    ClientConnected{stream: TcpStream, address: SocketAddr},
    ClientClosed,
    HostConnected{stream: TcpStream, address: SocketAddr},
    HostClosed,
    ClientInput{address: SocketAddr, msg: Message},
    HostUpdate,
    HostChangeState,
}

pub async fn run(listen_ip: &str, web_socket_port: u8, tcp_port: u8) {
    // init logger
    let _ = env_logger::try_init();

    // create MPSC channel for internal communication
    let (tx, rx) = mpsc::channel(16);

    // create Websocket listener
    create_client_listener(tx.clone(), listen_ip, web_socket_port).await;

    // create TCP listener
    create_host_listener(tx.clone(), listen_ip, tcp_port).await;

    // create (and run) main task
    main_task(tx, rx).await;
}

async fn main_task(mut channel_tx: Sender<InternalMessage>, mut channel_rx: Receiver<InternalMessage>) {

    let mut clients = HashMap::new();
    let mut state = "DUMMY";

    loop {
        // receive next internal message
        let message = match channel_rx.recv().await {
            Some(v) => v,
            None => break
        };

        // TODO handle internal message
        match message {
            InternalMessage::ClientConnected { mut stream, address} => handle_client_connected(&mut channel_tx, &mut clients, state, stream, address).await,
            InternalMessage::ClientClosed => unimplemented!(),
            InternalMessage::HostConnected { .. } => unimplemented!(),
            InternalMessage::HostClosed => unimplemented!(),
            InternalMessage::ClientInput{ .. } => unimplemented!(),
            InternalMessage::HostUpdate => unimplemented!(),
            InternalMessage::HostChangeState => unimplemented!(),
        }

    }

    // TODO cleanup
}

async fn handle_client_connected(channel_tx: &mut Sender<InternalMessage>, clients: &mut HashMap<SocketAddr, SplitSink<WebSocketStream<TcpStream>, Message>>, mut state: &str, mut stream: TcpStream, address: SocketAddr) {
// upgrade stream to websocket
    let ws_stream = match tokio_tungstenite::accept_async(stream).await {
        Ok(v) => v,
        Err(e) => {
            error!("main_task(..): websocket handshake failed\nclient: {}\nmsg: {:?}", address, e);
            return
        }
    };
    let (mut ws_write, ws_read) = ws_stream.split();

    // send initial state
    match ws_write.send(Message::from(state)).await {
        Ok(_) => {}
        Err(e) => {
            error!("main_task(..): sending initial state failed, closing websocket\nclient: {}\nmsg: {:?}", address, e);
            match ws_write.close().await {
                Ok(_) => {}
                Err(e) => {
                    error!("main_task(..): closing websocket failed\nclient: {}\nmsg: {:?}", address, e);
                }
            };
            return
        }
    };

    // insert write to clients
    clients.insert(address, ws_write);

    // create listener
    tokio::spawn(websocket_listen(channel_tx.clone(), ws_read, address));
}

async fn create_client_listener(channel: Sender<InternalMessage>, ip: &str, port: u8) {
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

async fn create_host_listener(channel: Sender<InternalMessage>, ip: &str, port: u8) {
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