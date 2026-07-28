#!/usr/bin/env bash
set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m'

info()  { echo -e "${BLUE}[INFO]${NC} $*"; }
ok()    { echo -e "${GREEN}[OK]${NC} $*"; }
warn()  { echo -e "${YELLOW}[WARN]${NC} $*"; }
err()   { echo -e "${RED}[ERR]${NC} $*"; }

ROOT_DIR="$(cd "$(dirname "$0")" && pwd)"

select_option() {
    local prompt="$1" opt1="$2" opt2="$3"
    echo ""
    echo -e "${BLUE}$prompt${NC}"
    echo "  1) $opt1"
    echo "  2) $opt2"
    echo ""
    while true; do
        read -rp "Select [1/2]: " choice
        case "$choice" in
            1) return 0 ;;
            2) return 1 ;;
            *) warn "Please enter 1 or 2." ;;
        esac
    done
}

confirm() {
    local msg="$1"
    echo ""
    warn "$msg"
    read -rp "Continue? [y/N]: " yn
    case "$yn" in
        y|Y|yes|Yes) return 0 ;;
        *) return 1 ;;
    esac
}

# ── Uninstall Server ───────────────────────────────────────────────────────

uninstall_server() {
    echo ""
    info "Uninstalling SyncNotes server..."

    cd "$ROOT_DIR/Server" 2>/dev/null || true

    if command -v docker &>/dev/null; then
        if docker ps -a --format '{{.Names}}' 2>/dev/null | grep -q syncnotes; then
            info "Stopping and removing Docker container..."
            docker compose down -v 2>/dev/null || docker-compose down -v 2>/dev/null || true
            docker rm -f syncnotes 2>/dev/null || true
            ok "Docker container removed."
        else
            info "No running SyncNotes Docker container found."
        fi

        if docker volume ls --format '{{.Name}}' 2>/dev/null | grep -q syncnotes; then
            if confirm "Remove Docker volumes (syncnotes_uploads, mysql_data)?"; then
                docker volume rm syncnotes_uploads 2>/dev/null || true
                docker volume rm syncnotes_mysql_data 2>/dev/null || true
                ok "Docker volumes removed."
            fi
        fi
    fi

    if [ -f "$ROOT_DIR/Server/.env" ]; then
        if confirm "Remove Server/.env configuration file?"; then
            rm -f "$ROOT_DIR/Server/.env"
            ok ".env removed."
        fi
    fi

    if [ -d "$ROOT_DIR/Server/uploads" ]; then
        if confirm "Remove uploaded files (Server/uploads/)?"; then
            rm -rf "$ROOT_DIR/Server/uploads"
            ok "Uploads removed."
        fi
    fi

    if [ -d "$ROOT_DIR/Server/instance" ]; then
        if confirm "Remove database files (Server/instance/)?"; then
            rm -rf "$ROOT_DIR/Server/instance"
            ok "Database files removed."
        fi
    fi

    echo ""
    ok "Server uninstalled."
}

# ── Uninstall App ──────────────────────────────────────────────────────────

uninstall_app() {
    echo ""
    info "Uninstalling SyncNotes desktop app..."

    # Remove binary
    for path in "/usr/local/bin/syncnotes" "$HOME/.local/bin/syncnotes"; do
        if [ -f "$path" ]; then
            if confirm "Remove binary at $path?"; then
                rm -f "$path"
                ok "Removed $path"
            fi
            break
        fi
    done

    # Remove config
    CONFIG_DIR="$HOME/.config/syncnotes"
    if [ -d "$CONFIG_DIR" ]; then
        if confirm "Remove configuration directory ($CONFIG_DIR)?"; then
            rm -rf "$CONFIG_DIR"
            ok "Configuration removed."
        fi
    fi

    # Remove autostart entry
    AUTOSTART_DIR="$HOME/.config/autostart"
    if [ -f "$AUTOSTART_DIR/syncnotes.desktop" ]; then
        if confirm "Remove autostart entry?"; then
            rm -f "$AUTOSTART_DIR/syncnotes.desktop"
            ok "Autostart entry removed."
        fi
    fi

    # Remove Desktop entry
    DESKTOP_FILE="$HOME/.local/share/applications/syncnotes.desktop"
    if [ -f "$DESKTOP_FILE" ]; then
        if confirm "Remove Desktop Menu entry?"; then
            rm -f "$DESKTOP_FILE"
            ok "Desktop entry removed."
        fi
    fi

    # Remove PATH addition from shell config
    for rc in "$HOME/.bashrc" "$HOME/.zshrc" "$HOME/.config/fish/config.fish" "$HOME/.profile"; do
        if [ -f "$rc" ]; then
            if grep -q "syncnotes" "$rc" 2>/dev/null; then
                if confirm "Remove PATH modifications from $rc?"; then
                    sed -i '/\.local\/bin\/syncnotes/d' "$rc" 2>/dev/null || true
                    sed -i '/syncnotes/d' "$rc" 2>/dev/null || true
                    ok "Cleaned $rc"
                fi
            fi
        fi
    done

    # Optionally remove Rust
    if command -v cargo &>/dev/null; then
        if confirm "Remove Rust toolchain (installed via rustup)?"; then
            rustup self uninstall -y 2>/dev/null || true
            ok "Rust removed."
        fi
    fi

    echo ""
    ok "Desktop app uninstalled."
}

# ── Main ───────────────────────────────────────────────────────────────────

echo ""
echo -e "${BLUE}══════════════════════════════════════${NC}"
echo -e "${BLUE}     SyncNotes Uninstaller (Linux)${NC}"
echo -e "${BLUE}══════════════════════════════════════${NC}"

if select_option "What would you like to uninstall?" \
    "Server (Docker)" "Desktop App"; then
    uninstall_server
else
    uninstall_app
fi
