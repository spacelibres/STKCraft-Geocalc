import ctypes as c,sys,math,json,hashlib
from pathlib import Path
root=Path(sys.argv[1]);libpath=Path(sys.argv[2]);L=c.CDLL(str(libpath));D=c.c_double;P=c.POINTER(D);I=c.c_int;S=c.c_char_p;V=D*3
L.erract_c.argtypes=[S,I,S];L.furnsh_c.argtypes=[S];L.failed_c.restype=I;L.getmsg_c.argtypes=[S,I,S]
L.subpnt_c.argtypes=[S,S,D,S,S,S,P,P,P]
L.ilumin_c.argtypes=[S,S,D,S,S,S,P,P,P,P,P,P]
L.reclat_c.argtypes=[P,P,P,P]
L.et2lst_c.argtypes=[D,I,D,S,I,I,c.POINTER(I),c.POINTER(I),c.POINTER(I),S,S]
L.spkpos_c.argtypes=[S,D,S,S,S,P,P]
L.tangpt_c.argtypes=[S,S,D,S,S,S,S,S,P,P,P,P,P,P,P]
L.recpgr_c.argtypes=[S,P,D,D,P,P,P]
L.erract_c(b'SET',0,b'RETURN')
for f in ['target/kernels/de442.bsp','target/kernels/pck00011.tpc','target/kernels/mars_iau2000_v1.tpc','target/kernels/gm_de431.tpc','target/kernels/moon_pa_de440_200625.bpc','target/kernels/moon_de440_250416.tf','target/kernels/moon_assoc_me.tf']:L.furnsh_c(str(root/f).encode())
for method in [b'Near point: ellipsoid',b'Intercept: ellipsoid']:
 p=V();epoch=D();sv=V();L.subpnt_c(method,b'MOON',0.,b'IAU_MOON',b'CN+S',b'EARTH',p,c.byref(epoch),sv)
 e=D();s=D();q=D();t=D();v=V();L.ilumin_c(b'ELLIPSOID',b'MOON',0.,b'IAU_MOON',b'CN+S',b'EARTH',p,c.byref(t),v,c.byref(q),c.byref(s),c.byref(e))
 radius=D();lon=D();lat=D();L.reclat_c(p,c.byref(radius),c.byref(lon),c.byref(lat));hh=I();mm=I();ss=I();tm=c.create_string_buffer(100);ampm=c.create_string_buffer(100);L.et2lst_c(epoch,301,lon,b'PLANETOCENTRIC',100,100,c.byref(hh),c.byref(mm),c.byref(ss),tm,ampm)
 print(json.dumps({'method':method.decode(),'emission_deg':math.degrees(e.value),'et2lst':tm.value.decode(),'point_bits':[x.hex() for x in p],'epoch_bits':epoch.value.hex(),'srfvec_bits':[x.hex() for x in sv],'illum_vector_bits':[x.hex() for x in v],'emission_rad_bits':e.value.hex()}))
for corr in [b'LT+S',b'CN+S']:
 direction=V();lt=D();L.spkpos_c(b'MOON',0.,b'J2000',corr,b'EARTH',direction,c.byref(lt));tan=V();alt=D();ran=D();surf=V();ep=D();vec=V()
 L.tangpt_c(b'ELLIPSOID',b'MOON',0.,b'IAU_MOON',corr,b'TANGENT POINT',b'EARTH',b'J2000',direction,tan,c.byref(alt),c.byref(ran),surf,c.byref(ep),vec)
 lon=D();lat=D();h=D();L.recpgr_c(b'MOON',surf,1737.4,0.,c.byref(lon),c.byref(lat),c.byref(h));print(json.dumps({'correction':corr.decode(),'surface_altitude':h.value,'direction_bits':[x.hex() for x in direction],'surface_bits':[x.hex() for x in surf],'tangent_bits':[x.hex() for x in tan],'epoch_bits':ep.value.hex(),'alt_bits':h.value.hex()}))
if L.failed_c():buf=c.create_string_buffer(256);L.getmsg_c(b'SHORT',256,buf);raise RuntimeError(buf.value.decode())

M=V*3
L.pxform_c.argtypes=[S,S,D,c.POINTER(V)]
for et in [0.,-1.3365157668936716,500000000.]:
 mat=M();L.pxform_c(b"J2000",b"IAU_MOON",et,mat);print(json.dumps({"frame_et":et,"matrix_bits":[x.hex() for row in mat for x in row]}))
