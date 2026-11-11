#!/usr/bin/env python3
"""Generate sub-observer and sub-solar point golden vectors with CSPICE.

Calls `subpnt`/`subslr` with `NEAR POINT: ELLIPSOID` in the IAU
body-fixed frame and writes the point and altitude (the norm of the
surface vector). `tests/surface_golden.rs` compares the Rust
implementation.

Offline tooling only; needs `spiceypy`.

Usage:
    python tools/gen_surface_golden.py --spk target/kernels/de442s.bsp \
        --pck target/kernels/pck00011.tpc
"""

from __future__ import annotations

import argparse
from pathlib import Path

import spiceypy as spice

EPOCHS = [-5.0e8, 0.0, 5.0e8]
OBSERVER_CASES = [(301, 399), (399, 301), (301, 10)]
SOLAR_CASES = [301, 399]


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--spk", required=True)
    parser.add_argument("--pck", required=True)
    parser.add_argument("--out", default="tests/golden/surface_points.csv")
    args = parser.parse_args()

    spice.furnsh(args.spk)
    spice.furnsh(args.pck)

    rows = []
    for et in EPOCHS:
        for target, observer in OBSERVER_CASES:
            spoint, _trgepc, srfvec = spice.subpnt(
                "NEAR POINT: ELLIPSOID",
                str(target),
                et,
                f"IAU_{spice.bodc2n(target)}",
                "NONE",
                str(observer),
            )
            alt = spice.vnorm(srfvec)
            rows.append(("observer", target, observer, et, spoint, alt))

        for target in SOLAR_CASES:
            spoint, _trgepc, srfvec = spice.subslr(
                "NEAR POINT: ELLIPSOID",
                str(target),
                et,
                f"IAU_{spice.bodc2n(target)}",
                "NONE",
                "SUN",
            )
            alt = spice.vnorm(srfvec)
            rows.append(("solar", target, 10, et, spoint, alt))

    spice.kclear()

    out = Path(args.out)
    out.parent.mkdir(parents=True, exist_ok=True)
    with out.open("w", encoding="utf-8", newline="\n") as handle:
        handle.write("# CSPICE subpnt/subslr NEAR POINT: ELLIPSOID in IAU frames.\n")
        handle.write("# Point in km; altitude is |srfvec| in km.\n")
        handle.write("kind,target,observer,et,x,y,z,altitude\n")
        for kind, target, observer, et, spoint, alt in rows:
            values = ",".join(repr(float(component)) for component in spoint)
            handle.write(
                f"{kind},{target},{observer},{et!r},{values},{float(alt)!r}\n"
            )

    print(f"wrote {out} ({len(rows)} rows)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
