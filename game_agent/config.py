"""Configuration constants for the game testing agent."""

from pathlib import Path

# ---------------------------------------------------------------------------
# Paths
# ---------------------------------------------------------------------------
BASE_DIR = Path(__file__).resolve().parent
SCREENSHOT_DIR = BASE_DIR / "screenshots"
LOG_DIR = BASE_DIR / "logs"

# Ensure directories exist at import time.
SCREENSHOT_DIR.mkdir(exist_ok=True)
LOG_DIR.mkdir(exist_ok=True)

# ---------------------------------------------------------------------------
# Window targeting
# ---------------------------------------------------------------------------
WINDOW_TITLE = "Path of Taxation"

# ---------------------------------------------------------------------------
# Screenshot settings
# ---------------------------------------------------------------------------
SCREENSHOT_FORMAT = "png"
MAX_SCREENSHOTS = 500  # auto-cleanup oldest files when exceeded

# ---------------------------------------------------------------------------
# Model settings
# ---------------------------------------------------------------------------
MODEL_ID = "Qwen/Qwen2.5-VL-7B-Instruct"
MAX_NEW_TOKENS = 512
TEMPERATURE = 0.3

# ---------------------------------------------------------------------------
# Loop settings
# ---------------------------------------------------------------------------
LOOP_INTERVAL_SEC = 2.0  # seconds between observations
MAX_LOOPS = 1000
ACTION_COOLDOWN_SEC = 0.3  # delay after every input action

# ---------------------------------------------------------------------------
# Safety
# ---------------------------------------------------------------------------
EMERGENCY_STOP_KEY = "f12"  # hold to abort the agent loop
MAX_ACTIONS_PER_MINUTE = 30
DRY_RUN = False  # if True, log actions but never execute them

# ---------------------------------------------------------------------------
# Modes: "observe", "assist", "auto", "record"
# ---------------------------------------------------------------------------
DEFAULT_MODE = "observe"

# ---------------------------------------------------------------------------
# Game-specific
# ---------------------------------------------------------------------------
GAME_WINDOW_WIDTH = 1920
GAME_WINDOW_HEIGHT = 1080
