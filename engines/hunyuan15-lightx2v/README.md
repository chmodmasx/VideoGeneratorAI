# HunyuanVideo 1.5 · LightX2V engine

This directory is an adapter, not a fork of LightX2V.

Responsibilities are deliberately split:

- `runtime.py`: VideoGeneratorAI JSONL protocol supervisor.
- `lightx2v_adapter.py`: the only module that knows the upstream LightX2V Python API.
- `engine.json`: provenance, hardware constraints and defaults.

The first implementation targets the original BF16 HunyuanVideo 1.5 480p/720p checkpoints with block offload and optional LightTAE. It does **not** silently run the base checkpoint at four or eight steps: the official base model is configured for 50 steps. Distilled models will be represented by separate engine presets once benchmarked.

The target hardware is an RTX 3090 24 GB (Ampere/sm86). FP8 LightX2V weights are not selected by default because Ampere has no native FP8 tensor path.

## Runtime configuration

The Rust host sends `prepare` with:

```json
{
  "model_root": "/path/to/hunyuanvideo-1.5",
  "output_root": "/path/to/outputs",
  "options": {
    "attention": "sage_attn2",
    "cpu_offload": true,
    "text_encoder_offload": true,
    "image_encoder_offload": true,
    "vae_offload": false,
    "lighttae_path": "/optional/path/to/lighttaehy1_5.safetensors"
  }
}
```

The runtime keeps a LightX2V pipeline resident while subsequent jobs use the same task/resolution/settings. Switching to a different transformer or generator configuration rebuilds the pipeline.

Cancellation is cooperative when the upstream runner exposes its progress callback. Process-level hard cancellation/restart remains a host responsibility and will be added before the engine is marked production-ready.


## Expected model root

The base `model_root` follows the upstream HunyuanVideo 1.5 checkpoint layout. It is not only the DiT directory.

At minimum, the installation workflow must account for:

- `tencent/HunyuanVideo-1.5` base checkpoints, including the resolution/task transformers and VAE;
- Qwen2.5-VL-7B-Instruct under the Hunyuan text-encoder layout;
- `google/byt5-small`;
- Glyph-SDXL-v2 ByT5 assets/checkpoint;
- the SigLIP vision encoder used by Hunyuan I2V.

The upstream Hunyuan instructions currently source that SigLIP component from `black-forest-labs/FLUX.1-Redux-dev`, which is gated on Hugging Face. VideoGeneratorAI must therefore model this as an authenticated/gated dependency rather than silently assuming it can be downloaded.

LightX2V resolves base DiTs as:

```text
model_root/
└── transformer/
    ├── 480p_t2v/
    ├── 720p_t2v/
    ├── 480p_i2v/
    └── 720p_i2v/
```

Only the transformer actually selected by the job must be present for that generation mode, but shared encoders/VAEs still need to satisfy the upstream Hunyuan layout. The future Model Manager will validate components individually rather than treating `model_root` as an opaque directory.
