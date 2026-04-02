"""Higher-level window management helpers.

Wraps the low-level functions in :mod:`screen_capture` into a convenient API
for the game loop.
"""

from __future__ import annotations

import logging
from typing import TypedDict

from game_agent.screen_capture import (
    WindowGeometry,
    find_game_window,
    focus_window,
    get_window_geometry,
)

logger = logging.getLogger(__name__)


class WindowBounds(TypedDict):
    """Dictionary representation of a window's bounding rectangle."""

    x: int
    y: int
    width: int
    height: int
    window_id: int


def ensure_window_focused(window_title: str) -> tuple[int, WindowGeometry] | None:
    """Find the game window by *window_title*, focus it, and return its info.

    Returns:
        A ``(window_id, WindowGeometry)`` tuple, or ``None`` if the window
        could not be found or focused.
    """
    window_id = find_game_window(window_title)
    if window_id is None:
        logger.error("Game window '%s' not found.", window_title)
        return None

    if not focus_window(window_id):
        logger.error("Could not focus window %d.", window_id)
        return None

    geometry = get_window_geometry(window_id)
    if geometry is None:
        logger.error("Could not get geometry for window %d.", window_id)
        return None

    logger.debug(
        "Window '%s' focused: id=%d, geometry=%s",
        window_title,
        window_id,
        geometry,
    )
    return window_id, geometry


def is_window_visible(window_id: int) -> bool:
    """Return ``True`` if *window_id* still exists and has valid geometry."""
    geometry = get_window_geometry(window_id)
    if geometry is None:
        return False
    return geometry.width > 0 and geometry.height > 0


def get_window_bounds(window_id: int) -> WindowBounds | None:
    """Return a :class:`WindowBounds` dict, or ``None`` on failure."""
    geometry = get_window_geometry(window_id)
    if geometry is None:
        return None
    return WindowBounds(
        x=geometry.x,
        y=geometry.y,
        width=geometry.width,
        height=geometry.height,
        window_id=window_id,
    )
