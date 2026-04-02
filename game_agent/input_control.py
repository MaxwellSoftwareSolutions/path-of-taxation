"""Keyboard and mouse input execution.

All ``pyautogui`` usage is contained in this module so that other modules can
be imported on headless systems without triggering X11 errors.
"""

from __future__ import annotations

import logging
import time
from collections import deque
from typing import TYPE_CHECKING

from game_agent import config
from game_agent.action_schema import (
    GameAction,
    HoldKeyParams,
    HotkeyParams,
    LeftClickParams,
    MouseMoveParams,
    ReleaseKeyParams,
    RightClickParams,
    TapKeyParams,
    TypeTextParams,
    WaitParams,
)

if TYPE_CHECKING:
    from game_agent.screen_capture import WindowGeometry

logger = logging.getLogger(__name__)

# ---------------------------------------------------------------------------
# Rate limiter
# ---------------------------------------------------------------------------

_action_timestamps: deque[float] = deque(maxlen=config.MAX_ACTIONS_PER_MINUTE)


def _rate_limit_ok() -> bool:
    """Return ``True`` if we are below the per-minute action cap."""
    now = time.monotonic()
    # Evict timestamps older than 60 seconds.
    while _action_timestamps and (now - _action_timestamps[0]) > 60.0:
        _action_timestamps.popleft()
    return len(_action_timestamps) < config.MAX_ACTIONS_PER_MINUTE


def _record_action() -> None:
    _action_timestamps.append(time.monotonic())


# ---------------------------------------------------------------------------
# Coordinate translation
# ---------------------------------------------------------------------------

def _game_to_screen(
    gx: int,
    gy: int,
    geometry: "WindowGeometry",
) -> tuple[int, int]:
    """Convert game-window-relative coordinates to screen-absolute."""
    return geometry.x + gx, geometry.y + gy


# ---------------------------------------------------------------------------
# Lazy pyautogui import
# ---------------------------------------------------------------------------

def _pag():  # noqa: ANN202
    """Import and configure ``pyautogui`` on first use."""
    import pyautogui  # noqa: PLC0415

    pyautogui.FAILSAFE = True
    pyautogui.PAUSE = config.ACTION_COOLDOWN_SEC
    return pyautogui


# ---------------------------------------------------------------------------
# Action executors
# ---------------------------------------------------------------------------

def _exec_wait(params: WaitParams) -> str:
    time.sleep(params.seconds)
    return f"waited {params.seconds:.1f}s"


def _exec_tap_key(params: TapKeyParams) -> str:
    pag = _pag()
    for _ in range(params.count):
        pag.press(params.key)
    return f"tapped '{params.key}' x{params.count}"


def _exec_hold_key(params: HoldKeyParams) -> str:
    pag = _pag()
    pag.keyDown(params.key)
    time.sleep(params.duration_sec)
    pag.keyUp(params.key)
    return f"held '{params.key}' for {params.duration_sec:.2f}s"


def _exec_release_key(params: ReleaseKeyParams) -> str:
    pag = _pag()
    pag.keyUp(params.key)
    return f"released '{params.key}'"


def _exec_mouse_move(params: MouseMoveParams, geometry: "WindowGeometry") -> str:
    pag = _pag()
    sx, sy = _game_to_screen(params.x, params.y, geometry)
    pag.moveTo(sx, sy)
    return f"moved mouse to game({params.x},{params.y}) -> screen({sx},{sy})"


def _exec_left_click(params: LeftClickParams, geometry: "WindowGeometry") -> str:
    pag = _pag()
    sx, sy = _game_to_screen(params.x, params.y, geometry)
    pag.click(sx, sy, button="left")
    return f"left-clicked game({params.x},{params.y}) -> screen({sx},{sy})"


def _exec_right_click(params: RightClickParams, geometry: "WindowGeometry") -> str:
    pag = _pag()
    sx, sy = _game_to_screen(params.x, params.y, geometry)
    pag.click(sx, sy, button="right")
    return f"right-clicked game({params.x},{params.y}) -> screen({sx},{sy})"


def _exec_hotkey(params: HotkeyParams) -> str:
    pag = _pag()
    pag.hotkey(*params.keys)
    return f"hotkey {'+'.join(params.keys)}"


def _exec_type_text(params: TypeTextParams) -> str:
    pag = _pag()
    pag.typewrite(params.text, interval=0.03)
    return f"typed '{params.text[:40]}...'" if len(params.text) > 40 else f"typed '{params.text}'"


# ---------------------------------------------------------------------------
# Public API
# ---------------------------------------------------------------------------

def execute_action(
    action: GameAction,
    window_geometry: "WindowGeometry",
    *,
    dry_run: bool = False,
) -> str:
    """Execute a validated ``GameAction``.

    Args:
        action: The action to execute.
        window_geometry: Current game window geometry for coordinate mapping.
        dry_run: If ``True``, log the action but do not perform it.

    Returns:
        A human-readable description of what was (or would be) done.

    Raises:
        RuntimeError: If the rate limit is exceeded.
    """
    if action.name == "stop":
        return "stop requested"

    if action.name == "wait":
        params = WaitParams.model_validate(action.params)
        desc = f"[DRY] would wait {params.seconds:.1f}s" if dry_run else _exec_wait(params)
        return desc

    # Everything else counts towards rate limit.
    if not _rate_limit_ok():
        msg = (
            f"Rate limit exceeded ({config.MAX_ACTIONS_PER_MINUTE} actions/min). "
            f"Refusing action '{action.name}'."
        )
        logger.warning(msg)
        raise RuntimeError(msg)

    if dry_run:
        return f"[DRY] would execute {action.name} with {action.params}"

    _record_action()
    params_obj = action.typed_params

    match action.name:
        case "tap_key":
            assert isinstance(params_obj, TapKeyParams)
            return _exec_tap_key(params_obj)
        case "hold_key":
            assert isinstance(params_obj, HoldKeyParams)
            return _exec_hold_key(params_obj)
        case "release_key":
            assert isinstance(params_obj, ReleaseKeyParams)
            return _exec_release_key(params_obj)
        case "mouse_move":
            assert isinstance(params_obj, MouseMoveParams)
            return _exec_mouse_move(params_obj, window_geometry)
        case "left_click":
            assert isinstance(params_obj, LeftClickParams)
            return _exec_left_click(params_obj, window_geometry)
        case "right_click":
            assert isinstance(params_obj, RightClickParams)
            return _exec_right_click(params_obj, window_geometry)
        case "hotkey":
            assert isinstance(params_obj, HotkeyParams)
            return _exec_hotkey(params_obj)
        case "type_text":
            assert isinstance(params_obj, TypeTextParams)
            return _exec_type_text(params_obj)
        case _:
            return f"unknown action '{action.name}'"
