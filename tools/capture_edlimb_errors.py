"""Capture CSPICE edlimb_c failures for invalid or degenerate ellipsoids."""
import hashlib
import json
from pathlib import Path
import spiceypy as spice
from spiceypy.utils.libspicehelper import libspice_path

cases=[([0.0,1.0,1.0],[3.0,0.0,0.0]),([-1.0,1.0,1.0],[3.0,0.0,0.0]),
       ([1.0,1.0,1.0],[0.0,0.0,0.0]),([1e-200,1.0,1.0],[3.0,0.0,0.0])]
rows=[]
for radii,view in cases:
    try:
        spice.edlimb(*radii,view)
    except spice.utils.exceptions.SpiceyError as error:
        rows.append({'radii':radii,'view':view,'short':error.short.strip()})
    else:
        raise RuntimeError(f'CSPICE unexpectedly accepted {radii}, {view}')
lib=Path(libspice_path)
print(json.dumps({'cspice':spice.tkvrsn('TOOLKIT'),
                  'library_sha256':hashlib.sha256(lib.read_bytes()).hexdigest(),
                  'rows':rows},indent=2))
