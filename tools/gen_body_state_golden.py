#!/usr/bin/env python3
"""Generate IAU body-fixed state-rotation golden vectors with CSPICE.

Calls `sxform('J2000', 'IAU_<body>', et)` and stores the rotation matrix
and its derivative in a committed CSV. `tests/body_state_golden.rs` loads
the same PCK and compares our analytic `Pck::body_state_rotation`.

The upper-right 3x3 block of an `sxform` matrix is the time derivative of
the rotation, so the CSV is an independent oracle for the analytic
derivative, including the nutation terms.

Offline tooling only; needs `spiceypy`.

Usage:
    python tools/gen_body_state_golden.py
"""

from __future__ import annotations

import argparse
from pathlib import Path

import spiceypy as spice

BODIES = [10, 199, 299, 301, 399, 499, 599, 699, 799, 899, 999]

EPOCHS = [-5.0e8, 0.0, 5.0e8]


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--out", default="tests/golden/body_state_rotations.csv")
    parser.add_argument("--pck", default="target/kernels/pck00011.tpc")
    args = parser.parse_args()

    spice.furnsh(args.pck)

    rows = []
    for body in BODIES:
        frame = f"IAU_{spice.bodc2n(body)}"
        for et in EPOCHS:
            matrix = spice.sxform("J2000", frame, et)
            # An `sxform` matrix is
            #
            #     [ R   0 ]
            #     [ dR  R ]
            #
            # so the derivative occupies rows 3..6 and columns 0..3.
            rotation = [matrix[row][column] for row in range(3) for column in range(3)]
            derivative = [matrix[row][column] for row in range(3, 6) for column in range(3)]
            rows.append((et, body, frame, rotation + derivative))

    spice.kclear()

    out = Path(args.out)
    out.parent.mkdir(parents=True, exist_ok=True)
    header = "et,body,frame," + ",".join(
        [f"m{row}{column}" for row in range(3) for column in range(3)]
        + [f"dm{row}{column}" for row in range(3) for column in range(3)]
    )
    with out.open("w", encoding="utf-8", newline="\n") as handle:
        handle.write("# CSPICE sxform('J2000', 'IAU_<body>', et) golden vectors.\n")
        handle.write("# position_fixed = m * position_j2000\n")
        handle.write("# velocity_fixed = m * velocity_j2000 + dm * position_j2000\n")
        handle.write(header + "\n")
        for et, body, frame, values in rows:
            numbers = ",".join(repr(float(value)) for value in values)
            handle.write(f"{et!r},{body},{frame},{numbers}\n")

    print(f"wrote {out} ({len(rows)} rotations)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
