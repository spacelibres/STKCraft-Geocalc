#!/usr/bin/env python3
"""Generate instrument-FOV golden data from CSPICE.

Uses the synthetic frame/FOV kernel `tests/golden/fov.tf`. Covers
`fovray` (ray booleans), `fovtrg` with POINT targets, and the
`gftfov`/`gfrfov` interval searches.

Usage:
    python tools/gen_fov_golden.py --spk target/kernels/de440s.bsp
"""

from __future__ import annotations

import argparse
from pathlib import Path

import spiceypy as spice

FK = "tests/golden/fov.tf"
DAY = 86400.0
STEP = 3600.0
NINTVLS = 4000

RAY_CASES = {
    12345: [
        (0.0, 0.0, 1.0),
        (0.05, 0.0, 1.0),
        (0.15, 0.0, 1.0),
        (-0.09, 0.0, 1.0),
        (0.0, 0.2, 1.0),
    ],
    12346: [
        (0.0, 0.0, 1.0),
        (0.15, 0.0, 1.0),
        (0.0, 0.05, 1.0),
        (0.25, 0.0, 1.0),
        (0.0, 0.15, 1.0),
    ],
    12347: [
        (0.1, 0.05, 1.0),
        (0.19, 0.09, 1.0),
        (0.25, 0.0, 1.0),
        (-0.19, 0.09, 1.0),
        (0.0, 0.2, 1.0),
    ],
    12348: [
        (0.0, 0.0, 1.0),
        (0.05, 0.02, 1.0),
        (0.15, 0.05, 1.0),
        (0.0, 0.2, 1.0),
    ],
    12349: [
        (0.0, 0.0, 1.0),
        (0.1, 0.05, 1.0),
        (0.25, 0.0, 1.0),
        (-0.19, 0.09, 1.0),
    ],
}

TARGET_CASES = [
    (12350, "MOON", 301),
    (12351, "MOON", 301),
]

EPOCHS = [-5.0e8, 0.0, 5.0e8]
TARGET_EPOCHS = [0.0 + index * 3600.0 for index in range(0, 24 * 10, 6)]


def cell(start: float, end: float) -> object:
    window = spice.cell_double(NINTVLS)
    spice.wninsd(start, end, window)
    return window


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--spk", required=True)
    parser.add_argument("--out-dir", default="tests/golden")
    args = parser.parse_args()

    spice.furnsh(args.spk)
    spice.furnsh(FK)

    ray_rows = []
    for instrument, directions in RAY_CASES.items():
        for et in EPOCHS:
            for direction in directions:
                visible = spice.fovray(
                    str(instrument), direction, "J2000", "NONE", "EARTH", et
                )
                ray_rows.append((instrument, et, *direction, bool(visible)))

    target_rows = []
    for instrument, target, _ in TARGET_CASES:
        for et in TARGET_EPOCHS:
            visible = spice.fovtrg(
                str(instrument), target, "POINT", "J2000", "NONE", "EARTH", et
            )
            target_rows.append((instrument, target, et, bool(visible)))

    event_rows = []
    for instrument, target, _ in TARGET_CASES:
        result = spice.cell_double(NINTVLS)
        spice.gftfov(
            str(instrument),
            target,
            "POINT",
            "J2000",
            "NONE",
            "EARTH",
            STEP,
            cell(0.0, 30.0 * DAY),
            result,
        )
        for index in range(spice.wncard(result)):
            left, right = spice.wnfetd(result, index)
            event_rows.append(("target", instrument, left, right))

    for instrument, direction, start, end in [
        (12345, (0.0, 0.0, 1.0), 0.0, 30.0 * DAY),
        (12345, (1.0, 0.0, 0.0), 0.0, 30.0 * DAY),
    ]:
        result = spice.cell_double(NINTVLS)
        spice.gfrfov(
            str(instrument),
            direction,
            "J2000",
            "NONE",
            "EARTH",
            STEP,
            cell(start, end),
            result,
        )
        for index in range(spice.wncard(result)):
            left, right = spice.wnfetd(result, index)
            event_rows.append(("ray", instrument, left, right))

    spice.kclear()

    out_dir = Path(args.out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)

    with (out_dir / "fov_ray.csv").open("w", encoding="utf-8", newline="\n") as handle:
        handle.write("# CSPICE fovray(inst, dvec, 'J2000', 'NONE', 'EARTH', et).\n")
        handle.write("instrument,et,dx,dy,dz,visible\n")
        for row in ray_rows:
            handle.write(
                f"{row[0]},{float(row[1])!r},{float(row[2])!r},{float(row[3])!r},"
                f"{float(row[4])!r},{int(row[5])}\n"
            )

    with (out_dir / "fov_target.csv").open("w", encoding="utf-8", newline="\n") as handle:
        handle.write("# CSPICE fovtrg(inst, target, 'POINT', 'J2000', 'NONE', 'EARTH', et).\n")
        handle.write("instrument,target,et,visible\n")
        for instrument, target, et, visible in target_rows:
            handle.write(f"{instrument},{target},{float(et)!r},{int(visible)}\n")

    with (out_dir / "fov_events.csv").open("w", encoding="utf-8", newline="\n") as handle:
        handle.write("# CSPICE gftfov/gfrfov result windows.\n")
        handle.write("family,instrument,start,end\n")
        for family, instrument, left, right in event_rows:
            handle.write(f"{family},{instrument},{float(left)!r},{float(right)!r}\n")

    print(
        f"wrote {len(ray_rows)} rays, {len(target_rows)} targets, "
        f"{len(event_rows)} windows"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
