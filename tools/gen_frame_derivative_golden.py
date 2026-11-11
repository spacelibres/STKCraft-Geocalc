#!/usr/bin/env python3
"""Generate golden data for frame rotations and their derivatives.

The oracle is CSPICE `sxform`, which returns the 6x6 state transformation
whose upper-left block is `pxform` and whose lower-left block is its time
derivative. This exercises the frame graph and the analytic body
orientation rates (`BODY<id>_POLE_RA`, `_POLE_DEC`, `_PM`, `_NUT_PREC_*`),
not just the representation algebra.

Offline tooling only; needs `spiceypy`.

Usage:
    python tools/gen_frame_derivative_golden.py --pck target/kernels/pck00011.tpc
"""

from __future__ import annotations

import argparse

import spiceypy as spice

CASES = [
    ("j2000_moon", "J2000", "IAU_MOON", 0.0),
    ("moon_j2000", "IAU_MOON", "J2000", 0.0),
    ("j2000_moon_past", "J2000", "IAU_MOON", -5.0e8),
    ("mars_j2000", "IAU_MARS", "J2000", 3.0e8),
    ("mars_j2000_zero", "IAU_MARS", "J2000", 0.0),
    ("mars_j2000_1e8", "IAU_MARS", "J2000", 1.0e8),
    ("mars_j2000_2e8", "IAU_MARS", "J2000", 2.0e8),
    ("mars_j2000_neg", "IAU_MARS", "J2000", -3.0e8),
    ("j2000_mars", "J2000", "IAU_MARS", 3.0e8),
    ("earth_mars", "IAU_EARTH", "IAU_MARS", 3.0e8),
    ("moon_earth", "IAU_MOON", "IAU_EARTH", 0.0),
    ("j2000_eclipj2000", "J2000", "ECLIPJ2000", 0.0),
    ("eclipj2000_j2000", "ECLIPJ2000", "J2000", 1.0e8),
    ("moon_self", "IAU_MOON", "IAU_MOON", 7.0e7),
]

HEADER = (
    "case,frame1,frame2,et,"
    "m00,m01,m02,m10,m11,m12,m20,m21,m22,"
    "d00,d01,d02,d10,d11,d12,d20,d21,d22"
)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--pck", default="target/kernels/pck00011.tpc")
    parser.add_argument("--out", default="tests/golden/frame_derivative.csv")
    args = parser.parse_args()

    spice.furnsh(args.pck)

    lines = ["# CSPICE sxform rotation and derivative by frame pair.", HEADER]
    for label, frame1, frame2, et in CASES:
        transform = spice.sxform(frame1, frame2, et)
        # Rows 3..5 hold the derivative in columns 0..2 and the rotation again
        # in columns 3..5, so only the lower-left block is the derivative.
        rotation = [repr(float(transform[row][column])) for row in range(3) for column in range(3)]
        derivative = [repr(float(transform[3 + row][column])) for row in range(3) for column in range(3)]
        lines.append(
            ",".join([label, frame1, frame2, repr(float(et)), *rotation, *derivative])
        )

    with open(args.out, "w", encoding="utf-8", newline="\n") as handle:
        handle.write("\n".join(lines))
        handle.write("\n")
    print(f"wrote {len(CASES)} cases to {args.out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
