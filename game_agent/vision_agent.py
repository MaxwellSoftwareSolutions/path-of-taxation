"""Vision-language model wrapper for game-frame analysis.

Loads Qwen2.5-VL-7B-Instruct via ``transformers`` and provides a simple
``analyse_frame`` method that accepts an image path and prompt pair, returning
a parsed JSON dict.

A lightweight ``DummyVisionAgent`` is provided for pipeline testing without
a GPU.
"""

from __future__ import annotations

import json
import logging
import re
from pathlib import Path
from typing import Any

logger = logging.getLogger(__name__)


# ---------------------------------------------------------------------------
# JSON extraction helper
# ---------------------------------------------------------------------------

_CODE_FENCE_RE = re.compile(r"```(?:json)?\s*([\s\S]*?)```", re.IGNORECASE)


def _extract_json(text: str) -> dict[str, Any]:
    """Parse JSON from *text*, stripping markdown fences if present."""
    # Try direct parse first.
    text = text.strip()
    try:
        return json.loads(text)
    except json.JSONDecodeError:
        pass

    # Try extracting from code fences.
    match = _CODE_FENCE_RE.search(text)
    if match:
        try:
            return json.loads(match.group(1).strip())
        except json.JSONDecodeError:
            pass

    # Last resort: find first { ... } block.
    start = text.find("{")
    end = text.rfind("}")
    if start != -1 and end != -1 and end > start:
        try:
            return json.loads(text[start : end + 1])
        except json.JSONDecodeError:
            pass

    raise ValueError(f"Could not parse JSON from model output:\n{text[:500]}")


# ---------------------------------------------------------------------------
# Real vision agent
# ---------------------------------------------------------------------------

class VisionAgent:
    """Wraps Qwen2.5-VL for single-image analysis.

    Args:
        model_id: HuggingFace model identifier.
        device: Target device (``"cuda"`` or ``"cpu"``).
        max_new_tokens: Maximum tokens to generate.
        temperature: Sampling temperature.
    """

    def __init__(
        self,
        model_id: str,
        *,
        device: str = "cuda",
        max_new_tokens: int = 512,
        temperature: float = 0.3,
    ) -> None:
        self.model_id = model_id
        self.device = device
        self.max_new_tokens = max_new_tokens
        self.temperature = temperature
        self._model: Any = None
        self._processor: Any = None
        self._loaded = False

        self._load_model()

    # -- model loading -----------------------------------------------------

    def _load_model(self) -> None:
        """Load the VLM and processor.  Warns and stays unloaded on failure."""
        try:
            import torch  # noqa: PLC0415
            from transformers import (  # noqa: PLC0415
                Qwen2_5_VLForConditionalGeneration,
                AutoProcessor,
            )

            logger.info("Loading VLM '%s' on device='%s' ...", self.model_id, self.device)

            if self.device == "cuda" and not torch.cuda.is_available():
                logger.warning(
                    "CUDA requested but unavailable.  Falling back to CPU "
                    "(this will be very slow)."
                )
                self.device = "cpu"

            self._model = Qwen2_5_VLForConditionalGeneration.from_pretrained(
                self.model_id,
                torch_dtype="auto",
                device_map="auto" if self.device == "cuda" else None,
            )
            if self.device == "cpu":
                self._model = self._model.to("cpu")

            self._processor = AutoProcessor.from_pretrained(self.model_id)
            self._loaded = True
            logger.info("VLM loaded successfully.")

        except Exception:
            logger.exception("Failed to load VLM '%s'. Agent will not be functional.", self.model_id)
            self._loaded = False

    @property
    def is_loaded(self) -> bool:
        return self._loaded

    # -- inference ---------------------------------------------------------

    def analyze_frame(
        self,
        image_path: str | Path,
        system_prompt: str,
        task_prompt: str,
        *,
        retries: int = 3,
    ) -> dict[str, Any]:
        """Send an image with prompts to the VLM and return parsed JSON.

        Args:
            image_path: Path to a PNG screenshot.
            system_prompt: System-level instruction.
            task_prompt: Per-frame task instruction.
            retries: Number of attempts to get valid JSON from the model.

        Returns:
            Parsed JSON dict from the model response.

        Raises:
            RuntimeError: If the model is not loaded.
            ValueError: If JSON parsing fails after all retries.
        """
        if not self._loaded:
            raise RuntimeError("VLM is not loaded -- cannot analyse frame.")

        from qwen_vl_utils import process_vision_info  # noqa: PLC0415

        image_path = Path(image_path).resolve()
        if not image_path.exists():
            raise FileNotFoundError(f"Screenshot not found: {image_path}")

        image_uri = f"file://{image_path}"

        messages = [
            {"role": "system", "content": [{"type": "text", "text": system_prompt}]},
            {
                "role": "user",
                "content": [
                    {"type": "image", "image": image_uri},
                    {"type": "text", "text": task_prompt},
                ],
            },
        ]

        last_error: Exception | None = None
        for attempt in range(1, retries + 1):
            try:
                raw_text = self._generate(messages)
                logger.debug("VLM raw output (attempt %d):\n%s", attempt, raw_text[:800])
                return _extract_json(raw_text)
            except (ValueError, json.JSONDecodeError) as exc:
                last_error = exc
                logger.warning(
                    "JSON parse failed on attempt %d/%d: %s",
                    attempt,
                    retries,
                    exc,
                )

        raise ValueError(f"Failed to parse JSON after {retries} attempts: {last_error}")

    def _generate(self, messages: list[dict[str, Any]]) -> str:
        """Run model inference and return the raw decoded string."""
        import torch  # noqa: PLC0415
        from qwen_vl_utils import process_vision_info  # noqa: PLC0415

        text = self._processor.apply_chat_template(
            messages,
            tokenize=False,
            add_generation_prompt=True,
        )
        image_inputs, video_inputs = process_vision_info(messages)
        inputs = self._processor(
            text=[text],
            images=image_inputs,
            videos=video_inputs,
            padding=True,
            return_tensors="pt",
        ).to(self._model.device)

        with torch.inference_mode():
            output_ids = self._model.generate(
                **inputs,
                max_new_tokens=self.max_new_tokens,
                temperature=self.temperature,
                do_sample=self.temperature > 0,
            )

        # Strip the input tokens to get only the generated output.
        generated_ids = output_ids[:, inputs["input_ids"].shape[1] :]
        return self._processor.batch_decode(
            generated_ids,
            skip_special_tokens=True,
            clean_up_tokenization_spaces=False,
        )[0]


# ---------------------------------------------------------------------------
# Dummy agent for pipeline testing (--no-model)
# ---------------------------------------------------------------------------

class DummyVisionAgent:
    """Drop-in replacement that returns a static observation without a GPU."""

    def __init__(self) -> None:
        logger.info("Using DummyVisionAgent (no model loaded).")

    @property
    def is_loaded(self) -> bool:
        return True  # Always "ready"

    def analyze_frame(
        self,
        image_path: str | Path,
        system_prompt: str,
        task_prompt: str,
        *,
        retries: int = 3,
    ) -> dict[str, Any]:
        """Return a canned observation."""
        _ = system_prompt, task_prompt, retries
        return {
            "scene_description": f"Dummy analysis of {Path(image_path).name}.",
            "game_state": "unknown",
            "player_visible": False,
            "player_health_pct": None,
            "enemies_visible": 0,
            "threats": [],
            "ui_elements": [],
            "recommended_action": {"name": "wait", "params": {"seconds": 2.0}},
            "confidence": 0.0,
            "reasoning": "No model loaded -- returning default observation.",
        }
