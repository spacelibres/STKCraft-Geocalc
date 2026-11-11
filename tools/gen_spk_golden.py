#!/usr/bin/env python3
"""Generate SPK state-vector golden vectors with CSPICE.

For a grid of epochs and target/observer pairs, calls `spkezr` in the
J2000 frame with no aberration correction and writes the six state
components. `tests/spk_golden.rs` compares the Rust implementation with
this file.

Offline tooling only; needs `spiceypy`.

Usage:
    python tools/gen_spk_golden.py --spk target/kernels/de440s.bsp \
        --out tests/golden/spk_states.csv
"""

from __future__ import annotations

import argparse
from pathlib import Path

import spiceypy as spice

TARGETS = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 301, 399]
OBSERVERS = [0, 10, 399]
EPOCHS = [
    -4.0e9,
    -3.0e9,
    -2.0e9,
    -1.0e9,
    -5.0e8,
    0.0,
    5.0e8,
    1.0e9,
    2.0e9,
    3.0e9,
    4.0e9,
]


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--spk", required=True)
    parser.add_argument("--out", default="tests/golden/spk_states.csv")
    args = parser.parse_args()

    spice.furnsh(args.spk)

    rows = []
    for et in EPOCHS:
        for target in TARGETS:
            for observer in OBSERVERS:
                if target == observer:
                    continue
                try:
                    state, _light_time = spice.spkezr(
                        str(target), et, "J2000", "NONE", str(observer)
                    )
                except Exception:
                    # Some pairs are not present in a given kernel.
                    continue
                rows.append((et, target, observer, state))

    spice.kclear()

    out = Path(args.out)
    out.parent.mkdir(parents=True, exist_ok=True)
    with out.open("w", encoding="utf-8", newline="\n") as handle:
        handle.write(
            "# CSPICE spkezr(target, et, J2000, NONE, observer) golden states.\n"
        )
        handle.write("# Position in km, velocity in km/s.\n")
        handle.write("et,target,observer,x,y,z,vx,vy,vz\n")
        for et, target, observer, state in rows:
            values = ",".join(repr(float(component)) for component in state)
            handle.write(f"{et!r},{target},{observer},{values}\n")

    print(f"wrote {out} ({len(rows)} rows)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
