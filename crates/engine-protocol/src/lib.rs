use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use videogenerator_domain::{GenerationRequest, JobId};
use videogenerator_engine_api::{EngineDescriptor, EngineEvent};

pub const PROTOCOL_VERSION: u32 = 1;

pub type EngineOptions = BTreeMap<String, Value>;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PrepareConfig {
    pub model_root: String,
    pub output_root: String,
    #[serde(default)]
    pub options: EngineOptions,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WireJob {
    pub request: GenerationRequest,
    pub output_path: String,
    #[serde(default)]
    pub options: EngineOptions,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum HostCommand {
    Hello {
        protocol: u32,
    },
    Prepare {
        request_id: u64,
        config: PrepareConfig,
    },
    Submit {
        request_id: u64,
        job: WireJob,
    },
    Cancel {
        request_id: u64,
        job_id: JobId,
    },
    Unload {
        request_id: u64,
    },
    Shutdown {
        request_id: u64,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EngineMessage {
    Ready {
        protocol: u32,
        descriptor: EngineDescriptor,
    },
    Ack {
        request_id: u64,
    },
    Event {
        event: EngineEvent,
    },
    Error {
        request_id: Option<u64>,
        code: String,
        message: String,
    },
}

pub fn encode_command(command: &HostCommand) -> Result<String, serde_json::Error> {
    serde_json::to_string(command).map(|mut line| {
        line.push('\n');
        line
    })
}

pub fn decode_message(line: &str) -> Result<EngineMessage, serde_json::Error> {
    serde_json::from_str(line)
}

#[cfg(test)]
mod tests {
    use super::{EngineMessage, HostCommand, PROTOCOL_VERSION, decode_message, encode_command};

    #[test]
    fn hello_is_one_json_line() {
        let encoded = encode_command(&HostCommand::Hello {
            protocol: PROTOCOL_VERSION,
        })
        .unwrap();

        assert!(encoded.ends_with('\n'));
        assert_eq!(encoded.lines().count(), 1);
    }

    #[test]
    fn ready_message_decodes() {
        let line = r#"{"type":"ready","protocol":1,"descriptor":{"id":"test","name":"Test","version":"1","capabilities":["TextToVideo"],"process_isolated":true}}"#;
        let decoded = decode_message(line).unwrap();

        assert!(matches!(
            decoded,
            EngineMessage::Ready {
                protocol: PROTOCOL_VERSION,
                ..
            }
        ));
    }
}
