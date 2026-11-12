"""Capture extreme SRFSCC exponent parsing against the native toolkit."""
import spiceypy as spice
import random

queries = [
    '1e-2147483649', '1e-99999999999999', '0e1000', '0e999',
    '0e10000', '0e3090', 'PIe-9999999999', 'e-9999999999',
    '.e-9999999999', 'e--9999999999', '1e-9999999999x',
    '1e-9999999999.0', '1e-9999999999,', '1e-9999999999e2',
    '0.00000000000000000001e320', '10000000000000000000e-320',
    ' @2000-JAN-1', ',@2000-JAN-1', '  @2000-JAN-1/12:00:00',
]
random.seed(20261112)
for _ in range(2000):
    integer = ''.join(str(random.randrange(10)) for _ in range(random.randrange(0, 28)))
    fraction = ''.join(str(random.randrange(10)) for _ in range(random.randrange(0, 28)))
    mantissa = random.choice(['', '+', '-']) + integer
    if random.randrange(2):
        mantissa += '.' + fraction
    exponent = random.choice(['E', 'd']) + random.choice(['', '+', '-'])
    exponent += random.choice(['', '0', '309', '1000', '999999999999999999', str(random.randrange(3000))])
    queries.append(mantissa + exponent)
spice.kclear()
print('name\texpected')
with spice.no_found_check():
    for query in queries:
        code, found = spice.srfscc(query, 301)
        print(f'{query}\t{f"I:{code}" if found else "N"}')
spice.kclear()
