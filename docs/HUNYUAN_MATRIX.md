# Hunyuan integration matrix

The matrix is deliberately split by capability. A single provider is not allowed to become an architectural dependency.

| Product capability | Preferred engine | Initial status |
| --- | --- | --- |
| Text to video | HunyuanVideo 1.5 + LightX2V | planned v0.2 |
| Image to video | HunyuanVideo 1.5 + LightX2V | planned v0.2 |
| 480p/720p preview/final | HunyuanVideo 1.5 | planned v0.2 |
| 1080p enhancement | HunyuanVideo 1.5 SR | planned v0.3 |
| Keyframes | OmniWeaving | planned v0.4 |
| Reference-to-video | OmniWeaving | planned v0.4 |
| Multi-image composition | OmniWeaving | planned v0.4 |
| Video editing / V2V | OmniWeaving | planned v0.4 |
| Character consistency/replacement | OmniWeaving, HunyuanCustom fallback | planned v0.4/v0.6 |
| Audio-driven avatar | HunyuanVideo-Avatar | planned v0.6 |
| Foley / ambience | HunyuanVideo-Foley | planned v0.5 |
| LoRA training | HunyuanVideo 1.5 trainer | planned v0.7 |
| Advanced external workflows | ComfyUI adapter | optional |
| Low-VRAM external provider | WanGP adapter | optional; license boundary |
| Prism | future | not a 24 GB target today |

## RTX 3090 policy

- Prefer Ampere-native paths; do not advertise FP8/NVFP4 paths as native when they are not.
- Preview and quality presets are engine-owned metadata.
- Offload is an engine concern, not a UI concern.
- Every integrated configuration must be benchmarked on the actual 24 GB target before being marked `certified`.
