use std::{
    collections::HashMap,
    ffi::OsString,
    fs,
    io::{BufRead, BufReader, Write},
    path::PathBuf,
    process::{Child, ChildStdin, Command, Stdio},
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
        mpsc::Sender,
    },
    thread,
};

use videogenerator_domain::JobId;
use videogenerator_engine_api::{Engine, EngineDescriptor, EngineError, EngineEvent};
use videogenerator_engine_protocol::{
    EngineMessage, HostCommand, PROTOCOL_VERSION, PrepareConfig, WireJob, decode_message, encode_command,
};

pub struct ProcessEngineConfig {
    pub executable: PathBuf,
    pub args: Vec<OsString>,
    pub working_dir: Option<PathBuf>,
    pub output_root: PathBuf,
    pub prepare: PrepareConfig,
}

pub struct ProcessEngine {
    descriptor: EngineDescriptor,
    stdin: Mutex<ChildStdin>,
    child: Mutex<Child>,
    routes: Arc<Mutex<HashMap<JobId, Sender<EngineEvent>>>>,
    next_request_id: AtomicU64,
    output_root: PathBuf,
}

impl ProcessEngine {
    pub fn spawn(config: ProcessEngineConfig) -> Result<Self, EngineError> {
        fs::create_dir_all(&config.output_root).map_err(|error| {
            EngineError::Transport(format!(
                "failed to create engine output directory {}: {error}",
                config.output_root.display()
            ))
        })?;

        let mut command = Command::new(&config.executable);
        command
            .args(&config.args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit());

        if let Some(working_dir) = &config.working_dir {
            command.current_dir(working_dir);
        }

        let mut child = command.spawn().map_err(|error| {
            EngineError::Transport(format!(
                "failed to start engine process {}: {error}",
                config.executable.display()
            ))
        })?;

        let mut stdin = child
            .stdin
            .take()
            .ok_or_else(|| EngineError::Transport("engine stdin was not piped".to_owned()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| EngineError::Transport("engine stdout was not piped".to_owned()))?;
        let mut reader = BufReader::new(stdout);

        write_command(
            &mut stdin,
            &HostCommand::Hello {
                protocol: PROTOCOL_VERSION,
            },
        )?;

        let ready = read_message(&mut reader)?;
        let descriptor = match ready {
            EngineMessage::Ready { protocol, descriptor } if protocol == PROTOCOL_VERSION => descriptor,
            EngineMessage::Ready { protocol, .. } => {
                return Err(EngineError::Transport(format!(
                    "engine protocol mismatch: host={PROTOCOL_VERSION}, engine={protocol}"
                )));
            }
            other => {
                return Err(EngineError::Transport(format!("expected ready message, got {other:?}")));
            }
        };

        if !descriptor.process_isolated {
            return Err(EngineError::Transport(
                "external engine descriptor must set process_isolated=true".to_owned(),
            ));
        }

        let prepare_request_id = 1;
        write_command(
            &mut stdin,
            &HostCommand::Prepare {
                request_id: prepare_request_id,
                config: config.prepare,
            },
        )?;

        match read_message(&mut reader)? {
            EngineMessage::Ack { request_id } if request_id == prepare_request_id => {}
            EngineMessage::Error { message, .. } => {
                return Err(EngineError::Rejected(format!("engine prepare failed: {message}")));
            }
            other => {
                return Err(EngineError::Transport(format!("expected prepare ack, got {other:?}")));
            }
        }

        let routes = Arc::new(Mutex::new(HashMap::new()));
        spawn_reader(reader, Arc::clone(&routes));

        Ok(Self {
            descriptor,
            stdin: Mutex::new(stdin),
            child: Mutex::new(child),
            routes,
            next_request_id: AtomicU64::new(2),
            output_root: config.output_root,
        })
    }

    fn next_request_id(&self) -> u64 {
        self.next_request_id.fetch_add(1, Ordering::Relaxed)
    }

    fn send(&self, command: &HostCommand) -> Result<(), EngineError> {
        let mut stdin = self
            .stdin
            .lock()
            .map_err(|_| EngineError::Transport("engine stdin lock poisoned".to_owned()))?;
        write_command(&mut stdin, command)
    }

    fn output_path(&self, job_id: JobId) -> PathBuf {
        self.output_root.join(format!("job-{}.mp4", job_id.0))
    }
}

impl Engine for ProcessEngine {
    fn descriptor(&self) -> EngineDescriptor {
        self.descriptor.clone()
    }

    fn submit(
        &self,
        request: videogenerator_domain::GenerationRequest,
        events: Sender<EngineEvent>,
    ) -> Result<(), EngineError> {
        if !self.descriptor.supports(request.kind) {
            return Err(EngineError::Rejected(format!(
                "{} does not support {}",
                self.descriptor.name,
                request.kind.label()
            )));
        }

        let job_id = request.job_id;
        self.routes
            .lock()
            .map_err(|_| EngineError::Transport("engine route lock poisoned".to_owned()))?
            .insert(job_id, events);

        let command = HostCommand::Submit {
            request_id: self.next_request_id(),
            job: WireJob {
                request,
                output_path: self.output_path(job_id).to_string_lossy().into_owned(),
                options: Default::default(),
            },
        };

        if let Err(error) = self.send(&command) {
            if let Ok(mut routes) = self.routes.lock() {
                routes.remove(&job_id);
            }
            return Err(error);
        }

        Ok(())
    }

    fn cancel(&self, job_id: JobId) -> Result<(), EngineError> {
        self.send(&HostCommand::Cancel {
            request_id: self.next_request_id(),
            job_id,
        })
    }
}

impl Drop for ProcessEngine {
    fn drop(&mut self) {
        let request_id = self.next_request_id();
        if let Ok(stdin) = self.stdin.get_mut() {
            let _ = write_command(stdin, &HostCommand::Shutdown { request_id });
        }

        if let Ok(child) = self.child.get_mut() {
            match child.try_wait() {
                Ok(Some(_)) => {}
                _ => {
                    let _ = child.kill();
                    let _ = child.wait();
                }
            }
        }
    }
}

fn write_command(writer: &mut ChildStdin, command: &HostCommand) -> Result<(), EngineError> {
    let line = encode_command(command)
        .map_err(|error| EngineError::Transport(format!("failed to encode engine command: {error}")))?;
    writer
        .write_all(line.as_bytes())
        .and_then(|()| writer.flush())
        .map_err(|error| EngineError::Transport(format!("failed to write engine command: {error}")))
}

fn read_message<R: BufRead>(reader: &mut R) -> Result<EngineMessage, EngineError> {
    let mut line = String::new();
    let read = reader
        .read_line(&mut line)
        .map_err(|error| EngineError::Transport(format!("failed to read engine response: {error}")))?;

    if read == 0 {
        return Err(EngineError::Transport("engine exited during handshake".to_owned()));
    }

    decode_message(line.trim_end()).map_err(|error| EngineError::Transport(format!("invalid engine response: {error}")))
}

fn spawn_reader<R>(reader: R, routes: Arc<Mutex<HashMap<JobId, Sender<EngineEvent>>>>)
where
    R: BufRead + Send + 'static,
{
    thread::spawn(move || {
        for line in reader.lines() {
            let message = match line {
                Ok(line) => match decode_message(&line) {
                    Ok(message) => message,
                    Err(error) => {
                        eprintln!("VideoGeneratorAI engine protocol decode error: {error}");
                        continue;
                    }
                },
                Err(error) => {
                    eprintln!("VideoGeneratorAI engine stdout error: {error}");
                    break;
                }
            };

            match message {
                EngineMessage::Event { event } => route_event(&routes, event),
                EngineMessage::Error { code, message, .. } => {
                    eprintln!("VideoGeneratorAI engine error [{code}]: {message}");
                }
                EngineMessage::Ack { .. } | EngineMessage::Ready { .. } => {}
            }
        }

        fail_remaining_routes(&routes);
    });
}

fn route_event(routes: &Arc<Mutex<HashMap<JobId, Sender<EngineEvent>>>>, event: EngineEvent) {
    let job_id = event_job_id(&event);
    let terminal = matches!(
        event,
        EngineEvent::Completed { .. } | EngineEvent::Failed { .. } | EngineEvent::Cancelled { .. }
    );

    let sender = routes.lock().ok().and_then(|routes| routes.get(&job_id).cloned());

    if let Some(sender) = sender {
        let _ = sender.send(event);
    }

    if terminal {
        if let Ok(mut routes) = routes.lock() {
            routes.remove(&job_id);
        }
    }
}

fn event_job_id(event: &EngineEvent) -> JobId {
    match event {
        EngineEvent::Started { job_id }
        | EngineEvent::Stage { job_id, .. }
        | EngineEvent::Progress { job_id, .. }
        | EngineEvent::Preview { job_id, .. }
        | EngineEvent::Completed { job_id, .. }
        | EngineEvent::Failed { job_id, .. }
        | EngineEvent::Cancelled { job_id } => *job_id,
    }
}

fn fail_remaining_routes(routes: &Arc<Mutex<HashMap<JobId, Sender<EngineEvent>>>>) {
    let pending = routes
        .lock()
        .map(|mut routes| routes.drain().collect::<Vec<_>>())
        .unwrap_or_default();

    for (job_id, sender) in pending {
        let _ = sender.send(EngineEvent::Failed {
            job_id,
            message: "engine process exited before the job completed".to_owned(),
        });
    }
}

#[cfg(test)]
mod tests {
    use std::{
        collections::BTreeMap,
        fs,
        path::{Path, PathBuf},
    };

    use videogenerator_domain::JobId;
    use videogenerator_engine_api::{Engine, EngineEvent};
    use videogenerator_engine_protocol::PrepareConfig;

    use super::{ProcessEngine, ProcessEngineConfig, event_job_id};

    #[test]
    fn event_job_id_is_stable_across_event_variants() {
        let id = JobId(42);
        let events = [
            EngineEvent::Started { job_id: id },
            EngineEvent::Progress {
                job_id: id,
                fraction: 0.5,
            },
            EngineEvent::Cancelled { job_id: id },
        ];

        for event in &events {
            assert_eq!(event_job_id(event), id);
        }
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn python_hunyuan_runtime_completes_handshake_and_prepare() {
        let workspace = workspace_root();
        let engine_dir = workspace.join("engines/hunyuan15-lightx2v");
        let runtime = engine_dir.join("runtime.py");
        assert!(runtime.is_file(), "runtime missing: {}", runtime.display());

        let scratch = std::env::temp_dir().join(format!(
            "videogenerator-engine-contract-{}",
            std::process::id()
        ));
        let model_root = scratch.join("model");
        let output_root = scratch.join("outputs");
        fs::create_dir_all(&model_root).unwrap();

        let engine = ProcessEngine::spawn(ProcessEngineConfig {
            executable: PathBuf::from("python3"),
            args: vec![runtime.as_os_str().to_owned()],
            working_dir: Some(engine_dir),
            output_root: output_root.clone(),
            prepare: PrepareConfig {
                model_root: model_root.to_string_lossy().into_owned(),
                output_root: output_root.to_string_lossy().into_owned(),
                options: BTreeMap::new(),
            },
        })
        .unwrap();

        let descriptor = engine.descriptor();
        assert_eq!(descriptor.id, "hunyuan15-lightx2v");
        assert!(descriptor.process_isolated);

        drop(engine);
        let _ = fs::remove_dir_all(scratch);
    }

    fn workspace_root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(2)
            .expect("engine-process crate must be two levels below workspace root")
            .to_path_buf()
    }
}
