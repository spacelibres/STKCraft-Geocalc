# Time-owned kernel-date integration

GeoCalc consumes the exact registry dependency `stkcraft-time =0.1.30`.
Version 0.1.30 adds the separate native scalar JDUTC path used by SPK10;
see [the historical integration scope](SPK10_TIME_SCOPE.md). Kernel-date
parsing remains the same independently tested implementation.
The published 0.1.29 archive has byte-identical `src/` files to 0.1.28; its
documentation clarifies the original native grammar and remaining limits.
The [registry source comparison](https://github.com/spacelibres/STKCraft-Geocalc/blob/v0.1.10/docs/review-2026-11-12/time-0.1.29-registry/source-comparison.json)
retains both package checksums and all 29 source-file hashes.
Parsing remains implemented by the separate Time project. The initial
handoff was followed by explicit authorization to repair that project:

- [0.1.27](https://github.com/spacelibres/STKCraft-time/releases/tag/v0.1.27),
  commit `44ec02c9c96bdcff5257743a0c4fa307f87618e1`, fixes seven one-ULP
  errors by preserving native arithmetic order.
- [0.1.28](https://github.com/spacelibres/STKCraft-time/releases/tag/v0.1.28),
  commit `2cf6733d88822b7d6a527b5d97e7b91ccb6280f9`, resolves the 99
  native-valid rejected forms and the real RSSD mixed-delimiter epoch.
  Its independent corpus checks 345 exact values and 71 rejections.

The [numerical issue](https://github.com/spacelibres/STKCraft-time/issues/1)
and [grammar issue](https://github.com/spacelibres/STKCraft-time/issues/2)
retain the original evidence. The earlier 0.1.18/0.1.22 API reviews are
historical observations, not the status of the new resolver.

Original [date captures](https://github.com/spacelibres/STKCraft-Geocalc/tree/e701f78c47e82bd47ae005a9d8d62733222fe19c/docs/review-2026-11-12/frame-kernel-errors)
contain eleven inputs and thirty-three clear/load/query operations. The pool
regression checks all eight numeric bit patterns, all three rejection outcomes
and the resulting missing-variable states. It does not claim identical native
short-error symbols for date failures. The 720-operation frame matrix remains
separately checked without alteration.

Kernel date constants represent seconds from J2000 without an implicit UTC,
TT or TDB conversion. They do not require an LSK to parse. A frame interprets
its epoch using the frame contract. Native accepts `@2001-FEB-29`; replacing
TPARSE with a strict Gregorian validity check would change these results.
GeoCalc retains the original token and calls Time directly on numeric reads.
It never formats and reparses the returned double through NPARSD, which could
change its bits, and never substitutes a default epoch.

The first complete GeoCalc gate rejected the RSSD FK literal
`@2000-JAN-1/12:00:00`. Both Debug and Release failures and exact source
snapshots are retained in
[the initial gate evidence](https://github.com/spacelibres/STKCraft-Geocalc/tree/e701f78c47e82bd47ae005a9d8d62733222fe19c/docs/review-2026-11-12/initial-kernel-date-gate).
The corrected integration requires a new complete gate; the failed run is
not reused as acceptance.

Full TPARSE grammar conformance remains unclaimed. Time documents residual
era/zone, custom-production and mutable TSETYR limits. Civil TimeContext
conversion keeps its coverage boundary. SPK10 now explicitly selects
Time 0.1.30's formal native JDUTC convention; see
[historical SPK10 acceptance](SPK10_TIME_SCOPE.md). No scientific historical
model is substituted.
