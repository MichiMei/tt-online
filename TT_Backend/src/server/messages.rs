use log::warn;
use serde_json::Value;
use serde::Serialize;

/// Representation of every possible message send by a client
#[derive(Serialize)]
pub enum ClientMessages {
    ClientLogin{name: String},
    Disconnecting{reason: String},
    Input{content: String},
}

/// Representation of every possible message send by the host
pub enum HostMessages {
    Disconnecting{reason: String},
    NewHost{address: String},
    Update{content: String},
    ChangeState{content: String},
}

/// Representation of every possible message send by the backend
pub enum BackendMessages {
    ClientConnected{name: String, address: String},
    ClientDisconnected{name: String, address: String, reason: String},
    Disconnecting{reason: String},
    NewHost{address: String},
    Input{input: String, name: String, address: String},
    Update{content: String},
    ChangeState{content: String},
}

pub fn parse_client_msg(msg_str: &str) -> Option<ClientMessages> {
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
            Some(ClientMessages::ClientLogin{name})
        }
        "Disconnecting" => {
            let reason = match get_value(&json, "reason") {
                None => return None,
                Some(v) => v
            };
            Some(ClientMessages::Disconnecting{reason})
        }
        "Input" => {
            let content = match get_value(&json, "content") {
                None => return None,
                Some(v) => v
            };
            Some(ClientMessages::Input{content})
        }
        _ => {
            warn!("parse_client_msg(..): Message 'type' {} is not supported!\nmsg: {}", type_str, msg_str);
            None
        }
    }
}

pub fn parse_host_msg(msg_str: &str) -> Option<HostMessages> {
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
            Some(HostMessages::Disconnecting{reason})
        }
        "NewHost" => {
            let address = match get_value(&json, "address") {
                None => return None,
                Some(v) => v
            };
            Some(HostMessages::NewHost{address})
        }
        "Update" => {
            let content = match get_value(&json, "content") {
                None => return None,
                Some(v) => v
            };
            Some(HostMessages::Update{content})
        }
        "ChangeState" => {
            let content = match get_value(&json, "content") {
                None => return None,
                Some(v) => v
            };
            Some(HostMessages::ChangeState{content})
        }
        _ => {
            warn!("parse_host_msg(..): Message 'type' {} is not supported!\nmsg: {}", type_str, msg_str);
            None
        }
    }
}

pub fn encode_backend_msg(msg: BackendMessages) -> String {

    // TODO
    unimplemented!()
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