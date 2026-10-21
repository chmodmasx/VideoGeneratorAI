from __future__ import annotations

import contextlib
import gc
import math
import os
import sys
import threading
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Callable


class GenerationCancelled(RuntimeError):
    pass


@dataclass(frozen=True)
class PipelineKey:
    task: str
    transformer: str
    width: int
    height: int
    frames: int
    steps: int
    fps: int
    attention: str


class Hunyuan15LightX2VAdapter:
    """Owns the upstream LightX2V API boundary.

    No JSONL protocol code belongs here. If LightX2V changes its Python API,
    this is the module that should change.
    """

    def __init__(self) -> None:
        self._config: dict[str, Any] | None = None
        self._pipeline: Any = None
        self._pipeline_key: PipelineKey | None = None

    def prepare(self, config: dict[str, Any]) -> None:
        model_root = Path(config["model_root"]).expanduser()
        output_root = Path(config["output_root"]).expanduser()
        if not model_root.exists():
            raise FileNotFoundError(f"model_root does not exist: {model_root}")
        output_root.mkdir(parents=True, exist_ok=True)

        self._config = {
            **config,
            "model_root": str(model_root.resolve()),
            "output_root": str(output_root.resolve()),
        }

    def unload(self) -> None:
        self._pipeline = None
        self._pipeline_key = None
        gc.collect()

        try:
            import torch

            if torch.cuda.is_available():
                torch.cuda.empty_cache()
        except ImportError:
            pass

    def generate(
        self,
        job: dict[str, Any],
        emit: Callable[[dict[str, Any]], None],
        cancelled: threading.Event,
    ) -> list[str]:
        if self._config is None:
            raise RuntimeError("engine has not been prepared")

        request = job["request"]
        job_id = request["job_id"][0] if isinstance(request["job_id"], list) else request["job_id"]
        task = self._task(request["kind"])
        input_paths = request.get("input_paths", [])

        if task == "i2v" and not input_paths:
            raise ValueError("ImageToVideo requires one input image path")

        settings = request["settings"]
        width = int(settings["width"])
        height = int(settings["height"])
        frames = int(settings["frames"])
        steps = int(settings["steps"])
        fps = int(settings["fps"])
        attention = str(self._config.get("options", {}).get("attention", "sage_attn2"))

        self._validate_base_configuration(width, height, frames, steps)

        transformer = f"{'720p' if height > 480 else '480p'}_{task}"
        key = PipelineKey(
            task=task,
            transformer=transformer,
            width=width,
            height=height,
            frames=frames,
            steps=steps,
            fps=fps,
            attention=attention,
        )

        emit(self._event("stage", job_id, name="Loading model"))
        pipe = self._ensure_pipeline(key, fps, cancelled, emit, job_id)

        if cancelled.is_set():
            raise GenerationCancelled()

        emit(self._event("stage", job_id, name="Sampling"))
        self._install_progress_callback(pipe, emit, cancelled, job_id)

        output_path = Path(job["output_path"]).expanduser()
        output_path.parent.mkdir(parents=True, exist_ok=True)

        kwargs: dict[str, Any] = {
            "seed": int(request["seed"]),
            "prompt": request["prompt"],
            "negative_prompt": request.get("negative_prompt", ""),
            "save_result_path": str(output_path),
        }
        if task == "i2v":
            kwargs["image_path"] = str(Path(input_paths[0]).expanduser())

        with contextlib.redirect_stdout(sys.stderr):
            pipe.generate(**kwargs)

        if cancelled.is_set():
            raise GenerationCancelled()

        emit(self._event("stage", job_id, name="Finalizing"))
        return [str(output_path)]

    def _ensure_pipeline(
        self,
        key: PipelineKey,
        fps: int,
        cancelled: threading.Event,
        emit: Callable[[dict[str, Any]], None],
        job_id: int,
    ) -> Any:
        if self._pipeline is not None and self._pipeline_key == key:
            return self._pipeline

        if cancelled.is_set():
            raise GenerationCancelled()

        self.unload()

        assert self._config is not None
        options = self._config.get("options", {})

        # stdout is reserved for VideoGeneratorAI's JSONL protocol. LightX2V
        # currently prints runner configuration during setup, so all upstream
        # stdout is redirected to stderr.
        with contextlib.redirect_stdout(sys.stderr):
            from lightx2v import LightX2VPipeline

            pipe = LightX2VPipeline(
                model_path=self._config["model_root"],
                model_cls="hunyuan_video_1.5",
                transformer_model_name=key.transformer,
                task=key.task,
            )

            pipe.enable_offload(
                cpu_offload=bool(options.get("cpu_offload", True)),
                offload_granularity="block",
                text_encoder_offload=bool(options.get("text_encoder_offload", True)),
                image_encoder_offload=bool(options.get("image_encoder_offload", key.task == "i2v")),
                vae_offload=bool(options.get("vae_offload", False)),
            )

            lighttae_path = options.get("lighttae_path")
            if lighttae_path:
                tae = Path(lighttae_path).expanduser()
                if not tae.is_file():
                    raise FileNotFoundError(f"LightTAE checkpoint not found: {tae}")
                pipe.enable_lightvae(
                    use_tae=True,
                    tae_path=str(tae),
                    use_lightvae=False,
                    vae_path=None,
                )

            pipe.create_generator(
                attn_mode=key.attention,
                infer_steps=key.steps,
                num_frames=key.frames,
                size=(key.height, key.width),
                guidance_scale=6.0,
                sample_shift=self._sample_shift(key.task, key.height),
                aspect_ratio=self._aspect_ratio(key.width, key.height),
                fps=fps,
            )

        self._pipeline = pipe
        self._pipeline_key = key
        emit(self._event("progress", job_id, fraction=0.02))
        return pipe

    @staticmethod
    def _install_progress_callback(
        pipe: Any,
        emit: Callable[[dict[str, Any]], None],
        cancelled: threading.Event,
        job_id: int,
    ) -> None:
        runner = getattr(pipe, "runner", None)
        if runner is None or not hasattr(runner, "set_progress_callback"):
            return

        def progress(current: float, total: float) -> None:
            if cancelled.is_set():
                raise GenerationCancelled()
            fraction = 0.0 if total <= 0 else float(current) / float(total)
            emit(Hunyuan15LightX2VAdapter._event("progress", job_id, fraction=fraction))

        runner.set_progress_callback(progress)

    @staticmethod
    def _task(kind: str) -> str:
        mapping = {
            "TextToVideo": "t2v",
            "ImageToVideo": "i2v",
        }
        try:
            return mapping[kind]
        except KeyError as error:
            raise ValueError(f"unsupported generation kind: {kind}") from error

    @staticmethod
    def _sample_shift(task: str, height: int) -> float:
        if height <= 480:
            return 5.0
        return 9.0 if task == "t2v" else 7.0

    @staticmethod
    def _aspect_ratio(width: int, height: int) -> str:
        divisor = math.gcd(width, height)
        return f"{width // divisor}:{height // divisor}"

    @staticmethod
    def _validate_base_configuration(width: int, height: int, frames: int, steps: int) -> None:
        if height not in {480, 720}:
            raise ValueError("base HunyuanVideo 1.5 runtime currently supports 480p or 720p height")
        if width <= 0 or frames <= 0:
            raise ValueError("width and frames must be positive")
        if steps != 50:
            raise ValueError(
                "base HunyuanVideo 1.5 is certified for 50 steps; distilled variants will be separate presets"
            )

    @staticmethod
    def _event(kind: str, job_id: int, **payload: Any) -> dict[str, Any]:
        variant = {
            "started": "Started",
            "stage": "Stage",
            "progress": "Progress",
            "cancelled": "Cancelled",
            "failed": "Failed",
        }[kind]
        return {
            "type": "event",
            "event": {
                variant: {
                    "job_id": job_id,
                    **payload,
                }
            },
        }
