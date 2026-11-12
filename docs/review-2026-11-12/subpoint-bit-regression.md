# Ellipsoid sub-point bitwise reference

The unchanged `tests/golden/subpoint_methods.csv` contains 42 CSPICE
SUBPNT/SUBSLR cases. On Windows x86_64, SpiceyPy 8.3.0 with CSPICE N0067
library SHA256 `7fd2dcc58d73ed5293847e67109897caba34240b8f9c38ad739c0b2ec6ce7715`
recaptured the same 42 rows with `tools/gen_subpoint_method_golden.py`.
The original and recaptured files were byte-identical, SHA256
`cbe360cfe7407c0d749a25e8f602c4e1c002588de6f514be90679d62cab892b9`.
The SPK and PCK hashes are recorded in the adjacent DSK illumination error
capture. The test now compares each of the three coordinates and altitude by
IEEE-754 bits and requires all 42 rows. This covers the listed epochs,
corrections and ellipsoid methods only; it does not establish complete
WebGeocalc sub-point coverage or other runtime profiles.
