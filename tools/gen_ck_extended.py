"""Native CK4/5/6 frame/SCLK integration and both DAF byte orders."""
from pathlib import Path
import csv, struct
import spiceypy as s

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / 'tests/golden'

def big_endian(path):
    original = path.read_bytes()
    result = bytearray(original)
    for offset in [8, 12, 76, 80, 84]:
        result[offset:offset+4] = original[offset:offset+4][::-1]
    result[88:96] = b'BIG-IEEE'
    record = struct.unpack_from('<i', original, 76)[0]
    while record:
        base = (record-1)*1024
        next_record, _, count = struct.unpack_from('<3d', original, base)
        for offset in range(base, base+24, 8):
            result[offset:offset+8] = original[offset:offset+8][::-1]
        for i in range(int(count)):
            at = base+24+i*40
            for offset in range(at, at+16, 8):
                result[offset:offset+8] = original[offset:offset+8][::-1]
            for offset in range(at+16, at+40, 4):
                result[offset:offset+4] = original[offset:offset+4][::-1]
            ints = struct.unpack_from('<6i', original, at+16)
            for word in range(ints[4], ints[5]+1):
                offset = (word-1)*8
                result[offset:offset+8] = original[offset:offset+8][::-1]
        record = int(next_record)
    output = path.with_name(path.stem+'_big.bc')
    output.write_bytes(result)
    return output

# Native reader must accept the converted containers and produce every
# sampled value itself; this is not a Rust little/big self comparison.
with (OUT/'ck_extended_endian.csv').open('w', newline='') as f:
    writer = csv.writer(f)
    writer.writerow(['file','inst','tick','tol','need_av','ref','status','clock',
                     *[f'm{i}' for i in range(9)], 'x','y','z'])
    for kind in [4,5,6]:
        s.kclear()
        path = big_endian(OUT/f'ck{kind}.bc')
        s.furnsh(str(path))
        rows = list(csv.reader((OUT/f'ck{kind}.csv').open()))[1:]
        for row in rows[::max(1,len(rows)//120)]:
            inst, tick, tol, need_av, reference = int(row[0]),float(row[1]),float(row[2]),bool(int(row[3])),int(row[4])
            try:
                with s.no_found_check():
                    if need_av:
                        matrix,av,clock,found = s.ckgpav(inst,tick,tol,s.frmnam(reference))
                    else:
                        matrix,clock,found = s.ckgp(inst,tick,tol,s.frmnam(reference)); av=[]
                status = 'FOUND' if found else 'NONE'
                values = [repr(clock),*[repr(float(x)) for x in matrix.flat],*([repr(float(x)) for x in av] if need_av else ['']*3)] if found else ['']*13
            except s.utils.exceptions.SpiceyError as error:
                status=error.short;values=['']*13
            writer.writerow([path.name,inst,repr(tick),repr(tol),int(need_av),reference,status,*values])

fk='KPL/FK\n\\begindata\n'
clocks='KPL/SCLK\n\\begindata\n'
instruments=[-740000,-740003,-740006,-740009,-750000,-750008,-750032,-750040,-750064,-750072,-750096,-750104,-760000,-760004,-760008,-760012]
for index,inst in enumerate(instruments):
    identifier=-840000-index; clock=abs(inst)//1000
    fk+=f"FRAME_CK_EXT_{index}={identifier}\nFRAME_{identifier}_NAME='CK_EXT_{index}'\nFRAME_{identifier}_CLASS=3\nFRAME_{identifier}_CLASS_ID={inst}\nFRAME_{identifier}_CENTER=-{clock}\n"
for clock in sorted({abs(inst)//1000 for inst in instruments}):
    clocks+=f'SCLK_DATA_TYPE_{clock}=1\nSCLK01_TIME_SYSTEM_{clock}=1\nSCLK01_N_FIELDS_{clock}=1\nSCLK01_MODULI_{clock}=(1000000000)\nSCLK01_OFFSETS_{clock}=(0)\nSCLK01_OUTPUT_DELIM_{clock}=1\nSCLK_PARTITION_START_{clock}=(0)\nSCLK_PARTITION_END_{clock}=(1000000000)\nSCLK01_COEFFICIENTS_{clock}=(0 1000 0.125)\n'
(OUT/'ck_extended.tf').write_text(fk)
(OUT/'ck_extended.tsc').write_text(clocks)
s.kclear()
for file in ['ck4.bc','ck5.bc','ck6.bc','ck_extended.tf','ck_extended.tsc']:
    s.furnsh(str(OUT/file))
s.furnsh(str(ROOT/'target/kernels/naif0012.tls'))
with (OUT/'ck_extended_frames.csv').open('w',newline='') as f:
    writer=csv.writer(f);writer.writerow(['state','from','to','et','status',*[f'x{i}' for i in range(18)]])
    for index,inst in enumerate(instruments):
        for tick in [0.,3.125,10.,15.,100.,100.00000001,103.125,110.,120.,150.,180.,190.,200.]:
            et=s.sct2e(-(abs(inst)//1000),tick)
            for need_state in [False,True]:
                for start,finish in [('J2000',f'CK_EXT_{index}'),(f'CK_EXT_{index}','B1950')]:
                    try:
                        matrix=s.sxform(start,finish,et) if need_state else s.pxform(start,finish,et)
                        values=list(matrix[:3,:3].flat)+list(matrix[3:,:3].flat) if need_state else list(matrix.flat)
                        status='FOUND';values=[repr(float(x)) for x in values]
                    except s.utils.exceptions.SpiceyError as error:
                        status=error.short;values=[]
                    writer.writerow([int(need_state),start,finish,repr(et),status,*values])
s.kclear()
print('Native CK4/5/6 endian and frame reference written')
