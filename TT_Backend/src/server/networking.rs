

mod websockets {
    /*use std::net::SocketAddr;
    use futures_util::stream::SplitStream;
    use log::{error, info, warn};
    use tokio::net::TcpStream;
    use tokio_tungstenite::WebSocketStream;
*/
    /*
    /// Returns the next text message from the websocket
    /// Drops all messages other than text
    /// Returns Ok(msg) or Err(()) if the socket was closed (indicating the write socket should get closed)
    pub async fn next_text_message(mut reader: &mut SplitStream<WebSocketStream<TcpStream>>, address: SocketAddr) -> Result<String, ()>{
        // TODO find out how closed websocket behaves and return Err(())
        loop {
            reader.
            let result = match reader.next().await{
                Some(res) => res,
                None => {
                    error!("websocket_next_json(..): returned None\nclient: {}\nprobably closed?", address);
                    continue
                }
            };
            match result {
                Ok(msg) => {
                    info!("websocket_next_json(..): message received\nclient: {}\nmsg: {}", address, msg);
                    if !msg.is_text() {
                        warn!("websocket_next_json(..): message ist not text. Dropping message!");
                        continue
                    }
                    let str_msg = msg.into_text().expect("websocket_next_json(..): impl error: message should have been convertible");
                    Ok(str_msg)
                }
                Err(e) => {
                    error!("websocket_listen(..): returned Err\nclient: {}\nmsg: {:?}", address, e);
                    continue
                }
            }
        }
    }*/


}

mod tcp_sockets {

}