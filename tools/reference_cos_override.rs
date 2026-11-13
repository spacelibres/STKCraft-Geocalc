//! Test-only LD_PRELOAD intervention. Never use this as a numerical backend.
//! Replaces the result for one captured argument to test causal propagation.
#![no_std]
use core::{ffi::{c_char,c_void},panic::PanicInfo};
extern "C" { fn dlsym(handle:*mut c_void, name:*const c_char)->*mut c_void; }
#[panic_handler] fn panic(_: &PanicInfo)->! {loop {}}
#[no_mangle] pub extern "C" fn rust_eh_personality() {}
#[no_mangle] pub unsafe extern "C" fn cos(value:f64)->f64 {
    if value.to_bits()==0x3fe701f621f7403f { return f64::from_bits(0x3fe8143f859d7b7d); }
    let original: unsafe extern "C" fn(f64)->f64 = core::mem::transmute(dlsym((-1isize) as *mut c_void, b"cos\0".as_ptr() as *const c_char));
    original(value)
}
#[no_mangle] pub unsafe extern "C" fn sincos(value:f64, sine:*mut f64, cosine:*mut f64) {
    let original: unsafe extern "C" fn(f64,*mut f64,*mut f64) = core::mem::transmute(dlsym((-1isize) as *mut c_void, b"sincos\0".as_ptr() as *const c_char));
    original(value,sine,cosine);
    if value.to_bits()==0x3fe701f621f7403f { *cosine=f64::from_bits(0x3fe8143f859d7b7d); }
}
