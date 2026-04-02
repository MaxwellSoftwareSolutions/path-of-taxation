"""X11 screen-capture utilities.

Uses ``xdotool`` and ``scrot`` via subprocess -- no Python X11 bindings
required at import time.
"""

from __future__ import annotations

import logging
import shutil
import subprocess
from pathlib import Path
from typing import NamedTuple

logger = logging.getLogger(__name__)


class WindowGeometry(NamedTuple):
    """Absolute position and size of an X11 window."""

    x: int
    y: int
    width: int
    height: int


# ---------------------------------------------------------------------------
# Internal helpers
# ---------------------------------------------------------------------------

def _run(cmd: list[str], *, timeout: float = 5.0) -> subprocess.CompletedProcess[str]:
    """Run a command and return CompletedProcess.  Never raises on failure."""
    try:
        return subprocess.run(
            cmd,
            capture_output=True,
            text=True,
            timeout=timeout,
        )
    except FileNotFoundError:
        logger.error("Command not found: %s", cmd[0])
        return subprocess.CompletedProcess(cmd, returncode=127, stdout="", stderr=f"{cmd[0]}: not found")
    except subprocess.TimeoutExpired:
        logger.error("Command timed out: %s", " ".join(cmd))
        return subprocess.CompletedProcess(cmd, returncode=124, stdout="", stderr="timeout")


def _require_tool(name: str) -> bool:
    """Return True if *name* is on PATH."""
    if shutil.which(name) is None:
        logger.error("Required tool '%s' is not installed.  Install it with your package manager.", name)
        return False
    return True


# ---------------------------------------------------------------------------
# Public API
# ---------------------------------------------------------------------------

def find_game_window(title: str) -> int | None:
    """Search for an X11 window whose name contains *title*.

    Returns the window ID (int) or ``None`` if not found.
    """
    if not _require_tool("xdotool"):
        return None

    result = _run(["xdotool", "search", "--name", title])
    if result.returncode != 0 or not result.stdout.strip():
        logger.warning("Window '%s' not found.", title)
        return None

    # xdotool may return multiple IDs (one per line).  Use the first.
    first_line = result.stdout.strip().splitlines()[0]
    try:
        window_id = int(first_line)
    except ValueError:
        logger.error("Could not parse window ID from xdotool output: %r", first_line)
        return None

    logger.debug("Found window '%s' with id %d.", title, window_id)
    return window_id


def focus_window(window_id: int) -> bool:
    """Activate (focus + raise) the window identified by *window_id*.

    Returns ``True`` on success.
    """
    if not _require_tool("xdotool"):
        return False

    result = _run(["xdotool", "windowactivate", "--sync", str(window_id)])
    if result.returncode != 0:
        logger.error("Failed to activate window %d: %s", window_id, result.stderr.strip())
        return False
    return True


def get_window_geometry(window_id: int) -> WindowGeometry | None:
    """Return the geometry of *window_id*, or ``None`` on failure."""
    if not _require_tool("xdotool"):
        return None

    # Position
    result_pos = _run(["xdotool", "getwindowgeometry", "--shell", str(window_id)])
    if result_pos.returncode != 0:
        logger.error("Failed to get geometry for window %d.", window_id)
        return None

    values: dict[str, int] = {}
    for line in result_pos.stdout.strip().splitlines():
        if "=" in line:
            k, v = line.split("=", 1)
            try:
                values[k.strip()] = int(v.strip())
            except ValueError:
                pass

    try:
        return WindowGeometry(
            x=values["X"],
            y=values["Y"],
            width=values["WIDTH"],
            height=values["HEIGHT"],
        )
    except KeyError as exc:
        logger.error("Missing geometry field %s in xdotool output.", exc)
        return None


def capture_window(window_id: int, output_path: str | Path) -> bool:
    """Capture a screenshot of the focused window and save to *output_path*.

    Uses ``import`` from ImageMagick to grab a specific window by ID, which is
    more reliable than ``scrot --focused`` (scrot can miss the target if focus
    races).  Falls back to ``scrot`` if ``import`` is unavailable.

    Returns ``True`` on success.
    """
    output_path = Path(output_path)
    output_path.parent.mkdir(parents=True, exist_ok=True)

    # Prefer ImageMagick's import for reliability (grabs by window id).
    if shutil.which("import"):
        result = _run([
            "import",
            "-window", str(window_id),
            str(output_path),
        ], timeout=10.0)
        if result.returncode == 0 and output_path.exists():
            logger.debug("Captured window %d -> %s (import).", window_id, output_path)
            return True
        logger.warning("import failed (%s), falling back to scrot.", result.stderr.strip())

    # Fallback: focus then scrot --focused.
    if not _require_tool("scrot"):
        return False

    focus_window(window_id)
    result = _run(["scrot", "--focused", str(output_path)], timeout=10.0)
    if result.returncode != 0:
        logger.error("scrot capture failed: %s", result.stderr.strip())
        return False

    if not output_path.exists():
        logger.error("scrot reported success but file missing: %s", output_path)
        return False

    logger.debug("Captured window %d -> %s (scrot).", window_id, output_path)
    return True


def capture_region(x: int, y: int, w: int, h: int, output_path: str | Path) -> bool:
    """Capture a rectangular region of the screen.

    Uses ``scrot --select`` with a pre-set geometry string so no interactive
    selection is needed.  Falls back to ImageMagick ``import -crop``.

    Returns ``True`` on success.
    """
    output_path = Path(output_path)
    output_path.parent.mkdir(parents=True, exist_ok=True)

    # Prefer ImageMagick import -crop
    if shutil.which("import"):
        geometry = f"{w}x{h}+{x}+{y}"
        result = _run([
            "import",
            "-window", "root",
            "-crop", geometry,
            str(output_path),
        ], timeout=10.0)
        if result.returncode == 0 and output_path.exists():
            logger.debug("Captured region %s -> %s.", geometry, output_path)
            return True

    # Fallback: scrot -a
    if not _require_tool("scrot"):
        return False

    area = f"{x},{y},{w},{h}"
    result = _run(["scrot", "-a", area, str(output_path)], timeout=10.0)
    if result.returncode != 0:
        logger.error("Region capture failed: %s", result.stderr.strip())
        return False

    if not output_path.exists():
        logger.error("Region capture reported success but file missing: %s", output_path)
        return False

    logger.debug("Captured region (%d,%d %dx%d) -> %s.", x, y, w, h, output_path)
    return True


def cleanup_screenshots(directory: str | Path, max_files: int) -> int:
    """Remove oldest screenshots if the directory exceeds *max_files*.

    Returns the number of files removed.
    """
    directory = Path(directory)
    if not directory.is_dir():
        return 0

    png_files = sorted(directory.glob("*.png"), key=lambda p: p.stat().st_mtime)
    to_remove = len(png_files) - max_files
    removed = 0
    if to_remove > 0:
        for path in png_files[:to_remove]:
            try:
                path.unlink()
                removed += 1
            except OSError as exc:
                logger.warning("Could not remove %s: %s", path, exc)
    return removed
