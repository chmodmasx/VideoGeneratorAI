use egui::{ComboBox, Slider, Ui};
use videogenerator_domain::{GenerationKind, GenerationRequest, JobId, QualityProfile, VideoSettings};
use videogenerator_jobs::JobManager;

pub struct CreateState {
    pub kind: GenerationKind,
    pub prompt: String,
    pub negative_prompt: String,
    pub profile: QualityProfile,
    pub settings: VideoSettings,
    pub seed: u64,
    last_error: Option<String>,
}

impl Default for CreateState {
    fn default() -> Self {
        Self {
            kind: GenerationKind::TextToVideo,
            prompt: String::new(),
            negative_prompt: String::new(),
            profile: QualityProfile::Preview,
            settings: VideoSettings::default(),
            seed: 0,
            last_error: None,
        }
    }
}

impl CreateState {
    pub fn show(&mut self, ui: &mut Ui, jobs: &mut JobManager) {
        ui.heading("Create");
        ui.label("Generate with intent-level controls. Backend details stay optional.");
        ui.add_space(8.0);

        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.kind, GenerationKind::TextToVideo, "Text to Video");
            ui.selectable_value(&mut self.kind, GenerationKind::ImageToVideo, "Image to Video");
        });

        ui.add_space(8.0);
        ui.label("Prompt");
        ui.add(
            egui::TextEdit::multiline(&mut self.prompt)
                .desired_rows(6)
                .hint_text("Describe the shot, motion, camera and subject..."),
        );

        ui.collapsing("Negative prompt", |ui| {
            ui.add(egui::TextEdit::multiline(&mut self.negative_prompt).desired_rows(2));
        });

        ui.separator();
        ui.horizontal_wrapped(|ui| {
            ComboBox::from_label("Profile")
                .selected_text(self.profile.label())
                .show_ui(ui, |ui| {
                    for profile in [
                        QualityProfile::Preview,
                        QualityProfile::Balanced,
                        QualityProfile::Quality,
                    ] {
                        if ui
                            .selectable_value(&mut self.profile, profile, profile.label())
                            .clicked()
                        {
                            self.apply_profile();
                        }
                    }
                });

            let mut resolution = (self.settings.width, self.settings.height);
            ComboBox::from_label("Resolution")
                .selected_text(format!("{}×{}", resolution.0, resolution.1))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut resolution, (832, 480), "832×480");
                    ui.selectable_value(&mut resolution, (1280, 720), "1280×720");
                });
            (self.settings.width, self.settings.height) = resolution;
        });

        ui.horizontal(|ui| {
            ui.add(Slider::new(&mut self.settings.frames, 25..=241).text("frames"));
            ui.add(Slider::new(&mut self.settings.steps, 4..=50).text("steps"));
        });
        ui.add(Slider::new(&mut self.seed, 0..=u32::MAX as u64).text("seed"));

        ui.add_space(12.0);
        let can_generate = !self.prompt.trim().is_empty();
        if ui
            .add_enabled(
                can_generate,
                egui::Button::new("Generate").min_size(egui::vec2(130.0, 36.0)),
            )
            .clicked()
        {
            let request = GenerationRequest {
                job_id: JobId(0),
                kind: self.kind,
                prompt: self.prompt.trim().to_owned(),
                negative_prompt: self.negative_prompt.trim().to_owned(),
                profile: self.profile,
                settings: self.settings.clone(),
                seed: self.seed,
                input_paths: Vec::new(),
            };
            self.last_error = jobs.submit(request).err().map(|error| error.to_string());
        }

        if let Some(error) = &self.last_error {
            ui.colored_label(egui::Color32::LIGHT_RED, error);
        }
    }

    fn apply_profile(&mut self) {
        self.settings = match self.profile {
            QualityProfile::Preview => VideoSettings {
                width: 832,
                height: 480,
                frames: 121,
                fps: 24,
                steps: 4,
            },
            QualityProfile::Balanced => VideoSettings {
                width: 832,
                height: 480,
                frames: 121,
                fps: 24,
                steps: 8,
            },
            QualityProfile::Quality => VideoSettings {
                width: 1280,
                height: 720,
                frames: 121,
                fps: 24,
                steps: 20,
            },
        };
    }
}
