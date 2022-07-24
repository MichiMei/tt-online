use std::io::Error;

const IP: &str = "127.0.0.1";
const WS_PORT: u16 = 8080;
const TCP_PORT: u16 = 8081;

mod server;

/*#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Error> {
    let _server = server::run(IP, WS_PORT, TCP_PORT).await;
    Ok(())
}*/


use serde_json::{json, Value};
use crate::server::messages::ClientMessages;

fn main() {
    let mut json = json!(null);
    println!("{}", json.to_string());
    json["name"] = json!["Michi"];
    println!("{}", json.to_string());

}