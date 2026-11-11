#!/usr/bin/env python3
"""Generate pointing-direction golden data from CSPICE `spkezp`.

The expected value is CSPICE VHAT(SPKEZP), without Python normalization.
This preserves the reference routine's scaled norm and operation order.

Usage:
    python tools/gen_pointing_golden.py --spk target/kernels/de440s.bsp
"""

from __future__ import annotations

import argparse
from pathlib import Path

import spiceypy as spice

EPOCHS = [-5.0e8, 0.0, 5.0e8]
CASES = [
    (301, 399, "MOON from EARTH"),
    (10, 399, "SUN from EARTH"),
    (399, 0, "EARTH from SSB"),
    (5, 399, "JUPITER BARYCENTER from EARTH"),
    (301, 0, "MOON from SSB"),
]
ABCORR = ["NONE", "LT", "LT+S", "CN+S", "XLT", "XCN+S"]


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--spk", required=True)
    parser.add_argument("--out", default="tests/golden/pointing.csv")
    args = parser.parse_args()

    spice.furnsh(args.spk)
    rows = []
    for et in EPOCHS:
        for target, observer, label in CASES:
            for abcorr in ABCORR:
                try:
                    position, _ = spice.spkezp(target, et, "J2000", abcorr, observer)
                except Exception:
                    continue
                direction = spice.vhat(position)
                rows.append((target, observer, et, abcorr, *direction))
    spice.kclear()

    out = Path(args.out)
    out.parent.mkdir(parents=True, exist_ok=True)
    with out.open("w", encoding="utf-8", newline="\n") as handle:
        handle.write("# CSPICE spkezp unit vector in J2000.\n")
        handle.write("target,observer,et,abcorr,x,y,z\n")
        for row in rows:
            handle.write(
                ",".join(repr(float(value)) if isinstance(value, float) else str(value) for value in row)
                + "\n"
            )
    print(f"wrote {out} ({len(rows)} rows)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
