#!/usr/bin/env python3
"""Generate golden data for the orientation representations.

The rotation and its derivative come from CSPICE `sxform`, quantised to a
CSV. The oracles are CSPICE `m2eul` (all twelve valid axis triples) and
`m2q`, plus native XF2RAV angular velocity, RAXISA axis/angle and XF2EUL
Euler-angle derivatives. No finite-difference oracle is used.

Keeping the matrices in the file rather than the frame names means the
Rust side tests the representation algebra alone, with no dependence on
its own frame graph.

Offline tooling only; needs `spiceypy`.

Usage:
    python tools/gen_orientation_golden.py --pck target/kernels/pck00011.tpc
"""

from __future__ import annotations

import argparse
import itertools
import math
from pathlib import Path

import spiceypy as spice

CASES = [
    ("saturn_enceladus_0", "IAU_SATURN", "IAU_ENCELADUS", 0.0),
    ("moon_earth_0", "IAU_MOON", "IAU_EARTH", 0.0),
    ("saturn_enceladus_late", "IAU_SATURN", "IAU_ENCELADUS", 5.0e8),
    ("j2000_moon", "J2000", "IAU_MOON", 0.0),
    ("earth_j2000", "IAU_EARTH", "J2000", -5.0e8),
]

AXES = ["X", "Y", "Z"]
AXIS_NUMBER = {"X": 1, "Y": 2, "Z": 3}


def vee(matrix):
    return [float(matrix[2][1]), float(matrix[0][2]), float(matrix[1][0])]


def omega(frame_one, rotation, derivative):
    transform = [[0.0]*6 for _ in range(6)]
    for row in range(3):
        for column in range(3):
            transform[row][column] = rotation[row][column]
            transform[row+3][column] = derivative[row][column]
            transform[row+3][column+3] = rotation[row][column]
    _, vector = spice.xf2rav(transform)
    return vector if frame_one else spice.mxv(rotation, vector)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--pck", default="target/kernels/pck00011.tpc")
    parser.add_argument("--matrices", default="tests/golden/orientation_matrices.csv")
    parser.add_argument("--values", default="tests/golden/orientation_values.csv")
    args = parser.parse_args()

    spice.furnsh(args.pck)

    matrix_rows = []
    value_rows = []

    for label, frame1, frame2, et in CASES:
        transform = spice.sxform(frame1, frame2, et)
        rotation = [[float(transform[row][column]) for column in range(3)] for row in range(3)]
        derivative = [
            [float(transform[3 + row][column]) for column in range(3)] for row in range(3)
        ]
        matrix_rows.append((label, et, rotation, derivative))

        for axis1, axis2, axis3 in itertools.product(AXES, repeat=3):
            if axis1 == axis2 or axis2 == axis3:
                continue
            angle3, angle2, angle1 = spice.m2eul(
                rotation, AXIS_NUMBER[axis3], AXIS_NUMBER[axis2], AXIS_NUMBER[axis1]
            )
            value_rows.append(
                (
                    label,
                    et,
                    "EULER",
                    axis1,
                    axis2,
                    axis3,
                    [float(angle3), float(angle2), float(angle1)],
                )
            )

        quaternion = [float(value) for value in spice.m2q(rotation)]
        value_rows.append((label, et, "QUAT", "", "", "", quaternion))
        axis, angle = spice.raxisa(rotation)
        value_rows.append((label, et, "AA", "", "", "", [angle, *axis]))
        value_rows.append((label, et, "OMEGA1", "", "", "", omega(True, rotation, derivative)))
        value_rows.append((label, et, "OMEGA2", "", "", "", omega(False, rotation, derivative)))

        # Direct native XF2EUL derivatives, including singular branches.
        for axis1, axis2, axis3 in itertools.product(AXES, repeat=3):
            if axis1 == axis2 or axis2 == axis3:
                continue
            values, _ = spice.xf2eul(transform, AXIS_NUMBER[axis3], AXIS_NUMBER[axis2], AXIS_NUMBER[axis1])
            value_rows.append((label, et, "EULER_RATE", axis1, axis2, axis3, values[3:]))

    spice.kclear()

    matrices_out = Path(args.matrices)
    matrices_out.parent.mkdir(parents=True, exist_ok=True)
    with matrices_out.open("w", encoding="utf-8", newline="\n") as handle:
        handle.write("# CSPICE sxform rotation and derivative.\n")
        handle.write(
            "case,et,"
            + ",".join(f"m{row}{column}" for row in range(3) for column in range(3))
            + ","
            + ",".join(f"d{row}{column}" for row in range(3) for column in range(3))
            + "\n"
        )
        for label, et, rotation, derivative in matrix_rows:
            numbers = [rotation[r][c] for r in range(3) for c in range(3)] + [
                derivative[r][c] for r in range(3) for c in range(3)
            ]
            handle.write(f"{label},{et!r}," + ",".join(repr(v) for v in numbers) + "\n")

    values_out = Path(args.values)
    with values_out.open("w", encoding="utf-8", newline="\n") as handle:
        handle.write(
            "# Orientation oracles: EULER (angle3, angle2, angle1), QUAT (q0..q3),\n"
            "# AA (angle, axis), OMEGA1/OMEGA2 (rad/s), EULER_RATE (rad/s).\n"
        )
        handle.write("case,et,kind,axis1,axis2,axis3,v1,v2,v3,v4\n")
        for label, et, kind, a1, a2, a3, values in value_rows:
            padded = list(values) + [0.0] * (4 - len(values))
            text = ",".join(repr(float(value)) for value in padded[:4])
            handle.write(f"{label},{et!r},{kind},{a1},{a2},{a3},{text}\n")

    print(f"wrote {matrices_out} ({len(matrix_rows)}) and {values_out} ({len(value_rows)})")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
