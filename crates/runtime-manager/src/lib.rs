use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const RUNTIME_MANIFEST_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitSource {
    pub repository: String,
    pub commit: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PythonRequirement {
    pub minimum: String,
    pub preferred: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeComponent {
    pub id: String,
    pub label: String,
    pub source: GitSource,
    #[serde(default)]
    pub optional: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeManifest {
    pub schema_version: u32,
    pub id: String,
    pub name: String,
    pub source: GitSource,
    pub python: PythonRequirement,
    #[serde(default)]
    pub components: Vec<RuntimeComponent>,
}

impl RuntimeManifest {
    pub fn parse_json(input: &str) -> Result<Self, RuntimeManagerError> {
        let manifest: Self = serde_json::from_str(input)?;
        manifest.validate()?;
        Ok(manifest)
    }

    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, RuntimeManagerError> {
        let contents = fs::read_to_string(path.as_ref()).map_err(|source| RuntimeManagerError::ReadManifest {
            path: path.as_ref().to_path_buf(),
            source,
        })?;
        Self::parse_json(&contents)
    }

    pub fn validate(&self) -> Result<(), RuntimeManagerError> {
        if self.schema_version != RUNTIME_MANIFEST_SCHEMA_VERSION {
            return Err(RuntimeManagerError::UnsupportedSchema {
                found: self.schema_version,
                supported: RUNTIME_MANIFEST_SCHEMA_VERSION,
            });
        }
        validate_git_source(&self.source)?;
        for component in &self.components {
            validate_git_source(&component.source)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeLayout {
    pub root: PathBuf,
    pub source_dir: PathBuf,
    pub venv_dir: PathBuf,
    pub components_dir: PathBuf,
}

impl RuntimeLayout {
    pub fn under(root: impl Into<PathBuf>) -> Self {
        let root = root.into();
        Self {
            source_dir: root.join("source"),
            venv_dir: root.join(".venv"),
            components_dir: root.join("components"),
            root,
        }
    }

    pub fn python(&self) -> PathBuf {
        self.venv_dir.join("bin/python")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandSpec {
    pub program: PathBuf,
    pub args: Vec<String>,
    pub current_dir: Option<PathBuf>,
    pub env: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstallStep {
    CreateDirectory {
        label: String,
        path: PathBuf,
    },
    Command {
        label: String,
        command: CommandSpec,
    },
    Gate {
        id: String,
        label: String,
        detail: String,
    },
}

pub fn build_install_plan(manifest: &RuntimeManifest, layout: &RuntimeLayout) -> Vec<InstallStep> {
    let mut plan = vec![InstallStep::CreateDirectory {
        label: "Create runtime root".to_owned(),
        path: layout.root.clone(),
    }];

    if !layout.source_dir.join(".git").is_dir() {
        plan.push(InstallStep::Command {
            label: "Clone pinned runtime source".to_owned(),
            command: CommandSpec {
                program: PathBuf::from("git"),
                args: vec![
                    "clone".to_owned(),
                    "--filter=blob:none".to_owned(),
                    manifest.source.repository.clone(),
                    layout.source_dir.to_string_lossy().into_owned(),
                ],
                current_dir: None,
                env: BTreeMap::new(),
            },
        });
    }

    plan.push(InstallStep::Command {
        label: "Fetch pinned runtime revision".to_owned(),
        command: CommandSpec {
            program: PathBuf::from("git"),
            args: vec![
                "-C".to_owned(),
                layout.source_dir.to_string_lossy().into_owned(),
                "fetch".to_owned(),
                "--depth=1".to_owned(),
                "origin".to_owned(),
                manifest.source.commit.clone(),
            ],
            current_dir: None,
            env: BTreeMap::new(),
        },
    });

    plan.push(InstallStep::Command {
        label: "Checkout pinned runtime revision".to_owned(),
        command: CommandSpec {
            program: PathBuf::from("git"),
            args: vec![
                "-C".to_owned(),
                layout.source_dir.to_string_lossy().into_owned(),
                "checkout".to_owned(),
                "--detach".to_owned(),
                manifest.source.commit.clone(),
            ],
            current_dir: None,
            env: BTreeMap::new(),
        },
    });

    if !layout.python().is_file() {
        plan.push(InstallStep::Command {
            label: "Create isolated Python environment".to_owned(),
            command: CommandSpec {
                program: PathBuf::from("uv"),
                args: vec![
                    "venv".to_owned(),
                    "--python".to_owned(),
                    manifest.python.preferred.clone(),
                    layout.venv_dir.to_string_lossy().into_owned(),
                ],
                current_dir: None,
                env: BTreeMap::new(),
            },
        });
    }

    plan.push(InstallStep::Gate {
        id: "cuda-pytorch".to_owned(),
        label: "Select CUDA/PyTorch runtime".to_owned(),
        detail: "VideoGeneratorAI must verify the host driver and choose a tested CUDA/PyTorch combination before installing LightX2V. The upstream package only declares torch>=2.1, which is not sufficiently reproducible for an automatic GPU install.".to_owned(),
    });

    plan.push(InstallStep::Command {
        label: "Install LightX2V from pinned source".to_owned(),
        command: CommandSpec {
            program: PathBuf::from("uv"),
            args: vec![
                "pip".to_owned(),
                "install".to_owned(),
                "--python".to_owned(),
                layout.python().to_string_lossy().into_owned(),
                "-v".to_owned(),
                "-e".to_owned(),
                layout.source_dir.to_string_lossy().into_owned(),
            ],
            current_dir: None,
            env: BTreeMap::new(),
        },
    });

    plan
}

pub fn build_sageattention_plan(
    component: &RuntimeComponent,
    layout: &RuntimeLayout,
    cuda_architectures: &str,
) -> Vec<InstallStep> {
    let source_dir = layout.components_dir.join(&component.id);
    let mut env = BTreeMap::new();
    env.insert("CUDA_ARCHITECTURES".to_owned(), cuda_architectures.to_owned());
    env.insert("EXT_PARALLEL".to_owned(), "4".to_owned());
    env.insert("NVCC_APPEND_FLAGS".to_owned(), "--threads 8".to_owned());

    let mut plan = vec![InstallStep::CreateDirectory {
        label: "Create runtime components directory".to_owned(),
        path: layout.components_dir.clone(),
    }];

    if !source_dir.join(".git").is_dir() {
        plan.push(InstallStep::Command {
            label: format!("Clone {}", component.label),
            command: CommandSpec {
                program: PathBuf::from("git"),
                args: vec![
                    "clone".to_owned(),
                    "--filter=blob:none".to_owned(),
                    component.source.repository.clone(),
                    source_dir.to_string_lossy().into_owned(),
                ],
                current_dir: None,
                env: BTreeMap::new(),
            },
        });
    }

    plan.push(InstallStep::Command {
        label: format!("Fetch pinned {}", component.label),
        command: CommandSpec {
            program: PathBuf::from("git"),
            args: vec![
                "-C".to_owned(),
                source_dir.to_string_lossy().into_owned(),
                "fetch".to_owned(),
                "--depth=1".to_owned(),
                "origin".to_owned(),
                component.source.commit.clone(),
            ],
            current_dir: None,
            env: BTreeMap::new(),
        },
    });

    plan.push(InstallStep::Command {
        label: format!("Checkout pinned {}", component.label),
        command: CommandSpec {
            program: PathBuf::from("git"),
            args: vec![
                "-C".to_owned(),
                source_dir.to_string_lossy().into_owned(),
                "checkout".to_owned(),
                "--detach".to_owned(),
                component.source.commit.clone(),
            ],
            current_dir: None,
            env: BTreeMap::new(),
        },
    });

    plan.push(InstallStep::Command {
            label: format!("Install {}", component.label),
            command: CommandSpec {
                program: PathBuf::from("uv"),
                args: vec![
                    "pip".to_owned(),
                    "install".to_owned(),
                    "--python".to_owned(),
                    layout.python().to_string_lossy().into_owned(),
                    "-v".to_owned(),
                    "-e".to_owned(),
                    source_dir.to_string_lossy().into_owned(),
                ],
                current_dir: Some(source_dir),
                env,
            },
        },
    });

    plan
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolStatus {
    Available { first_line: String },
    Missing,
    Failed { message: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostToolchainProbe {
    pub git: ToolStatus,
    pub uv: ToolStatus,
    pub nvidia_smi: ToolStatus,
}

pub fn probe_host_toolchain() -> HostToolchainProbe {
    HostToolchainProbe {
        git: probe_command("git", &["--version"]),
        uv: probe_command("uv", &["--version"]),
        nvidia_smi: probe_command("nvidia-smi", &["--version"]),
    }
}

#[derive(Debug, Error)]
pub enum RuntimeManagerError {
    #[error("failed to read runtime manifest {path}: {source}")]
    ReadManifest {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("invalid runtime manifest JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("unsupported runtime manifest schema {found}; supported schema is {supported}")]
    UnsupportedSchema { found: u32, supported: u32 },
    #[error("git source must use an https URL: {0}")]
    InvalidGitRepository(String),
    #[error("git commit must be a full 40-character hexadecimal SHA: {0}")]
    InvalidGitCommit(String),
}

fn probe_command(program: &str, args: &[&str]) -> ToolStatus {
    match Command::new(program).args(args).output() {
        Ok(output) if output.status.success() => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            let text = if stdout.trim().is_empty() { stderr } else { stdout };
            ToolStatus::Available {
                first_line: text.lines().next().unwrap_or_default().trim().to_owned(),
            }
        }
        Ok(output) => ToolStatus::Failed {
            message: String::from_utf8_lossy(&output.stderr).trim().to_owned(),
        },
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => ToolStatus::Missing,
        Err(error) => ToolStatus::Failed {
            message: error.to_string(),
        },
    }
}

fn validate_git_source(source: &GitSource) -> Result<(), RuntimeManagerError> {
    if !source.repository.starts_with("https://") {
        return Err(RuntimeManagerError::InvalidGitRepository(
            source.repository.clone(),
        ));
    }
    if source.commit.len() != 40 || !source.commit.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(RuntimeManagerError::InvalidGitCommit(source.commit.clone()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::{InstallStep, RuntimeLayout, RuntimeManifest, build_install_plan, build_sageattention_plan};

    #[test]
    fn repository_runtime_manifest_is_valid_and_pinned() {
        let manifest = RuntimeManifest::from_path(workspace_root().join(
            "engines/hunyuan15-lightx2v/runtime.json",
        ))
        .unwrap();

        assert_eq!(
            manifest.source.commit,
            "19c19614b067f1746ac24350dfd097e1c0b48591"
        );
        assert_eq!(manifest.python.preferred, "3.11");
        assert!(manifest.components.iter().any(|component| component.id == "sageattention2"));
    }

    #[test]
    fn install_plan_contains_explicit_cuda_pytorch_gate() {
        let manifest = RuntimeManifest::from_path(workspace_root().join(
            "engines/hunyuan15-lightx2v/runtime.json",
        ))
        .unwrap();
        let layout = RuntimeLayout::under("/tmp/videogenerator-lightx2v-test");
        let plan = build_install_plan(&manifest, &layout);

        assert!(plan.iter().any(|step| matches!(
            step,
            InstallStep::Gate { id, .. } if id == "cuda-pytorch"
        )));
    }

    #[test]
    fn sageattention_plan_targets_ampere_without_shell_strings() {
        let manifest = RuntimeManifest::from_path(workspace_root().join(
            "engines/hunyuan15-lightx2v/runtime.json",
        ))
        .unwrap();
        let sage = manifest.components.iter().find(|component| component.id == "sageattention2").unwrap();
        let layout = RuntimeLayout::under("/tmp/videogenerator-lightx2v-test");
        let plan = build_sageattention_plan(sage, &layout, "8.6");

        let install = plan.last().unwrap();
        let InstallStep::Command { command, .. } = install else {
            panic!("expected command step");
        };
        assert_eq!(command.program, PathBuf::from("uv"));
        assert_eq!(command.env.get("CUDA_ARCHITECTURES").map(String::as_str), Some("8.6"));
        assert!(!command.args.iter().any(|arg| arg.contains("&&")));
    }

    fn workspace_root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(2)
            .expect("runtime-manager crate must be two levels below workspace root")
            .to_path_buf()
    }
}
