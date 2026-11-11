#!/usr/bin/env python3
"""Compare STKCraft-geocalc against the live NASA WebGeocalc (WGC2) API.

Runs geometry calculation families and rejects unequal reference values.
The historical tolerance bounds are diagnostic only. Families include:

  1. State Vector, geometric and aberration-corrected.
  2. Angular Separation.
  3. Frame Transformation (J2000 <-> ECLIPJ2000).

WGC's "Solar System Kernels" set (kernel set 1) uses DE442. WGC output
has column-specific presentation precision; exact transported equality
does not prove high precision CSPICE parity. Pass `--spk target/kernels/de442s.bsp` (the
default) so both sides use the same ephemeris; comparing WGC against
DE440 instead shows a ~0.1 m frame-rotation-free ephemeris difference.

Only the Python standard library is used.

Usage:
    python tools/wgc2_geometry_compare.py
    python tools/wgc2_geometry_compare.py --insecure
    python tools/wgc2_geometry_compare.py --spk target/kernels/de442s.bsp
"""

from __future__ import annotations

import argparse
import math
from decimal import Decimal, InvalidOperation, localcontext, ROUND_HALF_UP
import json
import ssl
import struct
import subprocess
import sys
import time
import urllib.request
from pathlib import Path

API_BASE = "https://wgc2.jpl.nasa.gov:8443/webgeocalc/api"

EPOCHS = [-5.0e8, 0.0, 5.0e8]
ABCORR = ["NONE", "LT", "LT+S", "CN", "CN+S", "XLT", "XLT+S", "XCN", "XCN+S"]

# (label, target name, target id, observer name, observer id)
STATE_CASES = [
    ("MOON from EARTH", "MOON", 301, "EARTH", 399),
    ("SUN from EARTH", "SUN", 10, "EARTH", 399),
    ("JUPITER BARYCENTER from EARTH", "JUPITER BARYCENTER", 5, "EARTH", 399),
    ("EARTH from SSB", "EARTH", 399, "SOLAR SYSTEM BARYCENTER", 0),
    ("MARS BARYCENTER from MOON", "MARS BARYCENTER", 4, "MOON", 301),
]

# (label, target1 name, id, target2 name, id, observer name, id)
SEPARATION_CASES = [
    ("SUN-MOON from EARTH", "SUN", 10, "MOON", 301, "EARTH", 399),
    (
        "SUN-JUPITER BARYCENTER from EARTH",
        "SUN",
        10,
        "JUPITER BARYCENTER",
        5,
        "EARTH",
        399,
    ),
    ("MOON-MARS BARYCENTER from MOON", "MOON", 301, "MARS BARYCENTER", 4, "MOON", 301),
]
SEPARATION_ABCORR = ["NONE", "LT+S"]
SEPARATION_SHAPES = [
    ("POINT", "POINT"),
    ("POINT", "SPHERE"),
    ("SPHERE", "POINT"),
    ("SPHERE", "SPHERE"),
]

PHASE_CASES = [
    ("MOON from EARTH, lit by SUN", "MOON", 301, "SUN", 10, "EARTH", 399),
    ("EARTH from MOON, lit by SUN", "EARTH", 399, "SUN", 10, "MOON", 301),
    ("MOON from SUN, lit by EARTH", "MOON", 301, "EARTH", 399, "SUN", 10),
]
SIZE_CASES = [
    ("MOON from EARTH", "MOON", 301, "EARTH", 399),
    ("EARTH from SUN", "EARTH", 399, "SUN", 10),
    ("MOON from SUN", "MOON", 301, "SUN", 10),
]
PHASE_ABCORR = ["NONE", "LT+S"]
SIZE_ABCORR = ["NONE", "LT+S"]
ELEMENT_CASES = [
    ("MOON around EARTH", "MOON", 301, "EARTH", 399),
    ("EARTH around SUN", "EARTH", 399, "SUN", 10),
]
ELEMENT_ABCORR = ["NONE"]
LONGITUDE_CASES = [("MOON", 301, "IAU_MOON"), ("EARTH", 399, "IAU_EARTH")]
ILLUMINATION_CASES = [
    ("MOON", 301, "EARTH", 399, "IAU_MOON", (10.0, 20.0, 0.0)),
    ("MOON", 301, "EARTH", 399, "IAU_MOON", (-45.0, 30.0, 0.0)),
    ("EARTH", 399, "MOON", 301, "IAU_EARTH", (10.0, 20.0, 0.0)),
]
INTERCEPT_CASES = [
    ("MOON", 301, "EARTH", 399, "IAU_MOON"),
    ("EARTH", 399, "MOON", 301, "IAU_EARTH"),
]

FRAME_CASES = [("J2000", "IAU_MARS", 1.0e8), ("IAU_MOON", "IAU_EARTH", 0.0), ("J2000", "ECLIPJ2000", 0.0)]

# WebGeocalc's seven orientation representations and five angular velocity
# representations. The axis sequence is only sent for the Euler forms.
FRAME_REPRESENTATIONS = [
    ("EULER_ANGLES", "ZYX"),
    ("ANGLE_AND_AXIS", "-"),
    ("SPICE_QUATERNION", "-"),
    ("OTHER_QUATERNION", "-"),
    ("MATRIX_ROW_BY_ROW", "-"),
    ("MATRIX_FLAGGED", "-"),
    ("MATRIX_ALL_ONE_ROW", "-"),
]
FRAME_VELOCITIES = [
    ("NOT_INCLUDED", "-"),
    ("VECTOR_IN_FRAME1", "-"),
    ("VECTOR_IN_FRAME2", "-"),
    ("EULER_ANGLE_DERIVATIVES", "ZYX"),
    ("MATRIX", "-"),
]

# WebGeocalc spells body-fixed frames by name; this crate uses `IAU_<id>`.
FRAME_NAMES = {
    "IAU_MOON": "IAU_301",
    "IAU_EARTH": "IAU_399",
    "IAU_MARS": "IAU_499",
    "IAU_SATURN": "IAU_699",
    "IAU_ENCELADUS": "IAU_602",
}

# WGC prints eight decimals. These bounds sit a little above that floor.
TOLERANCE_POSITION_KM = 1e-6
TOLERANCE_VELOCITY_KMS = 1e-8
TOLERANCE_LIGHT_TIME_S = 1e-7
TOLERANCE_SEPARATION_DEG = 1e-8
TOLERANCE_ANGLE_DEG = 1e-8
TOLERANCE_SIZE_DEG = 2e-8
TOLERANCE_ELEMENT_KM = 1e-4
TOLERANCE_ECCENTRICITY = 1e-8
TOLERANCE_PERIOD_S = 1e-3
TOLERANCE_FRAME = 1e-8
# WebGeocalc rounds every numeric cell to eight decimals, so the same bound
# serves for angles, angular velocities and matrix elements alike.
TOLERANCE_FRAME_REP = 3e-8



# Exact numeric comparison is the default. Diagnostic tolerance runs never establish parity.
STRICT = True
STRICT_FAILURES = None
QUICK = False
WGC_OUTPUT_EXACT = False


def finite_decimal(value, label):
    if isinstance(value, bool) or not isinstance(value, (str, int, float, Decimal)):
        raise RuntimeError(f"{label}: invalid numeric type {type(value).__name__}")
    try:
        number = Decimal(str(value))
    except InvalidOperation as error:
        raise RuntimeError(f"{label}: invalid numeric value {value!r}") from error
    if not number.is_finite():
        raise RuntimeError(f"{label}: number must be finite: {value!r}")
    return number



def wgc_output_value(value, output_id=None):
    """Independent WGC output serialization compatibility, never a tolerance.

    Verified by seven controlled NASA VECTOR requests (21 values), including
    both magnitude thresholds, exact binary halfway values, shortest-decimal
    halfway values, and positive/negative large integers. See
    tools/tests/fixtures/wgc-output-format.json and raw request evidence.
    Numeric columns: fixed 8 decimal places, or 9 significant digits for
    0<abs(x)<1e-4 and abs(x)>=1e10; HALF_UP on shortest decimal double text.
    Date-valued ET columns use the explicitly requested 6-place time format.
    The rounded number is serialized as a double, as in the API response.
    """
    number=finite_decimal(value,"WGC output value")
    with localcontext() as ctx:
        ctx.prec=max(80,len(number.as_tuple().digits)+abs(number.adjusted())+20)
        if output_id in ("DATE","DATE2","TIME_AT_TARGET","TIME_AT_POINT"):
            quantum=Decimal("0.000001")
        elif number != 0 and (abs(number)<Decimal("0.0001") or abs(number)>=Decimal("10000000000")):
            quantum=Decimal(1).scaleb(number.adjusted()-8)
        else:
            quantum=Decimal("0.00000001")
        rounded=number.quantize(quantum,rounding=ROUND_HALF_UP)
    # JSON's Number output is the rounded IEEE double rather than an
    # arbitrary-precision decimal. Large values can lose fractional zeros.
    return finite_decimal(str(float(rounded)),"WGC serialized value")


def strict_difference(actual, reference, label="numeric cell", *, decimal_places=None, output_id=None):
    """Return raw difference; reject unequal values by default.

    An explicit per-column decimal_places compares presentation rounding only;
    it never establishes high precision CSPICE equality. No tolerance is used.
    """
    a = finite_decimal(actual, label)
    b = finite_decimal(reference, label)
    with localcontext() as ctx:
        ctx.prec = max(80, len(a.as_tuple().digits), len(b.as_tuple().digits))
        raw = abs(a - b)
        equal = (wgc_output_value(a,output_id) if WGC_OUTPUT_EXACT else a) == b
        if decimal_places is not None:
            quantum = Decimal(1).scaleb(-decimal_places)
            equal = a.quantize(quantum) == b.quantize(quantum)
    if STRICT and not equal:
        message = f"{label}: actual={actual!r} reference={reference!r}; raw difference={raw}"
        if WGC_OUTPUT_EXACT: message += f"; WGC-formatted actual={wgc_output_value(a,output_id)}"
        if STRICT_FAILURES is None:
            raise RuntimeError(message)
        STRICT_FAILURES.append(message)
    difference = float(raw)
    if not math.isfinite(difference):
        raise RuntimeError(f"{label}: difference must be finite: {raw}")
    return difference



def compare_cspice_f64(actual, reference, label="numeric cell"):
    """Independent CSPICE oracle comparison: finite IEEE-754 binary64 bits.

    Callers must supply actual unrounded doubles from both implementations.
    WGC presentation values are not an eligible high precision oracle.
    """
    if type(actual) is not float or type(reference) is not float:
        raise RuntimeError(f"CSPICE {label}: unrounded float64 values required")
    finite_decimal(actual, label)
    finite_decimal(reference, label)
    a, b = struct.pack(">d", actual), struct.pack(">d", reference)
    if a != b:
        raise RuntimeError(f"CSPICE {label}: bits {a.hex()} vs {b.hex()}; actual={actual!r} reference={reference!r}; raw difference={actual-reference!r}")
    return 0.0


def checked_count(label, expected, actual):
    if expected <= 0 or actual != expected:
        raise RuntimeError(f"{label}: row count expected {expected} (nonzero), actual {actual}")



def state_column_ids(representation):
    triples = {
        "RA_DEC": ["RIGHT_ASCENSION", "DECLINATION", "RANGE"],
        "LATITUDINAL": ["LONGITUDE", "LATITUDE", "RADIUS"],
        "CYLINDRICAL": ["LONGITUDE", "Z", "RADIUS"],
        "SPHERICAL": ["LONGITUDE", "COLATITUDE", "RADIUS"],
        "AZ_EL": ["AZIMUTH", "ELEVATION", "RANGE"],
        "PLANETODETIC": ["LONGITUDE", "LATITUDE", "ALTITUDE"],
        "PLANETOGRAPHIC": ["LONGITUDE", "LATITUDE", "ALTITUDE"],
    }
    if representation == "RECTANGULAR":
        values = ["DISTANCE", "SPEED", "X", "Y", "Z", "D_X_DT", "D_Y_DT", "D_Z_DT"]
    else:
        coords = triples[representation]
        values = coords + [f"D_{coordinate}_DT" for coordinate in coords] + ["SPEED"]
    return ["DATE"] + values + ["TIME_AT_TARGET", "LIGHT_TIME"]


def reference_result(wgc, payload):
    result = wgc.run(payload)
    columns, rows = result.get("columns", []), result.get("rows", [])
    rows_per_epoch = 1
    if payload["calculationType"] == "FRAME_TRANSFORMATION" and payload.get("orientationRepresentation") in ("MATRIX_ROW_BY_ROW", "MATRIX_FLAGGED"):
        rows_per_epoch = 6 if payload.get("angularVelocityRepresentation") == "MATRIX" else 3
    checked_count(payload["calculationType"] + " reference", len(payload["times"]) * rows_per_epoch, len(rows))
    ids = [column.get("outputID") for column in columns]
    if not ids or ids[0] != "DATE" or any(not isinstance(i, str) or not i for i in ids) or len(set(ids)) != len(ids):
        raise RuntimeError(f"reference column IDs invalid: {ids!r}")
    expected_ids = {
        "ANGULAR_SEPARATION": ["DATE", "ANGULAR_SEPARATION"],
        "ANGULAR_SIZE": (["DATE","MIN_ANGULAR_SIZE","MAX_ANGULAR_SIZE"] if payload.get("computationMethod")=="LIMB_EXTENTS" else ["DATE", "ANGULAR_SIZE"]),
        "PHASE_ANGLE": ["DATE", "PHASE_ANGLE"],
        "PLANETOCENTRIC_SOLAR_LONGITUDE": ["DATE", "PLANETOCENTRIC_SOLAR_LONGITUDE"],
        "OSCULATING_ELEMENTS": ["DATE", "PERIFOCAL_DISTANCE", "ECCENTRICITY", "INCLINATION", "ASCENDING_NODE_LONGITUDE", "ARGUMENT_OF_PERIAPSE", "MEAN_ANOMALY_AT_EPOCH", "ORBITING_BODY_RANGE", "ORBITING_BODY_SPEED", "PERIOD", ("GM" if payload.get("gmType")=="CUSTOM" else "CENTER_BODY_GM"), "TRUE_ANOMALY", "SEMIMAJOR_AXIS"],
    }.get(payload["calculationType"])
    if payload["calculationType"] == "STATE_VECTOR":
        expected_ids = state_column_ids(payload["stateRepresentation"])
    if expected_ids is not None and ids != expected_ids:
        raise RuntimeError(f"reference column IDs/order mismatch: {ids} vs {expected_ids}")
    for index, row in enumerate(rows):
        if len(row) != len(ids):
            raise RuntimeError(f"reference column count {len(row)} vs {len(ids)}")
        # All requests use seconds past J2000 TDB, so the echoed epoch is an identity.
        placeholder = rows_per_epoch > 1 and index % rows_per_epoch != 0 and row[0] == ""
        corrected_frame2=payload["calculationType"]=="FRAME_TRANSFORMATION" and payload.get("aberrationCorrection","NONE")!="NONE" and payload.get("timeLocation")=="FRAME2"
        if not placeholder and not corrected_frame2 and finite_decimal(row[0], "reference epoch") != finite_decimal(payload["times"][index // rows_per_epoch], "query epoch"):
            raise RuntimeError(f"reference row order/epoch identity mismatch at {index}")
        if "ROW_NUMBER" in ids:
            if finite_decimal(row[ids.index("ROW_NUMBER")], "ROW_NUMBER") != index % rows_per_epoch + 1:
                raise RuntimeError("reference matrix row order mismatch")
        for identifier, value in zip(ids[1:], row[1:]):
            if identifier in ("TANGENT_POINT_GEOMETRY","LTST"):
                if not isinstance(value, str) or not value:
                    raise RuntimeError("invalid tangent geometry type")
            else:
                finite_decimal(value, identifier)
    return result


def verified_pairs(queries, expected, values, mode):
    checked_count(mode + " expected", len(queries), len(expected))
    checked_count(mode + " local", len(queries), len(values))
    widths = {"state": (4, 14), "elements": (4, 16), "separation": (5, 7),
              "phase": (5, 7), "size": (4, 6), "longitude": (3, 4),
              "illum": (7, 19), "intercept": (4, 18), "tangent": (4, 28), "subpoint":(5,19), "pointing_full":(4,9)}
    for query, reference, ours in zip(queries, expected, values):
        if ours is None:
            raise RuntimeError(f"{mode}: crate failed for {query}")
        if mode in widths or mode == "state_rep":
            prefix, width = widths[mode] if mode in widths else (4, len(reference[0]) + 3)
            if mode=="size" and query.split(",")[-1]=="LIMB_EXTENTS": width=7
            if len(ours) != width:
                raise RuntimeError(f"{mode}: column count expected {width}, actual {len(ours)} for {query}")
            fields = query.split(",")
            for index in range(prefix):
                try:
                    equal = finite_decimal(ours[index], "identity") == finite_decimal(fields[index], "identity")
                except RuntimeError:
                    equal = ours[index] == fields[index]
                if not equal:
                    raise RuntimeError(f"{mode}: identity/order mismatch {ours[:prefix]} vs {fields[:prefix]}")
            for index, value in enumerate(ours[prefix:], start=prefix):
                if (mode,index) in (("illum",18),("intercept",14),("tangent",23),("tangent",27),("subpoint",18)):
                    if not isinstance(value,str) or not value: raise RuntimeError("string metadata missing")
                elif mode == "illum" and index in (15, 16):
                    if value not in ("true", "false"):
                        raise RuntimeError("illum output boolean type mismatch")
                else:
                    finite_decimal(value, mode + " output")
        yield query, reference, ours


def compare_table(result, ours, label):
    ids=[column["outputID"] for column in result["columns"]]
    if not ids or ids[0]!="DATE":raise RuntimeError(f"{label}: column IDs invalid")
    if ours.get("ids")==ids:offset=0
    elif ours.get("ids")==ids[1:] and "DATE2" not in ids:offset=1
    else:raise RuntimeError(f"{label}: column IDs/order mismatch {ours.get('ids')} vs {ids}")
    checked_count(label,len(result["rows"]),len(ours["rows"]))
    worst=0.
    for reference_row,our_row in zip(result["rows"],ours["rows"]):
        if len(reference_row)!=len(ids) or len(our_row)!=len(ids)-offset:raise RuntimeError(f"{label}: column count mismatch")
        for identifier,actual,reference in zip(ids[offset:],our_row,reference_row[offset:]):
            if identifier in ("DATE","DATE2","LIGHT_TIME") and reference=="" and actual=="":continue
            worst=max(worst,strict_difference(actual,reference,f"{label}/{identifier}",output_id=identifier))
    return worst


def compare_mapped_row(ids, reference, ours, mapping, label):
    if not ids or ids[0] != "DATE" or len(ids) != len(reference):
        raise RuntimeError(f"{label}: column count/IDs mismatch")
    if set(mapping) != set(ids[1:]) or len(mapping) != len(ids)-1:
        raise RuntimeError(f"{label}: comparison coverage mismatch: IDs {ids[1:]} mapping {list(mapping)}")
    worst=0.0
    for identifier, value in zip(ids[1:], reference[1:]):
        index=mapping[identifier]
        if index < 0 or index >= len(ours):
            raise RuntimeError(f"{label}/{identifier}: local column count mismatch")
        actual=ours[index]
        if identifier in ("LTST","TANGENT_POINT_GEOMETRY"):
            if not isinstance(value,str) or not isinstance(actual,str) or value != actual:
                message=f"{label}/{identifier}: actual={actual!r} reference={value!r}"
                if STRICT_FAILURES is None: raise RuntimeError(message)
                STRICT_FAILURES.append(message)
        else:
            worst=max(worst,strict_difference(actual,value,f"{label}/{identifier}",output_id=identifier))
    return worst


class Wgc:
    def __init__(self, insecure: bool, timeout: float, *, curl=False, proxy=None, trace=None, replay=None) -> None:
        self.context = (
            ssl._create_unverified_context() if insecure else ssl.create_default_context()
        )
        self.timeout = timeout
        self.replay = {}
        if replay:
            pending={}
            for line in Path(replay).read_text(encoding="utf-8").splitlines():
                event=json.loads(line);response=json.loads(event["raw_response"])
                if event["method"]=="POST" and event.get("request") is not None and "calculationId" in response:
                    pending[response["calculationId"]]=event["request"]
                if event["path"].endswith("/results"):
                    cid=event["path"].split("/")[-2]
                    if cid in pending: self.replay[json.dumps(pending[cid],sort_keys=True)]=event["raw_response"]
            if not self.replay: raise RuntimeError("raw NASA replay contains no completed calculations")
        self.curl = curl
        self.proxy = proxy
        self.insecure = insecure
        self.trace = Path(trace) if trace else None
        if self.trace:
            self.trace.parent.mkdir(parents=True,exist_ok=True)

    def _request(self, method: str, path: str, payload: dict | None = None) -> dict:
        deadline = getattr(self,"_remaining_timeout",self.timeout)
        if self.curl:
            command=["curl.exe","--silent","--show-error","--fail-with-body","--max-time",str(deadline),"-X",method,"-H","Content-Type: application/json","-H","Accept: application/json",f"{API_BASE}{path}"]
            if self.insecure: command.append("--insecure")
            if self.proxy: command += ["--proxy",self.proxy]
            if payload is not None: command += ["--data-binary","@-"]
            process=subprocess.run(command,input=json.dumps(payload) if payload is not None else None,
                text=True,encoding="utf-8",capture_output=True,timeout=deadline+1.)
            raw=process.stdout
            if process.returncode:
                raise RuntimeError(f"NASA request failed ({process.returncode}): {process.stderr}; body={raw[:1000]}")
        else:
            data=None if payload is None else json.dumps(payload).encode()
            request=urllib.request.Request(f"{API_BASE}{path}",data=data,
                headers={"Content-Type":"application/json","Accept":"application/json"},method=method)
            with urllib.request.urlopen(request,context=self.context,timeout=deadline) as response:
                raw=response.read().decode()
        if self.trace:
            with self.trace.open("a",encoding="utf-8") as handle:
                handle.write(json.dumps({"method":method,"path":path,"request":payload,"raw_response":raw},ensure_ascii=False)+"\n")
        return json.loads(raw,parse_float=Decimal)

    def run(self, payload: dict) -> dict:
        if self.replay:
            key=json.dumps(payload,sort_keys=True)
            if key not in self.replay: raise RuntimeError("no completed raw NASA reference for requested payload")
            return json.loads(self.replay[key],parse_float=Decimal)
        deadline = time.monotonic() + self.timeout
        def request(method, path, data=None):
            remaining = deadline - time.monotonic()
            if remaining <= 0:
                raise TimeoutError("WGC calculation timed out")
            self._remaining_timeout = remaining
            return self._request(method, path, data)
        created = request("POST", "/calculation/new", payload)
        if created.get("status") != "OK":
            raise RuntimeError(json.dumps(created,default=str)[:500])

        calculation_id = created["calculationId"]
        phase = created.get("result", {}).get("phase")

        while phase != "COMPLETE":
            if phase in ("FAILED", "CANCELLED", "EXPIRED"):
                raise RuntimeError(f"calculation {calculation_id} phase {phase}")
            if time.monotonic() > deadline:
                raise TimeoutError(f"calculation {calculation_id} timed out")
            time.sleep(min(0.4, max(0., deadline-time.monotonic())))
            status = request("GET", f"/calculation/{calculation_id}")
            phase = status.get("result", {}).get("phase")

        return request("GET", f"/calculation/{calculation_id}/results")


def time_block(times: list[float]) -> dict:
    return {
        "timeSystem": "TDB",
        "timeFormat": "SECONDS_PAST_J2000",
        "times": [repr(float(value)) for value in times],
        "outputTimeSystem": "TDB",
        "outputTimeFormat": "SECONDS_PAST_J2000",
    }


ACTIVE_SPK = "target/kernels/de442.bsp"
CLOCK_FK = "target/kernels/moon_de440_250416.tf,target/kernels/moon_assoc_me.tf"

def crate_queries(
    mode: str, lines: list[str], spk: str, parameter: str
) -> list[list[str] | None]:
    command = ["cargo", "run", "--quiet", "--bin", "wgc_geometry", "--", mode]
    if mode in (
        "frame",
        "size",
        "elements",
        "subpoint",
        "longitude",
        "illum",
        "intercept",
        "tangent",
        "pointing",
        "pointing_full",
        "state_rep",
        "separation",
    ):
        command.append(parameter)
    if mode in ("tangent","subpoint","illum","intercept","pointing_full"):
        command.append(CLOCK_FK)
    process = subprocess.run(
        command,
        cwd=Path(__file__).resolve().parent.parent,
        input="\n".join(lines) + "\n",
        text=True,
        encoding="utf-8",
        errors="replace",
        capture_output=True,
        env={**__import__("os").environ, "STKCRAFT_SPK": spk},
    )
    if process.returncode != 0:
        raise RuntimeError(
            "cargo run failed\n"
            f"stdout:\n{process.stdout}\nstderr:\n{process.stderr}"
        )

    output: list[list[str] | None] = []
    for line in process.stdout.splitlines():
        line = line.strip()
        if not line:
            continue
        if line.startswith("ERROR"):
            output.append(None)
            continue
        output.append(line.split(","))
    return output


def run_state(wgc: Wgc, spk: str) -> tuple[float, float, float, int]:
    queries: list[str] = []
    expected: list[tuple[float, list[float]]] = []
    for label, target_name, target_id, observer_name, observer_id in STATE_CASES:
        for abcorr in ABCORR:
            rows = reference_result(wgc,
                {
                    "calculationType": "STATE_VECTOR",
                    "kernels": [{"type": "KERNEL_SET", "id": 1}],
                    "target": target_name,
                    "observer": observer_name,
                    "referenceFrame": "J2000",
                    "aberrationCorrection": abcorr,
                    "stateRepresentation": "RECTANGULAR",
                    **time_block(EPOCHS),
                }
            )["rows"]
            if len(rows) != len(EPOCHS):
                raise RuntimeError(f"{label}: WGC row count {len(rows)}")
            for et, row in zip(EPOCHS, rows):
                queries.append(f"{target_id},{observer_id},{et!r},{abcorr}")
                expected.append((et, row))

    values = crate_queries("state", queries, spk, "-")
    worst_position = 0.0
    worst_velocity = 0.0
    worst_light_time = 0.0
    worst_label = ""
    checked = 0

    for query, (et, row), ours in verified_pairs(queries, expected, values, "state"):
        if ours is None:
            raise RuntimeError(f"crate failed for {query}")
        mapping={"DISTANCE":11,"SPEED":12,"X":4,"Y":5,"Z":6,"D_X_DT":7,"D_Y_DT":8,"D_Z_DT":9,"TIME_AT_TARGET":13,"LIGHT_TIME":10}
        compare_mapped_row(state_column_ids("RECTANGULAR"),row,ours,mapping,query)
        # our columns: target,observer,et,abcorr,x,y,z,vx,vy,vz,lt,distance,speed,target_epoch
        position = ours[4:7]
        velocity = ours[7:10]
        light_time = ours[10]
        wgc_position = row[3:6]
        wgc_velocity = row[6:9]
        wgc_light_time = row[10]

        for axis in range(3):
            difference = abs(float(position[axis])-float(wgc_position[axis]))
            if difference > worst_position:
                worst_position = difference
                worst_label = f"{query} position axis {axis}"
            worst_velocity = max(worst_velocity, abs(float(velocity[axis])-float(wgc_velocity[axis])))
        worst_light_time = max(worst_light_time, abs(float(light_time)-float(wgc_light_time)))
        checked += 1

    return worst_position, worst_velocity, worst_light_time, checked


def run_state_reps(wgc: Wgc, spk: str, pck: str) -> tuple[float, float, int]:
    """Sweep every WebGeocalc state representation.

    Returns `(worst relative error, worst absolute error, query count)`.
    """
    all_reps = [
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
    # (target name, id, observer name, id, frame name, frame code, reps)
    cases = [
        ("MOON", 301, "EARTH", 399, "J2000", 0, [r for r in all_reps if "PLANETO" not in r[0]]),
        (
            "MOON",
            301,
            "EARTH",
            399,
            "ECLIPJ2000",
            -1,
            [r for r in all_reps if "PLANETO" not in r[0]],
        ),
        ("MOON", 301, "EARTH", 399, "IAU_MOON", 301, all_reps),
        ("EARTH", 399, "MOON", 301, "IAU_EARTH", 399, all_reps),
    ]

    queries: list[str] = []
    expected: list[tuple[list[str], list[float | str]]] = []
    tolerances = {
        "km": 1e-5,
        "km_per_s": 1e-8,
        "deg": 1e-8,
        "deg_per_s": 1e-8,
        "s": 1e-6,
    }
    units = state_rep_units()

    if QUICK: cases=cases[:1]

    for (
        target_name,
        target_id,
        observer_name,
        observer_id,
        frame_name,
        frame_code,
        reps,
    ) in cases:
        for representation, azccw, elplsz in reps:
            for abcorr in (("NONE",) if QUICK else ("NONE", "LT+S")):
                payload = {
                    "calculationType": "STATE_VECTOR",
                    "kernels": [{"type": "KERNEL_SET", "id": 1}],
                    "target": target_name,
                    "observer": observer_name,
                    "referenceFrame": frame_name,
                    "aberrationCorrection": abcorr,
                    "stateRepresentation": representation,
                    **time_block(EPOCHS),
                }
                if representation == "AZ_EL":
                    payload["azccwFlag"] = azccw
                    payload["elplszFlag"] = elplsz
                result = reference_result(wgc, payload)
                ids = [column["outputID"] for column in result["columns"]]
                if len(result["rows"]) != len(EPOCHS):
                    raise RuntimeError(f"{frame_name}/{representation}: wrong row count")
                for et, row in zip(EPOCHS, result["rows"]):
                    flags = f"{str(azccw).lower()},{str(elplsz).lower()}"
                    queries.append(
                        f"{target_id},{observer_id},{et!r},{abcorr},{frame_code},{representation},{flags}"
                    )
                    expected.append((ids, row))

    values = crate_queries("state_rep", queries, spk, pck)
    worst_relative = 0.0
    worst_absolute = 0.0
    worst_label = ""
    offenders: list[tuple[float, str]] = []
    checked = 0

    for query, (ids, row), ours in verified_pairs(queries, expected, values, "state_rep"):
        if ours is None:
            raise RuntimeError(f"crate failed for {query}")
        numeric_ids = [identifier for identifier in ids if identifier != "DATE"]
        numbers = row[1:]
        ours_numbers = ours[4:]
        if len(ours_numbers) != len(numeric_ids):
            raise RuntimeError(
                f"column count mismatch for {query}: ours {len(ours_numbers)} wgc {len(numeric_ids)}"
            )
        for identifier, expected_value, our_value in zip(
            numeric_ids, numbers, ours_numbers
        ):
            unit = units.get(identifier)
            if unit is None:
                raise RuntimeError(f"no unit known for column {identifier}")
            expected_number = expected_value
            difference = strict_difference(our_value, expected_number, f"{query}/{identifier}",output_id=identifier)
            tolerance = tolerances[unit]
            relative = difference / tolerance
            if relative > worst_relative:
                worst_relative = relative
                worst_absolute = difference
                worst_label = f"{query} {identifier} ours={our_value} wgc={expected_number}"
            if relative > 1.0:
                offenders.append(
                    (
                        relative,
                        f"{query} {identifier} ours={our_value!r} wgc={expected_number!r}",
                    )
                )
            checked += 1

    for relative, label in sorted(offenders, reverse=True)[:10]:
        print(f"  over tolerance ({relative:.2e}x): {label}")

    return worst_relative, worst_absolute, checked


def state_rep_units() -> dict[str, str]:
    return {
        "DISTANCE": "km",
        "SPEED": "km_per_s",
        "X": "km",
        "Y": "km",
        "Z": "km",
        "D_X_DT": "km_per_s",
        "D_Y_DT": "km_per_s",
        "D_Z_DT": "km_per_s",
        "RIGHT_ASCENSION": "deg",
        "DECLINATION": "deg",
        "RANGE": "km",
        "D_RIGHT_ASCENSION_DT": "deg_per_s",
        "D_DECLINATION_DT": "deg_per_s",
        "D_RANGE_DT": "km_per_s",
        "LONGITUDE": "deg",
        "LATITUDE": "deg",
        "COLATITUDE": "deg",
        "RADIUS": "km",
        "ALTITUDE": "km",
        "AZIMUTH": "deg",
        "ELEVATION": "deg",
        "D_LONGITUDE_DT": "deg_per_s",
        "D_LATITUDE_DT": "deg_per_s",
        "D_COLATITUDE_DT": "deg_per_s",
        "D_RADIUS_DT": "km_per_s",
        "D_ALTITUDE_DT": "km_per_s",
        "D_AZIMUTH_DT": "deg_per_s",
        "D_ELEVATION_DT": "deg_per_s",
        "D_RANGE_DT": "km_per_s",
        "TIME_AT_TARGET": "s",
        "LIGHT_TIME": "s",
    }


def run_separation(wgc: Wgc, spk: str, pck: str) -> tuple[float, int]:
    queries: list[str] = []
    expected: list[float] = []
    for label, name1, id1, name2, id2, observer_name, observer_id in SEPARATION_CASES:
        for abcorr in SEPARATION_ABCORR:
            for shape1, shape2 in SEPARATION_SHAPES:
                # Barycenters (ids 0..9) have no radii in the PCK, so
                # WebGeocalc rejects SPHERE for them as well, and an
                # observer sitting at a body's center is inside it.
                if shape1 == "SPHERE" and (0 <= id1 < 10 or id1 == observer_id):
                    continue
                if shape2 == "SPHERE" and (0 <= id2 < 10 or id2 == observer_id):
                    continue
                rows = reference_result(wgc,
                    {
                        "calculationType": "ANGULAR_SEPARATION",
                        "kernels": [{"type": "KERNEL_SET", "id": 1}],
                        "specType": "TWO_TARGETS",
                        "target1": name1,
                        "shape1": shape1,
                        "frame1": "J2000",
                        "target2": name2,
                        "shape2": shape2,
                        "frame2": "J2000",
                        "observer": observer_name,
                        "aberrationCorrection": abcorr,
                        **time_block(EPOCHS),
                    }
                )["rows"]
                for et, row in zip(EPOCHS, rows):
                    queries.append(
                        f"{observer_id},{id1},{id2},{et!r},{abcorr},{shape1},{shape2}"
                    )
                    expected.append(row[1])

    values = crate_queries("separation", queries, spk, pck)
    worst = 0.0
    checked = 0
    for query, wgc_degrees, ours in verified_pairs(queries, expected, values, "separation"):
        if ours is None:
            raise RuntimeError(f"crate failed for {query}")
        crates_degrees = ours[6]
        worst = max(worst, strict_difference(crates_degrees, wgc_degrees, f"{query}: numeric cell"))
        checked += 1
    return worst, checked


def run_phase(wgc: Wgc, spk: str) -> tuple[float, int]:
    queries: list[str] = []
    expected: list[float] = []
    for label, target_name, target_id, illum_name, illum_id, obs_name, obs_id in PHASE_CASES:
        for abcorr in PHASE_ABCORR:
            rows = reference_result(wgc,
                {
                    "calculationType": "PHASE_ANGLE",
                    "kernels": [{"type": "KERNEL_SET", "id": 1}],
                    "target": target_name,
                    "illuminator": illum_name,
                    "observer": obs_name,
                    "aberrationCorrection": abcorr,
                    **time_block(EPOCHS),
                }
            )["rows"]
            for et, row in zip(EPOCHS, rows):
                queries.append(f"{target_id},{illum_id},{obs_id},{et!r},{abcorr}")
                expected.append(row[1])

    values = crate_queries("phase", queries, spk, "-")
    worst = 0.0
    checked = 0
    for query, wgc_degrees, ours in verified_pairs(queries, expected, values, "phase"):
        if ours is None:
            raise RuntimeError(f"crate failed for {query}")
        worst = max(worst, strict_difference(ours[6], wgc_degrees, f"{query}: numeric cell"))
        checked += 1
    return worst, checked


def run_size(wgc: Wgc, spk: str, pck: str):
    frames={301:"IAU_MOON",399:"IAU_EARTH",10:"IAU_SUN"};queries=[];expected=[]
    for label,name,target,obsname,observer in SIZE_CASES:
        for method in ("MAX_RADIUS","MIN_RADIUS","LIMB_EXTENTS"):
            for correction in SIZE_ABCORR:
                payload={"calculationType":"ANGULAR_SIZE","kernels":[{"type":"KERNEL_SET","id":1}],"target":name,"observer":obsname,
                    "aberrationCorrection":correction,"computationMethod":method,**time_block(EPOCHS)}
                if method=="LIMB_EXTENTS":payload["targetFrame"]=frames[target]
                result=reference_result(wgc,payload);ids=[column["outputID"] for column in result["columns"]]
                for et,row in zip(EPOCHS,result["rows"]):
                    queries.append(f"{observer},{target},{et!r},{correction},{method}");expected.append((ids,row))
    values=crate_queries("size",queries,spk,pck);worst=0.;checked=0
    for query,(ids,row),ours in verified_pairs(queries,expected,values,"size"):
        mapping={"MIN_ANGULAR_SIZE":5,"MAX_ANGULAR_SIZE":6} if ids==["DATE","MIN_ANGULAR_SIZE","MAX_ANGULAR_SIZE"] else {"ANGULAR_SIZE":5}
        worst=max(worst,compare_mapped_row(ids,row,ours,mapping,query));checked+=1
    return worst,checked


def run_elements(wgc: Wgc, spk: str, pck: str) -> tuple[dict[str, float], int]:
    queries: list[str] = []
    expected: list[list] = []
    for label, orb_name, orb_id, center_name, center_id in ELEMENT_CASES:
        for abcorr in ELEMENT_ABCORR:
            rows = reference_result(wgc,
                {
                    "calculationType": "OSCULATING_ELEMENTS",
                    "kernels": [{"type": "KERNEL_SET", "id": 1}],
                    "orbitingBody": orb_name,
                    "centerBody": center_name,
                    "referenceFrame": "J2000",
                    "aberrationCorrection": abcorr,
                    **time_block(EPOCHS),
                }
            )["rows"]
            for et, row in zip(EPOCHS, rows):
                queries.append(f"{orb_id},{center_id},{et!r},{abcorr}")
                expected.append(row)

    values = crate_queries("elements", queries, spk, pck)
    worst = {
        "angles": 0.0,
        "distance": 0.0,
        "eccentricity": 0.0,
        "period": 0.0,
        "speed": 0.0,
        "gm": 0.0,
    }
    checked = 0
    for query, row, ours in verified_pairs(queries, expected, values, "elements"):
        if ours is None:
            raise RuntimeError(f"crate failed for {query}")
        # WGC: date, rp, e, inc, lnode, argp, M, range, speed, period, GM, nu, a
        # ours: ... rp, e, inc, lnode, argp, M, nu, a, period, range, speed, GM
        pairs_angle = [(row[3], ours[6]), (row[4], ours[7]), (row[5], ours[8]),
                       (row[6], ours[9]), (row[11], ours[10])]
        for wgc_value, crate_value in pairs_angle:
            worst["angles"] = max(worst["angles"], strict_difference(crate_value, wgc_value, f"{query}: numeric cell"))
        worst["distance"] = max(
            worst["distance"],
            strict_difference(ours[4], row[1], f"{query}: numeric cell"),
            strict_difference(ours[11], row[12], f"{query}: numeric cell"),
            strict_difference(ours[13], row[7], f"{query}: numeric cell"),
        )
        worst["eccentricity"] = max(worst["eccentricity"], strict_difference(ours[5], row[2], f"{query}: numeric cell"))
        worst["period"] = max(worst["period"], strict_difference(ours[12], row[9], f"{query}: numeric cell"))
        worst["speed"] = max(worst["speed"], strict_difference(ours[14], row[8], f"{query}: numeric cell"))
        worst["gm"] = max(worst["gm"], strict_difference(ours[15], row[10], "GM"))
        checked += 1
    return worst, checked


def run_solar_longitude(wgc: Wgc, spk: str, pck: str) -> tuple[float, int]:
    queries: list[str] = []
    expected: list[float] = []
    for name, body, frame in LONGITUDE_CASES:
        rows = reference_result(wgc,
            {
                "calculationType": "PLANETOCENTRIC_SOLAR_LONGITUDE",
                "kernels": [{"type": "KERNEL_SET", "id": 1}],
                "body": name,
                "targetFrame": frame,
                "aberrationCorrection": "NONE",
                **time_block(EPOCHS),
            }
        )["rows"]
        for et, row in zip(EPOCHS, rows):
            queries.append(f"{body},{et!r},NONE")
            expected.append(row[1])

    values = crate_queries("longitude", queries, spk, pck)
    worst = 0.0
    checked = 0
    for query, wgc_degrees, ours in verified_pairs(queries, expected, values, "longitude"):
        if ours is None:
            raise RuntimeError(f"crate failed for {query}")
        worst = max(worst, strict_difference(ours[3], wgc_degrees, f"{query}: numeric cell"))
        checked += 1
    return worst, checked


def run_illumination(wgc: Wgc, spk: str, pck: str):
    queries=[]; expected=[]
    for name,target,obsname,observer,frame,(lon,lat,alt) in ILLUMINATION_CASES:
        result=reference_result(wgc,{"calculationType":"ILLUMINATION_ANGLES","kernels":[{"type":"KERNEL_SET","id":1}],
            "target":name,"observer":obsname,"targetFrame":frame,"shape1":"ELLIPSOID","coordinateRepresentation":"PLANETODETIC",
            "longitude":lon,"latitude":lat,"altitude":alt,"aberrationCorrection":"NONE",**time_block(EPOCHS)})
        ids=[column["outputID"] for column in result["columns"]]
        for et,row in zip(EPOCHS,result["rows"]):
            queries.append(f"{target},{observer},{lon!r},{lat!r},{alt!r},{et!r},NONE,10,{frame}");expected.append((ids,row))
    values=crate_queries("illum",queries,spk,pck);worst=0.;checked=0
    mapping={"INCIDENCE_ANGLE":7,"EMISSION_ANGLE":8,"PHASE_ANGLE":9,"OBSERVER_ALTITUDE":17,"TIME_AT_POINT":10,"LIGHT_TIME":11,"LTST":18}
    for query,(ids,row),ours in verified_pairs(queries,expected,values,"illum"):
        worst=max(worst,compare_mapped_row(ids,row,ours,mapping,query));checked+=1
    return worst,checked


def run_surface_intercept(wgc: Wgc, spk: str, pck: str):
    queries=[];expected=[]
    for name,target,obsname,observer,frame in INTERCEPT_CASES:
        result=reference_result(wgc,{"calculationType":"SURFACE_INTERCEPT_POINT","kernels":[{"type":"KERNEL_SET","id":1}],
            "target":name,"observer":obsname,"targetFrame":frame,"shape1":"ELLIPSOID","stateRepresentation":"RECTANGULAR",
            "aberrationCorrection":"NONE","directionVectorType":"DIRECTION_TO_OBJECT","directionObject":name,**time_block(EPOCHS)})
        ids=[column["outputID"] for column in result["columns"]]
        for et,row in zip(EPOCHS,result["rows"]):
            queries.append(f"{target},{observer},{et!r},NONE,{frame}");expected.append((ids,row))
    values=crate_queries("intercept",queries,spk,pck);worst=0.;checked=0
    mapping={"X":4,"Y":5,"Z":6,"INTERCEPT_RADIUS":8,"OBSERVER_ALTITUDE":7,"INCIDENCE_ANGLE":9,"EMISSION_ANGLE":10,"PHASE_ANGLE":11,"TIME_AT_POINT":12,"LIGHT_TIME":13,"LTST":14}
    for query,(ids,row),ours in verified_pairs(queries,expected,values,"intercept"):
        worst=max(worst,compare_mapped_row(ids,row,ours,mapping,query));checked+=1
    return worst,checked


def run_tangent(wgc: Wgc, spk: str, pck: str, target_frame="IAU_MOON"):
    queries=[];expected=[]
    for et in EPOCHS:
        for corr in (("NONE",) if QUICK else ("NONE","LT+S","CN+S")):
            result=reference_result(wgc,{"calculationType":"TANGENT_POINT","kernels":[{"type":"KERNEL_SET","id":1}],
                "target":"MOON","observer":"EARTH","targetFrame":target_frame,"aberrationCorrection":corr,"computationMethod":"ELLIPSOID",
                "correctionLocus":"TANGENT_POINT","directionVectorType":"DIRECTION_TO_OBJECT","directionObject":"MOON","coordinateRepresentation":"PLANETOGRAPHIC",**time_block([et])})
            ids=[column["outputID"] for column in result["columns"]]
            queries.append(f"301,399,{et!r},{corr},0.0,0.0,0.0,TANGENT_POINT,{target_frame}");expected.append((ids,result["rows"][0]))
    values=crate_queries("tangent",queries,spk,pck);worst=0.;checked=0
    mapping={"TANGENT_POINT_GEOMETRY":23,"TANGENT_POINT_LONGITUDE":14,"TANGENT_POINT_LATITUDE":15,"TANGENT_POINT_ALTITUDE":7,
        "SURFACE_POINT_LONGITUDE":17,"SURFACE_POINT_LATITUDE":18,"SURFACE_POINT_ALTITUDE":19,"DISTANCE_TO_TANGENT_POINT":8,
        "INCIDENCE_ANGLE":24,"EMISSION_ANGLE":25,"PHASE_ANGLE":26,"TIME_AT_POINT":12,"LIGHT_TIME":13,"LTST":27}
    for query,(ids,row),ours in verified_pairs(queries,expected,values,"tangent"):
        worst=max(worst,compare_mapped_row(ids,row,ours,mapping,query));checked+=1
    return worst,checked


def run_subpoints(wgc,spk,pck,solar):
    queries=[];expected=[]
    kind="SUB_SOLAR_POINT" if solar else "SUB_OBSERVER_POINT"
    for et in EPOCHS:
        for frame in ("IAU_MOON","MOON_ME"):
            for method,cli_method in (("Near point: ellipsoid","NEAR_POINT_ELLIPSOID"),("Intercept: ellipsoid","INTERCEPT_ELLIPSOID")):
                corr="NONE" if QUICK else "CN+S"
                payload={"calculationType":kind,"kernels":[{"type":"KERNEL_SET","id":1}],"target":"MOON","observer":"EARTH","targetFrame":frame,
                         "aberrationCorrection":corr,"subPointType":method,"stateRepresentation":"RECTANGULAR",**time_block([et])}
                result=reference_result(wgc,payload);ids=[c["outputID"] for c in result["columns"]]
                queries.append(f"{'solar' if solar else 'observer'},301,399,{et!r},{corr},{cli_method},{frame}");expected.append((ids,result["rows"][0]))
    values=crate_queries("subpoint",queries,spk,pck);worst=0.;checked=0
    mapping={"X":5,"Y":6,"Z":7,"SUB_POINT_RADIUS":14,"OBSERVER_ALTITUDE":8,"INCIDENCE_ANGLE":15,"EMISSION_ANGLE":16,"PHASE_ANGLE":17,"TIME_AT_POINT":9,"LIGHT_TIME":10}
    if not solar:mapping["LTST"]=18
    for query,(ids,row),ours in verified_pairs(queries,expected,values,"subpoint"):
        worst=max(worst,compare_mapped_row(ids,row,ours,mapping,query));checked+=1
    return worst,checked

def run_sub_observer(wgc,spk,pck):return run_subpoints(wgc,spk,pck,False)
def run_sub_solar(wgc,spk,pck):return run_subpoints(wgc,spk,pck,True)

POINTING_OUTPUTS={"RECTANGULAR":["X","Y","Z"],"RA_DEC":["RIGHT_ASCENSION","DECLINATION","RANGE"],
                  "LATITUDINAL":["LONGITUDE","LATITUDE","RADIUS"],"CYLINDRICAL":["LONGITUDE","Z","RADIUS"],
                  "SPHERICAL":["LONGITUDE","COLATITUDE","RADIUS"],"AZ_EL":["AZIMUTH","ELEVATION","RANGE"]}

def run_pointing(wgc,spk,pck):
    queries=[];expected=[]
    # Six coordinate systems, both magnitudes, all three direction semantics,
    # differing input/output frame centers, and anti applied after stellar correction.
    cases=[
      ("POSITION",301,399,0.,"NONE","J2000","MOON_ME","PRESERVE_ORIGINAL","RECTANGULAR",False,"VECTOR",[1.,2.,3.]),
      ("POSITION",301,399,0.,"LT+S","J2000","J2000","UNIT","RA_DEC",False,"VECTOR",[1.,2.,3.]),
      ("VELOCITY",10,399,5e8,"CN+S","IAU_MOON","IAU_EARTH","PRESERVE_ORIGINAL","LATITUDINAL",False,"VECTOR",[1.,2.,3.]),
      ("VELOCITY",301,399,0.,"NONE","B1950","J2000","UNIT","CYLINDRICAL",False,"VECTOR",[1.,2.,3.]),
      ("VECTOR",0,399,5e8,"LT","MOON_ME","IAU_EARTH","PRESERVE_ORIGINAL","SPHERICAL",True,"VECTOR",[3.,4.,5.]),
      ("VECTOR",0,399,0.,"S","J2000","MOON_ME","UNIT","AZ_EL",True,"VECTOR",[3.,4.,5.]),
      ("VECTOR",0,399,5e8,"S","J2000","J2000","PRESERVE_ORIGINAL","RECTANGULAR",True,"AXIS",[1.,0.,0.]),
    ]
    for kind,target,observer,et,corr,input_frame,output_frame,magnitude,representation,anti,ray,vector in cases:
        direction={"directionType":kind,"observer":str(observer),"aberrationCorrection":corr,"antiVectorFlag":anti}
        if kind in ("POSITION","VELOCITY"):direction["target"]=str(target)
        if kind=="VELOCITY":direction["referenceFrame"]=input_frame
        if kind=="VECTOR":
            direction.update({"directionVectorType":"REFERENCE_FRAME_AXIS" if ray=="AXIS" else "VECTOR_IN_REFERENCE_FRAME","directionFrame":input_frame})
            if ray=="AXIS":direction["directionFrameAxis"]="X"
            else:direction.update({"directionVectorX":vector[0],"directionVectorY":vector[1],"directionVectorZ":vector[2]})
        payload={"calculationType":"POINTING_DIRECTION","kernels":[{"type":"KERNEL_SET","id":1}],"direction":direction,"referenceFrame":output_frame,
                 "coordinateRepresentation":representation,"vectorMagnitude":magnitude,**time_block([et])}
        if representation=="AZ_EL":payload.update({"azccwFlag":False,"elplszFlag":False})
        result=reference_result(wgc,payload);ids=[c["outputID"] for c in result["columns"]]
        if ids!=["DATE"]+POINTING_OUTPUTS[representation]:raise RuntimeError(f"pointing IDs/order mismatch: {ids}")
        query=[kind,str(target),str(observer),repr(et),corr,input_frame,output_frame,magnitude,representation,str(anti).lower(),ray,*map(repr,vector),"X","false","false"]
        queries.append(",".join(query));expected.append((ids,result["rows"][0]))
    values=crate_queries("pointing_full",queries,spk,pck);worst=0.;checked=0
    for query,(ids,row),ours in verified_pairs(queries,expected,values,"pointing_full"):
        if finite_decimal(ours[4],"pointing boundary index")!=0:raise RuntimeError("unexpected pointing boundary row")
        worst=max(worst,compare_mapped_row(ids,row,ours,{i:6+n for n,i in enumerate(ids[1:])},query));checked+=1
    return worst,checked


def crate_frame_rep(
    lines: list[str], fk: str, pck: str
) -> list[dict[str, list[str]] | None]:
    """Runs `wgc-geometry frame_rep`, which prints several rows per query."""
    command = [
        "cargo",
        "run",
        "--quiet",
        "--bin",
        "wgc_geometry",
        "--",
        "frame_rep",
        fk,
        pck,
    ]
    process = subprocess.run(
        command,
        cwd=Path(__file__).resolve().parent.parent,
        input="\n".join(lines) + "\n",
        text=True,
        encoding="utf-8",
        errors="replace",
        capture_output=True,
        env={**__import__("os").environ,"STKCRAFT_SPK":ACTIVE_SPK},
    )
    if process.returncode != 0:
        raise RuntimeError(
            "cargo run failed\n"
            f"stdout:\n{process.stdout}\nstderr:\n{process.stderr}"
        )

    output: list[dict[str, list[str]] | None] = []
    for line in process.stdout.splitlines():
        line = line.strip()
        if not line:
            continue
        if line.startswith("ERROR"):
            output.append(None)
            continue
        fields = line.split(",")
        if fields[0] == "COLS":
            output.append({"ids": fields[1:], "rows": []})
        elif fields[0] == "ROW":
            if not output or output[-1] is None:
                raise RuntimeError(f"row without columns: {line}")
            output[-1]["rows"].append(fields[1:])  # type: ignore[index]
        else:
            raise RuntimeError(f"unexpected line: {line}")
    return output


def run_frames(wgc: Wgc, fk: str, pck: str) -> tuple[float, int, list[str]]:
    """Sweeps all seven orientations by all five angular velocities."""
    queries: list[str] = []
    expected: list[dict] = []
    labels: list[str] = []

    for frame1, frame2, et in FRAME_CASES:
        for orientation, orientation_axes in FRAME_REPRESENTATIONS:
            for velocity, velocity_axes in FRAME_VELOCITIES:
                if velocity == "EULER_ANGLE_DERIVATIVES" and orientation != "EULER_ANGLES":
                    # WebGeocalc rejects this pairing, and so does the crate.
                    continue

                payload: dict = {
                    "calculationType": "FRAME_TRANSFORMATION",
                    "kernels": [{"type": "KERNEL_SET", "id": 1}],
                    "aberrationCorrection": "NONE",
                    "frame1": frame1,
                    "frame2": frame2,
                    "orientationRepresentation": orientation,
                    "angularVelocityRepresentation": velocity,
                    **time_block([et]),
                }
                if orientation_axes != "-":
                    for index, axis in enumerate(orientation_axes, start=1):
                        payload[f"axis{index}"] = axis
                if orientation in ("EULER_ANGLES", "ANGLE_AND_AXIS"):
                    payload["angularUnits"] = "deg"
                if velocity in (
                    "VECTOR_IN_FRAME1",
                    "VECTOR_IN_FRAME2",
                    "EULER_ANGLE_DERIVATIVES",
                ):
                    payload["angularVelocityUnits"] = "deg/s"

                label = f"{frame1}->{frame2} {orientation}/{velocity}"
                expected.append(reference_result(wgc, payload))
                queries.append(
                    ",".join(
                        [
                            FRAME_NAMES.get(frame1, frame1),
                            FRAME_NAMES.get(frame2, frame2),
                            repr(float(et)),
                            orientation,
                            orientation_axes,
                            velocity,
                            velocity_axes,
                            "deg",
                        ]
                    )
                )
                labels.append(label)

    values = crate_frame_rep(queries, fk, pck)

    worst = 0.0
    checked = 0
    failures: list[str] = []
    for label, result, ours in verified_pairs(labels, expected, values, "frames"):
        if ours is None:
            failures.append(f"{label}: crate rejected the query")
            continue

        worst = max(worst, compare_table(result, ours, label))
        checked += 1

    return worst, checked, failures


def crate_frame(name: str) -> str:
    """WebGeocalc names a body-fixed frame, this crate wants the NAIF id."""
    return FRAME_NAMES.get(name, name)


def run_locations(wgc: Wgc, spk: str, pck: str) -> tuple[float, int, list[str]]:
    """State vectors with a point endpoint and the topocentric frame."""
    earth = "399"
    moon = "301"
    # -118 deg, 34 deg, 0.3 km above the Earth's reference spheroid, in
    # IAU_EARTH: the same station the azlcpo golden file uses.
    station = "-2485.1508717620527,-4673.889014901399,3546.614084137043"
    camera = "1000.0,2000.0,1500.0"
    spacecraft = "7000.0,100.0,200.0"
    spacecraft_velocity = "0.01,7.4,0.2"

    queries: list[str] = []
    payloads: list[dict] = []
    labels: list[str] = []

    def add(label: str, location: dict, time_et: float = 0.0) -> None:
        labels.append(label)
        payloads.append(location)
        queries.append(
            ",".join(
                [
                    location["targetType"],
                    str(location.get("target", "0")),
                    str(location.get("targetCenter", "0")),
                    crate_frame(location.get("targetFrame", "J2000")),
                    location.get("targetPosition", "0.0,0.0,0.0"),
                    location.get("targetVelocity", "0.0,0.0,0.0"),
                    repr(location.get("targetEpoch", 0.0)),
                    location["observerType"],
                    str(location.get("observer", "0")),
                    str(location.get("observerCenter", "0")),
                    crate_frame(location.get("observerFrame", "J2000")),
                    location.get("observerPosition", "0.0,0.0,0.0"),
                    location.get("observerVelocity", "0.0,0.0,0.0"),
                    repr(location.get("observerEpoch", 0.0)),
                    crate_frame(location["outref"]),
                    location["frameType"],
                    location["frameLocus"],
                    location["abcorr"],
                    location["representation"],
                    "1" if location.get("azccw", True) else "0",
                    "1" if location.get("elplsz", True) else "0",
                    repr(float(time_et)),
                ]
            )
        )

    for abcorr in ("NONE", "LT", "LT+S", "CN+S"):
        for locus in ("CENTER", "OBSERVER", "TARGET"):
            add(
                f"point observer {abcorr} {locus}",
                {
                    "targetType": "OBJECT",
                    "target": moon,
                    "observerType": "FIXED_POINT",
                    "observerCenter": earth,
                    "observerFrame": "IAU_EARTH",
                    "observerPosition": station,
                    "outref": "IAU_EARTH",
                    "frameType": "SPICE_FRAME",
                    "frameLocus": locus,
                    "abcorr": abcorr,
                    "representation": "RECTANGULAR",
                },
            )

    for abcorr in ("NONE", "LT", "LT+S"):
        add(
            f"point target {abcorr}",
            {
                "targetType": "FIXED_POINT",
                "targetCenter": moon,
                "targetFrame": "IAU_MOON",
                "targetPosition": camera,
                "observerType": "OBJECT",
                "observer": earth,
                "outref": "J2000",
                "frameType": "SPICE_FRAME",
                "frameLocus": "CENTER",
                "abcorr": abcorr,
                "representation": "RECTANGULAR",
            },
        )
        add(
            f"cv point target {abcorr}",
            {
                "targetType": "CONSTANT_VELOCITY_POINT",
                "targetCenter": earth,
                "targetFrame": "J2000",
                "targetPosition": spacecraft,
                "targetVelocity": spacecraft_velocity,
                "targetEpoch": 0.0,
                "observerType": "OBJECT",
                "observer": earth,
                "outref": "J2000",
                "frameType": "SPICE_FRAME",
                "frameLocus": "CENTER",
                "abcorr": abcorr,
                "representation": "RECTANGULAR",
            },
        )
        add(
            f"cv point observer {abcorr}",
            {
                "targetType": "OBJECT",
                "target": moon,
                "observerType": "CONSTANT_VELOCITY_POINT",
                "observerCenter": earth,
                "observerFrame": "J2000",
                "observerPosition": spacecraft,
                "observerVelocity": spacecraft_velocity,
                "observerEpoch": 0.0,
                "outref": "J2000",
                "frameType": "SPICE_FRAME",
                "frameLocus": "CENTER",
                "abcorr": abcorr,
                "representation": "RECTANGULAR",
            },
        )

    for abcorr in ("NONE", "LT+S"):
        for azccw in (False, True):
            for elplsz in (False, True):
                add(
                    f"topocentric {abcorr} azccw={azccw} elplsz={elplsz}",
                    {
                        "targetType": "OBJECT",
                        "target": moon,
                        "observerType": "FIXED_POINT",
                        "observerCenter": earth,
                        "observerFrame": "IAU_EARTH",
                        "observerPosition": station,
                        "outref": "IAU_EARTH",
                        "frameType": "TOPOCENTRIC_AT_POINT",
                        "frameLocus": "CENTER",
                        "abcorr": abcorr,
                        "representation": "AZ_EL",
                        "azccw": azccw,
                        "elplsz": elplsz,
                    },
                )

    if QUICK:
        selected=[0,12,13,21]
        queries=[queries[i] for i in selected]
        payloads=[payloads[i] for i in selected]
        labels=[labels[i] for i in selected]
    values = crate_location_queries(queries, spk, pck)

    worst = 0.0
    checked = 0
    failures: list[str] = []
    for label, payload, ours in verified_pairs(labels, payloads, values, "locations"):
        request = {
            "calculationType": "STATE_VECTOR",
            "kernels": [{"type": "KERNEL_SET", "id": 1}],
            "targetType": payload["targetType"],
            "observerType": payload["observerType"],
            "referenceFrame": payload["outref"],
            "aberrationCorrection": payload["abcorr"],
            "frameLocus": payload["frameLocus"],
            "stateRepresentation": payload["representation"],
            **time_block([0.0]),
        }
        if payload["targetType"] == "OBJECT":
            request["target"] = int(payload["target"])
        else:
            request["targetLocation"] = location_dictionary(payload, "target")
        if payload["observerType"] == "OBJECT":
            request["observer"] = int(payload["observer"])
        else:
            request["observerLocation"] = location_dictionary(payload, "observer")
        if payload["frameType"] == "TOPOCENTRIC_AT_POINT":
            request["frameType"] = "TOPOCENTRIC_AT_POINT"
            request["computationMethod"] = "ELLIPSOID"
            request["azccwFlag"] = payload["azccw"]
            request["elplszFlag"] = payload["elplsz"]

        if ours is None:
            failures.append(f"{label}: crate rejected the query")
            continue
        result = reference_result(wgc, request)
        worst = max(worst, compare_table(result, ours, label))
        checked += 1

    return worst, checked, failures


def location_dictionary(payload: dict, prefix: str) -> dict:
    body = int(payload[f"{prefix}Center"])
    names = {399: "EARTH", 301: "MOON", 10: "SUN"}
    values = [float(value) for value in payload[f"{prefix}Position"].split(",")]
    entry = {
        "centerBody": names.get(body, str(body)),
        "referenceFrame": payload[f"{prefix}Frame"],
        "coordinateRepresentation": "RECTANGULAR",
        "x": values[0],
        "y": values[1],
        "z": values[2],
    }
    if f"{prefix}Velocity" in payload:
        velocity = [float(value) for value in payload[f"{prefix}Velocity"].split(",")]
        entry.update(
            {
                "dxdt": velocity[0],
                "dydt": velocity[1],
                "dzdt": velocity[2],
                "timeSystem": "TDB",
                "timeFormat": "SECONDS_PAST_J2000",
                "epoch": repr(float(payload.get(f"{prefix}Epoch", 0.0))),
            }
        )
    return entry


def crate_location_queries(
    lines: list[str], spk: str, pck: str
) -> list[dict[str, list[str]] | None]:
    command = [
        "cargo",
        "run",
        "--quiet",
        "--bin",
        "wgc_geometry",
        "--",
        "location",
        pck,
    ]
    process = subprocess.run(
        command,
        cwd=Path(__file__).resolve().parent.parent,
        input="\n".join(lines) + "\n",
        text=True,
        encoding="utf-8",
        errors="replace",
        capture_output=True,
        env={**__import__("os").environ, "STKCRAFT_SPK": spk},
    )
    if process.returncode != 0:
        raise RuntimeError(
            "cargo run failed\n"
            f"stdout:\n{process.stdout}\nstderr:\n{process.stderr}"
        )

    output: list[dict[str, list[str]] | None] = []
    for line in process.stdout.splitlines():
        line = line.strip()
        if not line:
            continue
        if line.startswith("ERROR"):
            output.append(None)
            continue
        fields = line.split(",")
        if fields[0] == "COLS":
            output.append({"ids": fields[1:], "rows": []})
        elif fields[0] == "ROW":
            if not output or output[-1] is None:
                raise RuntimeError(f"row without columns: {line}")
            output[-1]["rows"].append(fields[1:])  # type: ignore[index]
        else:
            raise RuntimeError(f"unexpected line: {line}")
    return output


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--spk", default="target/kernels/de442.bsp")
    parser.add_argument("--pck", default="target/kernels/pck00011.tpc,target/kernels/mars_iau2000_v1.tpc,target/kernels/gm_de431.tpc,target/kernels/moon_pa_de440_200625.bpc")
    parser.add_argument("--clock-fk", default="target/kernels/moon_de440_250416.tf,target/kernels/moon_assoc_me.tf", help="explicit FK association used by tangent ET2LST metadata")
    parser.add_argument("--fk", default="-")
    parser.add_argument("--insecure", action="store_true")
    parser.add_argument("--timeout", type=float, default=120.0)
    parser.add_argument("--diagnostic-tolerances", action="store_true", help="measure raw differences only; never establishes parity")
    parser.add_argument("--wgc-output-exact",action="store_true",help="explicit exact equality after independently verified NASA output formatting; does not establish CSPICE bit parity")
    parser.add_argument("--curl",action="store_true",help="use curl for NASA HTTPS transport")
    parser.add_argument("--proxy",help="optional transport proxy")
    parser.add_argument("--replay",type=Path,help="replay completed calculations from a genuine raw NASA request trace; missing requests fail")
    parser.add_argument("--trace",type=Path,help="append raw NASA requests/responses as JSONL")
    parser.add_argument("--quick",action="store_true",help="explicit representative sweep of every group; report records reduced case coverage")
    parser.add_argument("--report", type=Path, help="save complete failures and raw differences as JSON")
    args = parser.parse_args()
    global STRICT, STRICT_FAILURES, QUICK, EPOCHS, ABCORR, WGC_OUTPUT_EXACT, CLOCK_FK, ACTIVE_SPK
    STRICT = not args.diagnostic_tolerances
    if not math.isfinite(args.timeout) or args.timeout <= 0:
        parser.error("--timeout must be finite and positive")
    WGC_OUTPUT_EXACT=args.wgc_output_exact
    CLOCK_FK=args.clock_fk
    ACTIVE_SPK=args.spk
    if WGC_OUTPUT_EXACT and args.diagnostic_tolerances: parser.error("output-exact and diagnostic modes are separate")
    QUICK=args.quick
    if QUICK:
        EPOCHS=[0.0]
        ABCORR=["NONE"]
        for key in ("STATE_CASES","SEPARATION_CASES","PHASE_CASES","SIZE_CASES","ELEMENT_CASES","LONGITUDE_CASES","ILLUMINATION_CASES","INTERCEPT_CASES","FRAME_CASES"):
            globals()[key]=globals()[key][:1]
        for key in ("SEPARATION_ABCORR","PHASE_ABCORR","SIZE_ABCORR"):
            globals()[key]=["NONE"]
    wgc = Wgc(args.insecure, args.timeout,curl=args.curl,proxy=args.proxy,trace=args.trace,replay=args.replay)
    groups = [
        ("State Vector", run_state, (wgc, args.spk)),
        ("State Representations", run_state_reps, (wgc, args.spk, args.pck)),
        ("Angular Separation", run_separation, (wgc, args.spk, args.pck)),
        ("Phase Angle", run_phase, (wgc, args.spk)),
        ("Angular Size", run_size, (wgc, args.spk, args.pck)),
        ("Osculating Elements", run_elements, (wgc, args.spk, args.pck)),
        ("Planetocentric Solar Longitude", run_solar_longitude, (wgc, args.spk, args.pck)),
        ("Illumination Angles", run_illumination, (wgc, args.spk, args.pck)),
        ("Surface Intercept", run_surface_intercept, (wgc, args.spk, args.pck)),
        ("Tangent Point", run_tangent, (wgc, args.spk, args.pck)),
        ("Sub-observer Point", run_sub_observer, (wgc,args.spk,args.pck)),
        ("Sub-solar Point", run_sub_solar, (wgc,args.spk,args.pck)),
        ("Pointing Direction", run_pointing, (wgc,args.spk,args.pck)),
        ("Frame Transformation", run_frames, (wgc, args.fk, args.pck)),
        ("Locations", run_locations, (wgc, args.spk, args.pck)),
    ]
    diagnostic_bounds = {
        "State Vector": {"position_km": TOLERANCE_POSITION_KM, "velocity_km_s": TOLERANCE_VELOCITY_KMS, "light_time_s": TOLERANCE_LIGHT_TIME_S},
        "State Representations": {"normalized_column_error": 1.0},
        "Angular Separation": {"degrees": TOLERANCE_SEPARATION_DEG},
        "Phase Angle": {"degrees": TOLERANCE_ANGLE_DEG},
        "Angular Size": {"degrees": TOLERANCE_SIZE_DEG},
        "Osculating Elements": {"angles_deg": TOLERANCE_ANGLE_DEG, "distance_km": TOLERANCE_ELEMENT_KM, "eccentricity": TOLERANCE_ECCENTRICITY, "period_s": TOLERANCE_PERIOD_S, "gm": 0.0, "speed_km_s": 0.0},
        "Planetocentric Solar Longitude": {"degrees": TOLERANCE_ANGLE_DEG},
        "Illumination Angles": {"degrees": TOLERANCE_ANGLE_DEG},
        "Surface Intercept": {"km": TOLERANCE_ELEMENT_KM},
        "Tangent Point": {"angle_deg": 1e-4, "distance_km": 5e-3},
        "Sub-observer Point": {}, "Sub-solar Point": {}, "Pointing Direction": {},
        "Frame Transformation": {"numeric_cell": TOLERANCE_FRAME_REP},
        "Locations": {"numeric_cell": TOLERANCE_FRAME_REP},
    }
    report = {"spk":args.spk,"pck":args.pck,"fk":args.fk,"clock_fk":args.clock_fk,"strict": STRICT, "wgc_output_format_equality": WGC_OUTPUT_EXACT, "reference": API_BASE, "representative_sweep": QUICK, "raw_reference_replay": str(args.replay) if args.replay else None, "precision_claim": ("exact equality after verified WGC output serialization; does not prove high precision CSPICE parity" if WGC_OUTPUT_EXACT else "exact equality to transported reference values; does not prove high precision CSPICE parity"), "groups": []}
    for label, runner, arguments in groups:
        print(f"=== {label} ===", flush=True)
        STRICT_FAILURES = []
        record = {"group": label, "failures": STRICT_FAILURES, "diagnostic_bounds_only": diagnostic_bounds[label]}
        try:
            result = runner(*arguments)
            if isinstance(result[-1], list):
                checked = result[-2]
                STRICT_FAILURES.extend(result[-1])
            else:
                checked = result[-1]
            if checked <= 0:
                raise RuntimeError("zero comparisons cannot establish parity")
            record["checked"] = checked
            record["raw_result"] = result
        except Exception as error:
            STRICT_FAILURES.append(f"{type(error).__name__}: {error}")
        record["passed"] = STRICT and not STRICT_FAILURES
        print("  " + (("PASS (WGC output formatting equality)" if WGC_OUTPUT_EXACT else "PASS (exact transported values)") if record["passed"] else "FAIL" if STRICT else "DIAGNOSTIC ONLY"))
        if not STRICT:
            print(f"  historical diagnostic bounds (never parity): {diagnostic_bounds[label]!r}")
        if "raw_result" in record:
            print(f"  raw results: {record['raw_result']!r}")
        for failure in STRICT_FAILURES[:20]:
            print(f"  {failure}")
        if len(STRICT_FAILURES) > 20:
            print(f"  {len(STRICT_FAILURES)-20} additional failures (use --report)")
        report["groups"].append(record)
    STRICT_FAILURES = None
    report["passed"] = STRICT and all(group["passed"] for group in report["groups"])
    if args.report:
        args.report.parent.mkdir(parents=True, exist_ok=True)
        args.report.write_text(json.dumps(report, indent=2, allow_nan=False), encoding="utf-8")
    print("OVERALL: " + (("PASS (WGC output formatting equality only)" if WGC_OUTPUT_EXACT else "PASS (exact transported values only)") if report["passed"] else "FAIL"))
    return 0 if report["passed"] else 1


if __name__ == "__main__":
    try:
        sys.exit(main())
    except (RuntimeError, ValueError, KeyError, TypeError, IndexError, TimeoutError) as error:
        print(f"OVERALL: FAIL: {error}", file=sys.stderr)
        sys.exit(1)
