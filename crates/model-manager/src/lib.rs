use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use thiserror::Error;
use videogenerator_domain::GenerationKind;

pub const MODEL_MANIFEST_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceProvider {
    HuggingFace,
    ModelScope,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelSource {
    pub provider: SourceProvider,
    pub repository: String,
    #[serde(default)]
    pub revision: Option<String>,
    #[serde(default)]
    pub gated: bool,
    #[serde(default)]
    pub include: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LicenseNotice {
    pub name: String,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub acceptance_required: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExpectedPathKind {
    File,
    Directory,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExpectedPath {
    pub relative_path: String,
    pub kind: ExpectedPathKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelComponent {
    pub id: String,
    pub label: String,
    pub destination: String,
    pub source: ModelSource,
    pub license: LicenseNotice,
    pub expected_paths: Vec<ExpectedPath>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelProfile {
    pub id: String,
    pub label: String,
    pub capability: GenerationKind,
    pub required_components: Vec<String>,
    #[serde(default)]
    pub recommended_components: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelBundleManifest {
    pub schema_version: u32,
    pub id: String,
    pub name: String,
    pub components: Vec<ModelComponent>,
    pub profiles: Vec<ModelProfile>,
}

impl ModelBundleManifest {
    pub fn parse_json(input: &str) -> Result<Self, ModelManagerError> {
        let manifest: Self = serde_json::from_str(input)?;
        manifest.validate()?;
        Ok(manifest)
    }

    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, ModelManagerError> {
        let contents = fs::read_to_string(path.as_ref()).map_err(|source| ModelManagerError::ReadManifest {
            path: path.as_ref().to_path_buf(),
            source,
        })?;
        Self::parse_json(&contents)
    }

    pub fn validate(&self) -> Result<(), ModelManagerError> {
        if self.schema_version != MODEL_MANIFEST_SCHEMA_VERSION {
            return Err(ModelManagerError::UnsupportedSchema {
                found: self.schema_version,
                supported: MODEL_MANIFEST_SCHEMA_VERSION,
            });
        }

        let mut component_ids = BTreeSet::new();
        for component in &self.components {
            if !component_ids.insert(component.id.as_str()) {
                return Err(ModelManagerError::DuplicateComponent(component.id.clone()));
            }
            validate_relative_path(&component.destination)?;
            if component.expected_paths.is_empty() {
                return Err(ModelManagerError::ComponentHasNoExpectedPaths(component.id.clone()));
            }
            for expected in &component.expected_paths {
                validate_relative_path(&expected.relative_path)?;
            }
        }

        let mut profile_ids = BTreeSet::new();
        for profile in &self.profiles {
            if !profile_ids.insert(profile.id.as_str()) {
                return Err(ModelManagerError::DuplicateProfile(profile.id.clone()));
            }
            for component_id in profile.required_components.iter().chain(&profile.recommended_components) {
                if !component_ids.contains(component_id.as_str()) {
                    return Err(ModelManagerError::UnknownProfileComponent {
                        profile: profile.id.clone(),
                        component: component_id.clone(),
                    });
                }
            }
        }

        Ok(())
    }

    pub fn inspect(&self, root: impl AsRef<Path>) -> BundleInspection {
        let root = root.as_ref();
        let components = self
            .components
            .iter()
            .map(|component| inspect_component(root, component))
            .collect::<Vec<_>>();

        let installed = components
            .iter()
            .filter(|component| component.installed)
            .map(|component| component.id.as_str())
            .collect::<BTreeSet<_>>();

        let profiles = self
            .profiles
            .iter()
            .map(|profile| {
                let missing_components = profile
                    .required_components
                    .iter()
                    .filter(|component| !installed.contains(component.as_str()))
                    .cloned()
                    .collect::<Vec<_>>();

                ProfileInspection {
                    id: profile.id.clone(),
                    available: missing_components.is_empty(),
                    missing_components,
                }
            })
            .collect();

        BundleInspection {
            components,
            profiles,
        }
    }

    pub fn component(&self, id: &str) -> Option<&ModelComponent> {
        self.components.iter().find(|component| component.id == id)
    }

    pub fn profile(&self, id: &str) -> Option<&ModelProfile> {
        self.profiles.iter().find(|profile| profile.id == id)
    }

    pub fn required_sources_for_profile(
        &self,
        profile_id: &str,
    ) -> Result<Vec<&ModelComponent>, ModelManagerError> {
        let profile = self
            .profile(profile_id)
            .ok_or_else(|| ModelManagerError::UnknownProfile(profile_id.to_owned()))?;

        let by_id = self
            .components
            .iter()
            .map(|component| (component.id.as_str(), component))
            .collect::<BTreeMap<_, _>>();

        profile
            .required_components
            .iter()
            .map(|component_id| {
                by_id.get(component_id.as_str()).copied().ok_or_else(|| {
                    ModelManagerError::UnknownProfileComponent {
                        profile: profile.id.clone(),
                        component: component_id.clone(),
                    }
                })
            })
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComponentInspection {
    pub id: String,
    pub installed: bool,
    pub missing_paths: Vec<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfileInspection {
    pub id: String,
    pub available: bool,
    pub missing_components: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BundleInspection {
    pub components: Vec<ComponentInspection>,
    pub profiles: Vec<ProfileInspection>,
}

impl BundleInspection {
    pub fn profile(&self, id: &str) -> Option<&ProfileInspection> {
        self.profiles.iter().find(|profile| profile.id == id)
    }
}

#[derive(Debug, Error)]
pub enum ModelManagerError {
    #[error("failed to read model manifest {path}: {source}")]
    ReadManifest {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("invalid model manifest JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("unsupported model manifest schema {found}; supported schema is {supported}")]
    UnsupportedSchema { found: u32, supported: u32 },
    #[error("duplicate model component id: {0}")]
    DuplicateComponent(String),
    #[error("duplicate model profile id: {0}")]
    DuplicateProfile(String),
    #[error("component {0} has no expected paths")]
    ComponentHasNoExpectedPaths(String),
    #[error("profile {profile} references unknown component {component}")]
    UnknownProfileComponent { profile: String, component: String },
    #[error("unknown model profile: {0}")]
    UnknownProfile(String),
    #[error("manifest path must be relative and cannot contain parent traversal: {0}")]
    UnsafeRelativePath(String),
}

fn inspect_component(root: &Path, component: &ModelComponent) -> ComponentInspection {
    let destination = root.join(&component.destination);
    let mut missing_paths = Vec::new();

    for expected in &component.expected_paths {
        let path = destination.join(&expected.relative_path);
        let matches_kind = match expected.kind {
            ExpectedPathKind::File => path.is_file(),
            ExpectedPathKind::Directory => path.is_dir(),
        };
        if !matches_kind {
            missing_paths.push(path);
        }
    }

    ComponentInspection {
        id: component.id.clone(),
        installed: missing_paths.is_empty(),
        missing_paths,
    }
}

fn validate_relative_path(path: &str) -> Result<(), ModelManagerError> {
    let candidate = Path::new(path);
    if candidate.is_absolute()
        || candidate
            .components()
            .any(|component| matches!(component, std::path::Component::ParentDir))
    {
        return Err(ModelManagerError::UnsafeRelativePath(path.to_owned()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::{Path, PathBuf},
    };

    use super::ModelBundleManifest;

    #[test]
    fn repository_hunyuan_manifest_is_valid() {
        let manifest = ModelBundleManifest::from_path(workspace_root().join(
            "engines/hunyuan15-lightx2v/models.json",
        ))
        .unwrap();

        assert!(manifest.profile("base-t2v-480p").is_some());
        assert!(manifest.profile("base-i2v-720p").is_some());

        let i2v = manifest.required_sources_for_profile("base-i2v-480p").unwrap();
        assert!(i2v.iter().any(|component| component.id == "siglip"));
    }

    #[test]
    fn t2v_does_not_require_gated_siglip() {
        let manifest = ModelBundleManifest::from_path(workspace_root().join(
            "engines/hunyuan15-lightx2v/models.json",
        ))
        .unwrap();

        let t2v = manifest.required_sources_for_profile("base-t2v-480p").unwrap();
        assert!(!t2v.iter().any(|component| component.id == "siglip"));
    }

    #[test]
    fn inspection_enables_only_complete_profiles() {
        let manifest = ModelBundleManifest::from_path(workspace_root().join(
            "engines/hunyuan15-lightx2v/models.json",
        ))
        .unwrap();

        let scratch = std::env::temp_dir().join(format!(
            "videogenerator-model-manager-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&scratch);

        for component_id in [
            "vae",
            "qwen25vl",
            "byt5",
            "glyph-byt5",
            "transformer-480p-t2v",
        ] {
            materialize_component(&scratch, manifest.component(component_id).unwrap());
        }

        let inspection = manifest.inspect(&scratch);
        assert!(inspection.profile("base-t2v-480p").unwrap().available);
        assert!(!inspection.profile("base-i2v-480p").unwrap().available);

        let _ = fs::remove_dir_all(scratch);
    }

    fn materialize_component(root: &Path, component: &super::ModelComponent) {
        let destination = root.join(&component.destination);
        for expected in &component.expected_paths {
            let path = destination.join(&expected.relative_path);
            match expected.kind {
                super::ExpectedPathKind::File => {
                    fs::create_dir_all(path.parent().unwrap()).unwrap();
                    fs::write(path, b"test").unwrap();
                }
                super::ExpectedPathKind::Directory => {
                    fs::create_dir_all(path).unwrap();
                }
            }
        }
    }

    fn workspace_root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(2)
            .expect("model-manager crate must be two levels below workspace root")
            .to_path_buf()
    }
}
