# Pinned kernel snapshots

A dated NAIF filename is not a guarantee of permanent download availability.
On 2026-11-12 the original generic PCK URL for
`earth_000101_270205_261110.bpc` returned HTTP 404. Main CI for the 0.1.9
commit stopped during bootstrap, before running its exactness gate.
Version 0.1.9 therefore remains an untagged, unpublished checkpoint despite
its passing local gate. The next verified release includes its changes.

The official
[WGC rolling source](https://naif.jpl.nasa.gov/pub/naif/pds/wgc/kernels/pck/earth_latest_high_prec.bpc)
still returned exactly the original 5,160,960 bytes when recaptured. The
snapshot is stored in the Git repository for reproducible CI. Its SHA256 is
`57f123e7e2bc5d51e20bda707b474d136f079c352500648551258096cbb9a7b1`,
unchanged from the original independent capture and the canonical manifest.
The original kernel, DAF comments, expected values, load order and all
scientific inputs are preserved. No newer Earth PCK is substituted.

[Capture headers and provenance](https://github.com/spacelibres/STKCraft-Geocalc/blob/v0.1.10/docs/reference/kernel-snapshots/capture.json)
record the actual NASA URL, time, response headers, byte count and original
capture relationship. The snapshot is reference data, excluded from the
production crate and executable linkage. GitHub platform archives include
it as reference evidence with the other reference documentation.
See [attribution](../../ATTRIBUTION.md).

Bootstrap stages this explicitly listed snapshot only after validating it
against the original kernel pin. It verifies the copied bytes again before
atomic replacement. Missing or changed snapshots fail; verify-only mode
does not stage files. Correct existing target files do not use the network.
Other kernels retain their original official download and hash checks.
The bootstrap report identifies the snapshot path alongside its original
official URL and SHA256. It does not claim a fresh download from the removed
generic URL.

Regressions cover exact staging without a network request, changed and
missing snapshots, verify-only behavior, the original capture hash, and
retained official-source metadata. The entire strict gate must still pass.
