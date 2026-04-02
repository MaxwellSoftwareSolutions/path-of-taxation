"""Structured logging with coloured console output and file persistence.

Usage::

    from game_agent.logger import setup_logging
    setup_logging()

    import logging
    log = logging.getLogger(__name__)
    log.info("ready")
"""

from __future__ import annotations

import logging
import sys
from datetime import datetime, timezone
from pathlib import Path
from typing import ClassVar

from game_agent import config

# ---------------------------------------------------------------------------
# ANSI colour codes
# ---------------------------------------------------------------------------

_RESET = "\033[0m"
_BOLD = "\033[1m"
_COLOURS: dict[int, str] = {
    logging.DEBUG: "\033[36m",      # cyan
    logging.INFO: "\033[32m",       # green
    logging.WARNING: "\033[33m",    # yellow
    logging.ERROR: "\033[31m",      # red
    logging.CRITICAL: "\033[1;31m", # bold red
}


# ---------------------------------------------------------------------------
# Custom formatter
# ---------------------------------------------------------------------------

class _ColouredFormatter(logging.Formatter):
    """Adds ANSI colours based on log level for terminal output."""

    def format(self, record: logging.LogRecord) -> str:
        colour = _COLOURS.get(record.levelno, "")
        msg = super().format(record)
        if colour:
            return f"{colour}{msg}{_RESET}"
        return msg


class _PlainFormatter(logging.Formatter):
    """Straightforward formatter for file output."""


# ---------------------------------------------------------------------------
# Session logger -- writes per-loop structured records
# ---------------------------------------------------------------------------

class SessionLogger:
    """Append-only structured log for a single agent session.

    Each line is a tab-separated record:
        timestamp  loop#  screenshot_path  game_state  action  result
    """

    _HEADER: ClassVar[str] = "timestamp\tloop\tscreenshot\tgame_state\taction\tresult"

    def __init__(self, session_name: str | None = None) -> None:
        ts = datetime.now(tz=timezone.utc).strftime("%Y%m%d_%H%M%S")
        name = session_name or ts
        self.path = config.LOG_DIR / f"session_{name}.tsv"
        self.path.parent.mkdir(parents=True, exist_ok=True)
        with self.path.open("w") as fh:
            fh.write(self._HEADER + "\n")

    def record(
        self,
        loop_number: int,
        screenshot_path: str,
        game_state: str,
        action: str,
        result: str,
    ) -> None:
        ts = datetime.now(tz=timezone.utc).isoformat(timespec="seconds")
        line = f"{ts}\t{loop_number}\t{screenshot_path}\t{game_state}\t{action}\t{result}"
        with self.path.open("a") as fh:
            fh.write(line + "\n")


# ---------------------------------------------------------------------------
# Setup
# ---------------------------------------------------------------------------

def setup_logging(*, verbose: bool = False) -> None:
    """Configure root logger with coloured console and file handlers.

    Call once at startup (from ``main.py``).
    """
    level = logging.DEBUG if verbose else logging.INFO

    root = logging.getLogger()
    root.setLevel(level)

    # Avoid duplicate handlers on repeated calls.
    if root.handlers:
        return

    # -- Console handler ---------------------------------------------------
    console = logging.StreamHandler(sys.stderr)
    console.setLevel(level)
    console.setFormatter(
        _ColouredFormatter(
            fmt="%(asctime)s  %(levelname)-8s  %(name)s  %(message)s",
            datefmt="%H:%M:%S",
        )
    )
    root.addHandler(console)

    # -- File handler (always DEBUG) ---------------------------------------
    log_file = config.LOG_DIR / "agent.log"
    file_handler = logging.FileHandler(log_file, encoding="utf-8")
    file_handler.setLevel(logging.DEBUG)
    file_handler.setFormatter(
        _PlainFormatter(
            fmt="%(asctime)s  %(levelname)-8s  %(name)s  %(message)s",
            datefmt="%Y-%m-%dT%H:%M:%S",
        )
    )
    root.addHandler(file_handler)
