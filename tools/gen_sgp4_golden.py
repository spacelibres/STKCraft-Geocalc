"""Independent CSPICE N0067 XXSGP4I/XXSGP4E TEME oracle."""
from pathlib import Path
import ctypes as C, csv, math
import spiceypy as spice
from spiceypy.utils.libspicehelper import libspice as lib
ROOT=Path(__file__).resolve().parents[1]
def arr(xs): return (C.c_double*len(xs))(*xs)
def error():
    if spice.failed():
        message=spice.getmsg("SHORT",128);spice.reset();return message
    return "OK"
spice.furnsh(str(ROOT/"target/kernels/naif0012.tls"))
g=[.001082616,-2.53881e-6,-1.65597e-6,.0743669161,120.,78.,6378.135,1.]
periods=[90.,120.,224.99,225.01,270.,600.,720.,1000.,1436.1,1500.]
times=[-50000.,-10080.,-1440.,-720.,-60.,-1.,0.,1.,60.,720.,1440.,10080.,50000.]
with (ROOT/"tests/golden/sgp4_teme.csv").open("w",newline="",encoding="utf-8") as f:
    w=csv.writer(f);w.writerow([*(f"g{i}" for i in range(8)),*(f"e{i}" for i in range(10)),"opmode","minutes","epoch_days","status",*(f"state{i}" for i in range(6))]);count=0;success=0
    for pi,period in enumerate(periods):
      for ei,ecc in enumerate([0.,1e-7,.01,.2,.6]):
       for ii,inc in enumerate([0.,.1,.2,.7,math.pi-1e-8]):
        e=[0.,0.,(pi%3)*1e-5,inc,(pi-4)*.37,ecc,(ei-2)*.79,(ii-2)*1.1,2*math.pi/period,123456789.+pi*1234.5]
        tv=arr([e[9]]+[0.]*7);lib.ttrans_(b"TDB",b"JDUTC",tv,C.c_int(3),C.c_int(5));assert error()=="OK";epoch=tv[0]-2433281.5
        for mode in [1,2]:
         for minutes in times:
          lib.xxsgp4i_(arr(g),arr(e),C.byref(C.c_int(mode)));status=error();state=arr([0.]*6)
          if status=="OK":
              lib.xxsgp4e_(C.byref(C.c_double(minutes)),state);status=error()
          w.writerow([*g,*e,mode,minutes,epoch,status,*(list(state) if status=="OK" else [""]*6)]);count+=1;success+=status=="OK"
print(f"{count} SGP4 cases, {success} native valid TEME states")

# A second independent parameter matrix varies gravity models, epochs,
# long unwrapped angles, resonances and the retrograde singularity threshold.
import random
rng=random.Random(0x53475034)
with (ROOT/"tests/golden/sgp4_teme.csv").open("a",newline="",encoding="utf-8") as f:
 w=csv.writer(f)
 for model in range(240):
  g2=list(g)
  if model%3==1:g2=[.00108262998905,-2.53215306e-6,-1.61098761e-6,.07436685316871385,120.,78.,6378.137,1.]
  if model%3==2:g2=[.001082616,-2.53881e-6,-1.65597e-6,.0743669161,125.,80.,6378.135,1.]
  period=rng.choice([90.,226.,269.,650.,700.,740.,1440.,1500.,10000.,35000.])*(1+rng.uniform(-.01,.01))
  a=(g2[3]/(2*math.pi/period))**(2/3)
  ecc=rng.uniform(0,max(0,min(.95,1-1.015/a)))
  inc=rng.choice([0.,1e-8,.19999,.20001,math.pi/2,math.pi-1e-5,math.pi-1e-6,math.pi,rng.uniform(0,math.pi)])
  e=[1e-8,-1e-13,rng.choice([0.,-1e-5,1e-6,1e-3]),inc,rng.uniform(-40,40),ecc,rng.uniform(-40,40),rng.uniform(-40,40),2*math.pi/period,rng.choice([-315576000.,0.,536500800.,846763200.,1577880000.])]
  tv=arr([e[9]]+[0.]*7);lib.ttrans_(b"TDB",b"JDUTC",tv,C.c_int(3),C.c_int(5));assert error()=="OK";epoch=tv[0]-2433281.5
  for mode in [1,2]:
   for minutes in times:
    lib.xxsgp4i_(arr(g2),arr(e),C.byref(C.c_int(mode)));status=error();state=arr([0.]*6)
    if status=="OK":lib.xxsgp4e_(C.byref(C.c_double(minutes)),state);status=error()
    w.writerow([*g2,*e,mode,minutes,epoch,status,*(list(state) if status=="OK" else [""]*6)]);count+=1;success+=status=="OK"
print(f"Broadened total: {count} SGP4 cases, {success} native valid TEME states")
