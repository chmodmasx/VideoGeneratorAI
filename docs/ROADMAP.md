# Roadmap

## v0.1 — Foundation

- [x] Rust workspace and native egui/eframe shell.
- [x] Stable domain types.
- [x] Engine capability boundary.
- [x] Serial single-GPU job manager with progress/cancellation.
- [x] Mock engine.
- [x] Shared UI-kit boundary.
- [x] Initial product navigation and Create UI.
- [ ] Run CI on the repository.

## v0.2 — HunyuanVideo 1.5

- Detect NVIDIA/CUDA/RAM/storage.
- Isolated Python runtime management.
- LightX2V engine adapter.
- T2V and I2V.
- RTX 3090 benchmark matrix.
- Model manager with hashes/licenses.

## v0.3 — Delivery pipeline

- Media library and lineage database.
- HunyuanVideo 1.5 SR.
- Chained Generate -> Enhance jobs.
- Reproducible project files.

## v0.4 — OmniWeaving

- Keyframes.
- Multiple references.
- Reference-to-video.
- Video editing/V2V.
- Character replacement.

## v0.5 — Audio

- HunyuanVideo-Foley runtime.
- Generate -> Foley chains.

## v0.6 — Characters/Avatar

- HunyuanVideo-Avatar.
- Evaluate HunyuanCustom fallback where OmniWeaving is insufficient.
- Optional WanGP compatibility provider only where its license permits.

## v0.7 — Training

- HunyuanVideo 1.5 LoRA training engine.
- Dataset preparation and validation.
- Training queue/checkpoints.

## v1.0

- Versioned process engine protocol.
- Engine crash recovery.
- Certified RTX 3090 presets.
- Linux packaging and reproducible release pipeline.
