"""Attached-PTY smoke test for Mox suspend forwarding and process-aware mouse scroll."""
import fcntl
import os
from pathlib import Path
import pty
import re
import select
import shlex
import struct
import subprocess
import sys
import tempfile
import termios
import time


binary = str(Path(sys.argv[1] if len(sys.argv) > 1 else "target/debug/mox").resolve())
socket = f"mox-native-smoke-{os.getpid()}"
env = dict(os.environ, TERM="xterm-256color", LC_ALL="C.UTF-8")
env["PATH"] = str(Path(binary).parent) + os.pathsep + env.get("PATH", "")
for name in ("TMUX", "TMUX_PANE"):
    env.pop(name, None)


def tmux(*args):
    return subprocess.check_output(["tmux", "-L", socket, *args], env=env).decode().rstrip("\n")


def fmt(target, expression):
    return tmux("display-message", "-p", "-t", target, expression).strip()


def drain(wait=0.15):
    deadline = time.monotonic() + wait
    output = bytearray()
    while time.monotonic() < deadline:
        if not select.select([master], [], [], 0.03)[0]:
            continue
        try:
            output.extend(os.read(master, 65536))
        except OSError:
            break
    terminal_output.extend(output)
    return bytes(output)


def send(data, wait=0.08):
    os.write(master, data)
    return drain(wait)


def wait_for(predicate, description, timeout=4.0):
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        drain(0.04)
        result = predicate()
        if result:
            return result
    raise AssertionError(f"timed out waiting for {description}")


def mouse(button, x=40, y=10):
    # SGR mouse: button 64/65 are wheel up/down; coordinates are one-based.
    send(f"\x1b[<{button};{x};{y}M".encode(), 0.2)


def visible_output(start=0):
    raw = bytes(terminal_output[start:]).decode("utf-8", errors="replace")
    return re.sub(r"\x1b(?:\[[0-?]*[ -/]*[@-~]|\][^\x07]*(?:\x07|\x1b\\))", "", raw)


def popup_key(key):
    # The feature popup bindings live under tmux's normal prefix table.
    send(b"\x02")
    send(key)


def cancel_picker():
    # Crossterm waits briefly to distinguish a standalone Escape from an
    # escape-prefixed terminal key; give that parser time before sending more.
    send(b"\x1b", 0.35)
    drain(0.25)


def wait_visible(text, start, description):
    try:
        return wait_for(
            lambda: visible_output(start) if text in visible_output(start) else None,
            description,
        )
    except AssertionError as error:
        raise AssertionError(
            f"{error}; output={visible_output(start)[-1200:]!r}; "
            f"key_table={fmt(active_client(), '#{client_key_table}')!r}"
        ) from error


def active_client():
    expected_tty = os.ttyname(slave)

    def find():
        for tty in tmux("list-clients", "-F", "#{client_tty}").splitlines():
            if tty == expected_tty:
                # The TTY is accepted as a tmux client target and is populated
                # even on tmux builds without the optional client_id format.
                return tty
        return None

    try:
        return wait_for(find, "attached tmux client")
    except AssertionError as error:
        raise AssertionError(
            f"{error}; slave={expected_tty!r}; clients="
            f"{tmux('list-clients', '-F', '#{client_tty}')!r}; "
            f"attach_exit={client.poll() if client else None}"
        ) from error


def wait_key_table(client, table):
    return wait_for(
        lambda: fmt(client, "#{client_key_table}") == table,
        f"client key table {table}",
    )


def wait_for_process(pane, command):
    try:
        return wait_for(
            lambda: fmt(pane, "#{pane_current_command}") == command,
            f"pane command {command}",
        )
    except AssertionError as error:
        raise AssertionError(
            f"{error}; current command is {fmt(pane, '#{pane_current_command}')!r}"
        ) from error


master, slave = pty.openpty()
fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 30, 100, 0, 0))
client = None
terminal_output = bytearray()
with tempfile.TemporaryDirectory(prefix="mox-native-smoke-") as tmp:
    tmp_path = Path(tmp)
    input_log = tmp_path / "received-keys.txt"
    input_program = (
        "import os,sys,tty\n"
        "tty.setraw(0)\n"
        "print('READY', flush=True)\n"
        "log=open(sys.argv[1], 'w', buffering=1)\n"
        "while True:\n"
        "    data=os.read(0,1)\n"
        "    if not data or data == b'\\x03': break\n"
        "    log.write(data.hex()+'\\n')\n"
        "    print('RX:'+data.hex(), flush=True)\n"
    )
    input_command = "python3 -u -c " + shlex.quote(input_program) + " " + shlex.quote(str(input_log))
    env["LESS"] = "-R"
    env["FZF_DEFAULT_OPTS"] = "--no-multi"
    try:
        tmux(
            "-f", "/dev/null", "new-session", "-d", "-s", "smoke",
            "-x", "100", "-y", "30", input_command,
        )
        tmux("set-option", "-s", "escape-time", "10")
        tmux("set-option", "-g", "mouse", "on")
        tmux("set-option", "-g", "remain-on-exit", "on")
        tmux("set-option", "-g", "@mox_autorestore", "off")
        subprocess.check_call(
            [binary, "init", "--apply", "--entry-key", "C-Space", "--socket", socket],
            env=env,
        )
        client = subprocess.Popen(
            ["tmux", "-L", socket, "attach-session", "-t", "smoke"],
            stdin=slave,
            stdout=slave,
            stderr=slave,
            env=env,
        )
        drain(0.5)
        client_id = active_client()
        pane = fmt(client_id, "#{pane_id}")
        wait_for(lambda: "READY" in tmux("capture-pane", "-p", "-t", pane), "input pane ready")

        # Suspend from normal root mode, then verify C-Space and each navigation
        # chord are sent to the raw pane while the client remains suspended.
        wait_key_table(client_id, "root")
        send(b"\x1bz")
        wait_key_table(client_id, "mox_suspend")
        assert "SUSPEND" in fmt(client_id, "#{E:status-left}")
        expected = []
        for key in ("\x00", "\x08", "\x0a", "\x0b", "\x0c"):
            send(key.encode("latin1"))
            expected.append(f"{ord(key):02x}")
            wait_for(
                lambda: input_log.exists() and input_log.read_text().splitlines() == expected,
                f"forwarded key {expected[-1]}",
            )
            wait_key_table(client_id, "mox_suspend")
        send(b"\x1bz")
        wait_key_table(client_id, "root")
        assert "NORMAL" in fmt(client_id, "#{E:status-left}")
        print("suspend: C-Space and C-h/j/k/l forwarded; table stayed suspended; M-z resumed")

        # Exercise the attached-client palette and project pickers, then select
        # a real file and verify the registered Neovim bridge receives it.
        marker = len(terminal_output)
        popup_key(b" ")
        wait_visible("Command palette", marker, "command palette popup")
        send(b"project")
        wait_visible("projects", marker, "palette filtering to Projects")
        cancel_picker()
        wait_key_table(client_id, "root")

        marker = len(terminal_output)
        popup_key(b"o")
        wait_visible("Projects", marker, "Projects popup")
        send(b"moch")
        wait_visible("moch", marker, "project search result")
        cancel_picker()
        wait_key_table(client_id, "root")

        nvim_socket = str(tmp_path / "registered-nvim.sock")
        nvim_command = shlex.join([
            "nvim", "--clean", "--listen", nvim_socket,
            "--cmd", "lua dofile('" + str(Path(__file__).resolve().parents[1] / "lua/mox.lua") + "')",
        ])
        nvim_pane = tmux(
            "new-window", "-d", "-P", "-F", "#{pane_id}", "-t", "smoke",
            "-n", "registered-nvim", nvim_command,
        )
        wait_for_process(nvim_pane, "nvim")
        tmux("select-window", "-t", nvim_pane)
        wait_for(
            lambda: fmt(nvim_pane, "#{@mox_nvim_server}") == nvim_socket,
            "registered Neovim bridge",
        )
        expected_file = str(Path.cwd() / "src/main.rs")
        marker = len(terminal_output)
        popup_key(b"f")
        wait_visible("Files", marker, "Files popup")
        send(b"src/main.rs")
        wait_visible("src/main.rs", marker, "file picker result")
        send(b"\r")

        def opened_file():
            result = subprocess.run(
                ["nvim", "--server", nvim_socket, "--remote-expr", "expand('%:p')"],
                env=env, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, timeout=1,
            )
            value = result.stdout.decode().strip() if result.returncode == 0 else ""
            return value if value == expected_file else None

        wait_for(opened_file, "selected file delivered through Neovim bridge", timeout=8)
        assert fmt(nvim_pane, "#{pane_current_command}") == "nvim"

        def nvim_expr(expression):
            result = subprocess.run(
                ["nvim", "--server", nvim_socket, "--remote-expr", expression],
                env=env, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, timeout=1,
            )
            return result.stdout.decode().strip() if result.returncode == 0 else ""

        nvim_expr("execute('set splitright')")
        nvim_expr("execute('vsplit')")
        wait_for(lambda: nvim_expr("winnr('$')") == "2", "Neovim vertical split")
        assert nvim_expr("winnr()") == "2", "new Neovim split should start on the right"
        tmux_left_pane = tmux(
            "split-window", "-h", "-b", "-d", "-P", "-F", "#{pane_id}",
            "-t", nvim_pane, "sh",
        )
        tmux("select-window", "-t", nvim_pane)
        tmux("select-pane", "-t", nvim_pane)
        send(b"\x08")
        wait_for(lambda: nvim_expr("winnr()") == "1", "Ctrl-h moving left inside Neovim")
        send(b"\x0c")
        wait_for(lambda: nvim_expr("winnr()") == "2", "Ctrl-l moving right inside Neovim")
        send(b"\x08")
        wait_for(lambda: nvim_expr("winnr()") == "1", "returning to Neovim left boundary")
        send(b"\x08")
        wait_for(
            lambda: fmt(client_id, "#{pane_id}") == tmux_left_pane,
            "Ctrl-h crossing from Neovim boundary to adjacent tmux pane",
            timeout=5,
        )
        assert fmt(nvim_pane, "#{pane_current_command}") == "nvim"
        print("navigation: Ctrl-h/l move inside Neovim first; boundary Ctrl-h selects adjacent tmux pane")
        print("UI: palette filter, project search/cancel, and Files-to-registered-Neovim selection passed")

        # A normal shell scroll event must enter copy-mode and move into history.
        shell_command = (
            "i=1; while [ \"$i\" -le 100 ]; do printf 'SHELL-LINE-%03d\\n' \"$i\"; "
            "i=$((i + 1)); done; exec sh"
        )
        shell_pane = tmux(
            "new-window", "-d", "-P", "-F", "#{pane_id}", "-t", "smoke",
            "-n", "shell-scroll", shlex.join(["sh", "-c", shell_command]),
        )
        wait_for(lambda: int(fmt(shell_pane, "#{history_size}") or "0") >= 70, "shell scrollback")
        tmux("select-window", "-t", shell_pane)
        mouse(64)
        wait_for(lambda: fmt(shell_pane, "#{pane_in_mode}") == "1", "shell copy-mode")
        try:
            wait_for(lambda: int(fmt(shell_pane, "#{scroll_position}") or "0") > 0, "shell history scroll")
        except AssertionError as error:
            raise AssertionError(
                f"{error}; pane_mode={fmt(shell_pane, '#{pane_mode}')}; "
                f"scroll={fmt(shell_pane, '#{scroll_position}')}; "
                f"screen={tmux('capture-pane', '-p', '-t', shell_pane)!r}"
            ) from error
        send(b"q")
        wait_for(lambda: fmt(shell_pane, "#{pane_in_mode}") == "0", "exit shell copy-mode")
        print("mouse: shell wheel enters copy-mode and scrolls retained history")

        # less is a real pager whose current command matches the line-scroll rule.
        pager_file = tmp_path / "pager.txt"
        pager_file.write_text("".join(f"PAGER-LINE-{i:03d}\n" for i in range(1, 120)))
        less_pane = tmux(
            "new-window", "-d", "-P", "-F", "#{pane_id}", "-t", "smoke",
            "-n", "line-scroll", shlex.join(["less", "-R", str(pager_file)]),
        )
        wait_for_process(less_pane, "less")
        tmux("select-window", "-t", less_pane)
        before = tmux("capture-pane", "-p", "-t", less_pane)
        assert "PAGER-LINE-001" in before, before
        mouse(65)
        after = wait_for(
            lambda: (screen if "PAGER-LINE-002" in (screen := tmux("capture-pane", "-p", "-t", less_pane)) else None),
            "less to consume mouse wheel as a line key",
        )
        assert fmt(less_pane, "#{pane_in_mode}") == "0", "less was incorrectly sent to tmux copy-mode"
        assert "PAGER-LINE-002" in after
        print("mouse: less received a line-scroll key instead of entering tmux copy-mode")

        # fzf takes the same process-aware line route. Its --expect hook exits
        # only when the wheel has been translated into an Up key.
        fzf_items = tmp_path / "fzf-items.txt"
        fzf_items.write_text("FZF-ONE\nFZF-TWO\nFZF-THREE\n")
        fzf_result = tmp_path / "fzf-result.txt"
        fzf_command = shlex.join(["fzf", "--expect=up", "--no-sort", "--no-mouse"])
        fzf_command += " < " + shlex.quote(str(fzf_items))
        fzf_command += " > " + shlex.quote(str(fzf_result))
        fzf_pane = tmux(
            "new-window", "-d", "-P", "-F", "#{pane_id}", "-t", "smoke",
            "-n", "fzf-scroll", fzf_command,
        )
        wait_for_process(fzf_pane, "fzf")
        tmux("select-window", "-t", fzf_pane)
        wait_for(
            lambda: "FZF-ONE" in tmux("capture-pane", "-p", "-t", fzf_pane),
            "fzf UI ready",
        )
        send(b"\x1b[B")
        mouse(64)
        assert fmt(fzf_pane, "#{pane_in_mode}") == "0"
        try:
            wait_for(
                lambda: fzf_result.exists() and fzf_result.read_text().strip(),
                "fzf mouse-up selection",
            )
        except AssertionError as error:
            result = fzf_result.read_text() if fzf_result.exists() else None
            raise AssertionError(
                f"{error}; result={result!r}; "
                f"pane_command={fmt(fzf_pane, '#{pane_current_command}')!r}; "
                f"screen={tmux('capture-pane', '-p', '-t', fzf_pane)!r}"
            ) from error
        assert fzf_result.read_text().splitlines()[0] == "up", fzf_result.read_text()
        print("mouse: fzf selection responds to the configured line-scroll key")

        # Neovim opts into terminal mouse reporting; WheelDown must reach it rather
        # than switching the pane into tmux copy-mode.
        nvim_socket = str(tmp_path / "nvim.sock")
        nvim_file = tmp_path / "nvim.txt"
        nvim_file.write_text("".join(f"NVIM-LINE-{i:03d}\n" for i in range(1, 160)))
        if subprocess.run(["nvim", "--version"], env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL).returncode == 0:
            nvim_command = shlex.join([
                "nvim", "--clean", "--listen", nvim_socket, "-c", "set mouse=a", str(nvim_file),
            ])
            nvim_pane = tmux(
                "new-window", "-d", "-P", "-F", "#{pane_id}", "-t", "smoke",
                "-n", "nvim-scroll", shlex.join(["sh", "-c", nvim_command]),
            )
            wait_for_process(nvim_pane, "nvim")
            tmux("select-window", "-t", nvim_pane)

            def nvim_topline():
                result = subprocess.run(
                    ["nvim", "--server", nvim_socket, "--remote-expr", "getwininfo()[0].topline"],
                    env=env, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, timeout=1,
                )
                return result.stdout.decode().strip() if result.returncode == 0 else ""

            before_top = wait_for(lambda: nvim_topline(), "Neovim RPC ready")
            mouse(65)
            after_top = wait_for(
                lambda: (value if (value := nvim_topline()) and value != before_top else None),
                "Neovim receiving mouse wheel",
            )
            assert int(after_top) > int(before_top), (before_top, after_top)
            assert fmt(nvim_pane, "#{pane_in_mode}") == "0"
            print("mouse: Neovim handled the SGR wheel event directly")
        else:
            print("mouse: Neovim unavailable; skipping optional mouse-app check")
    finally:
        subprocess.run(
            ["tmux", "-L", socket, "kill-server"], env=env,
            stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
        )
        if client:
            client.terminate()
            try:
                client.wait(timeout=2)
            except subprocess.TimeoutExpired:
                client.kill()
                client.wait()
        os.close(master)
        os.close(slave)
