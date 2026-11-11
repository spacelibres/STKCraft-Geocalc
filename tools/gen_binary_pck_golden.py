"""Generate native N0067 binary PCK type-2 record and frame fixtures."""
from pathlib import Path
import csv,random,ctypes as ct
import spiceypy as s
from spiceypy.utils.libspicehelper import libspice
from spiceypy.utils.support_types import SPICEDOUBLE_CELL
ROOT=Path(__file__).resolve().parents[1];OUT=ROOT/'tests/golden';K=ROOT/'target/kernels'
path=OUT/'binary_pck_type2.bpc'
if path.exists():path.unlink()
h=s.pckopn(str(path),'native orientation type2',0)
# Multiple records, non-J2000 reference, nontrivial high-order angle/rate values.
coeff=[((j*13+7)%31-15)*10.**(-j%6-5) for j in range(3*8*2)]
# Keep the middle angle away from the EUL2XF singularities.
for record in range(2):
    coeff[record*24+8]=.4;coeff[record*24+16]=10000000.125+record*12345
s.pckw02(h,999001,'ECLIPJ2000',-86400.,86400.,'non-J2000 orientation',86400.,2,7,coeff,-86400.)
# Same class/time later segment wins and must only mask its coverage.
s.pckw02(h,999001,'B1950',-100.,100.,'priority narrow orientation',200.,1,2,[1.,.02,-.001,.3,.03,.0007,8e6,.07,.003],-100.)
s.pckcls(h)
fk=OUT/'binary_pck_type2.tf';fk.write_text("\\begindata\nFRAME_PCK_TEST=1999001\nFRAME_1999001_NAME='PCK_TEST'\nFRAME_1999001_CLASS=2\nFRAME_1999001_CLASS_ID=999001\nFRAME_1999001_CENTER=301\n")
s.furnsh(str(path));s.furnsh(str(fk));s.furnsh(str(K/'pck00011.tpc'));s.furnsh(str(K/'moon_de440_250416.tf'));s.furnsh(str(K/'moon_pa_de440_200625.bpc'))
rng=random.Random(31008);times=[-2e9,-1e9,-1.,0.,1.,1e9,2e9]+[rng.uniform(-2e9,2e9) for _ in range(100)]
rows=[]
for a,b in [('J2000','MOON_PA_DE440'),('J2000','MOON_ME'),('MOON_PA','MOON_ME'),('MOON_ME','MOON_PA'),('MOON_ME','IAU_MOON'),('ECLIPJ2000','MOON_ME'),('MOON_PA_DE440','IAU_EARTH')]:
    for et in times:
        xf=s.sxform(a,b,et);rows.append([a,b,repr(et),*[repr(float(x)) for x in xf[:3,:3].flat],*[repr(float(x)) for x in xf[3:,:3].flat]])
for et in [-86400.,-86399.99,-100.00001,-100.,-99.99999,0.,99.99999,100.,100.00001,86399.99,86400.]:
    for a,b in [('J2000','PCK_TEST'),('PCK_TEST','J2000'),('PCK_TEST','MOON_ME')]:
        xf=s.sxform(a,b,et);rows.append([a,b,repr(et),*[repr(float(x)) for x in xf[:3,:3].flat],*[repr(float(x)) for x in xf[3:,:3].flat]])
with (OUT/'binary_pck_frames.csv').open('w',newline='') as f:
    w=csv.writer(f);w.writerow(['from','to','et',*[f'r{i}' for i in range(9)],*[f'dr{i}' for i in range(9)]]);w.writerows(rows)
# Native PCKMAT primitive separates polynomial/EUL2XF errors from frame chains.
with (OUT/'binary_pck_native.csv').open('w',newline='') as f:
    w=csv.writer(f);w.writerow(['classid','et','reference',*[f'r{i}' for i in range(9)],*[f'dr{i}' for i in range(9)]])
    for cid,ets in [(31008,times),(999001,[-86400.,-100.00001,-100.,0.,100.,100.00001,86400.])]:
        for et in ets:
            c=ct.c_int(cid);t=ct.c_double(et);ref=ct.c_int();xf=(ct.c_double*36)();found=ct.c_int();libspice.pckmat_(ct.byref(c),ct.byref(t),ct.byref(ref),xf,ct.byref(found));assert found.value
            matrix=[xf[col*6+row] for row in range(3) for col in range(3)];derivative=[xf[col*6+row+3] for row in range(3) for col in range(3)]
            w.writerow([cid,repr(et),ref.value,*map(repr,matrix),*map(repr,derivative)])
# Apparent states validate named-frame class and frame-center timing as
# well as the primitive frame matrices used by surface calculations.
s.furnsh(str(K/'de440s.bsp'))
with (OUT/'binary_pck_apparent.csv').open('w',newline='') as f:
    w=csv.writer(f);w.writerow(['target','observer','frame','et','correction','x','y','z','vx','vy','vz','lt'])
    for target,observer in [(301,399),(299,399),(399,301),(301,10)]:
        for frame in ['MOON_ME','MOON_PA','B1950','PCK_TEST']:
            for correction in ['NONE','LT','LT+S','CN','CN+S','XLT','XLT+S','XCN','XCN+S']:
                for et in ([-90.,0.,90.] if frame=='PCK_TEST' else [-2e9,0.,2e9]):
                    state,lt=s.spkezr(str(target),et,frame,correction,str(observer));w.writerow([target,observer,frame,repr(et),correction,*[repr(float(x)) for x in state],repr(lt)])
# Separate later file, narrower coverage; load order must follow PCKSFS.
layer=OUT/'binary_pck_layer.bpc'
if layer.exists():layer.unlink()
h=s.pckopn(str(layer),'priority orientation layer',0)
s.pckw02(h,999001,'J2000',-10.,10.,'later loaded orientation',20.,1,1,[2.,.001,.4,-.002,3.,.003],-10.);s.pckcls(h)
with (OUT/'binary_pck_layers.csv').open('w',newline='') as f:
    w=csv.writer(f);w.writerow(['order','et',*[f'r{i}' for i in range(9)],*[f'dr{i}' for i in range(9)]])
    for order,paths in [('ab',[path,layer]),('ba',[layer,path])]:
        s.kclear();s.furnsh(str(fk))
        for item in paths:s.furnsh(str(item))
        for et in [-86400.,-100.,-10.00001,-10.,0.,10.,10.00001,100.,86400.]:
            xf=s.sxform('J2000','PCK_TEST',et);w.writerow([order,repr(et),*[repr(float(x)) for x in xf[:3,:3].flat],*[repr(float(x)) for x in xf[3:,:3].flat]])
# Verify a second DAF byte order through the native reader, rather than
# assuming the SPK container tests cover NI=5 PCK packed summaries.
import struct
original=path.read_bytes();big=bytearray(original)
for offset in [8,12,76,80,84]:big[offset:offset+4]=original[offset:offset+4][::-1]
big[88:96]=b'BIG-IEEE'
record=struct.unpack_from('<i',original,76)[0]
while record:
    base=(record-1)*1024;next_record,_,count=struct.unpack_from('<3d',original,base)
    for offset in range(base,base+24,8):big[offset:offset+8]=original[offset:offset+8][::-1]
    for i in range(int(count)):
        at=base+24+i*40
        for offset in range(at,at+16,8):big[offset:offset+8]=original[offset:offset+8][::-1]
        for offset in range(at+16,at+40,4):big[offset:offset+4]=original[offset:offset+4][::-1]
        ints=struct.unpack_from('<6i',original,at+16)
        for word in range(ints[3],ints[4]+1):
            offset=(word-1)*8;big[offset:offset+8]=original[offset:offset+8][::-1]
    record=int(next_record)
big_path=OUT/'binary_pck_type2_big.bpc';big_path.write_bytes(big)
s.kclear();s.furnsh(str(fk));s.furnsh(str(big_path))
with (OUT/'binary_pck_big.csv').open('w',newline='') as f:
    w=csv.writer(f);w.writerow(['et',*[f'r{i}' for i in range(9)],*[f'dr{i}' for i in range(9)]])
    for et in [-86400.,-100.,-10.,0.,10.,100.,86400.]:
        xf=s.sxform('J2000','PCK_TEST',et);w.writerow([repr(et),*[repr(float(x)) for x in xf[:3,:3].flat],*[repr(float(x)) for x in xf[3:,:3].flat]])
s.kclear()
centers=sorted({s.frinfo(int(i))[0] for i in s.bltfrm(-1)})
defaults=[]
for body in centers:
    try:canonical=s.bodc2n(body)
    except Exception:canonical=''
    identifier,frame_name=s.cidfrm(body);defaults.append((body,canonical,identifier,frame_name))
source="// Generated by tools/gen_binary_pck_golden.py from native N0067 CIDFRM/BODC2N.\n"
source+="pub(super) const DEFAULTS: &[(i32,&str,i32)] = &[\n"
for body,canonical,identifier,_ in defaults:source+=f'    ({body},"{canonical}",{identifier}),\n'
source+='];\n';(ROOT/'src/frames/frame_body_defaults.rs').write_text(source)
with (OUT/'body_frame_defaults.csv').open('w',newline='') as f:
    w=csv.writer(f);w.writerow(['body','body_name','frame_id','frame_name']);w.writerows(defaults)
s.kclear();print(f'{len(rows)} binaryPCK frame transforms +114 directPCKMAT primitives generated')
