"""Generate NAIF text-kernel number parsing reference cases via PRSDP."""
from pathlib import Path
import csv
import re
import spiceypy as spice
root = Path(__file__).resolve().parents[1]
tokens = {'-3.8787','.1','-0.00001234','1.0000000000000001','1.234567890123456789012345','1234567890123456789012345','5.03D-5','1e-310','1D100','-PI','PI','D+3'}
for name in ['pck00011.tpc','gm_de431.tpc','mars_iau2000_v1.tpc']:
    text=(root/'target/kernels'/name).read_text()
    data='\n'.join(block.split('\\begintext')[0] for block in text.split('\\begindata')[1:])
    tokens.update(re.findall(r'(?<![A-Za-z_0-9])[-+]?(?:\d+\.\d*|\.\d+|\d+)(?:[EeDd][-+]?\d+)?',data))
with (root/'tests/golden/kernel_numbers.csv').open('w',newline='') as file:
    writer=csv.writer(file);writer.writerow(['text','cspice'])
    for token in sorted(tokens):
        writer.writerow([token,repr(float(spice.prsdp(token)))])
print('Generated',len(tokens),'independent parser references')
