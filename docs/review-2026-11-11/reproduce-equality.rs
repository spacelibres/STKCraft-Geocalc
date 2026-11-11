use stkcraft_geocalc::{search_scalar, Interval, Condition, FinderOptions};
fn main() {
    let found = search_scalar(|time| Ok(1e-9 * time),
        &[Interval { start: -10.0, end: 10.0 }], Condition::Equal, 0.0,
        FinderOptions { step: 1.0, tolerance: 1e-6, adjustment: 0.0 }).unwrap();
    println!("f(t)=1e-9*t, condition f(t)=0; exact solution [0,0]; local result: {found:?}");
}
