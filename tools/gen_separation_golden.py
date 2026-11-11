#!/usr/bin/env python3
"""Generate angular-separation and Geometry Finder event golden data.

Primitives: CSPICE `trgsep` for a grid of epochs, observers, target pairs
and aberration corrections, with `POINT` shapes and, in a separate file,
with every `POINT`/`SPHERE` combination.

Events: CSPICE `gfsep` with `LOCMIN`/`LOCMAX` for a few windows. `gfsep`
returns each extremum as a zero-width interval, so the event time is
exact enough to validate a time-domain search. A second event file covers
`SPHERE` shapes.

The SPK is not committed. Provide `--spk` or place `de440s.bsp` at
`target/kernels/` using `tools/fetch_spk.py`. `--pck` is required for the
`SPHERE` cases only.

Offline tooling only; needs `spiceypy`.

Usage:
    python tools/gen_separation_golden.py --spk target/kernels/de440s.bsp \
        --pck target/kernels/pck00011.tpc
"""

from __future__ import annotations

import argparse
from pathlib import Path

import spiceypy as spice

EPOCHS = [-1.0e9, 0.0, 1.0e9]
OBSERVERS = [0, 3, 399]
PAIRS = [(10, 301), (10, 399), (301, 399), (1, 4), (5, 6)]
ABCORR = ["NONE", "LT", "LT+S", "CN+S"]

DAY = 86400.0
EVENT_CASES = [
    # observer, target1, target2, abcorr, start, end
    (399, 10, 301, "NONE", 0.0, 50.0 * DAY),
    (399, 10, 301, "LT+S", 0.0, 50.0 * DAY),
    (0, 10, 399, "NONE", 0.0, 400.0 * DAY),
    (0, 301, 399, "LT", 0.0, 30.0 * DAY),
]
EVENT_KINDS = ["LOCMIN", "LOCMAX"]
EVENT_STEP = 3600.0

# (et, observer, target1, target2, shape1, shape2, abcorr)
#
# `556612040.0` is a total solar eclipse, where the apparent disks overlap
# and `trgsep` returns a negative separation.
SHAPE_CASES = [
    (0.0, 10, 301, 399, "SPHERE", "SPHERE", "LT+S"),
    (0.0, 10, 301, 399, "SPHERE", "POINT", "LT+S"),
    (0.0, 10, 301, 399, "POINT", "SPHERE", "LT+S"),
    (0.0, 10, 301, 399, "SPHERE", "SPHERE", "NONE"),
    (0.0, 399, 10, 301, "SPHERE", "SPHERE", "NONE"),
    (0.0, 399, 10, 301, "SPHERE", "SPHERE", "LT"),
    (0.0, 399, 10, 301, "SPHERE", "SPHERE", "CN+S"),
    (0.0, 0, 301, 399, "SPHERE", "SPHERE", "NONE"),
    (0.0, 0, 10, 301, "SPHERE", "POINT", "LT+S"),
    (-1.0e9, 399, 301, 10, "SPHERE", "SPHERE", "NONE"),
    (1.0e9, 399, 301, 10, "SPHERE", "SPHERE", "NONE"),
    (556612040.0, 399, 10, 301, "SPHERE", "SPHERE", "NONE"),
    (556612040.0, 399, 10, 301, "SPHERE", "SPHERE", "LT+S"),
    (556612040.0, 399, 10, 301, "POINT", "POINT", "NONE"),
]

# (observer, target1, target2, shape1, shape2, abcorr, start, end)
SHAPE_EVENT_CASES = [
    (399, 10, 301, "SPHERE", "SPHERE", "NONE", 0.0, 50.0 * DAY),
    (399, 10, 301, "SPHERE", "POINT", "NONE", 0.0, 50.0 * DAY),
    (
        399,
        10,
        301,
        "SPHERE",
        "SPHERE",
        "NONE",
        556512040.0,
        556712040.0,
    ),
]


def trgsep(et, observer, target1, target2, shape1, shape2, abcorr):
    return spice.trgsep(
        et,
        str(target1),
        shape1,
        "J2000",
        str(target2),
        shape2,
        "J2000",
        str(observer),
        abcorr,
    )


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--spk", required=True)
    parser.add_argument("--pck", default="target/kernels/pck00011.tpc")
    parser.add_argument("--separation-out", default="tests/golden/separation.csv")
    parser.add_argument("--events-out", default="tests/golden/separation_events.csv")
    parser.add_argument(
        "--shapes-out", default="tests/golden/separation_shapes.csv"
    )
    parser.add_argument(
        "--shape-events-out", default="tests/golden/separation_shape_events.csv"
    )
    args = parser.parse_args()

    spice.furnsh(args.spk)
    if Path(args.pck).exists():
        spice.furnsh(args.pck)

    separation_rows = []
    for et in EPOCHS:
        for observer in OBSERVERS:
            for target1, target2 in PAIRS:
                for abcorr in ABCORR:
                    try:
                        value = spice.trgsep(
                            et,
                            str(target1),
                            "POINT",
                            "J2000",
                            str(target2),
                            "POINT",
                            "J2000",
                            str(observer),
                            abcorr,
                        )
                    except Exception:
                        continue
                    separation_rows.append(
                        (et, observer, target1, target2, abcorr, value)
                    )

    separation_out = Path(args.separation_out)
    separation_out.parent.mkdir(parents=True, exist_ok=True)
    with separation_out.open("w", encoding="utf-8", newline="\n") as handle:
        handle.write(
            "# CSPICE trgsep(et, t1, POINT, J2000, t2, POINT, J2000, obs, abcorr).\n"
        )
        handle.write("# Angular separation in radians.\n")
        handle.write("et,observer,target1,target2,abcorr,separation\n")
        for et, observer, target1, target2, abcorr, value in separation_rows:
            handle.write(
                f"{et!r},{observer},{target1},{target2},{abcorr},{float(value)!r}\n"
            )

    event_rows = []
    for observer, target1, target2, abcorr, start, end in EVENT_CASES:
        confinement = spice.cell_double(100)
        spice.wninsd(start, end, confinement)
        for relate in EVENT_KINDS:
            result = spice.cell_double(10000)
            spice.gfsep(
                str(target1),
                "POINT",
                "J2000",
                str(target2),
                "POINT",
                "J2000",
                abcorr,
                str(observer),
                relate,
                0.0,
                0.0,
                EVENT_STEP,
                2000,
                confinement,
                result,
            )
            for index in range(spice.wncard(result)):
                left, right = spice.wnfetd(result, index)
                if abs(right - left) > 1e-3:
                    continue
                event_rows.append(
                    (
                        observer,
                        target1,
                        target2,
                        abcorr,
                        start,
                        end,
                        relate,
                        0.5 * (left + right),
                    )
                )

    shape_rows = []
    for et, observer, target1, target2, shape1, shape2, abcorr in SHAPE_CASES:
        try:
            value = trgsep(et, observer, target1, target2, shape1, shape2, abcorr)
        except Exception as error:
            print(f"skipped shape case {target1}/{target2}: {error}")
            continue
        shape_rows.append(
            (et, observer, target1, target2, shape1, shape2, abcorr, value)
        )

    shape_event_rows = []
    for (
        observer,
        target1,
        target2,
        shape1,
        shape2,
        abcorr,
        start,
        end,
    ) in SHAPE_EVENT_CASES:
        confinement = spice.cell_double(100)
        spice.wninsd(start, end, confinement)
        for relate in EVENT_KINDS:
            result = spice.cell_double(10000)
            spice.gfsep(
                str(target1),
                shape1,
                "J2000",
                str(target2),
                shape2,
                "J2000",
                abcorr,
                str(observer),
                relate,
                0.0,
                0.0,
                EVENT_STEP,
                2000,
                confinement,
                result,
            )
            for index in range(spice.wncard(result)):
                left, right = spice.wnfetd(result, index)
                if abs(right - left) > 1e-3:
                    continue
                shape_event_rows.append(
                    (
                        observer,
                        target1,
                        target2,
                        shape1,
                        shape2,
                        abcorr,
                        start,
                        end,
                        relate,
                        0.5 * (left + right),
                    )
                )

    spice.kclear()

    events_out = Path(args.events_out)
    events_out.parent.mkdir(parents=True, exist_ok=True)
    with events_out.open("w", encoding="utf-8", newline="\n") as handle:
        handle.write("# CSPICE gfsep local extrema, zero-width intervals.\n")
        handle.write("observer,target1,target2,abcorr,start,end,kind,et\n")
        for row in event_rows:
            observer, target1, target2, abcorr, start, end, relate, et = row
            handle.write(
                f"{observer},{target1},{target2},{abcorr},{start!r},{end!r},{relate},{et!r}\n"
            )

    shapes_out = Path(args.shapes_out)
    shapes_out.parent.mkdir(parents=True, exist_ok=True)
    with shapes_out.open("w", encoding="utf-8", newline="\n") as handle:
        handle.write(
            "# CSPICE trgsep with POINT/SPHERE shapes. Radians; a negative\n"
            "# value means the apparent disks overlap.\n"
        )
        handle.write(
            "et,observer,target1,target2,shape1,shape2,abcorr,separation\n"
        )
        for (
            et,
            observer,
            target1,
            target2,
            shape1,
            shape2,
            abcorr,
            value,
        ) in shape_rows:
            handle.write(
                f"{et!r},{observer},{target1},{target2},{shape1},{shape2},{abcorr},{float(value)!r}\n"
            )

    shape_events_out = Path(args.shape_events_out)
    with shape_events_out.open("w", encoding="utf-8", newline="\n") as handle:
        handle.write("# CSPICE gfsep local extrema with SPHERE shapes.\n")
        handle.write(
            "observer,target1,target2,shape1,shape2,abcorr,start,end,kind,et\n"
        )
        for row in shape_event_rows:
            handle.write(",".join(repr(value) if isinstance(value, float) else str(value) for value in row) + "\n")

    print(
        f"wrote {separation_out} ({len(separation_rows)} rows) and "
        f"{events_out} ({len(event_rows)} events)"
    )
    print(
        f"wrote {shapes_out} ({len(shape_rows)} rows) and "
        f"{shape_events_out} ({len(shape_event_rows)} events)"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
