# Constant-velocity location CLI references

`capture.py` records independent CSPICE N0067 `SPKCVT` and `SPKCVO` output
bits for the target-point and observer-point queries used by the CLI tests.
It records the toolkit, library and kernel hashes and the complete inputs.
`capture_matrix.py` extends the same inputs across nine aberration corrections;
its 18-row `matrix.csv` has SHA256
`7efe8eb0c0c45c418fe160de2b9463d4d77057c2230027ce611e792ebdc70235`.
Each case compares six state components and light time by IEEE-754 bits.
`capture_rotating.py` adds 108 cases across IAU_EARTH and IAU_MOON,
CENTER/OBSERVER/TARGET loci, the same nine corrections and both point roles;
`rotating.csv` has SHA256
`d110fcf430230d80d7a69d9495dfbe157681d9d8d7cf97e30426d526830cc11f`.
The 126 cases do not certify other frames, positions, velocities, epochs or
error combinations.

`capture_fixed.py` adds 162 fixed-target and fixed-observer cases using
SPKCPT/SPKCPO across J2000, IAU_EARTH and IAU_MOON, the three loci and nine
corrections. The seven output fields per case are compared by bits;
`fixed.csv` has SHA256
`c355a9e1c29785202e8346e5110bf7b339e27dbc11c0ee048006fddb204c5ff2`.
These cases use one point and epoch and do not cover all fixed-point inputs.

`capture_derived.py` records distance and speed using CSPICE VNORM on the
native state and target time computed as ET minus reception light time, ET
plus transmission light time, or ET for NONE. The 18-case `derived.csv`
contains 54 bitwise comparison values; its SHA256 is
`8d4dd2f4f6e9ed9955a0e1b29cd3557410d367d1880d4b85885fd86bdb7d964e`.
