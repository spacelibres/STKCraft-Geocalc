#!/usr/bin/env python3
"""Generate WebGeocalc state-representation golden vectors with CSPICE.

For every (case, epoch, aberration correction, representation) the script
calls `spkezr` to get the apparent state in the requested frame and then
converts that state with CSPICE's own `recrad`/`reclat`/`recsph`/`reccyl`/
`recgeo`/`recpgr` routines, so the numbers do not depend on this crate.

Coordinate rates are direct CSPICE XFMSTA/DAZLDR Jacobian transforms.
No finite differences or tolerance-based reference values are used.

The column order follows the live WebGeocalc `STATE_VECTOR` output, which
`tools/wgc2_geometry_compare.py` re-checks against the API.

Offline tooling only; needs `spiceypy`.

Usage:
    python tools/gen_state_rep_golden.py
"""

from __future__ import annotations

import argparse
import math
from pathlib import Path

import spiceypy as spice

EPOCHS = [-5.0e8, 0.0, 5.0e8]
ABCORR = ["NONE", "LT+S"]

# (target id, observer id, frame name, frame code used by the crate)
CASES = [
    (301, 399, "J2000", 0),
    (301, 399, "ECLIPJ2000", -1),
    (301, 399, "IAU_MOON", 301),
    (399, 301, "IAU_EARTH", 399),
]

REPRESENTATIONS = [
    ("RECTANGULAR", True, True),
    ("RA_DEC", True, True),
    ("LATITUDINAL", True, True),
    ("CYLINDRICAL", True, True),
    ("SPHERICAL", True, True),
    ("AZ_EL", True, True),
    ("AZ_EL", True, False),
    ("AZ_EL", False, True),
    ("AZ_EL", False, False),
    ("PLANETODETIC", True, True),
    ("PLANETOGRAPHIC", True, True),
]

PLANETARY = {"PLANETODETIC", "PLANETOGRAPHIC"}

TWO_PI = 2.0 * math.pi


def triple(state, frame_code, representation, azccw, elplsz):
    """The three coordinates WebGeocalc displays for a representation."""
    position = state[:3]

    if representation == "RECTANGULAR":
        return list(position)

    if representation == "RA_DEC":
        radius, right_ascension, declination = spice.recrad(position)
        return [math.degrees(right_ascension), math.degrees(declination), radius]

    if representation == "LATITUDINAL":
        radius, longitude, latitude = spice.reclat(position)
        return [math.degrees(longitude), math.degrees(latitude), radius]

    if representation == "CYLINDRICAL":
        radius, longitude, z = spice.reccyl(position)
        return [math.degrees(longitude), z, radius]

    if representation == "SPHERICAL":
        radius, colatitude, longitude = spice.recsph(position)
        return [math.degrees(longitude), math.degrees(colatitude), radius]

    if representation == "AZ_EL":
        radius, azimuth, elevation = spice.recazl(position, azccw, elplsz)
        return [float(azimuth)*spice.dpr(), float(elevation)*spice.dpr(), float(radius)]

    body = spice.bodc2n(frame_code)
    radii = spice.bodvrd(body, "RADII", 3)[1]
    re = radii[0]
    flattening = (radii[0] - radii[2]) / radii[0]

    if representation == "PLANETODETIC":
        longitude, latitude, altitude = spice.recgeo(position, re, flattening)
        return [math.degrees(longitude), math.degrees(latitude), altitude]

    longitude, latitude, altitude = spice.recpgr(body, position, re, flattening)
    return [math.degrees(longitude), math.degrees(latitude), altitude]


def analytic_rates(state, frame_code, representation, azccw, elplsz):
    if representation == "AZ_EL":
        rates = spice.mxv(spice.dazldr(*state[:3], azccw, elplsz), state[3:])
        return [float(rates[1])*spice.dpr(), float(rates[2])*spice.dpr(), float(rates[0])]
    system = {"RA_DEC": "LATITUDINAL", "PLANETODETIC": "GEODETIC"}.get(representation, representation)
    body = spice.bodc2n(frame_code) if frame_code > 0 else "EARTH"
    value = spice.xfmsta(state, "RECTANGULAR", system, body)
    if representation in ("RA_DEC", "LATITUDINAL"):
        return [float(value[4])*spice.dpr(), float(value[5])*spice.dpr(), float(value[3])]
    if representation == "CYLINDRICAL":
        return [float(value[4])*spice.dpr(), float(value[5]), float(value[3])]
    if representation == "SPHERICAL":
        return [float(value[5])*spice.dpr(), float(value[4])*spice.dpr(), float(value[3])]
    return [float(value[3])*spice.dpr(), float(value[4])*spice.dpr(), float(value[5])]


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--out", default="tests/golden/state_representations.csv")
    parser.add_argument("--spk", default="target/kernels/de442s.bsp")
    parser.add_argument("--pck", default="target/kernels/pck00011.tpc")
    args = parser.parse_args()

    spice.furnsh(args.spk)
    spice.furnsh(args.pck)

    rows = []
    for target, observer, frame, frame_code in CASES:
        target_name = spice.bodc2n(target)
        observer_name = spice.bodc2n(observer)
        for abcorr in ABCORR:
            for representation, azccw, elplsz in REPRESENTATIONS:
                if representation in PLANETARY and frame_code <= 0:
                    continue
                for et in EPOCHS:
                    state, light_time = spice.spkezr(
                        target_name, et, frame, abcorr, observer_name
                    )
                    position = state[:3]
                    velocity = state[3:]
                    distance = spice.vnorm(position)
                    speed = spice.vnorm(velocity)

                    if abcorr == "NONE":
                        target_epoch = et
                    elif abcorr.startswith("X"):
                        target_epoch = et + light_time
                    else:
                        target_epoch = et - light_time

                    values = triple(
                        state, frame_code, representation, azccw, elplsz
                    )
                    if representation == "RECTANGULAR":
                        # WGC leads with DISTANCE and SPEED, and the rate
                        # columns are the velocity components themselves.
                        row_values = [distance, speed, *position, *velocity]
                        rates = None
                    else:
                        rates = analytic_rates(state, frame_code, representation, azccw, elplsz)
                        row_values = [*values, *rates, speed]

                    row_values.append(target_epoch)
                    row_values.append(light_time)
                    rows.append(
                        (
                            et,
                            target,
                            observer,
                            abcorr,
                            frame_code,
                            representation,
                            azccw,
                            elplsz,
                            row_values,
                            rates,
                        )
                    )

    spice.kclear()

    width = max(len(row[8]) for row in rows)
    out = Path(args.out)
    out.parent.mkdir(parents=True, exist_ok=True)
    with out.open("w", encoding="utf-8", newline="\n") as handle:
        handle.write(
            "# CSPICE spkezr + recrad/reclat/recsph/reccyl/recgeo/recpgr\n"
            "# golden rows in WebGeocalc STATE_VECTOR column order, i.e. the\n"
            "# columns after DATE returned by StateRepresentation::output_ids.\n"
        )
        handle.write(
            "# Angles are degrees, angular rates degrees per second, positions\n"
            "# kilometres, velocities km/s.\n"
        )
        handle.write(
            "et,target,observer,abcorr,frame,representation,azccw,elplsz,"
            + ",".join(f"c{index}" for index in range(width))
            + "\n"
        )
        for (
            et,
            target,
            observer,
            abcorr,
            frame_code,
            representation,
            azccw,
            elplsz,
            row_values,
            _rates,
        ) in rows:
            text = ",".join(repr(float(value)) for value in row_values)
            handle.write(
                f"{et!r},{target},{observer},{abcorr},{frame_code},{representation},"
                f"{str(azccw).lower()},{str(elplsz).lower()},{text}\n"
            )

    print(f"wrote {out} ({len(rows)} rows)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
