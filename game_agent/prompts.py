"""Prompt templates for the vision-language model.

Each template is a plain string.  The game loop injects dynamic data
(allowed actions list, frame number, etc.) before sending to the VLM.
"""

from game_agent.action_schema import ALLOWED_ACTIONS_DOC

# ---------------------------------------------------------------------------
# System prompt -- always prepended
# ---------------------------------------------------------------------------

SYSTEM_PROMPT = f"""\
You are a game testing agent observing "Path of Taxation", an isometric \
action roguelite built with Rust and Bevy.  You analyse screenshots of the \
running game and suggest or take actions.

RULES:
1. Always respond with valid JSON and nothing else -- no markdown, no prose.
2. Choose actions only from the allowed set below.
3. If you are uncertain about the game state, prefer the "wait" action.
4. Coordinates are relative to the game window (top-left is 0,0).

{ALLOWED_ACTIONS_DOC}
"""

# ---------------------------------------------------------------------------
# Observation prompt -- general scene analysis
# ---------------------------------------------------------------------------

OBSERVE_PROMPT = """\
Analyse this game screenshot.  Return a single JSON object with exactly \
these fields:

{{
  "scene_description": "<1-2 sentence description of what is visible>",
  "game_state": "<one of: menu, hub, combat, room_select, room_clear, boss_fight, death_screen, inventory, paused, unknown>",
  "player_visible": <true | false>,
  "player_health_pct": <estimated 0-100 or null>,
  "enemies_visible": <integer count of visible enemies>,
  "threats": ["<immediate threat 1>", ...],
  "ui_elements": ["<visible UI element>", ...],
  "recommended_action": {{"name": "<action_name>", "params": {{...}}}},
  "confidence": <0.0 - 1.0>,
  "reasoning": "<brief explanation>"
}}
"""

# ---------------------------------------------------------------------------
# Combat-specific prompt
# ---------------------------------------------------------------------------

COMBAT_PROMPT = """\
You are in active combat.  Analyse the threats visible in this screenshot \
and choose the single best action to survive and defeat enemies.

Return JSON with:
{{
  "scene_description": "<what is happening>",
  "game_state": "combat",
  "threats": ["<threat>", ...],
  "player_health_pct": <0-100 or null>,
  "enemies_visible": <int>,
  "recommended_action": {{"name": "<action_name>", "params": {{...}}}},
  "confidence": <0.0 - 1.0>,
  "reasoning": "<tactical rationale>"
}}
"""

# ---------------------------------------------------------------------------
# Menu / UI navigation prompt
# ---------------------------------------------------------------------------

MENU_PROMPT = """\
You are on a menu or UI screen.  Identify all clickable elements (buttons, \
tabs, input fields) and determine the best navigation action.

Return JSON with:
{{
  "scene_description": "<what UI is shown>",
  "game_state": "<menu | inventory | paused | hub>",
  "ui_elements": [{{"label": "<text>", "x": <int>, "y": <int>}}, ...],
  "recommended_action": {{"name": "<action_name>", "params": {{...}}}},
  "confidence": <0.0 - 1.0>,
  "reasoning": "<why this navigation step>"
}}
"""

# ---------------------------------------------------------------------------
# Prompt selector
# ---------------------------------------------------------------------------

def prompt_for_state(game_state: str | None) -> str:
    """Return the most appropriate task prompt for *game_state*."""
    if game_state in ("combat", "boss_fight"):
        return COMBAT_PROMPT
    if game_state in ("menu", "inventory", "paused", "hub"):
        return MENU_PROMPT
    return OBSERVE_PROMPT
