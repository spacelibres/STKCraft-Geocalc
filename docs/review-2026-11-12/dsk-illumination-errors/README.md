# ILLUMF missing DSK error evidence

`capture.py` calls the independent CSPICE N0067 library through SpiceyPy.
`capture.json` records the exact short errors and hashes of the DLL and kernel
inputs. Run it from any directory after installing the reference kernels at the
paths used by the strict gate. The cases test an empty DSK set, a loaded lunar DSK with Earth as the
target, invalid-body/frame error precedence, and a target equal to the
illumination source for both ellipsoid and DSK models. The successful
ellipsoid result records raw IEEE-754 bits and both visibility flags. They do not establish all DSK loading,
frame, surface-selection, or error-precedence combinations. The existing
`tests/golden/dsk/illumination.csv` supplies the separate point-off-surface
reference cases.
