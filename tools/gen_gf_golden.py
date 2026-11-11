#!/usr/bin/env python3
"""Generate Geometry Finder golden data from CSPICE.

Covers the seven scalar search types exposed by WebGeocalc:
distance, range rate, phase angle, illumination angle, position
coordinate, sub-point coordinate, and surface-intercept coordinate.

`gf*` returns time windows; each row in the output is one window.
Local and absolute extrema are returned as zero-width windows by
CSPICE, so the same table validates both interval and event searches.

The SPK and PCK are not committed.

Usage:
    python tools/gen_gf_golden.py \
        --spk target/kernels/de440s.bsp \
        --pck target/kernels/pck00011.tpc
"""

from __future__ import annotations

import argparse
from pathlib import Path

import spiceypy as spice

DAY = 86400.0
STEP = 3600.0
NINTVLS = 4000
CONFINEMENT = (0.0, 30.0 * DAY)
ABCORR = "NONE"

CONDITIONS = [
    (">", 0.0, 0.0),
    ("<", 0.0, 0.0),
    ("ABSMAX", 0.0, 0.0),
    ("ABSMIN", 0.0, 0.0),
    ("LOCMAX", 0.0, 0.0),
    ("LOCMIN", 0.0, 0.0),
]


def cell(confinement: tuple[float, float]) -> object:
    window = spice.cell_double(NINTVLS)
    spice.wninsd(confinement[0], confinement[1], window)
    return window


def run(
    gf,
    arguments: tuple,
    relate: str,
    refval: float,
    adjust: float,
    confinement: tuple[float, float] = CONFINEMENT,
    step: float = STEP,
) -> list[tuple[float, float]]:
    result = spice.cell_double(NINTVLS)
    gf(*arguments, relate, refval, adjust, step, NINTVLS, cell(confinement), result)
    return [spice.wnfetd(result, index) for index in range(spice.wncard(result))]


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--spk", required=True)
    parser.add_argument("--pck", required=True)
    parser.add_argument("--out-dir", default="tests/golden")
    args = parser.parse_args()

    spice.furnsh(args.spk)
    spice.furnsh(args.pck)

    rows: list[tuple] = []

    # Distance: Moon from Earth.
    distance_conditions = [
        (">", 4.0e5, 0.0),
        ("<", 3.85e5, 0.0),
        ("=", 3.87e5, 0.0),
    ] + CONDITIONS[2:]
    for relate, refval, adjust in distance_conditions:
        for left, right in run(
            spice.gfdist, ("301", ABCORR, "399"), relate, refval, adjust
        ):
            rows.append(
                ("distance", "MOON from EARTH", relate, refval, adjust, left, right)
            )

    # Adjusted absolute extrema exercise the ADJUST argument.
    for relate, refval, adjust in [("ABSMAX", 0.0, 100.0), ("ABSMIN", 0.0, 100.0)]:
        for left, right in run(
            spice.gfdist, ("301", ABCORR, "399"), relate, refval, adjust
        ):
            rows.append(
                (
                    "distance",
                    "MOON from EARTH adjusted",
                    relate,
                    refval,
                    adjust,
                    left,
                    right,
                )
            )

    # Range rate: Moon from Earth.
    for relate, refval, adjust in [
        (">", 0.0, 0.0),
        ("<", 0.0, 0.0),
    ] + CONDITIONS[2:]:
        for left, right in run(
            spice.gfrr, ("301", ABCORR, "399"), relate, refval, adjust
        ):
            rows.append(("range_rate", "MOON from EARTH", relate, refval, adjust, left, right))

    # Phase angle: Moon lit by Sun, seen from Earth.
    for relate, refval, adjust in [
        (">", 1.5707963267948966, 0.0),
        ("<", 1.5707963267948966, 0.0),
        ("ABSMAX", 0.0, 0.0),
        ("ABSMIN", 0.0, 0.0),
    ]:
        for left, right in run(
            spice.gfpa, ("301", "10", ABCORR, "399"), relate, refval, adjust
        ):
            rows.append(
                ("phase", "MOON lit by SUN from EARTH", relate, refval, adjust, left, right)
            )

    # Illumination angles at a fixed Moon surface point.
    point = spice.latrec(1737.4, 10.0 * spice.rpd(), 20.0 * spice.rpd())
    for angle in ("INCIDENCE", "EMISSION", "PHASE"):
        for relate, refval, adjust in [
            (">", 0.5, 0.0),
            ("<", 0.5, 0.0),
            ("ABSMAX", 0.0, 0.0),
        ]:
            for left, right in run(
                spice.gfilum,
                ("ELLIPSOID", angle, "301", "10", "IAU_MOON", ABCORR, "399", point),
                relate,
                refval,
                adjust,
            ):
                rows.append(
                    (
                        "illum",
                        f"{angle} at (10,20) on MOON",
                        relate,
                        refval,
                        adjust,
                        left,
                        right,
                    )
                )

    # Position coordinate: Moon from Earth in J2000, X and latitude.
    for crdsys, coord in (("RECTANGULAR", "X"), ("LATITUDINAL", "LATITUDE")):
        for relate, refval, adjust in [
            (">", 0.0, 0.0),
            ("<", 0.0, 0.0),
            ("ABSMAX", 0.0, 0.0),
        ]:
            for left, right in run(
                spice.gfposc,
                ("301", "J2000", ABCORR, "399", crdsys, coord),
                relate,
                refval,
                adjust,
            ):
                rows.append(
                    (
                        "position",
                        f"MOON from EARTH {crdsys} {coord}",
                        relate,
                        refval,
                        adjust,
                        left,
                        right,
                    )
                )

    # Sub-observer point coordinate in planetographic coordinates.
    for crdsys, coord in (("PLANETOGRAPHIC", "LATITUDE"), ("PLANETOGRAPHIC", "LONGITUDE")):
        conditions = [(">", 0.0, 0.0), ("<", 0.0, 0.0)]
        if coord != "LONGITUDE":
            conditions.append(("ABSMAX", 0.0, 0.0))
        for relate, refval, adjust in conditions:
            for left, right in run(
                spice.gfsubc,
                (
                    "301",
                    "IAU_MOON",
                    "NEAR POINT: ELLIPSOID",
                    ABCORR,
                    "399",
                    crdsys,
                    coord,
                ),
                relate,
                refval,
                adjust,
            ):
                rows.append(
                    (
                        "subpoint",
                        f"MOON sub-observer {crdsys} {coord}",
                        relate,
                        refval,
                        adjust,
                        left,
                        right,
                    )
                )

    # Surface intercept of a fixed direction from the Earth toward the Moon.
    direction, _ = spice.spkezp(301, 0.0, "J2000", ABCORR, 399)
    for crdsys, coord in (("PLANETOGRAPHIC", "LATITUDE"), ("PLANETOGRAPHIC", "LONGITUDE")):
        conditions = [(">", 0.0, 0.0), ("<", 0.0, 0.0)]
        if coord != "LONGITUDE":
            conditions.append(("ABSMAX", 0.0, 0.0))
        for relate, refval, adjust in conditions:
            for left, right in run(
                spice.gfsntc,
                (
                    "301",
                    "IAU_MOON",
                    "ELLIPSOID",
                    ABCORR,
                    "399",
                    "J2000",
                    direction,
                    crdsys,
                    coord,
                ),
                relate,
                refval,
                adjust,
                (0.0, 300.0),
                10.0,
            ):
                rows.append(
                    (
                        "intercept",
                        f"MOON intercept {crdsys} {coord}",
                        relate,
                        refval,
                        adjust,
                        left,
                        right,
                    )
                )

    spice.kclear()

    out = Path(args.out_dir) / "gf_events.csv"
    out.parent.mkdir(parents=True, exist_ok=True)
    with out.open("w", encoding="utf-8", newline="\n") as handle:
        handle.write("# CSPICE Geometry Finder windows.\n")
        handle.write("# Each row is one result interval; extrema are zero-width.\n")
        handle.write("family,cases,relate,refval,start,end,adjust\n")
        for family, cases, relate, refval, adjust, left, right in rows:
            handle.write(
                f"{family},{cases},{relate},{float(refval)!r},{float(left)!r},{float(right)!r},{float(adjust)!r}\n"
            )

    print(f"wrote {out} ({len(rows)} windows)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
