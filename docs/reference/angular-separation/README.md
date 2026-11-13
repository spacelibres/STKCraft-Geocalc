# Coincident point separation observations

`wgc-coincident.jsonl` contains live WGC2 requests and raw response bodies for
coincident point targets, mixed point/sphere shapes, a coincident sphere and
stellar correction. WGC returns zero for point/point and a negative apparent
sphere radius for the mixed examples; it rejects the coincident sphere.
`wgc-finder-coincident.jsonl` records that the corresponding GF search rejects
non-distinct bodies with `SPICE(BODIESNOTDISTINCT)`.

`capture.json` records local DE442, PCK, CSPICE N0067 and trace hashes, plus
the binary64 reference for the local CSPICE position/VSEP and host asin path.
Native CSPICE `TRGSEP` rejects the coincident-body examples; it is not the
same entry point as the observed WGC calculator behavior. The reference is
limited to the recorded inputs and environment.
