#!/usr/bin/env python3
"""Generate surface-intercept and illumination-angle golden vectors.

Calls `sincpt` (ray from the observer toward the target center) and
`ilumin` on the IAU ellipsoid. `tests/surface_golden.rs` compares the
Rust implementation.

Offline tooling only; needs `spiceypy`.

Usage:
    python tools/gen_illumination_golden.py --spk target/kernels/de442s.bsp \
        --pck target/kernels/pck00011.tpc
"""

from __future__ import annotations

import argparse
from pathlib import Path

import spiceypy as spice

EPOCHS = [-5.0e8, 0.0, 5.0e8]
CASES = [(301, 399), (399, 301)]


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--spk", required=True)
    parser.add_argument("--pck", required=True)
    parser.add_argument("--intercept-out", default="tests/golden/surface_intercepts.csv")
    parser.add_argument("--illumination-out", default="tests/golden/illumination.csv")
    args = parser.parse_args()

    spice.furnsh(args.spk)
    spice.furnsh(args.pck)

    intercepts = []
    illumination = []
    for et in EPOCHS:
        for target, observer in CASES:
            fixref = f"IAU_{spice.bodc2n(target)}"
            tpos, _lt = spice.spkezp(target, et, "J2000", "NONE", observer)
            dvec = [float(component) for component in tpos]
            spoint, _trgepc, srfvec = spice.sincpt(
                "ELLIPSOID",
                str(target),
                et,
                fixref,
                "NONE",
                str(observer),
                "J2000",
                dvec,
            )
            alt = spice.vnorm(srfvec)
            intercepts.append((target, observer, et, spoint, alt))

            near, _trgepc, _srfvec = spice.subpnt(
                "NEAR POINT: ELLIPSOID",
                str(target),
                et,
                fixref,
                "NONE",
                str(observer),
            )
            _trgepc, _srfvec, phase, solar, emissn = spice.ilumin(
                "ELLIPSOID",
                str(target),
                et,
                fixref,
                "NONE",
                str(observer),
                near,
            )
            illumination.append((target, observer, et, near, phase, solar, emissn))

    spice.kclear()

    intercept_out = Path(args.intercept_out)
    intercept_out.parent.mkdir(parents=True, exist_ok=True)
    with intercept_out.open("w", encoding="utf-8", newline="\n") as handle:
        handle.write("# CSPICE sincpt along the observer-to-center ray.\n")
        handle.write("target,observer,et,x,y,z,altitude\n")
        for target, observer, et, spoint, alt in intercepts:
            values = ",".join(repr(float(component)) for component in spoint)
            handle.write(f"{target},{observer},{et!r},{values},{float(alt)!r}\n")

    illumination_out = Path(args.illumination_out)
    with illumination_out.open("w", encoding="utf-8", newline="\n") as handle:
        handle.write("# CSPICE ilumin at the near-side sub-observer point.\n")
        handle.write("target,observer,et,px,py,pz,phase,solar,emission\n")
        for target, observer, et, near, phase, solar, emissn in illumination:
            point = ",".join(repr(float(component)) for component in near)
            handle.write(
                f"{target},{observer},{et!r},{point},{float(phase)!r},{float(solar)!r},{float(emissn)!r}\n"
            )

    print(
        f"wrote {intercept_out} ({len(intercepts)} rows) and "
        f"{illumination_out} ({len(illumination)} rows)"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
