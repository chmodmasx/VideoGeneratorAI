# Security model

- No WebView and no remote JavaScript in the native UI.
- The desktop process does not execute arbitrary shell snippets from UI input.
- AI engines run as child processes with explicit working directories and environment variables.
- Model downloads must have explicit source metadata and, where available, expected hashes.
- Model code is not implicitly trusted because its weights are trusted.
- External providers such as ComfyUI are opt-in adapters, not privileged core components.
