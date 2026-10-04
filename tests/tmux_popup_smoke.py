"""Real popup input regression check. Run after cargo build; needs tmux and Python 3."""
import fcntl
import os
from pathlib import Path
import pty
import select
import struct
import subprocess
import sys
import tempfile
import termios
import time

binary = str(Path(sys.argv[1] if len(sys.argv) > 1 else "target/debug/mox").resolve())
socket = f"mox-popup-smoke-{os.getpid()}"
env = dict(os.environ, TERM="xterm-256color")
for name in ("TMUX", "TMUX_PANE"):
    env.pop(name, None)


def tmux(*args):
    return subprocess.check_output(["tmux", "-L", socket, *args], env=env).decode()


def drain(wait=0.6):
    deadline = time.monotonic() + wait
    output = b""
    while time.monotonic() < deadline:
        if select.select([master], [], [], 0.03)[0]:
            try:
                output += os.read(master, 65536)
            except OSError:
                break
    return output


def keys(data):
    os.write(master, data)
    return drain()


def fmt(value):
    return tmux("display-message", "-p", value).strip()


master, slave = pty.openpty()
fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 30, 100, 0, 0))
client = None
with tempfile.TemporaryDirectory(prefix="mox-clipboard-") as tmp:
    # Simulate a clipboard daemon retaining stdout without touching the real clipboard.
    clipboard = Path(tmp, "wl-copy")
    clipboard.write_text("#!/bin/sh\ncat >/dev/null\nsleep 3 &\n")
    clipboard.chmod(0o755)
    env["PATH"] = tmp + os.pathsep + env["PATH"]
    try:
        pane_command = (
            "python3 -c 'import sys,time; "
            'sys.stdout.write("\\033[2J\\033[H\\r\\n    targetalpha src/main.rs'
            '\\r\\nrowtwo https://example.com\\r\\nrowthree deadbeef1234\\r\\n"); '
            "sys.stdout.flush(); time.sleep(60)'"
        )
        tmux("-f", "/dev/null", "new-session", "-d", "-s", "smoke",
             "-x", "100", "-y", "30", pane_command)
        tmux("set-option", "-s", "escape-time", "10")
        subprocess.check_call([binary, "init", "--apply", "--socket", socket], env=env)
        client = subprocess.Popen(
            ["tmux", "-L", socket, "attach-session", "-t", "smoke"],
            stdin=slave, stdout=slave, stderr=slave, env=env,
        )
        drain()
        assert tmux("capture-pane", "-p").startswith("\n    targetalpha")
        for binding in (b"s", b"S"):
            assert b"FLASH JUMP" in keys(b"\x1bm" + binding)
            keys(b"targetalpha\r")
            assert fmt("#{copy_cursor_y},#{copy_cursor_x}") == "1,4"
            tmux("send-keys", "-X", "cancel")
        print("s/S preserve blank rows and indentation")

        keys(b"\x1bmy")
        assert b"QUICK YANK" in keys(b"h")
        keys(b"a")
        assert tmux("show-buffer").strip() == "deadbeef1234"
        # Opening another popup proves copying released the previous popup's PTY.
        assert b"[NORMAL]" in keys(b"\x1bme")
        keys(b"j")
        keys(b"y")
        assert tmux("show-buffer").strip() == "https://example.com"
        assert b"[NORMAL]" in keys(b"\x1bme")
        keys(b"/main")
        keys(b"\x1b")
        keys(b"g")
        keys(b"\x1b")
        print("quick labels and Extract selection/copy/search/cancel")

        keys(b"\x1bmv")
        tmux("send-keys", "-X", "top-line")
        tmux("send-keys", "-X", "cursor-down")
        tmux("send-keys", "-X", "start-of-line")
        keys(b"vl")
        status = fmt("#{E:status-left}")
        assert "◈ VISUAL" in status, (repr(status), fmt("#{pane_mode},#{selection_present},#{copy_cursor_y},#{copy_cursor_x},#{client_key_table}"), tmux("show-option", "-gv", "status-left"))
        assert b"CHAR MOTION" in keys(b"f")
        keys(b"t")
        keys(b"a")
        assert fmt("#{copy_cursor_y},#{copy_cursor_x},#{selection_present}") == "1,4,1"
        tmux("send-keys", "-X", "end-of-line")
        assert b"CHAR MOTION" in keys(b"F")
        keys(b"t")
        keys(b"a")
        assert fmt("#{copy_cursor_y},#{copy_cursor_x},#{selection_present}") == "1,9,1"
        assert b"REMOTE YANK" in keys(b"R")
        keys(b"\x03")
        assert fmt("#{pane_mode}") == "copy-mode"
        keys(b"q")
        assert fmt("#{pane_in_mode}") == "0"
        print("f/F preserve selection; visual icon; R cancel and copy exit")
    finally:
        subprocess.run(["tmux", "-L", socket, "kill-server"], env=env,
                       stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        if client:
            client.wait(timeout=3)
        os.close(master)
        os.close(slave)
