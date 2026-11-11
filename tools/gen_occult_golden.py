#!/usr/bin/env python3
"""Generate occultation golden data from CSPICE.

Three geometries exercise all four state codes:
  - Moon occulting the Sun from Earth: PARTIAL
  - Earth occulting the Sun from the Moon: FULL
  - Venus transiting the Sun from Earth: ANNULAR

`occult` supplies the state codes; `gfoclt` supplies search windows.

Usage:
    python tools/gen_occult_golden.py \
        --spk target/kernels/de440s.bsp \
        --pck target/kernels/pck00011.tpc
"""

from __future__ import annotations

import argparse
from pathlib import Path

import spiceypy as spice

DAY = 86400.0
STEP = 600.0
NINTVLS = 20000
ABCORR = "LT"

FAMILIES = [
    (
        "moon_sun",
        "MOON",
        "IAU_MOON",
        "SUN",
        "IAU_SUN",
        "EARTH",
        0.0,
        1200.0 * DAY,
    ),
    (
        "earth_sun",
        "EARTH",
        "IAU_EARTH",
        "SUN",
        "IAU_SUN",
        "MOON",
        0.0,
        2.0e6,
    ),
    (
        "venus_sun",
        "VENUS",
        "IAU_VENUS",
        "SUN",
        "IAU_SUN",
        "EARTH",
        1.39e8,
        1.41e8,
    ),
]


def cell(start: float, end: float) -> object:
    window = spice.cell_double(NINTVLS)
    spice.wninsd(start, end, window)
    return window


def run(
    front: str,
    fframe: str,
    back: str,
    bframe: str,
    observer: str,
    start: float,
    end: float,
    kind: str,
) -> list[tuple[float, float]]:
    result = spice.cell_double(NINTVLS)
    spice.gfoclt(
        kind,
        front,
        "ELLIPSOID",
        fframe,
        back,
        "ELLIPSOID",
        bframe,
        ABCORR,
        observer,
        STEP,
        cell(start, end),
        result,
    )
    return [spice.wnfetd(result, index) for index in range(spice.wncard(result))]


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--spk", required=True)
    parser.add_argument("--pck", required=True)
    parser.add_argument("--out-dir", default="tests/golden")
    args = parser.parse_args()

    spice.furnsh(args.spk)
    spice.furnsh(args.pck)

    events = []
    states = []
    for name, front, fframe, back, bframe, observer, start, end in FAMILIES:
        any_windows = run(front, fframe, back, bframe, observer, start, end, "ANY")
        for left, right in any_windows:
            for et in (left - 2.0 * STEP, 0.5 * (left + right), right + 2.0 * STEP):
                code = spice.occult(
                    front,
                    "ELLIPSOID",
                    fframe,
                    back,
                    "ELLIPSOID",
                    bframe,
                    ABCORR,
                    observer,
                    et,
                )
                states.append((name, front, back, observer, et, int(code)))
        for kind in ("ANY", "FULL", "PARTIAL", "ANNULAR"):
            for left, right in run(front, fframe, back, bframe, observer, start, end, kind):
                events.append((name, kind, left, right))

    spice.kclear()

    out_dir = Path(args.out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)

    with (out_dir / "occult_states.csv").open("w", encoding="utf-8", newline="\n") as handle:
        handle.write("# CSPICE occult(front, ELLIPSOID, fframe, back, ELLIPSOID,\n")
        handle.write(f"#   bframe, {ABCORR}, observer, et).\n")
        handle.write("family,front,back,observer,et,code\n")
        for row in states:
            handle.write(
                f"{row[0]},{row[1]},{row[2]},{row[3]},{float(row[4])!r},{row[5]}\n"
            )

    with (out_dir / "occult_events.csv").open("w", encoding="utf-8", newline="\n") as handle:
        handle.write("# CSPICE gfoclt windows.\n")
        handle.write("family,type,start,end\n")
        for family, kind, left, right in events:
            handle.write(f"{family},{kind},{float(left)!r},{float(right)!r}\n")

    print(f"wrote {len(states)} states and {len(events)} windows")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
