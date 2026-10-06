#!/usr/bin/env bash
set -e

REPO="fatuhsa/mtui"
TAG="v0.1.0"
ARCH=$(uname -m)

echo -e "\033[1;36m==>\033[0m Installing \033[1;32mmtui\033[0m (Mobile Terminal Music Player)..."

# 1. Install runtime dependencies (no heavy rust/cargo compiler needed!)
if command -v pkg >/dev/null 2>&1; then
    echo -e "\033[1;34m::\033[0m Installing runtime dependencies via pkg..."
    pkg update -y
    pkg install -y mpv ffmpeg chafa curl tar
elif command -v apt-get >/dev/null 2>&1; then
    echo -e "\033[1;34m::\033[0m Installing runtime dependencies via apt..."
    sudo apt-get update
    sudo apt-get install -y mpv ffmpeg chafa curl tar
elif command -v pacman >/dev/null 2>&1; then
    echo -e "\033[1;34m::\033[0m Installing runtime dependencies via pacman..."
    sudo pacman -Sy --noconfirm mpv ffmpeg chafa curl tar
fi

# Determine install target directory
if [ -n "$PREFIX" ] && [ -d "$PREFIX/bin" ]; then
    BIN_DIR="$PREFIX/bin"
elif [ -d "$HOME/.local/bin" ]; then
    BIN_DIR="$HOME/.local/bin"
elif [ -d "$HOME/.cargo/bin" ]; then
    BIN_DIR="$HOME/.cargo/bin"
else
    BIN_DIR="/usr/local/bin"
fi

# 2. Try instant prebuilt binary installation for aarch64 (Termux Android)
INSTALLED=0
if [ "$ARCH" = "aarch64" ]; then
    echo -e "\033[1;34m::\033[0m Detected architecture: \033[1;32m$ARCH\033[0m. Downloading instant prebuilt release..."
    TAR_URL="https://github.com/${REPO}/releases/download/${TAG}/mtui-termux-aarch64.tar.gz"
    TEMP_DIR=$(mktemp -d)
    
    if curl -sSL -f "$TAR_URL" -o "$TEMP_DIR/mtui.tar.gz"; then
        tar -xzf "$TEMP_DIR/mtui.tar.gz" -C "$TEMP_DIR"
        chmod +x "$TEMP_DIR/mtui"
        cp "$TEMP_DIR/mtui" "$BIN_DIR/mtui"
        rm -rf "$TEMP_DIR"
        INSTALLED=1
        echo -e "\033[1;32m::\033[0m Prebuilt binary installed to $BIN_DIR/mtui (no compilation needed!)"
    else
        echo -e "\033[1;33m::\033[0m Prebuilt download failed, falling back to Cargo build..."
    fi
fi

# 3. Fallback: compile via Cargo if prebuilt is not available for this architecture
if [ "$INSTALLED" -eq 0 ]; then
    echo -e "\033[1;34m::\033[0m Compiling mtui from source via Cargo..."
    if command -v pkg >/dev/null 2>&1; then
        pkg install -y rust git
    fi
    cargo install --git "https://github.com/${REPO}.git" --bin mtui --force
    if [ -f "$HOME/.cargo/bin" ] && [ -f "$HOME/.cargo/bin/mtui" ] && [ "$BIN_DIR" != "$HOME/.cargo/bin" ]; then
        cp "$HOME/.cargo/bin/mtui" "$BIN_DIR/mtui"
    fi
fi

echo ""
echo -e "\033[1;32m✔ mtui has been installed successfully!\033[0m"
echo -e "Run \033[1;36mmtui\033[0m to start playing music."
