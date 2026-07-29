#!/usr/bin/env bash
set -euo pipefail

RED='\033[0;31m'; GREEN='\033[0;32m'; BLUE='\033[0;34m'; YELLOW='\033[1;33m'; CYAN='\033[0;36m'; BOLD='\033[1m'; DIM='\033[2m'; NC='\033[0m'

info()  { echo -e "  ${BLUE}→${NC} $*"; }
ok()    { echo -e "  ${GREEN}✔${NC} $*"; }
warn()  { echo -e "  ${YELLOW}⚠${NC} $*"; }
err()   { echo -e "  ${RED}✘${NC} $*"; }
step()  { echo -e "\n  ${CYAN}${BOLD}$*${NC}"; }
skip()  { echo -e "  ${DIM}·${NC} ${DIM}$*${NC}"; }

ROOT_DIR="$(cd "$(dirname "$0")" && pwd)"
REMOVED=()
SKIPPED=()

banner() {
    clear 2>/dev/null || true
    echo -e "  ${CYAN}╭──────────────────────────────────────────╮${NC}"
    echo -e "  ${CYAN}│${NC}    ${BOLD}${BLUE}SyncNotes${NC} ${DIM}— uninstaller${NC}               ${CYAN}│${NC}"
    echo -e "  ${CYAN}╰──────────────────────────────────────────╯${NC}"
}

select_option() {
    local prompt="$1" opt1="$2" opt2="$3"
    local selected=0
    echo ""
    echo -e "  ${BLUE}${prompt}${NC}"
    while true; do
        if [ $selected -eq 0 ]; then
            echo -e "    ${GREEN}▸${NC} $opt1"
            echo -e "      $opt2"
        else
            echo -e "      $opt1"
            echo -e "    ${GREEN}▸${NC} $opt2"
        fi
        IFS= read -rsn 1 key
        if [[ -z $key ]]; then
            echo ""; return $selected
        elif [[ $key == $'\033' ]]; then
            read -rsn 2 -t 0.005 seq || true
            case "${seq:-}" in
                '[A') selected=0 ;;
                '[B') selected=1 ;;
            esac
        elif [[ $key == '1' ]]; then
            echo ""; return 0
        elif [[ $key == '2' ]]; then
            echo ""; return 1
        fi
        echo -en "\033[2A\033[J"
    done
}

confirm() {
    local msg="$1"
    echo ""
    warn "$msg"
    read -rp "  Remove? [y/N]: " yn
    case "$yn" in y|Y|yes|Yes) return 0 ;; *) return 1 ;; esac
}

remove_if_confirmed() {
    # remove_if_confirmed "label" "prompt" cmd args...
    local label="$1" prompt="$2"; shift 2
    if confirm "$prompt"; then
        "$@"
        ok "Removed $label"
        REMOVED+=("$label")
    else
        skip "Kept $label"
        SKIPPED+=("$label")
    fi
}

summary() {
    echo ""
    step "Summary"
    if [ ${#REMOVED[@]} -eq 0 ] && [ ${#SKIPPED[@]} -eq 0 ]; then
        info "Nothing found to remove."
    else
        for item in "${REMOVED[@]:-}"; do [ -n "$item" ] && echo -e "  ${GREEN}✔${NC} removed ${DIM}$item${NC}"; done
        for item in "${SKIPPED[@]:-}"; do [ -n "$item" ] && echo -e "  ${DIM}· kept $item${NC}"; done
    fi
    echo ""
    echo -e "  ${GREEN}✔${NC} ${BOLD}Done.${NC}"
    echo ""
}

# ── Uninstall Server ───────────────────────────────────────────────────────

uninstall_server() {
    step "Server Uninstall"

    if command -v docker &>/dev/null; then
        if docker ps -a --format '{{.Names}}' 2>/dev/null | grep -q syncnotes; then
            info "Stopping and removing Docker container..."
            (cd "$ROOT_DIR/Server" 2>/dev/null && (docker compose down -v 2>/dev/null || docker-compose down -v 2>/dev/null)) || true
            docker rm -f syncnotes 2>/dev/null || true
            ok "Container removed"
            REMOVED+=("Docker container")
        else
            skip "No running SyncNotes container found"
        fi

        mapfile -t leftover_volumes < <(docker volume ls --format '{{.Name}}' 2>/dev/null | grep -i syncnotes || true)
        if [ ${#leftover_volumes[@]} -gt 0 ]; then
            remove_if_confirmed "Docker volumes (${leftover_volumes[*]})" "Remove leftover Docker volumes: ${leftover_volumes[*]}?" \
                docker volume rm "${leftover_volumes[@]}"
        fi
    else
        skip "Docker not installed — nothing to stop"
    fi

    if [ -f "$ROOT_DIR/Server/.env" ]; then
        remove_if_confirmed ".env" "Remove Server/.env configuration file?" rm -f "$ROOT_DIR/Server/.env"
    fi
    if [ -d "$ROOT_DIR/Server/uploads" ]; then
        remove_if_confirmed "uploaded files" "Remove Server/uploads (your synced notes)?" rm -rf "$ROOT_DIR/Server/uploads"
    fi
    if [ -d "$ROOT_DIR/Server/instance" ]; then
        remove_if_confirmed "database files" "Remove Server/instance (SQLite database)?" rm -rf "$ROOT_DIR/Server/instance"
    fi

    summary
}

# ── Uninstall App ──────────────────────────────────────────────────────────

uninstall_app() {
    step "Desktop App Uninstall"

    if pkill -f "syncnotes-app" 2>/dev/null || pkill -f "/syncnotes$" 2>/dev/null; then
        ok "Stopped running instances"
        sleep 1
    else
        skip "No running instance found"
    fi

    local found_binary=""
    for path in "/usr/local/bin/syncnotes" "$HOME/.local/bin/syncnotes"; do
        [ -f "$path" ] && found_binary="$path" && break
    done

    if [ -n "$found_binary" ]; then
        remove_if_confirmed "binary ($found_binary)" "Remove binary at $found_binary?" rm -f "$found_binary"
    else
        skip "No installed binary found"
    fi

    for path in "/usr/local/bin/syncnotes-icon.png" "$HOME/.local/bin/syncnotes-icon.png"; do
        [ -f "$path" ] && rm -f "$path"
    done

    CONFIG_DIR="$HOME/.config/syncnotes"
    if [ -d "$CONFIG_DIR" ]; then
        remove_if_confirmed "app configuration" "Remove app configuration ($CONFIG_DIR)?" rm -rf "$CONFIG_DIR"
    else
        skip "No app configuration found"
    fi

    if [ -f "$HOME/.config/autostart/syncnotes.desktop" ] || [ -f "$HOME/.config/autostart/SyncNotes.desktop" ]; then
        rm -f "$HOME/.config/autostart/syncnotes.desktop" "$HOME/.config/autostart/SyncNotes.desktop" 2>/dev/null
        rmdir "$HOME/.config/autostart" 2>/dev/null || true
        ok "Removed autostart entry"
        REMOVED+=("autostart entry")
    fi

    DESKTOP_FILE="$HOME/.local/share/applications/syncnotes.desktop"
    if [ -f "$DESKTOP_FILE" ]; then
        rm -f "$DESKTOP_FILE"
        ok "Removed desktop menu entry"
        REMOVED+=("desktop menu entry")
    fi

    for rc in "$HOME/.bashrc" "$HOME/.zshrc" "$HOME/.config/fish/config.fish" "$HOME/.profile"; do
        if [ -f "$rc" ] && grep -q "syncnotes" "$rc" 2>/dev/null; then
            remove_if_confirmed "PATH entry in $(basename "$rc")" "Remove SyncNotes PATH lines from $(basename "$rc")?" \
                sed -i '/\.local\/bin\/syncnotes/d; /syncnotes/d' "$rc"
        fi
    done

    if command -v rustup &>/dev/null; then
        if confirm "Also remove the Rust toolchain (installed via rustup)? This affects other Rust projects too."; then
            rustup self uninstall -y 2>/dev/null || true
            ok "Rust toolchain removed"
            REMOVED+=("Rust toolchain")
        else
            skip "Kept Rust toolchain"
            SKIPPED+=("Rust toolchain")
        fi
    fi

    summary
}

# ── Main ───────────────────────────────────────────────────────────────────

banner

if select_option "What would you like to uninstall?" \
    "Desktop App" "Server (Docker)"; then
    uninstall_app
else
    uninstall_server
fi
