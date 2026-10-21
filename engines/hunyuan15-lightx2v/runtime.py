from __future__ import annotations

import json
import sys
import threading
import traceback
from typing import Any

from lightx2v_adapter import GenerationCancelled, Hunyuan15LightX2VAdapter

PROTOCOL_VERSION = 1
PROTOCOL_STDOUT = sys.stdout

_write_lock = threading.Lock()
_state_lock = threading.Lock()
_cancelled = threading.Event()
_adapter = Hunyuan15LightX2VAdapter()
_active_thread: threading.Thread | None = None
_active_job_id: int | None = None


def emit(message: dict[str, Any]) -> None:
    line = json.dumps(message, separators=(",", ":"))
    with _write_lock:
        PROTOCOL_STDOUT.write(line + "\n")
        PROTOCOL_STDOUT.flush()


def error(request_id: int | None, code: str, message: str) -> None:
    emit(
        {
            "type": "error",
            "request_id": request_id,
            "code": code,
            "message": message,
        }
    )


def job_id(job: dict[str, Any]) -> int:
    raw = job["request"]["job_id"]
    if isinstance(raw, list):
        return int(raw[0])
    return int(raw)


def run_job(job: dict[str, Any]) -> None:
    global _active_job_id

    current_job_id = job_id(job)
    terminal_event: dict[str, Any]

    try:
        emit(
            {
                "type": "event",
                "event": {"Started": {"job_id": current_job_id}},
            }
        )
        outputs = _adapter.generate(job, emit, _cancelled)
        terminal_event = {
            "type": "event",
            "event": {
                "Completed": {
                    "job_id": current_job_id,
                    "outputs": outputs,
                }
            },
        }
    except GenerationCancelled:
        terminal_event = {
            "type": "event",
            "event": {"Cancelled": {"job_id": current_job_id}},
        }
    except Exception as exc:
        traceback.print_exc(file=sys.stderr)
        terminal_event = {
            "type": "event",
            "event": {
                "Failed": {
                    "job_id": current_job_id,
                    "message": f"{type(exc).__name__}: {exc}",
                }
            },
        }

    # Make the engine schedulable before the host can observe a terminal event.
    # This prevents the next serial job from racing with cleanup of this one.
    with _state_lock:
        _active_job_id = None
    emit(terminal_event)


def handle(command: dict[str, Any]) -> bool:
    global _active_thread, _active_job_id

    command_type = command.get("type")
    request_id = command.get("request_id")

    if command_type == "hello":
        requested = int(command.get("protocol", -1))
        if requested != PROTOCOL_VERSION:
            error(None, "protocol_mismatch", f"host={requested}, engine={PROTOCOL_VERSION}")
            return False

        emit(
            {
                "type": "ready",
                "protocol": PROTOCOL_VERSION,
                "descriptor": {
                    "id": "hunyuan15-lightx2v",
                    "name": "HunyuanVideo 1.5 · LightX2V",
                    "version": "0.1.0",
                    "capabilities": ["TextToVideo", "ImageToVideo"],
                    "process_isolated": True,
                },
            }
        )
        return True

    if command_type == "prepare":
        try:
            _adapter.prepare(command["config"])
        except Exception as exc:
            traceback.print_exc(file=sys.stderr)
            error(request_id, "prepare_failed", f"{type(exc).__name__}: {exc}")
            return True

        emit({"type": "ack", "request_id": request_id})
        return True

    if command_type == "submit":
        job = command["job"]
        current_job_id = job_id(job)

        with _state_lock:
            if _active_job_id is not None:
                error(request_id, "busy", "engine already has an active job")
                return True

            _cancelled.clear()
            _active_job_id = current_job_id
            _active_thread = threading.Thread(
                target=run_job,
                args=(job,),
                name=f"hunyuan-job-{current_job_id}",
                daemon=True,
            )
            _active_thread.start()

        emit({"type": "ack", "request_id": request_id})
        return True

    if command_type == "cancel":
        requested_job_id = int(command["job_id"])
        with _state_lock:
            if _active_job_id == requested_job_id:
                _cancelled.set()
        emit({"type": "ack", "request_id": request_id})
        return True

    if command_type == "unload":
        with _state_lock:
            busy = _active_job_id is not None
        if busy:
            error(request_id, "busy", "cannot unload while a job is running")
            return True
        _adapter.unload()
        emit({"type": "ack", "request_id": request_id})
        return True

    if command_type == "shutdown":
        _cancelled.set()
        emit({"type": "ack", "request_id": request_id})
        return False

    error(request_id, "unknown_command", f"unsupported command: {command_type!r}")
    return True


def main() -> int:
    for raw_line in sys.stdin:
        line = raw_line.strip()
        if not line:
            continue

        try:
            command = json.loads(line)
        except json.JSONDecodeError as exc:
            error(None, "invalid_json", str(exc))
            continue

        try:
            if not handle(command):
                return 0
        except Exception as exc:
            traceback.print_exc(file=sys.stderr)
            error(command.get("request_id"), "runtime_error", f"{type(exc).__name__}: {exc}")

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
