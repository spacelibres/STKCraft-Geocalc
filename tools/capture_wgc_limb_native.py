"""Capture CSPICE limb ellipse-axis widths; this is not a WGC raw result.

Run with SpiceyPy on Windows using the local DE442 and pck00011 kernels.
The first case supplies the binary64 fixture in tests/angular_size_wgc.rs.
"""
import math

import spiceypy as spice

for path in ('target/kernels/de442.bsp', 'target/kernels/pck00011.tpc'):
    spice.furnsh(path)
for epoch, correction in ((0.0, 'NONE'), (0.0, 'LT+S'), (500000000.0, 'LT+S')):
    position, light_time = spice.spkpos('EARTH', epoch, 'J2000', correction, 'SUN')
    target_epoch = epoch if correction == 'NONE' else epoch - light_time
    rotation = spice.pxform('J2000', 'IAU_EARTH', target_epoch)
    view = -spice.mxv(rotation, position)
    radii = spice.bodvrd('EARTH', 'RADII', 3)[1]
    limb = spice.edlimb(*radii, view)
    distance = spice.vnorm(spice.vsub(limb.center, view))
    widths = [2.0 * math.atan2(spice.vnorm(axis), distance)
              for axis in (limb.semi_minor, limb.semi_major)]
    print(epoch, correction, *(value.hex() for value in widths))
