# Text-kernel conformance scope

Version 0.1.9 uses an ordered, typed and case-sensitive text pool for frame,
planetary-constant and instrument contexts. Original captures are preserved in
[the review evidence](https://github.com/spacelibres/STKCraft-Geocalc/tree/e701f78c47e82bd47ae005a9d8d62733222fe19c/docs/review-2026-11-12).
The independent C programs use the official N0067 static library and pinned
static UCRT. Capture manifests identify the archive, compiler, actual linked
libraries, C sources, inputs and raw outputs by SHA256.

## Exercised coverage

- `kernel_pool.tsv`: 34 cases, 178 original native operations. The pool unit
  regression checks 34 resets, 41 loads and 70 typed queries. Integration tests
  check ten frame matrices, four body-constant vectors and the complete GETFOV
  result. NAMFRM/FRINFO records remain supporting evidence, not an assertion
  of exhaustive public introspection parity.
- `frame_kernel_errors.tsv`: 120 cases, 720 operations, including 480 PXFORM
  and SXFORM queries. Every matrix component and native short error is checked.
  Missing, numeric, character and multiple-value fields exercise validation
  timing, identity queries, fixed and Euler frames, and built-in precedence.
- `pck_pool_updates.tsv`: eight cases, 136 native operations. The regression
  checks eight resets, sixteen loads, thirty-two RADII/GM queries and thirty-two
  transforms. The existing Option-valued constant APIs assert absence for
  native failures; they do not claim native error-symbol compatibility. The
  additional POLE_RA/POLE_DEC/PM vector records remain original supporting
  evidence; their full direct-query error API has not been implemented.

No original external numerical expectation is edited. Strings remain
character-valued even when their contents resemble numbers. A failed append
removes its active variable, preserves earlier assignments and does not apply
later statements. Derived PCK caches follow that effective pool.

## Migration behavior

Public Rust signatures are retained. Fixed frame metadata validation now
occurs when the transformation requires it. Loading an unused malformed TK
specification on a class-2 frame succeeds, as in N0067. A same-frame query can
return identity without validating unrelated orientation data. Loading errors
can leave earlier assignments active; callers must not assume transactionality.
Character replacement or variable removal no longer leaves stale PCK numbers.

## Open limits

Kernel dates use the pinned Time resolver, with eight native values and three
rejection outcomes checked by the pool regression. Native cache capacity/eviction,
general query-history behavior and all malformed/degenerate inputs are not
exhaustively emulated. The 120-case matrix does not establish every dynamic
family's complete error cross-product. Date short-error symbols and wider
TPARSE productions remain unverified. See [Time integration](TIME_KERNEL_DATES.md)
and [remaining gaps](../STRICT_REMAINING_GAPS.md).
