import os
import shutil
import signal
import subprocess
import sys
from concurrent.futures import ThreadPoolExecutor, as_completed

QL_BIN: str = "tests/qlbin.exe" if sys.platform == "win32" else "tests/qlbin"
OLD_QL_BIN: str = (
    "target/debug/quantum_launcher.exe"
    if sys.platform == "win32"
    else "target/debug/quantum_launcher"
)


def prepare_ql_bin() -> None:
    shutil.copy(OLD_QL_BIN, QL_BIN)
    with open('tests/qldir.txt', 'w') as f:
        f.write('')


def run(args: list[str]) -> None:
    try:
        subprocess.run(args)
    except subprocess.CalledProcessError as e:
        print(f"Error: Process failed with exit code {e.returncode}")
        sout = e.stdout.decode()
        if len(sout) > 0:
            print(f"Stdout:\n{sout}")
        sout = e.stderr.decode()
        if len(sout) > 0:
            print(f"Stderr:\n{sout}")
        sys.exit(1)


def run_parallel(commands: list[list[str]], max_workers: int | None = None) -> None:
    with ThreadPoolExecutor(max_workers=max_workers) as executor:
        futures = {executor.submit(run, cmd): cmd for cmd in commands}

        try:
            for future in as_completed(futures):
                future.result()  # Will raise if the subprocess failed
        except Exception as e:
            print("Early exit: A subprocess failed. Cancelling remaining...")
            print(e)
            for f in futures:
                f.cancel()
            sys.exit(1)


def kill_process(pid: int) -> None:
    try:
        os.kill(pid, signal.SIGTERM)  # SIGTERM is a termination signal
    except ProcessLookupError:
        print(f"    Process {pid} not found.")
    except PermissionError:
        print(f"    Permission denied to kill process {pid}.")
    except Exception as e:
        print(f"Error occurred: {e}")
