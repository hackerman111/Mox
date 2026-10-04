#!/usr/bin/env bash

set -e

CURRENT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
RELEASE_BIN="$CURRENT_DIR/target/release/mox"
DEBUG_BIN="$CURRENT_DIR/target/debug/mox"
PLUGIN_VERSION="$(sed -n 's/^version = "\([^"]*\)"/\1/p' "$CURRENT_DIR/Cargo.toml" | head -n1)"

is_current_binary() {
    [ -x "$1" ] && [ "$("$1" --version 2>/dev/null)" = "Mox $PLUGIN_VERSION" ]
}

if is_current_binary "$RELEASE_BIN"; then
    BINARY="$RELEASE_BIN"
elif is_current_binary "$DEBUG_BIN"; then
    BINARY="$DEBUG_BIN"
else
    PLATFORM=""
    case "$(uname -s):$(uname -m)" in
        Linux:x86_64|Linux:amd64) PLATFORM="linux-x86_64" ;;
        Linux:aarch64|Linux:arm64) PLATFORM="linux-aarch64" ;;
        Darwin:x86_64) PLATFORM="macos-x86_64" ;;
        Darwin:arm64|Darwin:aarch64) PLATFORM="macos-aarch64" ;;
    esac

    if [ -n "$PLATFORM" ] && [ -n "$PLUGIN_VERSION" ] \
        && { command -v curl >/dev/null 2>&1 || command -v wget >/dev/null 2>&1; }; then
        mkdir -p "$(dirname "$RELEASE_BIN")"
        DOWNLOAD_URL="https://github.com/hackerman111/Mox/releases/download/v$PLUGIN_VERSION/mox-$PLATFORM"
        if command -v curl >/dev/null 2>&1; then
            curl -fsSL "$DOWNLOAD_URL" -o "$RELEASE_BIN" || rm -f "$RELEASE_BIN"
        else
            wget -q "$DOWNLOAD_URL" -O "$RELEASE_BIN" || rm -f "$RELEASE_BIN"
        fi
        if [ -s "$RELEASE_BIN" ]; then
            chmod +x "$RELEASE_BIN"
            BINARY="$RELEASE_BIN"
        fi
    fi

    if [ -z "${BINARY:-}" ] && command -v cargo >/dev/null 2>&1; then
        (cd "$CURRENT_DIR" && cargo build --release --quiet)
        BINARY="$RELEASE_BIN"
    elif [ -z "${BINARY:-}" ]; then
        tmux display-message "Mox: binary unavailable; install Cargo or provide a matching prebuilt binary"
        exit 1
    fi
fi

ENTRY_KEY="$(tmux show-option -gqv "@mox_entry_key")"
if [ -z "$ENTRY_KEY" ]; then
    ENTRY_KEY="$(tmux show-option -gqv "@moch_entry_key")"
fi
if [ -z "$ENTRY_KEY" ]; then
    ENTRY_KEY="M-m"
fi

"$BINARY" init --entry-key "$ENTRY_KEY" --apply
