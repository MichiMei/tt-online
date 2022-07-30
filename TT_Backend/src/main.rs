use std::io::Error;

const IP: &str = "127.0.0.1";
const WS_PORT: u16 = 8080;
const TCP_PORT: u16 = 8081;

mod server;


#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Error> {
    let mut server = server::Server::new();
    server.run(IP, WS_PORT, TCP_PORT).await;
    Ok(())
}



/*
use crate::server::messages::ClientMessage;
fn main() {
    let x = ClientMessage::Input {content: String::from("test")};
    print!("{}", x);

}
*/