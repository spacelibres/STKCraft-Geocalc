#!/usr/bin/env python3
"""Generate `fovtrg` ELLIPSOID golden visibility with CSPICE.

The committed `fov.tf` instruments point along fixed J2000-ish axes, so no
real body ever falls inside them. This script therefore synthesises a
frame kernel whose instrument frames point at the target, then sweeps the
boresight offset and the FOV half-angle across the target's apparent
angular radius. That produces cases well outside, well inside, and
straddling the visibility boundary for all four FOV shapes.

Outputs:

* `tests/golden/fov_ellipsoid.tf`   synthetic IK/FK, committed
* `tests/golden/fov_ellipsoid.csv`  instrument, geometry, expected flag

Offline tooling only; needs `spiceypy`.

Usage:
    python tools/gen_fov_ellipsoid_golden.py \
        --spk target/kernels/de440s.bsp --pck target/kernels/pck00011.tpc
"""

from __future__ import annotations

import argparse
import math
from pathlib import Path

import spiceypy as spice

SHAPES = ["CIRCLE", "ELLIPSE", "RECTANGLE", "POLYGON"]

# (label, target id, observer id, et, abcorr)
CASES = [
    ("moon_from_earth", 301, 399, 0.0, "NONE"),
    ("earth_from_moon", 399, 301, 0.0, "NONE"),
    ("moon_from_earth_lt", 301, 399, 0.0, "LT+S"),
    ("moon_from_earth_epoch", 301, 399, 5.0e8, "NONE"),
]

# Boresight offsets, as a fraction of the apparent angular radius. The
# larger values place the whole disk outside a narrow FOV, which is where
# the visibility flag turns false.
OFFSETS = [0.0, 0.4, 0.9, 1.1, 1.5, 2.5, 4.0]
# FOV half-angle, as a fraction of the apparent angular radius.
FACTORS = [0.3, 0.8, 0.98, 1.02, 1.2, 2.0]


def unit(vector):
    length = math.sqrt(sum(value * value for value in vector))
    return [value / length for value in vector]


def cross(a, b):
    return [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]


def basis(direction):
    """Rows of a rotation taking `direction` to +Z."""
    d = unit(direction)
    helper = [0.0, 0.0, 1.0] if abs(d[2]) < 0.9 else [1.0, 0.0, 0.0]
    e1 = unit(cross(helper, d))
    e2 = cross(d, e1)
    return [e1, e2, d]


def tilted(direction, angle, phase):
    rows = basis(direction)
    e1, e2 = rows[0], rows[1]
    offset = [
        math.tan(angle) * (math.cos(phase) * e1[i] + math.sin(phase) * e2[i])
        for i in range(3)
    ]
    return unit([direction[i] + offset[i] for i in range(3)])


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--spk", default="target/kernels/de440s.bsp")
    parser.add_argument("--pck", default="target/kernels/pck00011.tpc")
    parser.add_argument("--kernel", default="tests/golden/fov_ellipsoid.tf")
    parser.add_argument("--out", default="tests/golden/fov_ellipsoid.csv")
    args = parser.parse_args()

    spice.furnsh(args.spk)
    spice.furnsh(args.pck)

    frame_blocks: list[str] = []
    instrument_blocks: list[str] = []
    pending: list[tuple] = []

    frame_id = 1410000
    instrument_id = 1420000

    for label, target, observer, et, abcorr in CASES:
        name = spice.bodc2n(target)
        radii = spice.bodvrd(name, "RADII", 3)[1]
        max_radius = max(radii)
        position, _ = spice.spkezp(target, et, "J2000", abcorr, observer)
        distance = math.sqrt(sum(value * value for value in position))
        alpha = float(math.asin(min(max_radius / distance, 0.999)))
        direction = unit(position)

        for offset_index, offset in enumerate(OFFSETS):
            frame_id += 1
            frame_name = f"ELLFRAME{frame_id}"
            boresight = tilted(direction, offset * alpha, 0.7 + 1.1 * offset_index)
            rows_matrix = basis(boresight)
            # NAIF reads `TKFRAME_..._MATRIX` so that the values written in
            # order form the transpose of the J2000 -> frame matrix; writing
            # the rows of that matrix in order therefore needs the values
            # taken row by row.
            flat = [float(rows_matrix[row][column]) for row in range(3) for column in range(3)]
            # SPICE truncates long text-kernel records, so keep every
            # matrix row on its own line.
            matrix_text = ",\n                             ".join(
                ", ".join(repr(value) for value in flat[index : index + 3])
                for index in (0, 3, 6)
            )
            frame_blocks.append(
                f"""FRAME_{frame_id}_NAME     = '{frame_name}'
FRAME_ELLFRAME{frame_id}     = {frame_id}
FRAME_{frame_id}_CLASS    = 4
FRAME_{frame_id}_CLASS_ID = {frame_id}
FRAME_{frame_id}_CENTER   = 399
TKFRAME_{frame_id}_RELATIVE = 'J2000'
TKFRAME_{frame_id}_SPEC     = 'MATRIX'
TKFRAME_{frame_id}_MATRIX   = ( {matrix_text} )
"""
            )

            for factor in FACTORS:
                half = factor * alpha
                if half <= 0.0 or half >= math.pi * 0.5:
                    continue
                for shape in SHAPES:
                    instrument_id += 1
                    prefix = f"INS{instrument_id}_"
                    if shape == "POLYGON":
                        instrument_blocks.append(
                            f"""{prefix}FOV_CLASS_SPEC = 'CORNERS'
{prefix}FOV_SHAPE      = 'POLYGON'
{prefix}FOV_FRAME      = '{frame_name}'
{prefix}BORESIGHT      = ( 0.0, 0.0, 1.0 )
{prefix}FOV_BOUNDARY   = ( {math.tan(half)!r}, {math.tan(half * 0.6)!r}, 1.0,
                          {-math.tan(half)!r}, {math.tan(half * 0.6)!r}, 1.0,
                          {-math.tan(half)!r}, {-math.tan(half * 0.6)!r}, 1.0,
                          {math.tan(half)!r}, {-math.tan(half * 0.6)!r}, 1.0 )
"""
                        )
                    else:
                        cross_angle = half if shape == "CIRCLE" else half * 0.6
                        instrument_blocks.append(
                            f"""{prefix}FOV_CLASS_SPEC   = 'ANGLES'
{prefix}FOV_SHAPE        = '{shape}'
{prefix}FOV_FRAME        = '{frame_name}'
{prefix}FOV_ANGLE_UNITS  = 'RADIANS'
{prefix}BORESIGHT        = ( 0.0, 0.0, 1.0 )
{prefix}FOV_REF_VECTOR   = ( 1.0, 0.0, 0.0 )
{prefix}FOV_REF_ANGLE    = {half!r}
{prefix}FOV_CROSS_ANGLE  = {cross_angle!r}
"""
                        )
                    pending.append(
                        (
                            instrument_id,
                            et,
                            observer,
                            target,
                            abcorr,
                            shape,
                            f"IAU_{name}",
                        )
                    )

    kernel = Path(args.kernel)
    kernel.parent.mkdir(parents=True, exist_ok=True)
    kernel.write_text(
        "KPL/FK\n\n"
        "Synthetic FOV kernel for STKCraft-geocalc ellipsoid visibility tests.\n"
        "Generated by tools/gen_fov_ellipsoid_golden.py; do not edit by hand.\n\n"
        "\\begindata\n\n" + "\n".join(frame_blocks + instrument_blocks) + "\\begintext\n",
        encoding="utf-8",
        newline="\n",
    )

    spice.furnsh(str(kernel))

    rows: list[tuple] = []
    for (
        instrument,
        et,
        observer,
        target,
        abcorr,
        shape,
        target_frame,
    ) in pending:
        visible = spice.fovtrg(
            str(instrument),
            str(target),
            "ELLIPSOID",
            target_frame,
            abcorr,
            str(observer),
            et,
        )
        rows.append(
            (instrument, et, observer, target, abcorr, shape, float(visible))
        )

    spice.kclear()

    out = Path(args.out)
    with out.open("w", encoding="utf-8", newline="\n") as handle:
        handle.write(
            "# CSPICE fovtrg(ins, target, 'ELLIPSOID', 'IAU_<target>', abcorr, obs, et).\n"
        )
        handle.write("instrument,et,observer,target,abcorr,shape,visible\n")
        for row in rows:
            instrument, et, observer, target, abcorr, shape, visible = row
            handle.write(
                f"{instrument},{et!r},{observer},{target},{abcorr},{shape},{int(visible)}\n"
            )

    print(f"wrote {kernel} and {out} ({len(rows)} rows)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
