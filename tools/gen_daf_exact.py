"""DAF summary names and addresses from CSPICE DAFGN/DAFGS."""
from pathlib import Path
import spiceypy as s
import math
root=Path(__file__).resolve().parents[1]
orientation=root/'tests/golden/daf_orientation.bpc'
if orientation.exists():orientation.unlink()
handle=s.pckopn(str(orientation),'DAF exact orientation fixture',0)
for classid,name in [(3000,'First binary PCK segment'),(3001,'  Leading blanks are significant')]:
    s.pckw02(handle,classid,'J2000',0.,10.,name,10.,1,1,[.1,.01,.2,.02,.3,.03],0.)
s.pckcls(handle)
attitude=root/'tests/golden/daf_attitude.bc'
if attitude.exists():attitude.unlink()
handle=s.ckopn(str(attitude),'DAF exact attitude fixture',0)
s.ckw03(handle,0.,10.,-999000,'J2000',True,'First attitude segment',2,[0.,10.],
        [[1.,0.,0.,0.],[math.cos(.1),0.,0.,math.sin(.1)]],[[0.,0.,.01],[0.,0.,.01]],1,[0.])
s.ckcls(handle)
paths=['tests/golden/spk_priority.bsp','tests/golden/spk_layer_a.bsp','tests/golden/spk_layer_b.bsp','tests/golden/daf_orientation.bpc','tests/golden/daf_attitude.bc']
with (root/'tests/golden/daf_exact.txt').open('w',encoding='utf-8') as out:
    for path in paths:
        handle=s.dafopr(str(root/path));nd,ni=s.dafhsf(handle);s.dafbfs(handle);index=0
        while s.daffna():
            name=s.dafgn();dc,ic=s.dafus(s.dafgs(5),nd,ni)
            out.write('|'.join([path,str(index),name,*map(lambda x:repr(float(x)),dc),*map(lambda x:str(int(x)),ic)])+'\n')
            index+=1
        s.dafcls(handle)
