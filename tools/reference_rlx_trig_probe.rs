extern crate rlx_libm;
use std::io::{self,BufRead};
fn main(){for line in io::stdin().lock().lines(){let bits=u64::from_str_radix(&line.unwrap(),16).unwrap();let x=f64::from_bits(bits);println!("{:016x},{:016x}",rlx_libm::sin(x).to_bits(),rlx_libm::cos(x).to_bits());}}
