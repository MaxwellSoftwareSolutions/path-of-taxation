"""Strict action schema validated with Pydantic.

Every action the agent can take is modelled here.  The VLM returns a JSON blob
that is parsed into one of these models before execution.
"""

from __future__ import annotations

from typing import Any, Literal

from pydantic import BaseModel, Field, model_validator

from game_agent import config

# ---------------------------------------------------------------------------
# Coordinate bounds helpers
# ---------------------------------------------------------------------------
_MAX_X = config.GAME_WINDOW_WIDTH
_MAX_Y = config.GAME_WINDOW_HEIGHT


def _clamp(value: int, lo: int, hi: int) -> int:
    return max(lo, min(value, hi))


# ---------------------------------------------------------------------------
# Per-action parameter models
# ---------------------------------------------------------------------------

class WaitParams(BaseModel):
    """Parameters for the ``wait`` action."""

    seconds: float = Field(default=1.0, ge=0.0, le=30.0)


class TapKeyParams(BaseModel):
    """Parameters for the ``tap_key`` action."""

    key: str = Field(..., min_length=1, max_length=32)
    count: int = Field(default=1, ge=1, le=20)


class HoldKeyParams(BaseModel):
    """Parameters for the ``hold_key`` action."""

    key: str = Field(..., min_length=1, max_length=32)
    duration_sec: float = Field(default=0.5, ge=0.0, le=10.0)


class ReleaseKeyParams(BaseModel):
    """Parameters for the ``release_key`` action."""

    key: str = Field(..., min_length=1, max_length=32)


class MouseMoveParams(BaseModel):
    """Parameters for the ``mouse_move`` action."""

    x: int = Field(..., ge=0, le=_MAX_X)
    y: int = Field(..., ge=0, le=_MAX_Y)


class LeftClickParams(BaseModel):
    """Parameters for the ``left_click`` action."""

    x: int = Field(..., ge=0, le=_MAX_X)
    y: int = Field(..., ge=0, le=_MAX_Y)


class RightClickParams(BaseModel):
    """Parameters for the ``right_click`` action."""

    x: int = Field(..., ge=0, le=_MAX_X)
    y: int = Field(..., ge=0, le=_MAX_Y)


class HotkeyParams(BaseModel):
    """Parameters for the ``hotkey`` action (key combination)."""

    keys: list[str] = Field(..., min_length=1, max_length=6)


class TypeTextParams(BaseModel):
    """Parameters for the ``type_text`` action."""

    text: str = Field(..., min_length=1, max_length=256)


class StopParams(BaseModel):
    """Parameters for the ``stop`` action (none required)."""


# ---------------------------------------------------------------------------
# Mapping from action name -> parameter model
# ---------------------------------------------------------------------------

ActionName = Literal[
    "wait",
    "tap_key",
    "hold_key",
    "release_key",
    "mouse_move",
    "left_click",
    "right_click",
    "hotkey",
    "type_text",
    "stop",
]

_PARAMS_MODEL_MAP: dict[ActionName, type[BaseModel]] = {
    "wait": WaitParams,
    "tap_key": TapKeyParams,
    "hold_key": HoldKeyParams,
    "release_key": ReleaseKeyParams,
    "mouse_move": MouseMoveParams,
    "left_click": LeftClickParams,
    "right_click": RightClickParams,
    "hotkey": HotkeyParams,
    "type_text": TypeTextParams,
    "stop": StopParams,
}


# ---------------------------------------------------------------------------
# Top-level GameAction model
# ---------------------------------------------------------------------------

class GameAction(BaseModel):
    """A single validated game action returned by the VLM."""

    name: ActionName
    params: dict[str, Any] = Field(default_factory=dict)

    # After basic parsing, validate params against the specific model.
    @model_validator(mode="after")
    def validate_params(self) -> "GameAction":
        model_cls = _PARAMS_MODEL_MAP[self.name]
        # This will raise ValidationError if params are invalid.
        model_cls.model_validate(self.params)
        return self

    @property
    def typed_params(self) -> BaseModel:
        """Return the params as the correctly typed Pydantic model."""
        model_cls = _PARAMS_MODEL_MAP[self.name]
        return model_cls.model_validate(self.params)


# ---------------------------------------------------------------------------
# Convenience: list of allowed action names for prompt generation
# ---------------------------------------------------------------------------

ALLOWED_ACTIONS: list[str] = list(_PARAMS_MODEL_MAP.keys())

ALLOWED_ACTIONS_DOC = """
Allowed actions:
  wait          - params: {seconds: float}
  tap_key       - params: {key: str, count?: int}
  hold_key      - params: {key: str, duration_sec?: float}
  release_key   - params: {key: str}
  mouse_move    - params: {x: int, y: int}
  left_click    - params: {x: int, y: int}
  right_click   - params: {x: int, y: int}
  hotkey        - params: {keys: [str, ...]}
  type_text     - params: {text: str}
  stop          - no params
""".strip()
