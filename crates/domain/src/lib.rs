use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct JobId(pub u64);

impl std::fmt::Display for JobId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GenerationKind {
    TextToVideo,
    ImageToVideo,
    VideoToVideo,
    ReferenceToVideo,
    Avatar,
    Foley,
    SuperResolution,
    LoraTraining,
}

impl GenerationKind {
    pub const fn label(self) -> &'static str {
        match self {
            Self::TextToVideo => "Text to Video",
            Self::ImageToVideo => "Image to Video",
            Self::VideoToVideo => "Video to Video",
            Self::ReferenceToVideo => "Reference to Video",
            Self::Avatar => "Avatar",
            Self::Foley => "Foley",
            Self::SuperResolution => "Super Resolution",
            Self::LoraTraining => "LoRA Training",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum QualityProfile {
    Preview,
    Balanced,
    Quality,
}

impl QualityProfile {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Preview => "Preview",
            Self::Balanced => "Balanced",
            Self::Quality => "Quality",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VideoSettings {
    pub width: u32,
    pub height: u32,
    pub frames: u32,
    pub fps: u32,
    pub steps: u32,
}

impl Default for VideoSettings {
    fn default() -> Self {
        Self {
            width: 832,
            height: 480,
            frames: 121,
            fps: 24,
            steps: 8,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GenerationRequest {
    pub job_id: JobId,
    pub kind: GenerationKind,
    pub prompt: String,
    pub negative_prompt: String,
    pub profile: QualityProfile,
    pub settings: VideoSettings,
    pub seed: u64,
    pub input_paths: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HardwareProfile {
    pub gpu_name: String,
    pub vram_gib: f32,
    pub compute_capability: Option<String>,
}

impl Default for HardwareProfile {
    fn default() -> Self {
        Self {
            gpu_name: "RTX 3090 target".to_owned(),
            vram_gib: 24.0,
            compute_capability: Some("sm86".to_owned()),
        }
    }
}
