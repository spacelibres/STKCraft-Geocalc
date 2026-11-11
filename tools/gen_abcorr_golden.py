#!/usr/bin/env python3
"""Generate aberration-corrected state golden vectors with CSPICE.

Calls `spkezr` with every supported `ABCORR` value over a grid of
epochs and target/observer pairs, writing the six state components and
the one-way light time. `tests/abcorr_golden.rs` compares the Rust
implementation against this file.

The SPK is not committed. Provide `--spk`, or place `de440s.bsp` at
`target/kernels/` using `tools/fetch_spk.py`.

Offline tooling only; needs `spiceypy`.

Usage:
    python tools/gen_abcorr_golden.py --spk target/kernels/de440s.bsp \
        --out tests/golden/abcorr_states.csv
"""

from __future__ import annotations

import argparse
from pathlib import Path

import spiceypy as spice

ABCORR = ["NONE", "LT", "LT+S", "CN", "CN+S", "XLT", "XLT+S", "XCN", "XCN+S"]
EPOCHS = [-2.0e9, 0.0, 2.0e9]
TARGETS = [1, 2, 3, 4, 5, 6, 7, 8, 10, 301, 399]
OBSERVERS = [0, 3, 399]


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--spk", required=True)
    parser.add_argument("--out", default="tests/golden/abcorr_states.csv")
    args = parser.parse_args()

    spice.furnsh(args.spk)

    rows = []
    for et in EPOCHS:
        for target in TARGETS:
            for observer in OBSERVERS:
                if target == observer:
                    continue
                for abcorr in ABCORR:
                    try:
                        state, light_time = spice.spkezr(
                            str(target), et, "J2000", abcorr, str(observer)
                        )
                    except Exception:
                        # A pair can be outside coverage for a retarded epoch.
                        continue
                    rows.append((et, target, observer, abcorr, state, light_time))

    spice.kclear()

    out = Path(args.out)
    out.parent.mkdir(parents=True, exist_ok=True)
    with out.open("w", encoding="utf-8", newline="\n") as handle:
        handle.write(
            "# CSPICE spkezr(target, et, J2000, abcorr, observer) golden states.\n"
        )
        handle.write("# Position in km, velocity in km/s, light time in s.\n")
        handle.write("et,target,observer,abcorr,x,y,z,vx,vy,vz,lt\n")
        for et, target, observer, abcorr, state, light_time in rows:
            values = ",".join(repr(float(component)) for component in state)
            handle.write(
                f"{et!r},{target},{observer},{abcorr},{values},{float(light_time)!r}\n"
            )

    print(f"wrote {out} ({len(rows)} rows)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
