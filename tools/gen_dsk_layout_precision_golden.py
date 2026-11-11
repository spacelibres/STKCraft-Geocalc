"""DAS layout variants verified by the independent native reader."""
from pathlib import Path
import csv,struct
import numpy as np
import spiceypy as s
root=Path(__file__).resolve().parents[1];out=root/'tests/golden/dsk'
v=np.array([[3.,0.,0.],[-2.,0.,0.],[0.,4.,0.],[0.,-3.,0.],[0.,0.,5.],[0.,0.,-2.]])
p=np.array([[1,3,5],[3,2,5],[2,4,5],[4,1,5],[3,1,6],[2,3,6],[4,2,6],[1,4,6]],dtype=np.int32)
d,i=s.dskmi2(v,p,5.,4,100000,100000,100000,True,400000);lo,hi=s.dskrb2(v,p,1,np.zeros(10))
path=out/'multi_directory.bds'
if path.exists():path.unlink()
h=s.dskopn(str(path),'Native mixed clusters and multiple DAS directories',0)
for j in range(1000):
 s.dskw02(h,301,10000+j,2,'IAU_MOON',1,np.zeros(10),-np.pi,np.pi,-np.pi/2,np.pi/2,lo,hi,-1e9,1e9,v,p,d,i)
 s.daswbr(h)
s.dskcls(h,False)
def directories(b):
 nrr,nrc,ncr,ncc=struct.unpack_from('<4i',b,68);pointer=nrr+ncr+2
 while pointer:
  directory=list(struct.unpack_from('<256i',b,(pointer-1)*1024));yield pointer,directory;pointer=directory[1]
b=bytearray(path.read_bytes());print('native layout',len(b),'bytes',len(list(directories(b))),'directories')
# Add actual reserved physical records; directory pointers are physical.
base=bytearray((out/'optimized.bds').read_bytes());reserved=bytearray(base[:1024]+b'R'*2048+base[1024:])
struct.pack_into('<i',reserved,68,2)
for pointer,words in directories(base):
 for j in [0,1]:
  if words[j]:words[j]+=2
 struct.pack_into('<256i',reserved,(pointer+1)*1024,*words)
(out/'reserved.bds').write_bytes(reserved)
# Source DAS binary-format conversion: only integer/double data records
# and directory/header integers are endian dependent; comments stay bytes.
for name in ['reserved','multi_directory']:
 b=bytearray((out/(name+'.bds')).read_bytes());big=bytearray(b)
 for pointer,words in directories(b):
  struct.pack_into('>256i',big,(pointer-1)*1024,*words)
  kind=[3,1,2][words[8]-1];physical=pointer+1
  for count in words[9:]:
   if not count:break
   kind=([2,3,1] if count>0 else [3,1,2])[kind-1]
   start=(physical-1)*1024;end=start+abs(count)*1024
   width={1:1,2:8,3:4}[kind]
   for address in range(start,end,width):big[address:address+width]=b[address:address+width][::-1]
   physical+=abs(count)
 for address in range(68,84,4):big[address:address+4]=b[address:address+4][::-1]
 big[84:92]=b'BIG-IEEE';(out/(name+'_big.bds')).write_bytes(big)
rows=[]
for file in ['reserved.bds','reserved_big.bds','multi_directory.bds','multi_directory_big.bds']:
 h=s.dasopr(str(out/file));segment=s.dlabfs(h);index=0
 while True:
  desc=s.dskgd(h,segment)
  # Native logical integer header, DLA, and all component arrays.
  raw=[segment.bwdptr,segment.fwdptr,segment.ibase,segment.isize,segment.dbase,segment.dsize,segment.cbase,segment.csize]
  rows.append([file,index,';'.join(map(str,raw)),int(desc.surfce),';'.join(map(repr,map(float,s.dskv02(h,segment,1,6).ravel()))),';'.join(map(str,s.dskp02(h,segment,1,8).ravel()))])
  try:segment=s.dlafns(h,segment)
  except s.utils.exceptions.NotFoundError:break
  index+=1
 s.dascls(h)
with (out/'layouts.csv').open('w',newline='') as file:
 w=csv.writer(file);w.writerow(['file','segment','dla','surface','vertices','plates']);w.writerows(rows)
print(len(rows),'native reader layout cases')
