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
