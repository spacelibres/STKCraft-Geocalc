use std::ffi::CString;
type D = f64;
extern "C" {
    fn furnsh_c(path:*const i8); fn failed_c()->i32;
    fn subpnt_c(m:*const i8,t:*const i8,et:D,f:*const i8,a:*const i8,o:*const i8,p:*mut D,epoch:*mut D,v:*mut D);
    fn ilumin_c(m:*const i8,t:*const i8,et:D,f:*const i8,a:*const i8,o:*const i8,p:*const D,epoch:*mut D,v:*mut D,phase:*mut D,inc:*mut D,emi:*mut D);
    fn spkpos_c(t:*const i8,et:D,f:*const i8,a:*const i8,o:*const i8,p:*mut D,lt:*mut D);
    fn tangpt_c(m:*const i8,t:*const i8,et:D,f:*const i8,a:*const i8,l:*const i8,o:*const i8,df:*const i8,d:*const D,tp:*mut D,alt:*mut D,r:*mut D,sp:*mut D,ep:*mut D,v:*mut D);
    fn recpgr_c(t:*const i8,p:*const D,re:D,flat:D,lon:*mut D,lat:*mut D,alt:*mut D);
}
fn s(x:&str)->CString{CString::new(x).unwrap()}
fn main(){unsafe{
    for file in ["de442.bsp","pck00011.tpc","mars_iau2000_v1.tpc","gm_de431.tpc","moon_pa_de440_200625.bpc","moon_de440_250416.tf","moon_assoc_me.tf"]{let p=s(&format!("target/kernels/{file}"));furnsh_c(p.as_ptr());assert_eq!(failed_c(),0);}
    let moon=s("MOON");let earth=s("EARTH");let frame=s("IAU_MOON");let correction=s("CN+S");let ellipsoid=s("ELLIPSOID");
    for method in ["Near point: ellipsoid","Intercept: ellipsoid"]{
        let m=s(method);let(mut point,mut vec,mut ep)=([0.;3],[0.;3],0.);
        subpnt_c(m.as_ptr(),moon.as_ptr(),0.,frame.as_ptr(),correction.as_ptr(),earth.as_ptr(),point.as_mut_ptr(),&mut ep,vec.as_mut_ptr());
        let(mut te,mut sv,mut phase,mut inc,mut emi)=(0.,[0.;3],0.,0.,0.);
        ilumin_c(ellipsoid.as_ptr(),moon.as_ptr(),0.,frame.as_ptr(),correction.as_ptr(),earth.as_ptr(),point.as_ptr(),&mut te,sv.as_mut_ptr(),&mut phase,&mut inc,&mut emi);
        assert_eq!(failed_c(),0);println!("{method}: {:016x}",emi.to_bits());
    }
    let j2000=s("J2000");let locus=s("TANGENT POINT");
    for corr in ["LT+S","CN+S"]{
        let c=s(corr);let(mut direction,mut lt)=([0.;3],0.);
        spkpos_c(moon.as_ptr(),0.,j2000.as_ptr(),c.as_ptr(),earth.as_ptr(),direction.as_mut_ptr(),&mut lt);
        let(mut tp,mut alt,mut range,mut sp,mut ep,mut vec)=([0.;3],0.,0.,[0.;3],0.,[0.;3]);
        tangpt_c(ellipsoid.as_ptr(),moon.as_ptr(),0.,frame.as_ptr(),c.as_ptr(),locus.as_ptr(),earth.as_ptr(),j2000.as_ptr(),direction.as_ptr(),tp.as_mut_ptr(),&mut alt,&mut range,sp.as_mut_ptr(),&mut ep,vec.as_mut_ptr());
        let(mut lon,mut lat,mut height)=(0.,0.,0.);recpgr_c(moon.as_ptr(),sp.as_ptr(),1737.4,0.,&mut lon,&mut lat,&mut height);
        assert_eq!(failed_c(),0);println!("{corr}: {:016x}",height.to_bits());
    }
}}
