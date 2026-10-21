use std::sync::mpsc::Sender;

use serde::{Deserialize, Serialize};
use thiserror::Error;
use videogenerator_domain::{GenerationKind, GenerationRequest, JobId};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EngineDescriptor {
    pub id: String,
    pub name: String,
    pub version: String,
    pub capabilities: Vec<GenerationKind>,
    pub process_isolated: bool,
}

impl EngineDescriptor {
    pub fn supports(&self, kind: GenerationKind) -> bool {
        self.capabilities.contains(&kind)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EngineEvent {
    Started { job_id: JobId },
    Stage { job_id: JobId, name: String },
    Progress { job_id: JobId, fraction: f32 },
    Preview { job_id: JobId, path: String },
    Completed { job_id: JobId, outputs: Vec<String> },
    Failed { job_id: JobId, message: String },
    Cancelled { job_id: JobId },
}

#[derive(Debug, Error)]
pub enum EngineError {
    #[error("engine rejected job: {0}")]
    Rejected(String),
    #[error("engine transport failed: {0}")]
    Transport(String),
}

pub trait Engine: Send + Sync + 'static {
    fn descriptor(&self) -> EngineDescriptor;
    fn submit(&self, request: GenerationRequest, events: Sender<EngineEvent>) -> Result<(), EngineError>;
    fn cancel(&self, job_id: JobId) -> Result<(), EngineError>;
}
