# Path of Taxation -- AI Game Testing Agent

A local-first Linux X11 desktop agent that captures screenshots of a running
Bevy game, analyses them with a local vision-language model
(Qwen2.5-VL-7B-Instruct), and optionally executes keyboard/mouse actions to
test the game autonomously.

## Prerequisites

- Linux with X11 (Wayland is not supported)
- Python 3.10+
- NVIDIA GPU with CUDA (for the VLM; CPU fallback available but very slow)
- The game "Path of Taxation" running in a visible window

## Setup

### System dependencies

Arch / CachyOS:

```bash
sudo pacman -S scrot xdotool python-xlib imagemagick
```

Debian / Ubuntu:

```bash
sudo apt install scrot xdotool python3-xlib imagemagick
```

### Python environment

```bash
cd /home/hex/path-of-taxation/game_agent
python -m venv venv
source venv/bin/activate
pip install -r requirements.txt
```

The first run will download the Qwen2.5-VL-7B-Instruct model (~15 GB).

## Usage

All commands assume you are in the `game_agent/` directory with the venv
activated.

### Observe mode (default) -- watch and log, no actions

```bash
python -m game_agent --mode observe
```

### Auto mode -- fully autonomous play

```bash
python -m game_agent --mode auto --max-loops 100
```

### Assist mode -- suggest actions, wait for confirmation

```bash
python -m game_agent --mode assist
```

### Record mode -- save every frame and analysis for later review

```bash
python -m game_agent --mode record --session playtest_01
```

### Pipeline testing (no GPU required)

```bash
python -m game_agent --mode observe --no-model --dry-run
```

### Additional flags

| Flag            | Description                                    |
|-----------------|------------------------------------------------|
| `--dry-run`     | Log actions but never execute them             |
| `--no-model`    | Use dummy analysis (no GPU needed)             |
| `--max-loops N` | Stop after N iterations                        |
| `--session NAME`| Label for record-mode session directory        |
| `-v / --verbose`| Enable DEBUG-level console logging             |

## Architecture

```
main.py            CLI entry point (argparse)
config.py          All tuneable constants
game_loop.py       Core observe-analyse-act loop
vision_agent.py    Qwen2.5-VL wrapper + DummyVisionAgent
prompts.py         System / task prompt templates
action_schema.py   Pydantic models for every allowed action
input_control.py   pyautogui keyboard/mouse execution + rate limiter
screen_capture.py  xdotool + scrot/import screenshot capture
window_control.py  Higher-level window management
logger.py          Coloured console + file + session logging
```

### Operating modes

| Mode      | Captures | Analyses | Executes | Saves session |
|-----------|----------|----------|----------|---------------|
| observe   | yes      | yes      | no       | no            |
| assist    | yes      | yes      | on confirm | no          |
| auto      | yes      | yes      | yes      | no            |
| record    | yes      | yes      | no       | yes           |

## Safety features

1. **pyautogui.FAILSAFE** -- move the mouse to any screen corner to
   instantly abort all automation.
2. **Rate limiter** -- at most 30 actions per minute (configurable).
3. **Dry-run mode** -- test the full pipeline without touching inputs.
4. **Max-loops cap** -- the agent always stops after a finite number of
   iterations (default 1000).
5. **Action validation** -- every VLM-suggested action is validated through
   Pydantic before execution; invalid actions are logged and skipped.
6. **Coordinate bounds checking** -- mouse coordinates are validated against
   the game window dimensions.

## Output

- `screenshots/` -- captured frames (auto-cleaned to 500 max)
- `logs/agent.log` -- full debug log
- `logs/session_*.tsv` -- per-session structured log
- `sessions/<name>/` -- (record mode) frames + JSON analyses

## License

Internal tool -- not distributed.
