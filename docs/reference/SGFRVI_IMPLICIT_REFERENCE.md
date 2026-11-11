# N0067 implicit generic-reference oracle limitation

GenericSegment implements the source formulas for reference-index kinds 0 and 1. Repeated-call native validation is blocked by an upstream C translation issue in `sgfrvi.c`: `dpimax` is automatic storage, but is assigned only inside the saved `first` flag branch. Later calls compare against an uninitialized value.

Reproducer after `python tools/gen_pck_3_20.py`:

```python
import spiceypy as s
s.furnsh('tests/golden/binary_pck_3_20.bpc')
s.furnsh('tests/golden/binary_pck_3_20.tf')
print(s.sxform('J2000', 'PCK_MODEL_930021', -30000.25))
print(s.sxform('J2000', 'PCK_MODEL_930021', -30000.25))
```

The first identical call succeeds; the second raises SPICE(INDEXTOOLARGE), reporting the valid implicit step 10000 as an invalid index. The native source has not been patched. The 6,408-case combined PCK3/PCK20 regression uses explicit index kinds 2, 3 and 4 for PCK3. Fixed and variable packet directories are covered. Implicit kinds 0 and 1 must not be reported as fully oracle-validated from this matrix.
