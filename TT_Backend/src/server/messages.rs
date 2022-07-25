use std::fmt::{Display, Formatter};
use log::warn;
use serde_json::{json, Value};

/// Representation of every possible message send by a client
#[derive(Debug, Clone)]
pub enum ClientMessage {
    ClientLogin{ name: String },
    Disconnect { reason: String },
    Input{ content: String },
}

impl Display for ClientMessage {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

/// Representation of every possible message send by the host
#[derive(Debug, Clone)]
pub enum HostMessage {
    Disconnect { reason: String },
    Update { content: String },
    ChangeState { content: String },
}

impl Display for HostMessage {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

/// Representation of every possible message send by the backend
#[derive(Debug, Clone)]
pub enum BackendMessage {
    ClientConnected { name: String, address: String },
    ClientDisconnected { name: String, address: String, reason: String },
    Disconnect { reason: String },
    Input { input: String, name: String, address: String },
    Update { content: String },
    ChangeState { content: String },
}

impl Display for BackendMessage {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

pub fn parse_client_msg(msg_str: &str) -> Option<ClientMessage> {
    let json: Value = match serde_json::from_str(msg_str) {
        Ok(v) => v,
        Err(e) => {
            warn!("parse_client_msg(..): Parsing message failed!\nmsg: {}\nerror: {}", msg_str, e);
            return None
        }
    };

    let type_str = match get_value(&json, "type") {
        None => return None,
        Some(v) => v
    };

    match type_str.as_str() {
        "ClientLogin" => {
            let name = match get_value(&json, "name") {
                None => return None,
                Some(v) => v
            };
            Some(ClientMessage::ClientLogin{name})
        }
        "Disconnecting" => {
            let reason = match get_value(&json, "reason") {
                None => return None,
                Some(v) => v
            };
            Some(ClientMessage::Disconnect {reason})
        }
        "Input" => {
            let content = match get_value(&json, "content") {
                None => return None,
                Some(v) => v
            };
            Some(ClientMessage::Input{content})
        }
        _ => {
            warn!("parse_client_msg(..): Message 'type' {} is not supported!\nmsg: {}", type_str, msg_str);
            None
        }
    }
}

pub fn parse_host_msg(msg_str: &str) -> Option<HostMessage> {
    let json: Value = match serde_json::from_str(msg_str) {
        Ok(v) => v,
        Err(e) => {
            warn!("parse_host_msg(..): Parsing message failed!\nmsg: {}\nerror: {}", msg_str, e);
            return None
        }
    };

    let type_str = match get_value(&json, "type") {
        None => return None,
        Some(v) => v
    };

    match type_str.as_str() {
        "Disconnecting" => {
            let reason = match get_value(&json, "reason") {
                None => return None,
                Some(v) => v
            };
            Some(HostMessage::Disconnect {reason})
        }
        "Update" => {
            let content = match get_value(&json, "content") {
                None => return None,
                Some(v) => v
            };
            Some(HostMessage::Update{content})
        }
        "ChangeState" => {
            let content = match get_value(&json, "content") {
                None => return None,
                Some(v) => v
            };
            Some(HostMessage::ChangeState{content})
        }
        _ => {
            warn!("parse_host_msg(..): Message 'type' {} is not supported!\nmsg: {}", type_str, msg_str);
            None
        }
    }
}

pub fn encode_backend_msg(msg: BackendMessage) -> String {
    match msg {
        BackendMessage::ClientConnected{name, address} => {
            let mut json = json!(null);
            json["type"] = json!("ClientConnected");
            json["name"] = json!(name);
            json["address"] = json!(address);
            String::from(json.as_str().unwrap())
        }
        BackendMessage::ClientDisconnected{name, address, reason} => {
            let mut json = json!(null);
            json["type"] = json!("ClientDisconnected");
            json["name"] = json!(name);
            json["address"] = json!(address);
            json["reason"] = json!(reason);
            String::from(json.as_str().unwrap())
        }
        BackendMessage::Disconnect {reason} => {
            let mut json = json!(null);
            json["type"] = json!("Disconnecting");
            json["reason"] = json!(reason);
            String::from(json.as_str().unwrap())
        }
        BackendMessage::Input{input, name, address} => {
            let mut json = json!(null);
            json["type"] = json!("Input");
            json["input"] = json!(input);
            json["name"] = json!(name);
            json["address"] = json!(address);
            String::from(json.as_str().unwrap())
        }
        BackendMessage::Update{content} => {
            let mut json = json!(null);
            json["type"] = json!("Update");
            json["content"] = json!(content);
            String::from(json.as_str().unwrap())
        }
        BackendMessage::ChangeState{content} => {
            let mut json = json!(null);
            json["type"] = json!("ChangeState");
            json["content"] = json!(content);
            String::from(json.as_str().unwrap())
        }
    }
}

fn get_value(json: &Value, key: &str) -> Option<String> {
    let value = json[key].clone();
    if value.is_null() {
        warn!("get_value(..): Message is malformed, missing '{}' field!\nmsg: {}", key, json.to_string());
        return None
    }

    let value_str = match value.as_str() {
        None => {
            warn!("get_value(..): Message is malformed, '{}' field contains not a String!\nmsg: {}", key, json.to_string());
            return None
        }
        Some(v) => v
    };

    Some(String::from(value_str))
}