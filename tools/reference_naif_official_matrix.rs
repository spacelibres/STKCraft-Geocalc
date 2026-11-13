use std::ffi::CString;
extern "C" {
    fn furnsh_c(path:*const i8);
    fn pxform_c(from:*const i8,to:*const i8,et:f64,mat:*mut f64);
    fn failed_c()->i32;
}
fn main(){
    let p=CString::new("target/kernels/pck00011.tpc").unwrap();
    let from=CString::new("J2000").unwrap();let to=CString::new("IAU_MOON").unwrap();
    unsafe{furnsh_c(p.as_ptr());assert_eq!(failed_c(),0);}
    for et in [0.0,-1.3365157668936716,500000000.0]{
        let mut m=[0.0f64;9];unsafe{pxform_c(from.as_ptr(),to.as_ptr(),et,m.as_mut_ptr());assert_eq!(failed_c(),0);}
        println!("{} {}",et,m.iter().map(|x|format!("{:016x}",x.to_bits())).collect::<Vec<_>>().join(","));
    }
}
