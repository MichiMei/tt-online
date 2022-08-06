use std::fmt::{Display, Formatter};
use log::warn;
use serde_json::{json, Value};

/// Representation of every possible message send by a client
#[derive(Debug, Clone)]
pub enum ClientMessage {
    ClientLogin{ name: String },
    Disconnect { reason: String },
    Input{ state_id: i32, content: String },
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
    Update { state_id: i32, content: String },
    ChangeState { state_id: i32, content: String },
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
    Input { state_id: i32, input: String, name: String, address: String },
    Update { state_id: i32, content: String },
    ChangeState { state_id: i32, content: String },
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

    let type_str = match get_string(&json, "type") {
        None => return None,
        Some(v) => v
    };

    match type_str.as_str() {
        "ClientLogin" => {
            let name = match get_string(&json, "name") {
                None => return None,
                Some(v) => v
            };
            Some(ClientMessage::ClientLogin{name})
        }
        "Disconnecting" => {
            let reason = match get_string(&json, "reason") {
                None => return None,
                Some(v) => v
            };
            Some(ClientMessage::Disconnect {reason})
        }
        "Input" => {
            let state_id = match get_i32(&json, "state_id") {
                None => return None,
                Some(v) => v
            };
            let content = match get_string(&json, "content") {
                None => return None,
                Some(v) => v
            };
            Some(ClientMessage::Input{state_id, content})
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

    let type_str = match get_string(&json, "type") {
        None => return None,
        Some(v) => v
    };

    match type_str.as_str() {
        "Disconnecting" => {
            let reason = match get_string(&json, "reason") {
                None => return None,
                Some(v) => v
            };
            Some(HostMessage::Disconnect {reason})
        }
        "Update" => {
            let state_id = match get_i32(&json, "state_id") {
                None => return None,
                Some(v) => v
            };
            let content = match get_string(&json, "content") {
                None => return None,
                Some(v) => v
            };
            Some(HostMessage::Update{state_id, content})
        }
        "ChangeState" => {
            let state_id = match get_i32(&json, "state_id") {
                None => return None,
                Some(v) => v
            };
            let content = match get_string(&json, "content") {
                None => return None,
                Some(v) => v
            };
            Some(HostMessage::ChangeState{state_id, content})
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
            String::from(json.to_string())
        }
        BackendMessage::ClientDisconnected{name, address, reason} => {
            let mut json = json!(null);
            json["type"] = json!("ClientDisconnected");
            json["name"] = json!(name);
            json["address"] = json!(address);
            json["reason"] = json!(reason);
            String::from(json.to_string())
        }
        BackendMessage::Disconnect {reason} => {
            let mut json = json!(null);
            json["type"] = json!("Disconnecting");
            json["reason"] = json!(reason);
            String::from(json.to_string())
        }
        BackendMessage::Input{state_id, input, name, address} => {
            let mut json = json!(null);
            json["type"] = json!("Input");
            json["state_id"] = json!(state_id);
            json["input"] = json!(input);
            json["name"] = json!(name);
            json["address"] = json!(address);
            String::from(json.to_string())
        }
        BackendMessage::Update{state_id, content} => {
            let mut json = json!(null);
            json["type"] = json!("Update");
            json["state_id"] = json!(state_id);
            json["content"] = json!(content);
            String::from(json.to_string())
        }
        BackendMessage::ChangeState{state_id, content} => {
            let mut json = json!(null);
            json["type"] = json!("ChangeState");
            json["state_id"] = json!(state_id);
            json["content"] = json!(content);
            String::from(json.to_string())
        }
    }
}

fn get_string(json: &Value, key: &str) -> Option<String> {
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

fn get_i32(json: &Value, key: &str) -> Option<i32> {
    let value = json[key].clone();
    if value.is_null() {
        warn!("get_value(..): Message is malformed, missing '{}' field!\nmsg: {}", key, json.to_string());
        return None
    }

    let value_i64 = match value.as_i64() {
        None => {
            warn!("get_value(..): Message is malformed, '{}' field contains not an Integer!\nmsg: {}", key, json.to_string());
            return None
        }
        Some(v) => v
    };

    Some(value_i64 as i32)
}