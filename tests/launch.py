import os
import re
import subprocess
import sys
import time

from . import procs
from .types import Version, PID

_ANSI_ESCAPE: re.Pattern[str] = re.compile(r'\x1b\[[0-9;]*[mK]')
_PID_LOG: re.Pattern[str] = re.compile(r'(?:\[info\] Launched!\s+|-\s+)PID: (\d+)')

IS_XWAYLAND = os.environ.get("WAYLAND_DISPLAY") is not None and os.environ.get("DISPLAY") is not None
IS_X11 = os.getenv("XDG_SESSION_TYPE") == "x11"
IS_WINDOWS: bool = sys.platform.startswith("win")


def _remove_ansi_colors(text: str):
    return _ANSI_ESCAPE.sub('', text)


def _launch(instance: Version) -> PID | None:
    process = subprocess.Popen(
        [procs.QL_BIN, "launch", instance, "test`"],
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True
    )

    if not process.stdout:
        print("Error: Launcher has no stdout!")
        sys.exit(1)

    pid: PID | None = None

    for line in process.stdout:
        clean_line = _remove_ansi_colors(line)
        if "No ID found!" in clean_line:
            print("Error: Game crashed instantly!")
            process.kill()
            return None
        match = re.search(_PID_LOG, clean_line)
        if match:
            pid = PID(match.group(1))
            break

    if not pid:
        print("Error: No PID found!")
        process.kill()
        return None

    return pid


def _is_process_alive(pid: int) -> bool:
    try:
        os.kill(pid, 0)
    except ProcessLookupError:
        return False  # No such process
    except PermissionError:
        return True  # Process exists, but not owned by you
    else:
        return True  # Process is alive


# ---------------------------
# Windows Implementation
# ---------------------------
if sys.platform.startswith("win"):
    import ctypes
    from ctypes import wintypes

    user32 = ctypes.WinDLL("user32", use_last_error=True)
    WM_CLOSE = 0x0010

    EnumWindows = user32.EnumWindows
    EnumWindowsProc = ctypes.WINFUNCTYPE(wintypes.BOOL, wintypes.HWND, wintypes.LPARAM)
    GetWindowThreadProcessId = user32.GetWindowThreadProcessId
    IsWindowVisible = user32.IsWindowVisible
    SendMessageW = user32.SendMessageW
    PostMessageW = user32.PostMessageW

    g_found: list[tuple[int, str]] = []
    g_pid: int = 0


    def _get_windows_for_pid(pid: int) -> list[int]:
        # Return list of hwnd for windows belonging to pid
        found: list[int] = []

        @EnumWindowsProc
        def _enum(hwnd, l_param):
            # Check visibility (optional; keeps parity with xdotool which finds top-level windows)
            try:
                if not IsWindowVisible(hwnd):
                    return True  # continue
            except:
                # If any call fails, still continue enumeration
                pass

            # window_pid_raw = lpdwProcessId
            window_pid_raw = wintypes.DWORD()
            GetWindowThreadProcessId(hwnd, ctypes.byref(window_pid_raw))
            window_pid = window_pid_raw.value

            if window_pid == pid:
                found.append(hwnd)
            return True

        # Start enumeration
        if not EnumWindows(_enum, 0):
            # EnumWindows returns 0 on failure; raise for debugging, but return whatever found
            err = ctypes.get_last_error()
            print(f"    Warning: EnumWindows failed with error {err}")
        return found


    def _attempt_to_close_window(hwnd: int) -> None:
        # Send WM_CLOSE to each handle; prefer SendMessage (synchronous) then fallback to PostMessage
        try:
            SendMessageW(hwnd, WM_CLOSE, 0, 0)
        except:
            try:
                PostMessageW(hwnd, WM_CLOSE, 0, 0)
            except:
                pass


    def _close_window_windows(pid: PID) -> bool:
        wins = _get_windows_for_pid(pid)
        if not wins:
            return False

        print(f"\n✅ Window found!")
        for hwnd in wins:
            _attempt_to_close_window(hwnd)
        procs.kill_process(pid)
        return True


def _close_window_unix(result: bytes, pid: int) -> None:
    window_ids: list[str] = result.decode().strip().splitlines()
    print(f"\n✅ Window found! {'(by name), ' if len(window_ids) == 0 else ''}")
    procs.kill_process(pid)


def _wait_for_window(pid: PID, timeout: int, name: str) -> bool:
    start_time = time.time()
    check_interval = max(1, timeout // 30)
    print(f"Checking {name} ({pid}).", end="")
    sys.stdout.flush()

    while time.time() - start_time < timeout:
        if not _is_process_alive(pid):
            print("\nError: Game crashed!")
            return False

        if IS_WINDOWS:
            if _close_window_windows(pid):
                return True
        elif IS_XWAYLAND or IS_X11:
            try:
                # Uses same logic as of x11
                result = subprocess.check_output(["xdotool", "search", "--pid", str(pid)])
                _close_window_unix(result, pid)
                return True
            except subprocess.CalledProcessError:
                try:
                    result = subprocess.check_output(["xdotool", "search", "--classname", "Minecraft*", "windowclose"])
                    _close_window_unix(result, pid)
                    return True
                except subprocess.CalledProcessError:
                    pass  # No window yet
        else:
            print("\nWayland without XWayland detected — no xdotool support")

        time.sleep(check_interval)
        print(".", end="")
        sys.stdout.flush()

    print("\nError: Timeout waiting for window!")
    return False


def test(name: Version, timeout: int) -> bool:
    pid: PID | None = _launch(name)
    if pid:
        res = _wait_for_window(pid, timeout, name)
        print()
        if not res:
            print("Test failed (window)!")
            return False
    else:
        print("Test failed (launch)!")
        return False
    return True
