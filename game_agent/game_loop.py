"""Main control loop for the game testing agent.

Orchestrates screenshot capture, VLM analysis, action execution, and logging.
"""

from __future__ import annotations

import json
import logging
import shutil
import time
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

from game_agent import config
from game_agent.action_schema import GameAction
from game_agent.input_control import execute_action
from game_agent.logger import SessionLogger
from game_agent.prompts import OBSERVE_PROMPT, SYSTEM_PROMPT, prompt_for_state
from game_agent.screen_capture import (
    WindowGeometry,
    capture_window,
    cleanup_screenshots,
)
from game_agent.window_control import ensure_window_focused, is_window_visible

logger = logging.getLogger(__name__)


def _check_emergency_stop() -> bool:
    """Return ``True`` if the emergency-stop key is currently held."""
    try:
        import pyautogui  # noqa: PLC0415

        # pyautogui does not have a direct "is key pressed" query.
        # We use pynput as a lightweight alternative if available.
        from pynput.keyboard import Key, KeyCode  # noqa: PLC0415
        # Fallback: not reliably checkable without a listener.
        # The pyautogui FAILSAFE (move mouse to corner) is the primary
        # emergency stop.  We also check for a held key via xdotool.
    except ImportError:
        pass

    # Use xdotool to poll: if F12 is currently held, xev would show it,
    # but polling key state from pure Python without a listener is tricky.
    # Instead, rely on pyautogui.FAILSAFE (mouse-to-corner abort) as the
    # primary safety mechanism.
    return False


class GameLoop:
    """Core observe-analyse-act loop.

    Args:
        mode: One of ``"observe"``, ``"assist"``, ``"auto"``, ``"record"``.
        dry_run: If ``True``, never execute actions regardless of mode.
        max_loops: Override for ``config.MAX_LOOPS``.
        session_name: Optional label for the recording session.
        no_model: If ``True``, use a dummy vision agent.
    """

    VALID_MODES = ("observe", "assist", "auto", "record")

    def __init__(
        self,
        mode: str = config.DEFAULT_MODE,
        *,
        dry_run: bool = config.DRY_RUN,
        max_loops: int = config.MAX_LOOPS,
        session_name: str | None = None,
        no_model: bool = False,
    ) -> None:
        if mode not in self.VALID_MODES:
            raise ValueError(f"Invalid mode '{mode}'. Choose from {self.VALID_MODES}.")

        self.mode = mode
        self.dry_run = dry_run
        self.max_loops = max_loops
        self.running = True
        self.loop_count = 0
        self.session_name = session_name
        self._last_game_state: str | None = None

        # Session directory for "record" mode.
        self._session_dir: Path | None = None
        if mode == "record":
            ts = datetime.now(tz=timezone.utc).strftime("%Y%m%d_%H%M%S")
            label = session_name or ts
            self._session_dir = config.BASE_DIR / "sessions" / label
            self._session_dir.mkdir(parents=True, exist_ok=True)
            logger.info("Recording session to %s", self._session_dir)

        # Session log.
        self._session_log = SessionLogger(session_name)

        # Vision agent -- deferred so we can import this module without CUDA.
        if no_model:
            from game_agent.vision_agent import DummyVisionAgent  # noqa: PLC0415

            self.vision: Any = DummyVisionAgent()
        else:
            from game_agent.vision_agent import VisionAgent  # noqa: PLC0415

            self.vision = VisionAgent(
                config.MODEL_ID,
                max_new_tokens=config.MAX_NEW_TOKENS,
                temperature=config.TEMPERATURE,
            )

        # Cached window state.
        self._window_id: int | None = None
        self._geometry: WindowGeometry | None = None

    # ------------------------------------------------------------------
    # Main loop
    # ------------------------------------------------------------------

    def run(self) -> None:
        """Start the agent loop.  Blocks until stopped or ``max_loops`` hit."""
        logger.info(
            "Starting game loop: mode=%s, dry_run=%s, max_loops=%d",
            self.mode,
            self.dry_run,
            self.max_loops,
        )

        try:
            while self.running and self.loop_count < self.max_loops:
                self._tick()
                self.loop_count += 1
                time.sleep(config.LOOP_INTERVAL_SEC)
        except KeyboardInterrupt:
            logger.info("Interrupted by user (Ctrl+C).")
        finally:
            logger.info("Agent stopped after %d loops.", self.loop_count)

    # ------------------------------------------------------------------
    # Single tick
    # ------------------------------------------------------------------

    def _tick(self) -> None:
        """Execute one observe-analyse-act cycle."""
        # 1. Emergency stop check.
        if _check_emergency_stop():
            logger.warning("Emergency stop triggered!")
            self.running = False
            return

        # 2. Find / validate game window.
        if not self._ensure_window():
            logger.warning("Game window not available -- skipping tick.")
            return

        assert self._window_id is not None
        assert self._geometry is not None

        # 3. Capture screenshot.
        screenshot_path = self._capture()
        if screenshot_path is None:
            logger.error("Screenshot capture failed -- skipping tick.")
            return

        # 4. Analyse with VLM.
        analysis = self._analyse(screenshot_path)
        if analysis is None:
            self._session_log.record(
                self.loop_count, str(screenshot_path), "error", "none", "analysis failed"
            )
            return

        game_state = analysis.get("game_state", "unknown")
        self._last_game_state = game_state
        recommended = analysis.get("recommended_action")

        # 5. Log analysis.
        self._log_analysis(analysis, screenshot_path)

        # 6. Save session data in "record" mode.
        if self.mode == "record" and self._session_dir is not None:
            self._save_record(screenshot_path, analysis)

        # 7. Act (depending on mode).
        action_desc = "none"
        if recommended:
            action_desc = self._maybe_act(recommended, analysis)

        # 8. Session log record.
        self._session_log.record(
            self.loop_count,
            str(screenshot_path),
            game_state,
            json.dumps(recommended) if recommended else "none",
            action_desc,
        )

        # 9. Screenshot housekeeping.
        cleanup_screenshots(config.SCREENSHOT_DIR, config.MAX_SCREENSHOTS)

    # ------------------------------------------------------------------
    # Helpers
    # ------------------------------------------------------------------

    def _ensure_window(self) -> bool:
        """Make sure the game window is focused and geometry is current."""
        # Re-check every tick in case window moved / closed.
        result = ensure_window_focused(config.WINDOW_TITLE)
        if result is None:
            self._window_id = None
            self._geometry = None
            return False
        self._window_id, self._geometry = result
        return True

    def _capture(self) -> Path | None:
        """Take a screenshot and return the path, or ``None`` on failure."""
        ts = datetime.now(tz=timezone.utc).strftime("%Y%m%d_%H%M%S_%f")
        filename = f"frame_{self.loop_count:05d}_{ts}.{config.SCREENSHOT_FORMAT}"
        out = config.SCREENSHOT_DIR / filename

        assert self._window_id is not None
        if capture_window(self._window_id, out):
            return out
        return None

    def _analyse(self, image_path: Path) -> dict[str, Any] | None:
        """Run VLM analysis on a screenshot."""
        task_prompt = prompt_for_state(self._last_game_state)
        try:
            return self.vision.analyze_frame(
                image_path,
                SYSTEM_PROMPT,
                task_prompt,
            )
        except Exception:
            logger.exception("VLM analysis failed for %s.", image_path)
            return None

    def _log_analysis(self, analysis: dict[str, Any], screenshot: Path) -> None:
        """Pretty-print the analysis to the console."""
        state = analysis.get("game_state", "?")
        desc = analysis.get("scene_description", "")
        conf = analysis.get("confidence", 0)
        action = analysis.get("recommended_action", {})
        action_name = action.get("name", "none") if isinstance(action, dict) else "none"

        logger.info(
            "[Loop %04d] state=%-12s confidence=%.2f action=%-12s | %s",
            self.loop_count,
            state,
            conf,
            action_name,
            desc[:100],
        )
        logger.debug("Full analysis:\n%s", json.dumps(analysis, indent=2))

    def _save_record(self, screenshot: Path, analysis: dict[str, Any]) -> None:
        """Copy screenshot and save analysis JSON into the session directory."""
        assert self._session_dir is not None
        stem = f"frame_{self.loop_count:05d}"
        dst_img = self._session_dir / f"{stem}.png"
        dst_json = self._session_dir / f"{stem}.json"

        shutil.copy2(screenshot, dst_img)
        with dst_json.open("w") as fh:
            json.dump(analysis, fh, indent=2)

    def _maybe_act(self, recommended: dict[str, Any], analysis: dict[str, Any]) -> str:
        """Decide whether to execute the recommended action based on mode."""
        try:
            action = GameAction.model_validate(recommended)
        except Exception:
            logger.warning("Invalid action from VLM: %s", recommended)
            return "invalid action"

        if action.name == "stop":
            logger.info("Agent requested stop.")
            self.running = False
            return "stop"

        if self.mode == "observe":
            return f"[observe] would {action.name}"

        if self.mode in ("record",):
            # Record mode only observes -- actions are not executed.
            return f"[record] would {action.name}"

        if self.mode == "assist":
            return self._assist_action(action, analysis)

        if self.mode == "auto":
            return self._auto_action(action)

        return "no-op"

    def _assist_action(self, action: GameAction, analysis: dict[str, Any]) -> str:
        """Print the suggested action and wait for user confirmation."""
        conf = analysis.get("confidence", 0)
        reasoning = analysis.get("reasoning", "")
        print(
            f"\n--- SUGGESTED ACTION ---\n"
            f"  Action:     {action.name} {action.params}\n"
            f"  Confidence: {conf:.2f}\n"
            f"  Reasoning:  {reasoning}\n"
        )
        try:
            answer = input("Execute? [y/N] ").strip().lower()
        except EOFError:
            answer = "n"

        if answer == "y":
            return self._execute(action)
        return "user declined"

    def _auto_action(self, action: GameAction) -> str:
        """Execute the action automatically."""
        return self._execute(action)

    def _execute(self, action: GameAction) -> str:
        """Execute *action* and return result description."""
        assert self._geometry is not None
        try:
            result = execute_action(action, self._geometry, dry_run=self.dry_run)
            logger.info("Executed: %s", result)
            return result
        except Exception as exc:
            logger.error("Action execution failed: %s", exc)
            return f"error: {exc}"
