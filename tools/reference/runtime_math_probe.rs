//! Isolated diagnostic process: never linked into production calculations.
#[cfg(all(windows, target_arch = "x86_64"))]
fn main() {
    #[cfg_attr(target_feature = "crt-static", link(name = "libucrt", kind = "static"))]
    #[cfg_attr(not(target_feature = "crt-static"), link(name = "ucrt"))]
    extern "C" {
        fn _get_FMA3_enable() -> i32;
        fn _set_FMA3_enable(flag: i32) -> i32;
    }
    let input: Vec<f64> = std::env::args()
        .skip(1)
        .map(|s| s.parse().expect("finite diagnostic argument"))
        .collect();
    assert_eq!(input.len(), 2);
    let original = unsafe { _get_FMA3_enable() };
    println!(
        "{{\"kind\":\"rust-runtime\",\"default_fma\":{},\"cpu_fma\":{}}}",
        original,
        std::is_x86_feature_detected!("fma")
    );
    for requested in [original, 0, 1] {
        let enabled = unsafe { _set_FMA3_enable(requested) };
        let x = input[0];
        let y = input[1];
        let values = [
            x.asin(),
            x.acos(),
            x.sin(),
            x.cos(),
            x.atan2(y),
            x.ln(),
            x.exp(),
            x.sinh(),
            x.cosh(),
            x.powf(y),
        ];
        println!(
            "{{\"kind\":\"rust-math\",\"requested\":{},\"enabled\":{},\"bits\":{:?}}}",
            requested,
            enabled,
            values.map(f64::to_bits)
        );
    }
    unsafe { _set_FMA3_enable(original) };
    assert_eq!(unsafe { _get_FMA3_enable() }, original);
}

#[cfg(not(all(windows, target_arch = "x86_64")))]
fn main() {
    panic!("This diagnostic requires x86_64 Windows and the UCRT.");
}
