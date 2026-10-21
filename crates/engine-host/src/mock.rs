use std::{
    collections::HashSet,
    sync::{Arc, Mutex, mpsc::Sender},
    thread,
    time::Duration,
};

use videogenerator_domain::{GenerationKind, GenerationRequest, JobId};
use videogenerator_engine_api::{Engine, EngineDescriptor, EngineError, EngineEvent};

#[derive(Clone, Default)]
pub struct MockEngine {
    cancelled: Arc<Mutex<HashSet<JobId>>>,
}

impl MockEngine {
    fn is_cancelled(&self, job_id: JobId) -> bool {
        self.cancelled.lock().map(|jobs| jobs.contains(&job_id)).unwrap_or(true)
    }
}

impl Engine for MockEngine {
    fn descriptor(&self) -> EngineDescriptor {
        EngineDescriptor {
            id: "mock".to_owned(),
            name: "Mock Engine".to_owned(),
            version: "0.1".to_owned(),
            capabilities: vec![
                GenerationKind::TextToVideo,
                GenerationKind::ImageToVideo,
                GenerationKind::VideoToVideo,
                GenerationKind::ReferenceToVideo,
                GenerationKind::Avatar,
                GenerationKind::Foley,
                GenerationKind::SuperResolution,
                GenerationKind::LoraTraining,
            ],
            process_isolated: false,
        }
    }

    fn submit(&self, request: GenerationRequest, events: Sender<EngineEvent>) -> Result<(), EngineError> {
        let engine = self.clone();
        thread::spawn(move || {
            let job_id = request.job_id;
            let _ = events.send(EngineEvent::Started { job_id });

            let stages = [
                ("Preparing", 0.08_f32),
                ("Encoding", 0.20),
                ("Sampling", 0.78),
                ("Decoding", 0.94),
                ("Finalizing", 1.0),
            ];

            let mut previous = 0.0_f32;
            for (stage, end) in stages {
                let _ = events.send(EngineEvent::Stage {
                    job_id,
                    name: stage.to_owned(),
                });
                let increments = 10;
                for step in 1..=increments {
                    if engine.is_cancelled(job_id) {
                        let _ = events.send(EngineEvent::Cancelled { job_id });
                        return;
                    }
                    let t = step as f32 / increments as f32;
                    let fraction = previous + (end - previous) * t;
                    let _ = events.send(EngineEvent::Progress { job_id, fraction });
                    thread::sleep(Duration::from_millis(45));
                }
                previous = end;
            }

            let _ = events.send(EngineEvent::Completed {
                job_id,
                outputs: vec![format!("mock://job-{job_id}.mp4")],
            });
        });
        Ok(())
    }

    fn cancel(&self, job_id: JobId) -> Result<(), EngineError> {
        self.cancelled
            .lock()
            .map_err(|_| EngineError::Transport("mock cancellation state poisoned".to_owned()))?
            .insert(job_id);
        Ok(())
    }
}
