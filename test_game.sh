#!/bin/bash
# Game testing framework - launches the game, takes screenshots, simulates input
# Usage: ./test_game.sh [action]
# Actions: launch, screenshot, click, key, kill, test-combat, test-full

GAME_DIR="/home/hex/path-of-taxation"
SCREENSHOT_DIR="$GAME_DIR/test_screenshots"
GAME_BIN="$GAME_DIR/target/debug/pot-client"
WINDOW_NAME="Path of Taxation"
LOG_FILE="$GAME_DIR/test_screenshots/test_run.log"

mkdir -p "$SCREENSHOT_DIR"

get_window_id() {
    xdotool search --name "$WINDOW_NAME" 2>/dev/null | head -1
}

wait_for_window() {
    for i in $(seq 1 90); do
        WID=$(get_window_id)
        if [ -n "$WID" ]; then
            echo "$WID"
            return 0
        fi
        sleep 0.5
    done
    echo "ERROR: Window not found after 45s" >&2
    return 1
}

launch_game() {
    cd "$GAME_DIR" || exit 1
    cargo build --bin pot-client >/dev/null || return 1
    "$GAME_BIN" >"$LOG_FILE" 2>&1 &
    echo "$!"
}

take_screenshot() {
    local name="${1:-screenshot}"
    local ts=$(date +%Y%m%d_%H%M%S)
    local file="$SCREENSHOT_DIR/${name}_${ts}.png"
    local WID=$(get_window_id)
    if [ -n "$WID" ]; then
        import -window "$WID" "$file" 2>/dev/null
        echo "$file"
    else
        echo "ERROR: No game window found" >&2
        return 1
    fi
}

send_key() {
    local key="$1"
    local WID=$(get_window_id)
    if [ -n "$WID" ]; then
        xdotool key --window "$WID" "$key"
    fi
}

send_keys() {
    local keys="$1"
    local WID=$(get_window_id)
    if [ -n "$WID" ]; then
        xdotool type --window "$WID" "$keys"
    fi
}

click_at() {
    local x="$1" y="$2"
    local WID=$(get_window_id)
    if [ -n "$WID" ]; then
        xdotool mousemove --window "$WID" "$x" "$y"
        sleep 0.05
        xdotool click --window "$WID" 1
    fi
}

hold_key() {
    local key="$1" duration="$2"
    local WID=$(get_window_id)
    if [ -n "$WID" ]; then
        xdotool keydown --window "$WID" "$key"
        sleep "$duration"
        xdotool keyup --window "$WID" "$key"
    fi
}

case "$1" in
    launch)
        # Kill existing instance
        pkill -f pot-client 2>/dev/null
        sleep 0.5
        # Build and launch
        PID=$(launch_game)
        echo "PID: $PID"
        WID=$(wait_for_window)
        echo "Window ID: $WID"
        sleep 2
        take_screenshot "launch"
        ;;

    screenshot)
        take_screenshot "${2:-manual}"
        ;;

    key)
        send_key "$2"
        ;;

    click)
        click_at "$2" "$3"
        ;;

    kill)
        pkill -f pot-client 2>/dev/null
        echo "Killed"
        ;;

    test-combat)
        # Launch game, wait, take combat screenshots
        echo "=== Testing combat ==="
        pkill -f pot-client 2>/dev/null
        sleep 0.5
        launch_game >/dev/null
        WID=$(wait_for_window)
        sleep 3
        take_screenshot "01_initial_spawn"

        # Move around with WASD
        hold_key "w" 0.5
        hold_key "d" 0.5
        take_screenshot "02_after_move"

        # Attack (click toward enemies)
        click_at 1100 500
        sleep 0.3
        click_at 1100 500
        sleep 0.3
        take_screenshot "03_after_attack"

        # Dodge
        send_key "Shift_L"
        sleep 0.3
        take_screenshot "04_after_dodge"

        # Use different abilities
        send_key "2"
        click_at 900 400
        sleep 0.5
        take_screenshot "05_ability_2"

        send_key "3"
        click_at 960 540
        sleep 0.5
        take_screenshot "06_ability_3"

        # Let combat play for a few seconds
        sleep 3
        take_screenshot "07_mid_combat"

        # Toggle debug overlay
        send_key "F1"
        sleep 0.3
        take_screenshot "08_debug_overlay"
        send_key "F1"

        # Toggle FPS overlay
        send_key "F2"
        sleep 0.3
        take_screenshot "09_fps_overlay"

        echo "=== Combat test complete ==="
        ls -la "$SCREENSHOT_DIR"/
        ;;

    test-full)
        # Full game loop test
        echo "=== Full loop test ==="
        pkill -f pot-client 2>/dev/null
        sleep 0.5
        launch_game >/dev/null
        WID=$(wait_for_window)
        sleep 3
        take_screenshot "full_01_spawn"

        # Fight enemies
        for i in $(seq 1 10); do
            click_at $((800 + RANDOM % 300)) $((400 + RANDOM % 300))
            sleep 0.2
        done
        take_screenshot "full_02_fighting"

        # Wait for combat to resolve
        sleep 5
        take_screenshot "full_03_after_combat"

        # Try pause
        send_key "Escape"
        sleep 0.5
        take_screenshot "full_04_paused"
        send_key "Escape"
        sleep 0.3

        # Try inventory
        send_key "Tab"
        sleep 0.5
        take_screenshot "full_05_inventory"
        send_key "Tab"

        sleep 5
        take_screenshot "full_06_later"

        echo "=== Full loop test complete ==="
        ls -la "$SCREENSHOT_DIR"/
        ;;

    *)
        echo "Usage: $0 {launch|screenshot [name]|key <key>|click <x> <y>|kill|test-combat|test-full}"
        ;;
esac
