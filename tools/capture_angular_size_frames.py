"""Capture CSPICE limb-axis widths for the explicit Earth-frame regression.

Run in a provisioned checkout with SpiceyPy and the recorded local kernels.
The output is independent native evidence, not unrounded WGC output.
"""
import hashlib
import json
import math
from pathlib import Path

import spiceypy as spice

ROOT = Path(__file__).resolve().parents[1]
files = ['de442.bsp','pck00011.tpc','earth_1962_260806_2126_combined.bpc']
kernels = [ROOT/'target/kernels'/name for name in files]
for kernel in kernels:
    spice.furnsh(str(kernel))
radii = spice.bodvrd('EARTH','RADII',3)[1]
rows = []
for frame in ['IAU_EARTH','ITRF93']:
    for correction in ['NONE','LT+S','CN+S']:
        for epoch in [0.0,100000000.0,500000000.0]:
            position,light_time = spice.spkpos('EARTH',epoch,'J2000',correction,'MOON')
            target_epoch = epoch if correction == 'NONE' else epoch-light_time
            view = -spice.mxv(spice.pxform('J2000',frame,target_epoch),position)
            limb = spice.edlimb(*radii,view)
            distance = spice.vnorm(spice.vsub(limb.center,view))
            widths = [2.0*math.atan2(spice.vnorm(axis),distance)
                      for axis in (limb.semi_minor,limb.semi_major)]
            rows.append({'frame':frame,'correction':correction,'et':epoch,
                         'width_rad_hex':[value.hex() for value in widths]})
result = {'cspice':spice.tkvrsn('TOOLKIT'),
          'kernels':{path.name:hashlib.sha256(path.read_bytes()).hexdigest() for path in kernels},
          'rows':rows}
print(json.dumps(result,indent=2))
