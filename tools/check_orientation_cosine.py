"""Check one captured cosine using the convergent decimal Taylor series."""
from decimal import Decimal, localcontext
import math
x = float.fromhex('0x1.701f621f7403fp-1')
with localcontext() as context:
    context.prec = 100
    a = Decimal(x)
    term = value = Decimal(1)
    for n in range(1, 60):
        term *= -(a * a) / Decimal((2 * n - 1) * 2 * n)
        value += term
    print('high precision', value)
    print('rounded', float(value).hex(), 'windows', math.cos(x).hex())
    assert float(value).hex() == '0x1.8143f859d7b7cp-1'
