use std::{
    collections::VecDeque,
    sync::{Arc, mpsc},
};

use videogenerator_domain::{GenerationRequest, JobId};
use videogenerator_engine_api::{Engine, EngineError, EngineEvent};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobState {
    Queued,
    Running,
    Completed,
    Failed,
    Cancelled,
}

impl JobState {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Queued => "Queued",
            Self::Running => "Running",
            Self::Completed => "Completed",
            Self::Failed => "Failed",
            Self::Cancelled => "Cancelled",
        }
    }

    pub const fn is_terminal(self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Cancelled)
    }
}

#[derive(Debug, Clone)]
pub struct JobRecord {
    pub id: JobId,
    pub request: GenerationRequest,
    pub state: JobState,
    pub stage: String,
    pub progress: f32,
    pub latest_preview: Option<String>,
    pub outputs: Vec<String>,
    pub error: Option<String>,
}

/// Owns scheduling independently of any inference backend.
///
/// The foundation scheduler deliberately runs one engine job at a time. This is
/// the safe default for a single-GPU workstation such as the initial RTX 3090
/// target. Parallel scheduling can later be added as an explicit policy rather
/// than becoming an accidental property of an engine implementation.
pub struct JobManager {
    engine: Arc<dyn Engine>,
    jobs: VecDeque<JobRecord>,
    active_id: Option<JobId>,
    next_id: u64,
    tx: mpsc::Sender<EngineEvent>,
    rx: mpsc::Receiver<EngineEvent>,
}

impl JobManager {
    pub fn new(engine: Arc<dyn Engine>) -> Self {
        let (tx, rx) = mpsc::channel();
        Self {
            engine,
            jobs: VecDeque::new(),
            active_id: None,
            next_id: 1,
            tx,
            rx,
        }
    }

    pub fn engine(&self) -> &Arc<dyn Engine> {
        &self.engine
    }

    pub fn jobs(&self) -> &VecDeque<JobRecord> {
        &self.jobs
    }

    pub fn submit(&mut self, mut request: GenerationRequest) -> Result<JobId, EngineError> {
        let descriptor = self.engine.descriptor();
        if !descriptor.supports(request.kind) {
            return Err(EngineError::Rejected(format!(
                "{} does not support {}",
                descriptor.name,
                request.kind.label()
            )));
        }

        let id = JobId(self.next_id);
        self.next_id += 1;
        request.job_id = id;

        self.jobs.push_back(JobRecord {
            id,
            request,
            state: JobState::Queued,
            stage: "Queued".to_owned(),
            progress: 0.0,
            latest_preview: None,
            outputs: Vec::new(),
            error: None,
        });

        self.schedule_next();
        Ok(id)
    }

    pub fn cancel(&mut self, id: JobId) -> Result<(), EngineError> {
        if self.active_id == Some(id) {
            return self.engine.cancel(id);
        }

        if let Some(job) = self.find_mut(id).filter(|job| job.state == JobState::Queued) {
            job.state = JobState::Cancelled;
            job.stage = "Cancelled".to_owned();
        }
        Ok(())
    }

    pub fn poll(&mut self) {
        let mut terminal_event_seen = false;
        while let Ok(event) = self.rx.try_recv() {
            terminal_event_seen |= matches!(
                event,
                EngineEvent::Completed { .. } | EngineEvent::Failed { .. } | EngineEvent::Cancelled { .. }
            );
            self.apply(event);
        }

        if terminal_event_seen || self.active_id.is_none() {
            self.schedule_next();
        }
    }

    pub fn active(&self) -> Option<&JobRecord> {
        let id = self.active_id?;
        self.jobs.iter().find(|job| job.id == id)
    }

    fn find_mut(&mut self, id: JobId) -> Option<&mut JobRecord> {
        self.jobs.iter_mut().find(|job| job.id == id)
    }

    fn schedule_next(&mut self) {
        if self.active_id.is_some() {
            return;
        }

        let Some(index) = self.jobs.iter().position(|job| job.state == JobState::Queued) else {
            return;
        };

        let id = self.jobs[index].id;
        let request = self.jobs[index].request.clone();
        self.jobs[index].stage = "Dispatching".to_owned();
        self.active_id = Some(id);

        if let Err(error) = self.engine.submit(request, self.tx.clone()) {
            self.active_id = None;
            if let Some(job) = self.find_mut(id) {
                job.state = JobState::Failed;
                job.stage = "Failed".to_owned();
                job.error = Some(error.to_string());
            }
            self.schedule_next();
        }
    }

    fn apply(&mut self, event: EngineEvent) {
        match event {
            EngineEvent::Started { job_id } => {
                if let Some(job) = self.find_mut(job_id) {
                    job.state = JobState::Running;
                    job.stage = "Starting".to_owned();
                }
            }
            EngineEvent::Stage { job_id, name } => {
                if let Some(job) = self.find_mut(job_id) {
                    job.stage = name;
                }
            }
            EngineEvent::Progress { job_id, fraction } => {
                if let Some(job) = self.find_mut(job_id) {
                    job.progress = fraction.clamp(0.0, 1.0);
                }
            }
            EngineEvent::Preview { job_id, path } => {
                if let Some(job) = self.find_mut(job_id) {
                    job.latest_preview = Some(path);
                }
            }
            EngineEvent::Completed { job_id, outputs } => {
                if let Some(job) = self.find_mut(job_id) {
                    job.state = JobState::Completed;
                    job.stage = "Completed".to_owned();
                    job.progress = 1.0;
                    job.outputs = outputs;
                }
                if self.active_id == Some(job_id) {
                    self.active_id = None;
                }
            }
            EngineEvent::Failed { job_id, message } => {
                if let Some(job) = self.find_mut(job_id) {
                    job.state = JobState::Failed;
                    job.stage = "Failed".to_owned();
                    job.error = Some(message);
                }
                if self.active_id == Some(job_id) {
                    self.active_id = None;
                }
            }
            EngineEvent::Cancelled { job_id } => {
                if let Some(job) = self.find_mut(job_id) {
                    job.state = JobState::Cancelled;
                    job.stage = "Cancelled".to_owned();
                }
                if self.active_id == Some(job_id) {
                    self.active_id = None;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use videogenerator_domain::{GenerationKind, GenerationRequest, JobId, QualityProfile, VideoSettings};
    use videogenerator_engine_api::{Engine, EngineDescriptor, EngineError, EngineEvent};

    use super::{JobManager, JobState};

    #[derive(Default)]
    struct RecordingEngine {
        submitted: std::sync::Mutex<Vec<JobId>>,
    }

    impl Engine for RecordingEngine {
        fn descriptor(&self) -> EngineDescriptor {
            EngineDescriptor {
                id: "recording".to_owned(),
                name: "Recording Engine".to_owned(),
                version: "test".to_owned(),
                capabilities: vec![GenerationKind::TextToVideo],
                process_isolated: false,
            }
        }

        fn submit(
            &self,
            request: GenerationRequest,
            _events: std::sync::mpsc::Sender<EngineEvent>,
        ) -> Result<(), EngineError> {
            self.submitted.lock().unwrap().push(request.job_id);
            Ok(())
        }

        fn cancel(&self, _job_id: JobId) -> Result<(), EngineError> {
            Ok(())
        }
    }

    fn request() -> GenerationRequest {
        GenerationRequest {
            job_id: JobId(0),
            kind: GenerationKind::TextToVideo,
            prompt: "test".to_owned(),
            negative_prompt: String::new(),
            profile: QualityProfile::Preview,
            settings: VideoSettings::default(),
            seed: 0,
            input_paths: Vec::new(),
        }
    }

    #[test]
    fn serializes_jobs_by_default() {
        let engine = Arc::new(RecordingEngine::default());
        let mut manager = JobManager::new(engine.clone());

        let first = manager.submit(request()).unwrap();
        let second = manager.submit(request()).unwrap();

        assert_eq!(engine.submitted.lock().unwrap().as_slice(), &[first]);
        assert_eq!(manager.jobs()[1].id, second);
        assert_eq!(manager.jobs()[1].state, JobState::Queued);
    }

    #[test]
    fn queued_job_can_be_cancelled_without_engine_dispatch() {
        let engine = Arc::new(RecordingEngine::default());
        let mut manager = JobManager::new(engine.clone());

        let _first = manager.submit(request()).unwrap();
        let second = manager.submit(request()).unwrap();
        manager.cancel(second).unwrap();

        assert_eq!(engine.submitted.lock().unwrap().len(), 1);
        assert_eq!(manager.jobs()[1].state, JobState::Cancelled);
    }
}
