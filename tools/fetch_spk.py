#!/usr/bin/env python3
"""Download a SPK kernel for validation.

The kernel is large and is never committed. `tests/spk_golden.rs` looks
for it at `target/kernels/de440s.bsp` (or at `$STKCRAFT_SPK`).

Usage:
    python tools/fetch_spk.py
    python tools/fetch_spk.py --url <url> --out target/kernels/de440s.bsp
"""

from __future__ import annotations

import argparse
import ssl
import urllib.request
from pathlib import Path

DEFAULT_URL = "https://naif.jpl.nasa.gov/pub/naif/generic_kernels/spk/planets/de440s.bsp"


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--url", default=DEFAULT_URL)
    parser.add_argument("--out", default="target/kernels/de440s.bsp")
    parser.add_argument("--insecure", action="store_true")
    args = parser.parse_args()

    out = Path(args.out)
    if out.exists():
        print(f"{out} already exists ({out.stat().st_size} bytes)")
        return 0

    out.parent.mkdir(parents=True, exist_ok=True)
    context = (
        ssl._create_unverified_context()
        if args.insecure
        else ssl.create_default_context()
    )
    request = urllib.request.Request(args.url, headers={"User-Agent": "stkcraft-geocalc"})
    with urllib.request.urlopen(request, context=context, timeout=300) as response:
        out.write_bytes(response.read())

    print(f"wrote {out} ({out.stat().st_size} bytes)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
