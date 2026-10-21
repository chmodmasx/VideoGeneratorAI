use std::sync::Arc;

use egui::{Align, Layout, RichText};
use videogenerator_domain::HardwareProfile;
use videogenerator_engine_api::Engine;
use videogenerator_jobs::{JobManager, JobState};
use videogenerator_ui_kit::{theme, tokens};

use crate::{create::CreateState, navigation::Section};

pub struct VideoGeneratorApp {
    section: Section,
    create: CreateState,
    jobs: JobManager,
    hardware: HardwareProfile,
}

impl VideoGeneratorApp {
    pub fn new(ctx: &egui::Context, engine: Arc<dyn Engine>) -> Self {
        theme::install(ctx);
        Self {
            section: Section::Create,
            create: CreateState::default(),
            jobs: JobManager::new(engine),
            hardware: HardwareProfile::default(),
        }
    }

    fn navigation(&mut self, ui: &mut egui::Ui) {
        ui.add_space(6.0);
        ui.label(RichText::new("VideoGeneratorAI").strong().size(18.0));
        ui.label(RichText::new("LOCAL VIDEO STUDIO").color(tokens::MUTED_TEXT).size(10.0));
        ui.add_space(16.0);

        for section in Section::ALL {
            if matches!(section, Section::Library) {
                ui.add_space(12.0);
                ui.separator();
                ui.add_space(6.0);
            }
            let selected = self.section == section;
            if ui.selectable_label(selected, section.label()).clicked() {
                self.section = section;
            }
        }
    }

    fn header(&self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.heading(self.section.label());
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.label(format!(
                    "{} · {:.0} GB · {}",
                    self.hardware.gpu_name,
                    self.hardware.vram_gib,
                    self.hardware.compute_capability.as_deref().unwrap_or("unknown")
                ));
            });
        });
    }

    fn queue_view(&mut self, ui: &mut egui::Ui) {
        ui.heading("Queue");
        if self.jobs.jobs().is_empty() {
            ui.label("No jobs yet.");
            return;
        }

        let mut cancel = None;
        for job in self.jobs.jobs().iter().rev() {
            egui::Frame::group(ui.style()).show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.strong(format!("#{} · {}", job.id, job.request.kind.label()));
                    ui.label(job.state.label());
                });
                ui.label(&job.request.prompt);
                ui.add(egui::ProgressBar::new(job.progress).show_percentage().text(&job.stage));
                if matches!(job.state, JobState::Queued | JobState::Running) && ui.small_button("Cancel").clicked() {
                    cancel = Some(job.id);
                }
                if let Some(error) = &job.error {
                    ui.colored_label(egui::Color32::LIGHT_RED, error);
                }
                for output in &job.outputs {
                    ui.monospace(output);
                }
            });
            ui.add_space(6.0);
        }
        if let Some(id) = cancel {
            let _ = self.jobs.cancel(id);
        }
    }

    fn placeholder(ui: &mut egui::Ui, title: &str, body: &str, engine: &str) {
        ui.heading(title);
        ui.label(body);
        ui.add_space(16.0);
        egui::Frame::group(ui.style()).show(ui, |ui| {
            ui.strong("Planned engine");
            ui.monospace(engine);
            ui.label("The UI module is intentionally independent from the runtime adapter.");
        });
    }

    fn bottom_status(&self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let descriptor = self.jobs.engine().descriptor();
            ui.label(format!("Engine: {} {}", descriptor.name, descriptor.version));
            ui.separator();
            if let Some(job) = self.jobs.active() {
                ui.label(format!(
                    "Job #{} · {} · {:.0}%",
                    job.id,
                    job.stage,
                    job.progress * 100.0
                ));
            } else {
                ui.label("Idle");
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.label("offline-first");
            });
        });
    }
}

impl eframe::App for VideoGeneratorApp {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.jobs.poll();
        if self.jobs.active().is_some() {
            ctx.request_repaint_after(std::time::Duration::from_millis(33));
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::Panel::left("nav")
            .resizable(false)
            .default_size(150.0)
            .show(ui, |ui| self.navigation(ui));

        egui::Panel::top("header").show(ui, |ui| self.header(ui));
        egui::Panel::bottom("status").show(ui, |ui| self.bottom_status(ui));

        egui::CentralPanel::default().show(ui, |ui| {
            ui.add_space(8.0);
            match self.section {
                Section::Create => self.create.show(ui, &mut self.jobs),
                Section::Compose => Self::placeholder(
                    ui,
                    "Compose",
                    "References, keyframes, multi-image composition and video editing.",
                    "Hunyuan OmniWeaving",
                ),
                Section::Character => Self::placeholder(
                    ui,
                    "Character",
                    "Subject consistency and character replacement workflows.",
                    "OmniWeaving / HunyuanCustom",
                ),
                Section::Avatar => Self::placeholder(
                    ui,
                    "Avatar",
                    "Audio-driven talking and singing character generation.",
                    "HunyuanVideo-Avatar",
                ),
                Section::Sound => Self::placeholder(
                    ui,
                    "Sound",
                    "Generate synchronized Foley and ambience for existing video.",
                    "HunyuanVideo-Foley",
                ),
                Section::Enhance => Self::placeholder(
                    ui,
                    "Enhance",
                    "Upscale selected generations to a final delivery resolution.",
                    "HunyuanVideo 1.5 SR",
                ),
                Section::Train => Self::placeholder(
                    ui,
                    "Train",
                    "Prepare datasets and train HunyuanVideo 1.5 LoRAs in an isolated trainer runtime.",
                    "HunyuanVideo 1.5 trainer",
                ),
                Section::Library => Self::placeholder(
                    ui,
                    "Library",
                    "Project media, generated assets and lineage metadata will live here.",
                    "Rust media catalog",
                ),
                Section::Queue => self.queue_view(ui),
                Section::Models => Self::placeholder(
                    ui,
                    "Models",
                    "Install, verify and manage model versions without bundling weights into the application.",
                    "Model Manager",
                ),
                Section::Settings => Self::placeholder(
                    ui,
                    "Settings",
                    "Hardware detection, engine runtimes, storage and application preferences.",
                    "Rust core",
                ),
            }
        });
    }
}
