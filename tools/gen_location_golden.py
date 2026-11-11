#!/usr/bin/env python3
"""Generate golden data for point locations and the topocentric frame.

Two files are written:

* `location_coordinates.csv` — the eight WebGeocalc location coordinate
  systems, converted to rectangular with the CSPICE inverses `latrec`,
  `sphrec`, `cylrec`, `radrec`, `georec`, `pgrrec`, and `azlrec`.
* `location_states.csv` — apparent states with a point target or a point
  observer, from `spkcpt`, `spkcvo`, and `spkcvt`, covering every
  `refloc` locus and every aberration correction.

`location_topocentric.csv` covers the local topocentric frame through
`azlcpo`, which builds it internally the way WebGeocalc's
`TOPOCENTRIC_AT_POINT` does.

Offline tooling only; needs `spiceypy`.

Usage:
    python tools/gen_location_golden.py --spk target/kernels/de440s.bsp \
        --pck target/kernels/pck00011.tpc
"""

from __future__ import annotations

import argparse
import math

import spiceypy as spice

DEG = math.pi / 180.0

# Earth and Mars spheroid parameters are read from the PCK so the oracle
# uses the same radii as the crate.
EARTH_RE = 6378.1366
EARTH_F = 1.0 / 298.257
MARS_RE = 3396.19
MARS_F = 1.0 - 3376.20 / 3396.19


def spheroid(body: str) -> tuple[float, float]:
    """Equatorial radius and flattening from the loaded text PCK."""
    radii = spice.bodvrd(body, "RADII", 3)[1]
    equatorial, polar = float(radii[0]), float(radii[2])
    return equatorial, 1.0 - polar / equatorial


def coordinate_cases(earth: tuple[float, float], mars: tuple[float, float]) -> list[tuple]:
    """(case, rectangular) triples from the CSPICE inverse routines."""
    rows = []
    radius, longitude, latitude = 4000.0, 30.0 * DEG, 45.0 * DEG
    rows.append(("latrec", spice.latrec(radius, longitude, latitude)))
    rows.append(("sphrec", spice.sphrec(5000.0, 50.0 * DEG, 30.0 * DEG)))
    rows.append(("cylrec", spice.cylrec(4000.0, 30.0 * DEG, 500.0)))
    rows.append(("radrec", spice.radrec(4000.0, 30.0 * DEG, 45.0 * DEG)))
    rows.append(
        (
            "georec_earth",
            spice.georec(30.0 * DEG, 45.0 * DEG, 0.5, earth[0], earth[1]),
        )
    )
    rows.append(
        (
            "georec_mars",
            spice.georec(30.0 * DEG, 45.0 * DEG, 0.5, mars[0], mars[1]),
        )
    )
    rows.append(
        (
            "pgrrec_earth",
            spice.pgrrec("EARTH", 30.0 * DEG, 45.0 * DEG, 0.5, earth[0], earth[1]),
        )
    )
    rows.append(
        (
            "pgrrec_mars",
            spice.pgrrec("MARS", 30.0 * DEG, 45.0 * DEG, 0.5, mars[0], mars[1]),
        )
    )
    rows.append(
        (
            "pgrrec_mars_west",
            spice.pgrrec("MARS", -30.0 * DEG, -20.0 * DEG, 12.0, mars[0], mars[1]),
        )
    )
    for azccw in (0, 1):
        for elplsz in (0, 1):
            rows.append(
                (
                    f"azlrec_{azccw}_{elplsz}",
                    spice.azlrec(
                        4000.0, 30.0 * DEG, 20.0 * DEG, bool(azccw), bool(elplsz)
                    ),
                )
            )
    return rows


def state_cases(earth: tuple[float, float]) -> list[dict]:
    """Point target or point observer cases, one per correction and locus."""
    earth_position = spice.georec(
        -118.0 * DEG, 34.0 * DEG, 0.3, earth[0], earth[1]
    )
    moon_position = [1000.0, 2000.0, 1500.0]
    cv_position = [7000.0, 100.0, 200.0]
    cv_velocity = [0.01, 7.4, 0.2]

    cases = []
    for abcorr in ("NONE", "LT", "CN", "LT+S", "CN+S"):
        for refloc in ("CENTER", "OBSERVER", "TARGET"):
            cases.append(
                {
                    "case": f"point_observer_{abcorr}_{refloc}".replace("+", "p"),
                    "target_kind": "OBJECT",
                    "target": "MOON",
                    "target_center": "",
                    "target_frame": "",
                    "target_position": [0.0, 0.0, 0.0],
                    "target_velocity": [0.0, 0.0, 0.0],
                    "target_epoch": 0.0,
                    "observer_kind": "POINT",
                    "observer": "",
                    "observer_center": "EARTH",
                    "observer_frame": "IAU_EARTH",
                    "observer_position": earth_position,
                    "observer_velocity": [0.0, 0.0, 0.0],
                    "observer_epoch": 0.0,
                    "outref": "J2000",
                    "refloc": refloc,
                    "abcorr": abcorr,
                }
            )

    for abcorr in ("NONE", "LT", "LT+S"):
        cases.append(
            {
                "case": f"point_target_{abcorr}".replace("+", "p"),
                "target_kind": "POINT",
                "target": "",
                "target_center": "MOON",
                "target_frame": "IAU_MOON",
                "target_position": moon_position,
                "target_velocity": [0.0, 0.0, 0.0],
                "target_epoch": 0.0,
                "observer_kind": "OBJECT",
                "observer": "EARTH",
                "observer_center": "",
                "observer_frame": "",
                "observer_position": [0.0, 0.0, 0.0],
                "observer_velocity": [0.0, 0.0, 0.0],
                "observer_epoch": 0.0,
                "outref": "J2000",
                "refloc": "CENTER",
                "abcorr": abcorr,
            }
        )

    for abcorr in ("NONE", "LT", "LT+S"):
        cases.append(
            {
                "case": f"cv_target_{abcorr}".replace("+", "p"),
                "target_kind": "CV_POINT",
                "target": "",
                "target_center": "EARTH",
                "target_frame": "J2000",
                "target_position": cv_position,
                "target_velocity": cv_velocity,
                "target_epoch": 0.0,
                "observer_kind": "OBJECT",
                "observer": "EARTH",
                "observer_center": "",
                "observer_frame": "",
                "observer_position": [0.0, 0.0, 0.0],
                "observer_velocity": [0.0, 0.0, 0.0],
                "observer_epoch": 0.0,
                "outref": "J2000",
                "refloc": "CENTER",
                "abcorr": abcorr,
            }
        )
        cases.append(
            {
                "case": f"cv_observer_{abcorr}".replace("+", "p"),
                "target_kind": "OBJECT",
                "target": "MOON",
                "target_center": "",
                "target_frame": "",
                "target_position": [0.0, 0.0, 0.0],
                "target_velocity": [0.0, 0.0, 0.0],
                "target_epoch": 0.0,
                "observer_kind": "CV_POINT",
                "observer": "",
                "observer_center": "EARTH",
                "observer_frame": "J2000",
                "observer_position": cv_position,
                "observer_velocity": cv_velocity,
                "observer_epoch": 0.0,
                "outref": "J2000",
                "refloc": "CENTER",
                "abcorr": abcorr,
            }
        )

    # A rotating output frame centred on a third body, so that the three
    # frame loci give different answers. Planet 499 is not in the DE
    # ephemerides (only the Mars barycenter is), so the Earth is used as
    # the frame centre and the observer sits on the Moon.
    for refloc in ("CENTER", "OBSERVER", "TARGET"):
        cases.append(
            {
                "case": f"point_observer_iau_earth_{refloc}",
                "target_kind": "OBJECT",
                "target": "EARTH",
                "target_center": "",
                "target_frame": "",
                "target_position": [0.0, 0.0, 0.0],
                "target_velocity": [0.0, 0.0, 0.0],
                "target_epoch": 0.0,
                "observer_kind": "POINT",
                "observer": "",
                "observer_center": "MOON",
                "observer_frame": "IAU_MOON",
                "observer_position": moon_position,
                "observer_velocity": [0.0, 0.0, 0.0],
                "observer_epoch": 0.0,
                "outref": "IAU_EARTH",
                "refloc": refloc,
                "abcorr": "LT+S",
            }
        )
    return cases


def evaluate(case: dict, et: float) -> tuple[list[float], float]:
    abcorr = case["abcorr"]
    refloc = case["refloc"]
    outref = case["outref"]

    target_state = list(case["target_position"]) + list(case["target_velocity"])
    observer_state = list(case["observer_position"]) + list(case["observer_velocity"])

    if case["observer_kind"] == "OBJECT":
        if case["target_kind"] == "POINT":
            state, light_time = spice.spkcpt(
                target_state[:3],
                case["target_center"],
                case["target_frame"],
                et,
                outref,
                refloc,
                abcorr,
                case["observer"],
            )
        else:
            state, light_time = spice.spkcvt(
                target_state,
                case["target_epoch"],
                case["target_center"],
                case["target_frame"],
                et,
                outref,
                refloc,
                abcorr,
                case["observer"],
            )
    elif case["target_kind"] == "OBJECT":
        if case["observer_kind"] == "POINT":
            state, light_time = spice.spkcpo(
                case["target"],
                et,
                outref,
                refloc,
                abcorr,
                observer_state[:3],
                case["observer_center"],
                case["observer_frame"],
            )
        else:
            state, light_time = spice.spkcvo(
                case["target"],
                et,
                outref,
                refloc,
                abcorr,
                observer_state,
                case["observer_epoch"],
                case["observer_center"],
                case["observer_frame"],
            )
    else:
        raise RuntimeError("both endpoints are points")

    return [float(value) for value in state], float(light_time)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--spk", default="target/kernels/de440s.bsp")
    parser.add_argument("--pck", default="target/kernels/pck00011.tpc")
    parser.add_argument("--epoch", type=float, default=1.0e8)
    parser.add_argument("--directory", default="tests/golden")
    args = parser.parse_args()

    spice.furnsh(args.spk)
    spice.furnsh(args.pck)

    earth = spheroid("EARTH")
    mars = spheroid("MARS")

    coordinate_lines = [
        "# Coordinates from the CSPICE inverse conversions.",
        "case,x,y,z",
    ]
    for label, rectangular in coordinate_cases(earth, mars):
        values = ",".join(repr(float(value)) for value in rectangular)
        coordinate_lines.append(f"{label},{values}")

    state_header = ",".join(
        [
            "case",
            "target_kind",
            "target",
            "target_center",
            "target_frame",
            "trg_x",
            "trg_y",
            "trg_z",
            "trg_vx",
            "trg_vy",
            "trg_vz",
            "trg_epoch",
            "observer_kind",
            "observer",
            "observer_center",
            "observer_frame",
            "obs_x",
            "obs_y",
            "obs_z",
            "obs_vx",
            "obs_vy",
            "obs_vz",
            "obs_epoch",
            "outref",
            "refloc",
            "abcorr",
            "sx",
            "sy",
            "sz",
            "svx",
            "svy",
            "svz",
            "light_time",
        ]
    )
    state_lines = ["# Apparent states with a point endpoint (CSPICE).", state_header]
    for case in state_cases(earth):
        state, light_time = evaluate(case, args.epoch)
        fields = [
            case["case"],
            case["target_kind"],
            case["target"],
            case["target_center"],
            case["target_frame"],
            *[repr(float(value)) for value in case["target_position"]],
            *[repr(float(value)) for value in case["target_velocity"]],
            repr(float(case["target_epoch"])),
            case["observer_kind"],
            case["observer"],
            case["observer_center"],
            case["observer_frame"],
            *[repr(float(value)) for value in case["observer_position"]],
            *[repr(float(value)) for value in case["observer_velocity"]],
            repr(float(case["observer_epoch"])),
            case["outref"],
            case["refloc"],
            case["abcorr"],
            *[repr(float(value)) for value in state],
            repr(light_time),
        ]
        state_lines.append(",".join(fields))

    earth_position = spice.georec(
        -118.0 * DEG, 34.0 * DEG, 0.3, earth[0], earth[1]
    )
    topocentric_lines = [
        "# azlcpo: range, azimuth, elevation, range rate.",
        "case,azccw,elplsz,abcorr,range,azimuth,elevation,range_rate,light_time",
    ]
    for abcorr in ("NONE", "LT+S"):
        for azccw in (0, 1):
            for elplsz in (0, 1):
                azel, light_time = spice.azlcpo(
                    "ELLIPSOID",
                    "MOON",
                    args.epoch,
                    abcorr,
                    bool(azccw),
                    bool(elplsz),
                    earth_position,
                    "EARTH",
                    "IAU_EARTH",
                )
                range_value, azimuth, elevation, range_rate = (
                    float(azel[0]),
                    float(azel[1]),
                    float(azel[2]),
                    float(azel[3]),
                )
                topocentric_lines.append(
                    ",".join(
                        [
                            f"moon_{abcorr}_{azccw}_{elplsz}".replace("+", "p"),
                            str(azccw),
                            str(elplsz),
                            abcorr,
                            repr(range_value),
                            repr(azimuth),
                            repr(elevation),
                            repr(range_rate),
                            repr(float(light_time)),
                        ]
                    )
                )

    directory = args.directory
    for name, lines in (
        ("location_coordinates.csv", coordinate_lines),
        ("location_states.csv", state_lines),
        ("location_topocentric.csv", topocentric_lines),
    ):
        with open(f"{directory}/{name}", "w", encoding="utf-8", newline="\n") as handle:
            handle.write("\n".join(lines))
            handle.write("\n")
        print(f"wrote {len(lines) - 2} cases to {directory}/{name}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
