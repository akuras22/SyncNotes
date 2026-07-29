#!/usr/bin/env bash
set -euo pipefail

RED='\033[0;31m'; GREEN='\033[0;32m'; BLUE='\033[0;34m'; YELLOW='\033[1;33m'; CYAN='\033[0;36m'; NC='\033[0m'

info()  { echo -e "  ${BLUE}→${NC} $*"; }
ok()    { echo -e "  ${GREEN}✔${NC} $*"; }
warn()  { echo -e "  ${YELLOW}⚠${NC} $*"; }
err()   { echo -e "  ${RED}✘${NC} $*"; }
header(){ echo -e "\n  ${CYAN}── $* ──${NC}"; }

ROOT_DIR="$(cd "$(dirname "$0")" && pwd)"

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
    read -rp "  Continue? [y/N]: " yn
    case "$yn" in y|Y|yes|Yes) return 0 ;; *) return 1 ;; esac
}

# ── Uninstall Server ───────────────────────────────────────────────────────

uninstall_server() {
    header "Server Uninstall"

    if command -v docker &>/dev/null; then
        if docker ps -a --format '{{.Names}}' 2>/dev/null | grep -q syncnotes; then
            info "Stopping and removing Docker container..."
            docker compose down -v 2>/dev/null || docker-compose down -v 2>/dev/null || true
            docker rm -f syncnotes 2>/dev/null || true
            ok "Container removed"
        else
            info "No running SyncNotes container found"
        fi

        if docker volume ls --format '{{.Name}}' 2>/dev/null | grep -q syncnotes; then
            if confirm "Remove Docker volumes (uploads, mysql_data)?"; then
                docker volume rm syncnotes_uploads syncnotes_mysql_data 2>/dev/null || true
                ok "Volumes removed"
            fi
        fi
    fi

    [ -f "$ROOT_DIR/Server/.env" ] && rm -f "$ROOT_DIR/Server/.env" && ok ".env removed"
    [ -d "$ROOT_DIR/Server/uploads" ] && rm -rf "$ROOT_DIR/Server/uploads" && ok "Uploads removed"
    [ -d "$ROOT_DIR/Server/instance" ] && rm -rf "$ROOT_DIR/Server/instance" && ok "Database files removed"

    ok "Server uninstalled"
}

# ── Uninstall App ──────────────────────────────────────────────────────────

uninstall_app() {
    header "Desktop App Uninstall"

    for path in "/usr/local/bin/syncnotes" "$HOME/.local/bin/syncnotes"; do
        if [ -f "$path" ]; then
            if confirm "Remove binary at $path?"; then
                rm -f "$path"
                ok "Removed $path"
            fi
            break
        fi
    done

    for path in "/usr/local/bin/logo.png" "$HOME/.local/bin/logo.png"; do
        [ -f "$path" ] && rm -f "$path" && ok "Removed $path"
    done

    CONFIG_DIR="$HOME/.config/syncnotes"
    [ -d "$CONFIG_DIR" ] && rm -rf "$CONFIG_DIR" && ok "Config removed"

    [ -f "$HOME/.config/autostart/syncnotes.desktop" ] && rm -f "$HOME/.config/autostart/syncnotes.desktop" && ok "Autostart entry removed"

    DESKTOP_FILE="$HOME/.local/share/applications/syncnotes.desktop"
    [ -f "$DESKTOP_FILE" ] && rm -f "$DESKTOP_FILE" && ok "Desktop menu entry removed"

    for rc in "$HOME/.bashrc" "$HOME/.zshrc" "$HOME/.config/fish/config.fish" "$HOME/.profile"; do
        if [ -f "$rc" ] && grep -q "syncnotes" "$rc" 2>/dev/null; then
            if confirm "Remove PATH lines from $(basename "$rc")?"; then
                sed -i '/\.local\/bin\/syncnotes/d; /syncnotes/d' "$rc" 2>/dev/null || true
                ok "Cleaned $(basename "$rc")"
            fi
        fi
    done

    if command -v cargo &>/dev/null; then
        if confirm "Remove Rust toolchain (installed via rustup)?"; then
            rustup self uninstall -y 2>/dev/null || true
            ok "Rust removed"
        fi
    fi

    ok "Desktop app uninstalled"
}

# ── Main ───────────────────────────────────────────────────────────────────

clear
echo ""
echo -e "  ${CYAN}┌──────────────────────────────────────────┐${NC}"
echo -e "  ${CYAN}│${NC}        ${BLUE}SyncNotes Uninstaller${NC}            ${CYAN}│${NC}"
echo -e "  ${CYAN}└──────────────────────────────────────────┘${NC}"
echo ""

if select_option "What would you like to uninstall?" \
    "Server (Docker)" "Desktop App"; then
    uninstall_server
else
    uninstall_app
fi
