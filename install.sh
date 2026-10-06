#!/usr/bin/env bash
set -e

echo -e "\033[1;36m==>\033[0m Installing \033[1;32mmtui\033[0m (Mobile Terminal Music Player)..."

# 1. Install prerequisites in Termux
if command -v pkg >/dev/null 2>&1; then
    echo -e "\033[1;34m::\033[0m Installing required packages via pkg..."
    pkg update -y
    pkg install -y rust mpv ffmpeg chafa git
elif command -v apt-get >/dev/null 2>&1; then
    echo -e "\033[1;34m::\033[0m Installing required packages via apt..."
    sudo apt-get update
    sudo apt-get install -y cargo rustc mpv ffmpeg chafa git
elif command -v pacman >/dev/null 2>&1; then
    echo -e "\033[1;34m::\033[0m Installing required packages via pacman..."
    sudo pacman -Sy --noconfirm rust mpv ffmpeg chafa git
fi

# 2. Build or install mtui binary via Cargo
echo -e "\033[1;34m::\033[0m Compiling and installing mtui binary..."
cargo install --git https://github.com/fatuhsa/mtui.git --bin mtui --force

# If PREFIX exists (Termux), copy to $PREFIX/bin for immediate availability
if [ -n "$PREFIX" ] && [ -f "$HOME/.cargo/bin/mtui" ]; then
    cp "$HOME/.cargo/bin/mtui" "$PREFIX/bin/mtui"
fi

echo ""
echo -e "\033[1;32m✔ mtui has been installed successfully!\033[0m"
echo -e "Run \033[1;36mmtui\033[0m to start playing music."
