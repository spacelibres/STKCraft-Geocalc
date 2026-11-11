#!/usr/bin/env python3
"""Concurrent, resumable downloader for NAIF kernels.

Kernels are large and are never committed. Every golden test in this
repository looks for them under `target/kernels/`, so this tool fills that
directory in one pass.

Behaviour:

* One thread per file, `--jobs` files at a time.
* Resume: each file is written to `<name>.part`. On restart the tool sends
  `Range: bytes=<part size>-` and continues. If the server ignores the
  range request the file restarts from zero instead of corrupting.
* Retries with exponential backoff. The `.part` file survives a retry, so
  a dropped connection costs one backoff, not the whole file.
* `--detach` re-launches the work in its own process, writes a log under
  `target/logs/`, and returns immediately, so nothing blocks a terminal
  window or an editor session.

Only the Python standard library is used.

Usage:
    # See what is already present, without touching the network.
    python tools/fetch_kernels.py --status

    # Confirm every URL is live and its size matches the local file.
    python tools/fetch_kernels.py --check

    # Download everything missing, 4 files at a time, in this window.
    python tools/fetch_kernels.py

    # Start it in the background and come back later.
    python tools/fetch_kernels.py --detach
    # then watch progress with:
    #   type target\\logs\\fetch-<stamp>.log        (Windows)
    #   tail -f target/logs/fetch-<stamp>.log       (POSIX)

    # Only some files, or a custom manifest.
    python tools/fetch_kernels.py --only de442s.bsp,pck00011.tpc
    python tools/fetch_kernels.py --manifest my-kernels.json
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import ssl
import subprocess
import sys
import threading
import time
import urllib.error
import urllib.request
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path

USER_AGENT = "stkcraft-geocalc-fetch/1.0"
BASE = "https://naif.jpl.nasa.gov/pub/naif/generic_kernels"
CHUNK = 1 << 20

# name, path under BASE, expected bytes. The sizes are the authoritative
# values published by NAIF; `--check` re-reads them from the server.
DEFAULT_CATALOGUE: list[tuple[str, str, int]] = [
    ("de440s.bsp", "spk/planets/de440s.bsp", 32726016),
    ("de440.bsp", "spk/planets/de440.bsp", 119799808),
    ("de442s.bsp", "spk/planets/de442s.bsp", 32701440),
    ("pck00011.tpc", "pck/pck00011.tpc", 131226),
    ("mars_iau2000_v1.tpc", "pck/mars_iau2000_v1.tpc", 3559),
    ("gm_de431.tpc", "pck/gm_de431.tpc", 6291),
    ("gm_de440.tpc", "pck/gm_de440.tpc", 12406),
    ("naif0012.tls", "lsk/naif0012.tls", 5257),
    ("moon_080317.tf", "fk/satellites/moon_080317.tf", 21437),
    ("moon_de440_250416.tf", "fk/satellites/moon_de440_250416.tf", 19478),
    ("jup347_nameid.tf", "fk/satellites/jup347_nameid.tf", 3771),
]


@dataclass
class Task:
    name: str
    url: str
    dest: Path
    size: int = 0
    sha256: str = ""

    @property
    def part(self) -> Path:
        return self.dest.with_name(self.dest.name + ".part")


class Log:
    """Serialised line output that is safe to call from worker threads."""

    def __init__(self, path: Path | None, echo: bool) -> None:
        self._lock = threading.Lock()
        self._echo = echo
        self._handle = None
        if path is not None:
            path.parent.mkdir(parents=True, exist_ok=True)
            self._handle = path.open("a", encoding="utf-8", newline="\n")

    def line(self, text: str) -> None:
        stamp = datetime.now(timezone.utc).strftime("%H:%M:%S")
        rendered = f"{stamp} {text}"
        with self._lock:
            if self._echo:
                print(rendered, flush=True)
            if self._handle is not None:
                self._handle.write(rendered + "\n")
                self._handle.flush()

    def close(self) -> None:
        if self._handle is not None:
            self._handle.close()
            self._handle = None


def human(count: float) -> str:
    for unit in ("B", "KiB", "MiB", "GiB", "TiB"):
        if abs(count) < 1024.0 or unit == "TiB":
            return f"{count:.1f} {unit}"
        count /= 1024.0
    return f"{count:.1f} TiB"


def load_manifest(path: Path) -> list[Task]:
    payload = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(payload, list):
        raise ValueError("manifest must be a JSON list of objects")
    tasks: list[Task] = []
    for entry in payload:
        tasks.append(
            Task(
                name=str(entry["name"]),
                url=str(entry["url"]),
                dest=Path(entry.get("out", f"target/kernels/{entry['name']}")),
                size=int(entry.get("size", 0) or 0),
                sha256=str(entry.get("sha256", "") or ""),
            )
        )
    return tasks


def catalogue(destination: Path) -> list[Task]:
    return [
        Task(name=name, url=f"{BASE}/{relative}", dest=destination / name)
        for name, relative, size in DEFAULT_CATALOGUE
    ]


def sha256_of(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(CHUNK), b""):
            digest.update(block)
    return digest.hexdigest()


def verify(task: Task, log: Log) -> bool:
    if task.size and task.dest.stat().st_size != task.size:
        log.line(
            f"[{task.name}] size mismatch: {task.dest.stat().st_size} != {task.size}"
        )
        return False
    if task.sha256:
        actual = sha256_of(task.dest)
        if actual.lower() != task.sha256.lower():
            log.line(f"[{task.name}] sha256 mismatch: {actual} != {task.sha256}")
            return False
    return True


def fetch(task: Task, context: ssl.SSLContext, timeout: float, retries: int, log: Log) -> bool:
    """Download one file, resuming from `task.part`. Returns success."""

    task.dest.parent.mkdir(parents=True, exist_ok=True)

    if task.dest.exists():
        if verify(task, log):
            log.line(f"[{task.name}] already complete ({human(task.dest.stat().st_size)})")
            return True
        log.line(f"[{task.name}] existing file failed verification; re-downloading")
        task.dest.unlink()

    for attempt in range(1, retries + 1):
        offset = task.part.stat().st_size if task.part.exists() else 0
        headers = {"User-Agent": USER_AGENT}
        if offset:
            headers["Range"] = f"bytes={offset}-"

        try:
            request = urllib.request.Request(task.url, headers=headers)
            with urllib.request.urlopen(request, context=context, timeout=timeout) as response:
                status = response.status
                mode = "ab"
                if status == 200:
                    if offset:
                        log.line(f"[{task.name}] server ignored the range request; restarting")
                    offset = 0
                    mode = "wb"
                elif status != 206:
                    raise urllib.error.HTTPError(
                        task.url, status, "unexpected status", response.headers, None
                    )

                total = 0
                content_range = response.headers.get("Content-Range", "")
                if "/" in content_range:
                    try:
                        total = int(content_range.rsplit("/", 1)[1])
                    except ValueError:
                        total = 0
                else:
                    length = response.headers.get("Content-Length")
                    if length:
                        total = offset + int(length)

                log.line(
                    f"[{task.name}] resume at {human(offset)}"
                    + (f" of {human(total)}" if total else "")
                )

                written = offset
                started = time.monotonic()
                last_report = started
                with task.part.open(mode) as handle:
                    while True:
                        block = response.read(CHUNK)
                        if not block:
                            break
                        handle.write(block)
                        written += len(block)
                        now = time.monotonic()
                        if now - last_report >= 2.0:
                            last_report = now
                            rate = (written - offset) / max(now - started, 1e-9)
                            share = f"{100.0 * written / total:5.1f}%" if total else "  ?  "
                            log.line(
                                f"[{task.name}] {share} {human(written)}"
                                + (f"/{human(total)}" if total else "")
                                + f" {human(rate)}/s"
                            )

            size = task.part.stat().st_size
            if task.size and size != task.size:
                log.line(f"[{task.name}] incomplete ({size} of {task.size}); will retry")
                if size > task.size:
                    task.part.unlink()
                raise OSError("size mismatch after transfer")

            os.replace(task.part, task.dest)
            if not verify(task, log):
                return False
            log.line(f"[{task.name}] done, {human(task.dest.stat().st_size)}")
            return True

        except urllib.error.HTTPError as error:
            if error.code == 416:
                # The range was not satisfiable: the part file is either
                # already complete or larger than the remote object.
                size = task.part.stat().st_size if task.part.exists() else 0
                if task.size and size == task.size:
                    os.replace(task.part, task.dest)
                    return verify(task, log)
                log.line(f"[{task.name}] range not satisfiable; restarting from zero")
                if task.part.exists():
                    task.part.unlink()
                continue
            if error.code in (403, 404):
                log.line(f"[{task.name}] {error.code} {task.url}")
                return False
            log.line(f"[{task.name}] HTTP {error.code} on attempt {attempt}/{retries}")
        except (urllib.error.URLError, TimeoutError, OSError) as error:
            log.line(f"[{task.name}] {type(error).__name__}: {error} (attempt {attempt}/{retries})")

        if attempt < retries:
            delay = min(2.0**attempt, 60.0)
            log.line(f"[{task.name}] retrying in {delay:.0f}s")
            time.sleep(delay)

    log.line(f"[{task.name}] giving up after {retries} attempts")
    return False


def run(tasks: list[Task], jobs: int, context: ssl.SSLContext, timeout: float, retries: int, log: Log) -> int:
    pending = [task for task in tasks if not (task.dest.exists() and verify(task, log))]
    if not pending:
        log.line("nothing to download")
        return 0

    log.line(f"downloading {len(pending)} file(s) with {jobs} worker(s)")
    failures: list[str] = []
    lock = threading.Lock()
    queue = list(pending)

    def worker() -> None:
        while True:
            with lock:
                if not queue:
                    return
                task = queue.pop(0)
            if not fetch(task, context, timeout, retries, log):
                with lock:
                    failures.append(task.name)

    threads = [threading.Thread(target=worker, name=f"fetch-{index}") for index in range(jobs)]
    for thread in threads:
        thread.start()
    for thread in threads:
        thread.join()

    if failures:
        log.line("failed: " + ", ".join(sorted(failures)))
        return 1
    log.line("all files complete")
    return 0


def status(tasks: list[Task]) -> int:
    problems = 0
    for task in tasks:
        if task.dest.exists():
            size = task.dest.stat().st_size
            if task.size and size != task.size:
                print(f"  wrong size {task.name}  {human(size)} (expected {human(task.size)})")
                problems += 1
            else:
                print(f"  complete   {task.name}  {human(size)}")
        elif task.part.exists():
            print(f"  partial    {task.name}  {human(task.part.stat().st_size)}")
            problems += 1
        else:
            print(f"  missing    {task.name}")
            problems += 1
    return 0 if problems == 0 else 1


def check(tasks: list[Task], context: ssl.SSLContext, timeout: float) -> int:
    """HEAD every task and compare the remote size with the local file."""

    failures = 0
    for task in tasks:
        try:
            request = urllib.request.Request(
                task.url, method="HEAD", headers={"User-Agent": USER_AGENT}
            )
            with urllib.request.urlopen(request, context=context, timeout=timeout) as response:
                length = response.headers.get("Content-Length")
                remote = int(length) if length else 0
                local = task.dest.stat().st_size if task.dest.exists() else None
                note = "missing locally"
                if local is not None:
                    note = "matches" if remote == local else f"local is {human(local)}"
                elif task.part.exists():
                    note = f"partial {human(task.part.stat().st_size)}"
                print(f"  {response.status} {human(remote):>10}  {task.name:<24} {note}")
        except urllib.error.HTTPError as error:
            print(f"  {error.code} {'':>10}  {task.name:<24} {task.url}")
            failures += 1
        except (urllib.error.URLError, TimeoutError, OSError) as error:
            print(f"  ERR {'':>10} {task.name:<24} {type(error).__name__}: {error}")
            failures += 1
    return 0 if failures == 0 else 1


def detach(argv: list[str], log_path: Path) -> int:
    """Re-launch this script as a detached process and return immediately."""

    command = [sys.executable, str(Path(__file__).resolve()), *argv]
    if "--detach" in command:
        command.remove("--detach")
    log_path.parent.mkdir(parents=True, exist_ok=True)
    handle = log_path.open("a", encoding="utf-8", newline="\n")
    options: dict = {"stdout": handle, "stderr": subprocess.STDOUT, "stdin": subprocess.DEVNULL}
    if os.name == "nt":
        options["creationflags"] = (
            subprocess.DETACHED_PROCESS
            | subprocess.CREATE_NEW_PROCESS_GROUP
            | getattr(subprocess, "CREATE_NO_WINDOW", 0)
        )
    else:
        options["start_new_session"] = True
    process = subprocess.Popen(command, close_fds=True, **options)
    handle.close()
    print(f"started pid {process.pid}")
    print(f"log: {log_path}")
    print("check progress with:")
    print(f"  {'type' if os.name == 'nt' else 'tail -f'} {log_path}")
    return 0


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--dest", default="target/kernels", help="destination directory")
    parser.add_argument("--manifest", help="JSON manifest instead of the built-in catalogue")
    parser.add_argument("--only", help="comma-separated names to keep")
    parser.add_argument("--exclude", help="comma-separated names to skip")
    parser.add_argument("--jobs", type=int, default=4, help="files downloaded at once")
    parser.add_argument("--retries", type=int, default=6, help="attempts per file")
    parser.add_argument("--timeout", type=float, default=120.0, help="socket timeout, seconds")
    parser.add_argument("--insecure", action="store_true", help="skip TLS verification")
    parser.add_argument("--detach", action="store_true", help="run in a background process")
    parser.add_argument("--log", help="log file for --detach")
    parser.add_argument("--status", action="store_true", help="report progress and exit")
    parser.add_argument(
        "--check",
        action="store_true",
        help="HEAD every URL and compare the remote size with the local file",
    )
    args = parser.parse_args(argv)

    destination = Path(args.dest)
    tasks = load_manifest(Path(args.manifest)) if args.manifest else catalogue(destination)

    if args.only:
        keep = {name.strip() for name in args.only.split(",") if name.strip()}
        tasks = [task for task in tasks if task.name in keep]
    if args.exclude:
        drop = {name.strip() for name in args.exclude.split(",") if name.strip()}
        tasks = [task for task in tasks if task.name not in drop]
    if not tasks:
        print("no files selected", file=sys.stderr)
        return 2

    if args.status:
        return status(tasks)

    if args.check:
        context = (
            ssl._create_unverified_context() if args.insecure else ssl.create_default_context()
        )
        return check(tasks, context, args.timeout)

    if args.detach:
        stamp = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ")
        log_path = Path(args.log) if args.log else Path("target/logs") / f"fetch-{stamp}.log"
        return detach(argv, log_path)

    log = Log(None, echo=True)
    context = ssl._create_unverified_context() if args.insecure else ssl.create_default_context()
    try:
        return run(tasks, max(1, args.jobs), context, args.timeout, max(1, args.retries), log)
    finally:
        log.close()


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
