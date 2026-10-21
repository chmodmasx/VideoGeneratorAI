# Architecture

## Goal

VideoGeneratorAI is a native desktop application whose product model remains stable while AI runtimes change independently.

The application must never encode Hunyuan/ComfyUI/WanGP implementation details into core domain objects. Backends advertise capabilities and accept jobs through a versioned engine contract.

## Layers

```text
Native UI (egui)
      |
Application/core services (Rust)
      |
Job scheduler + media/model managers (Rust)
      |
Engine API
      |
+------------------+------------------+------------------+
|                  |                  |                  |
Hunyuan/LightX2V   OmniWeaving        Foley / Avatar    External providers
process             process            processes          ComfyUI/WanGP
      |
Python/PyTorch runtimes, isolated per engine
```

## Crates

- `domain`: stable data types; no UI or engine implementation dependencies.
- `engine-api`: capabilities, requests, events and the engine boundary.
- `engine-host`: process hosting/adapters; currently includes only the deterministic `MockEngine`.
- `jobs`: engine-independent queue/state machine.
- `ui-kit`: shared design tokens and generic visual primitives; no domain-specific features.
- `ui-egui`: native product UI. It talks to application abstractions, never Python directly.
- `apps/videogenerator`: thin desktop launcher.

## Process isolation

Real inference engines will run out-of-process. Each runtime gets its own environment and pinned dependencies.

```text
engines/runtime/
  hunyuan15-lightx2v/
  omniweaving/
  hunyuan-avatar/
  hunyuan-foley/
  comfyui/
```

A crashed inference runtime must turn one job into `failed`; it must not terminate the desktop process.

## Dependency rule

Dependencies point inward:

```text
ui-egui -> ui-kit
UI -> jobs -> engine-api -> domain
app -> UI + engine-host
engine-host -> engine-api -> domain
```

`domain` knows nothing about egui, Python, CUDA, Hunyuan or filesystem layout.

## Hardware policy

The first certified profile is NVIDIA RTX 3090 24 GB (`sm86`). Engine manifests will carry hardware constraints and precision recommendations. Hardware-specific values belong to engine manifests/presets, not shared UI code.
