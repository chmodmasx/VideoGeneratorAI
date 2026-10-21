# Model and runtime management

VideoGeneratorAI deliberately treats **runtime installation** and **model installation** as separate responsibilities.

## Runtime Manager

The Runtime Manager owns executable software needed by an engine:

- pinned upstream source revision;
- isolated Python environment;
- CUDA/PyTorch compatibility gate;
- attention/runtime components such as SageAttention;
- probes for host tooling.

It does not know which model profile the user wants.

Runtime installation plans are arrays of executable + arguments + environment. They are **not shell strings**. This prevents an engine manifest from becoming an arbitrary shell script.

The first LightX2V runtime is pinned to commit:

```text
19c19614b067f1746ac24350dfd097e1c0b48591
```

Python 3.11 is preferred. Automatic installation intentionally stops at a CUDA/PyTorch gate until VideoGeneratorAI has a tested matrix for the target NVIDIA driver/CUDA/PyTorch combinations.

For the RTX 3090 target, SageAttention is compiled specifically for CUDA architecture `8.6`.

## Model Manager

The Model Manager owns data artifacts:

- source/repository;
- include filters;
- destination inside a shared model root;
- gating/authentication requirements;
- license notice;
- expected files/directories;
- capability profiles assembled from components.

It does not create Python environments and does not launch inference.

## HunyuanVideo 1.5 profiles

Downloading the entire upstream HunyuanVideo 1.5 repository is wasteful when the user only needs one task/resolution. The initial manifest therefore exposes:

- Base T2V 480p;
- Base T2V 720p;
- Base I2V 480p;
- Base I2V 720p.

Shared T2V components are VAE, Qwen2.5-VL, ByT5 and Glyph-SDXL-v2. I2V additionally needs the SigLIP vision encoder. Each profile then selects exactly one transformer.

The SigLIP dependency currently follows Tencent's official setup and comes from the gated `black-forest-labs/FLUX.1-Redux-dev` repository. The UI must expose that gate and authentication requirement instead of failing later during inference.

## Download execution

This phase validates and plans. Network downloads are intentionally a later adapter:

```text
Model Manager
      ↓
Download Plan
      ↓
Provider adapter
  ├── Hugging Face
  └── ModelScope
```

Provider credentials never belong in engine manifests or command-line strings persisted by the application.
