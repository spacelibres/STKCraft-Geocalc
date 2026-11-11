#!/usr/bin/env python3
"""Generate sub-point golden vectors for both definitions.

CSPICE `subpnt`/`subslr` accept `'NEAR POINT/ELLIPSOID'` and
`'INTERCEPT/ELLIPSOID'`. The two agree for a spheroid and differ for a
triaxial ellipsoid, so the Earth is the interesting target here and the
Moon is kept as the degenerate control.

Offline tooling only; needs `spiceypy`.

Usage:
    python tools/gen_subpoint_method_golden.py \
        --spk target/kernels/de440s.bsp --pck target/kernels/pck00011.tpc
"""

from __future__ import annotations

import argparse
from pathlib import Path

import spiceypy as spice

EPOCHS = [-5.0e8, 0.0, 5.0e8]
METHODS = ["NEAR POINT/ELLIPSOID", "INTERCEPT/ELLIPSOID"]

# (target, observer, abcorr) for sub-observer points
OBSERVER_CASES = [
    (399, 301, "NONE"),
    (399, 10, "NONE"),
    (399, 301, "LT+S"),
    (301, 399, "NONE"),
]

# (target, abcorr) for sub-solar points; the Sun is the illuminator.
SOLAR_CASES = [(399, "NONE"), (399, "LT+S"), (301, "NONE")]


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--spk", default="target/kernels/de440s.bsp")
    parser.add_argument("--pck", default="target/kernels/pck00011.tpc")
    parser.add_argument("--out", default="tests/golden/subpoint_methods.csv")
    args = parser.parse_args()

    spice.furnsh(args.spk)
    spice.furnsh(args.pck)

    rows = []
    for target, observer, abcorr in OBSERVER_CASES:
        frame = f"IAU_{spice.bodc2n(target)}"
        for method in METHODS:
            for et in EPOCHS:
                point, _, srfvec = spice.subpnt(
                    method, str(target), et, frame, abcorr, str(observer)
                )
                rows.append(
                    (
                        "observer",
                        target,
                        observer,
                        et,
                        abcorr,
                        method,
                        [float(value) for value in point],
                        float(spice.vnorm(srfvec)),
                    )
                )

    for target, abcorr in SOLAR_CASES:
        frame = f"IAU_{spice.bodc2n(target)}"
        for method in METHODS:
            for et in EPOCHS:
                point, _, srfvec = spice.subslr(method, str(target), et, frame, abcorr, "SUN")
                rows.append(
                    (
                        "solar",
                        target,
                        10,
                        et,
                        abcorr,
                        method,
                        [float(value) for value in point],
                        float(spice.vnorm(srfvec)),
                    )
                )

    spice.kclear()

    out = Path(args.out)
    out.parent.mkdir(parents=True, exist_ok=True)
    with out.open("w", encoding="utf-8", newline="\n") as handle:
        handle.write(
            "# CSPICE subpnt/subslr with both ellipsoid definitions.\n"
            "# Point in the IAU body-fixed frame (km); altitude in km.\n"
        )
        handle.write("kind,target,observer,et,abcorr,method,x,y,z,altitude\n")
        for kind, target, observer, et, abcorr, method, point, altitude in rows:
            text = ",".join(repr(float(value)) for value in point)
            handle.write(
                f"{kind},{target},{observer},{et!r},{abcorr},{method},{text},{altitude!r}\n"
            )

    print(f"wrote {out} ({len(rows)} rows)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
