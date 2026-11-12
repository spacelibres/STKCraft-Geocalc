# Surface-name reference captures

These captures exercise `SRFSCC` against SpiceyPy 8.3.0 and CSPICE N0067.
The native library SHA-256 is
`7fd2dcc58d73ed5293847e67109897caba34240b8f9c38ad739c0b2ec6ce7715`.
The scripts record native output, independently of the Rust implementation.

`numeric-extreme-probe.py` captures 19 explicit boundary inputs and 2,000
seeded generated inputs with no surface mapping in the pool. Its TSV SHA-256
is `b869d294c09d685e8c4de86eac010ea6c8a8035e0d74a4a5be013b015d880775`.
Regenerate with Python and compare the resulting rows; line endings may change
the file hash. The Rust test asserts the row count and exact integer or
not-found outcome for each input.

`mapping-fuzz-probe.py` generates a 104-name text kernel and captures 419
lookups, including long names, compressed whitespace and 36-character
collisions. The text kernel SHA-256 is
`f49e2ac13671ea8bc0155484ffa4ba3bd001222a28c949835599117468c1f27b`;
the TSV SHA-256 is
`399be576d8194c6531e1b13bcc97763715963b894981d8e93aedae96a3e1e58a`.

`named-latsrf-probe.py` captures 30 LATSRF calls with name and ID lists,
including successful coordinates, missing DSK data and unknown names. The
reference output SHA-256 is
`d801a61cf3e05f8b53e1f468f7868eb592957c23124a7c18b793a2a95afddab6`.
Input SHA-256 values are `7a9f6911866796201ca668fd55ecdc48b537178e884352d100b8fa96a22dab32`
for `optimized.bds`, `d00456675a5b950b95ef18e1846f269d58c5c137c9ad845372a348bd7a37255c`
for `frames.tf`, `c1c7feeab882263fc493a9d5a5b2ddd71b54826cdf65d8d17a76126b260a49f2`
for `de440s.bsp`, and `3dff7b1dbeceaa01f25467767d3fa25816051c85d162d1edf04acb310ee28bb1`
for `pck00011.tpc`. The native capture uses the library identified above;
the test checks the returned coordinate bits and short errors.

`latsrf-method-probe.py` captures 1,065 method and frame combinations,
including empty strings, invalid frames and method errors. The output
SHA-256 is `bd6274f4e1590b129f34f197e8a5f3d843baaf3b88f21b95b24eedc4d1f16946`.
The method-string LATSRF entry point checks returned coordinate bits and
native short errors, including frame-versus-method error precedence.
`latsrf-no-dsk-probe.py` captures 24 additional cases with no loaded DSK,
including method and frame failures. Its output SHA-256 is
`0288e919b73a8811addd8b1708fd82af912ff17fc8ed61bff6ad5f2056689068`.

`sincpt-method-probe.py` captures 756 SINCPT calls across seven aberration
corrections, methods, frames and ray directions. Its output SHA-256 is
`acda52281081b733b0a7e0d91fee03e9b860693a53080fcd4258e31d2b899bfe`.
The method-string entry point compares all seven point, epoch and vector
double fields as bits, along with misses and native short errors. The separate
`sincpt-inertial-probe.py` captures four target-centered inertial and missing
radii cases; its output SHA-256 is
`3e383c6cf1b3ff3335af20ceb1c3823de16984e2ed096a7bb1a90d3e3f6b1da0`.
Additional kernel and error combinations remain open.
`sincpt-radii-probe.py` records five nonpositive-axis cases; the captured
output SHA-256 is
`2a394ab2e25b4afb78afe82c4160b680479d1876839cda7ec26f3b43a0ae65e0`.

`method-lexer-probe.py` calls N0067's private `ZZLEXMET` directly via the
native library and records token byte ranges or short errors for 16 explicit
cases and 2,000 seeded generated inputs. The output SHA-256 is
`bfcfde7955fd575ef5cf4218bee2e5974f51f7394b015d330302b1cced1de3db`.
The corresponding test-only Rust lexer checks every recorded range and error.
`method-probe.py` separately records 11,338 `ZZPRSMET` outcomes, including
30 explicit cases, seeded sequences, combinations of shape and clauses,
4,000 seeded surface-list cases, 4,000 mixed-delimiter cases, and 12
surface-capacity cases.
Its output SHA-256 is
`32e3d20ca1ac18a8ee33f0522f1fa847df8f657c00a2eb87e35bc7aa63c238eb`.
Use Python UTF-8 mode when regenerating the multibyte truncation case.
The public method parser compares all captured fields and errors. Its resolved
surface IDs also drive the LATSRF test above. This does not establish complete
grammar coverage, other operations' method validation, or CLI/MCP integration.

Other captures cover static mappings, malformed assignments, failed cache
updates, numeric fallback, fixed-width names, and native hash buckets. They
are selected examples, not an exhaustive proof of surface-name parity. The
resolver and method parser are lower-level components; other operation-specific
method validation and CLI/MCP surface selection remain separate work.
