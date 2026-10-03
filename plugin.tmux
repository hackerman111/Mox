#!/usr/bin/env bash

set -e

CURRENT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
RELEASE_BIN="$CURRENT_DIR/target/release/mox"
DEBUG_BIN="$CURRENT_DIR/target/debug/mox"

if [ -f "$RELEASE_BIN" ]; then
    BINARY="$RELEASE_BIN"
elif [ -f "$DEBUG_BIN" ]; then
    BINARY="$DEBUG_BIN"
elif [ -f "$CURRENT_DIR/target/release/moch" ]; then
    BINARY="$CURRENT_DIR/target/release/moch"
else
    # Build binary if not already built
    if command -v cargo >/dev/null 2>&1; then
        (cd "$CURRENT_DIR" && cargo build --release --quiet)
        BINARY="$RELEASE_BIN"
    else
        tmux display-message "Mox: cargo not found and binary not compiled"
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
