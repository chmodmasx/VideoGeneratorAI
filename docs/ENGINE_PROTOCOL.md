# Engine protocol

The in-process Rust `Engine` trait in v0.1 is a development boundary. Production engines will be isolated processes speaking a small versioned protocol over JSON Lines on stdin/stdout (or an equivalent local IPC transport if measurements justify it).

## Commands

- `hello`
- `describe`
- `probe`
- `prepare`
- `submit`
- `cancel`
- `unload`
- `shutdown`

## Events

- `started`
- `stage`
- `progress`
- `preview`
- `completed`
- `failed`
- `cancelled`

## Invariants

1. An engine may not write arbitrary application state.
2. Paths supplied to an engine are explicit capabilities for that job.
3. Engine stdout is protocol-only; diagnostics go to stderr.
4. Cancellation is idempotent.
5. A completed job records the engine/model/preset versions required for reproduction.
6. The protocol does not expose Python objects or backend-specific classes.
