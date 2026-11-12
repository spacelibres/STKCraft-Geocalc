"""Capture SRFSCC numeric fallback grammar with an empty native pool."""
import random
import spiceypy as spice

random.seed(260112)
queries=['',' ','@2000-JAN-1','PI','-PI','E','D2','1e','1e+','1e-',
         '1..0','0x10','1,0','1 0','+ 1','1.5','-.5','2147483647.9',
         '2147483648','-2147483648.9','-2147483649','1_0']
for _ in range(500):
    integer=random.randint(-999999,999999)
    fraction=random.choice(['','.0','.5','.999','.', '.0001'])
    exponent=random.choice(['','e0','E+2','d-1','e3','D-4'])
    text=f'{integer}{fraction}{exponent}'
    if random.randrange(4)==0: text=' '+text+' '
    if random.randrange(5)==0 and len(text)>2:
        at=random.randrange(1,len(text));text=text[:at]+random.choice([' ',','])+text[at:]
    queries.append(text)
spice.kclear()
print('name\texpected')
with spice.no_found_check():
    for query in queries:
        try:
            code,found=spice.srfscc(query,301)
            expected=f'I:{code}' if found else 'N'
        except spice.utils.exceptions.SpiceyError as error:
            expected='E:'+error.short
        print(f'{query}\t{expected}')
spice.kclear()
