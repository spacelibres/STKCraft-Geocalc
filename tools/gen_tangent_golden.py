#!/usr/bin/env python3
"""Generate tangent-point golden data from CSPICE `tangpt`.

The target frame is the IAU body-fixed frame derived from the text PCK.
The direction vector is given in J2000. The SPK and PCK are not
committed; provide them with `--spk` and `--pck`.

Usage:
    python tools/gen_tangent_golden.py \
        --spk target/kernels/de440s.bsp \
        --pck target/kernels/pck00011.tpc
"""

from __future__ import annotations

import argparse
from pathlib import Path

import spiceypy as spice

TARGET = 301
OBSERVER = 399
FIXREF = "IAU_MOON"
ABCORR = ["NONE", "LT", "LT+S", "CN+S"]
EPOCHS = [-5.0e7, 0.0, 5.0e7]
DIRECTIONS = [
    # (label, azimuth offset from the centre ray in degrees, elevation)
    ("centre", 0.0, 0.0),
    ("limb", 0.35, 0.0),
    ("off-axis", 0.5, 25.0),
    ("fixed", None, None),
]


def direction_to_target(et: float, abcorr: str) -> list[float]:
    position, _ = spice.spkezp(TARGET, et, "J2000", abcorr, OBSERVER)
    length = sum(value * value for value in position) ** 0.5
    return [value / length for value in position]


def offset_direction(center: list[float], azimuth: float, elevation: float) -> list[float]:
    import math

    # Build a local frame around the centre ray and rotate the ray.
    seed = [0.0, 0.0, 1.0]
    if abs(center[2]) > 0.9:
        seed = [0.0, 1.0, 0.0]
    first = [
        center[1] * seed[2] - center[2] * seed[1],
        center[2] * seed[0] - center[0] * seed[2],
        center[0] * seed[1] - center[1] * seed[0],
    ]
    norm = sum(value * value for value in first) ** 0.5
    first = [value / norm for value in first]
    second = [
        center[1] * first[2] - center[2] * first[1],
        center[2] * first[0] - center[0] * first[2],
        center[0] * first[1] - center[1] * first[0],
    ]
    a = math.radians(azimuth)
    e = math.radians(elevation)
    result = []
    for axis in range(3):
        result.append(
            math.cos(a) * center[axis]
            + math.sin(a) * (math.cos(e) * first[axis] + math.sin(e) * second[axis])
        )
    norm = sum(value * value for value in result) ** 0.5
    return [value / norm for value in result]


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--spk", required=True)
    parser.add_argument("--pck", required=True)
    parser.add_argument("--out", default="tests/golden/tangent.csv")
    args = parser.parse_args()

    spice.furnsh(args.spk)
    spice.furnsh(args.pck)

    rows = []
    for et in EPOCHS:
        for abcorr in ABCORR:
            center = direction_to_target(et, abcorr)
            for label, azimuth, elevation in DIRECTIONS:
                if label == "fixed":
                    direction = [0.3, -0.4, 0.8660254037844386]
                else:
                    direction = offset_direction(center, azimuth, elevation)
                try:
                    tanpt, alt, rng, srfpt, trgepc, _ = spice.tangpt(
                        "ELLIPSOID",
                        str(TARGET),
                        et,
                        FIXREF,
                        abcorr,
                        "TANGENT POINT",
                        str(OBSERVER),
                        "J2000",
                        direction,
                    )
                except Exception:
                    continue
                rows.append(
                    (
                        TARGET,
                        OBSERVER,
                        float(et),
                        abcorr,
                        *(float(value) for value in direction),
                        *(float(value) for value in tanpt),
                        float(alt),
                        float(rng),
                        *(float(value) for value in srfpt),
                        float(trgepc),
                    )
                )

    spice.kclear()

    out = Path(args.out)
    out.parent.mkdir(parents=True, exist_ok=True)
    with out.open("w", encoding="utf-8", newline="\n") as handle:
        handle.write("# CSPICE tangpt(ELLIPSOID, 301, et, IAU_MOON, abcorr,\n")
        handle.write("#   'TANGENT POINT', 399, 'J2000', dvec).\n")
        handle.write(
            "target,observer,et,abcorr,dx,dy,dz,x,y,z,alt,range,sx,sy,sz,trgepc\n"
        )
        for row in rows:
            handle.write(",".join(repr(value) if isinstance(value, float) else str(value) for value in row))
            handle.write("\n")

    print(f"wrote {out} ({len(rows)} rows)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
