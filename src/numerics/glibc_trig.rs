// RLX — versatile ML compiler + runtime.
// Copyright (C) 2026 Eugene Hauptmann, Nataliya Kosmyna.
// SPDX-License-Identifier: MIT OR Apache-2.0

//! glibc's double `sin` / `cos` (IBM Accurate Mathematical Library,
//! `sysdeps/ieee754/dbl-64/s_sin.c`) as x86-64 runs it on an FMA CPU —
//! `__sin_fma` / `__cos_fma`, gcc contracting every product whose uses are all
//! adds/subtracts (a product with several such uses is fused into each) — with the
//! huge-argument reduction `__branred` (`branred.c`, built without FMA).

// Preserve the reference constants, evaluation order and non-finite behavior.
#![allow(clippy::approx_constant, clippy::assign_op_pattern, clippy::eq_op)]

fn fma(a: f64, b: f64, c: f64) -> f64 {
    a.mul_add(b, c)
}

const S1: f64 = -0.16666666666666666;
const S2: f64 = 0.008333333333332329;
const S3: f64 = -0.00019841269834414642;
const S4: f64 = 2.755729806860771e-06;
const S5: f64 = -2.5022014848318398e-08;
const BIG: f64 = 52776558133248.0;
const HP0: f64 = 1.5707963267948966;
const HP1: f64 = 6.123233995736766e-17;
const MP1: f64 = 1.5707963407039642;
const MP2: f64 = -1.3909067564377153e-08;
const PP3: f64 = -4.97899623147991e-17;
const PP4: f64 = -1.9034889620193266e-25;
const HPINV: f64 = 0.6366197723675814;
const TOINT: f64 = 6755399441055744.0;
const SN3: f64 = -0.16666666666666488; // -1.66666666666664880952546298448555E-01
const SN5: f64 = 0.008333332142857223; // 8.33333214285722277379541354343671E-03
const CS2: f64 = 0.5; // 4.99999999999999999999950396842453E-01
const CS4: f64 = -0.04166666666666644; // -4.16666666666664434524222570944589E-02
const CS6: f64 = 0.001388888740079376; // 1.38888874007937613028114285595617E-03

const TOVERP: [f64; 75] = [
    10680707.0, 7228996.0, 1387004.0, 2578385.0, 16069853.0, 12639074.0, 9804092.0, 4427841.0,
    16666979.0, 11263675.0, 12935607.0, 2387514.0, 4345298.0, 14681673.0, 3074569.0, 13734428.0,
    16653803.0, 1880361.0, 10960616.0, 8533493.0, 3062596.0, 8710556.0, 7349940.0, 6258241.0,
    3772886.0, 3769171.0, 3798172.0, 8675211.0, 12450088.0, 3874808.0, 9961438.0, 366607.0,
    15675153.0, 9132554.0, 7151469.0, 3571407.0, 2607881.0, 12013382.0, 4155038.0, 6285869.0,
    7677882.0, 13102053.0, 15825725.0, 473591.0, 9065106.0, 15363067.0, 6271263.0, 9264392.0,
    5636912.0, 4652155.0, 7056368.0, 13614112.0, 10155062.0, 1944035.0, 9527646.0, 15080200.0,
    6658437.0, 6231200.0, 6832269.0, 16767104.0, 5075751.0, 3212806.0, 1398474.0, 7579849.0,
    6349435.0, 12618859.0, 4703257.0, 12806093.0, 14477321.0, 2786137.0, 12875403.0, 9837734.0,
    14528324.0, 13719321.0, 343717.0,
];

/// glibc `__sincostab` (`sysdeps/ieee754/dbl-64/sincostab.c`): per table point
/// `sn, ssn, cs, ccs` as IEEE bits.
const SINCOSTAB: [u64; 440] = [
    0x0000000000000000,
    0x0000000000000000,
    0x3ff0000000000000,
    0x0000000000000000,
    0x3f7fffeaaaaeeeef,
    0xbc1e45e2ec67b77c,
    0x3fefffc000155552,
    0x3c8f4a01a0196dae,
    0x3f8fffaaaaeeeed5,
    0xbc02ab639a9f0777,
    0x3fefff000155549f,
    0x3c828a28a03a5ef3,
    0x3f97ff7001033255,
    0x3bfefe2b51527336,
    0x3feffdc006bff7e6,
    0x3c8ae6dae86977bd,
    0x3f9ffeaaaeeee86f,
    0xbc3cd406fb224ae2,
    0x3feffc00155527d3,
    0xbc83b54492d89b5b,
    0x3fa3feb2b12d45d5,
    0x3c34ec54203d1c11,
    0x3feff9c03414a7ba,
    0x3c6991f4be6c59bf,
    0x3fa7fdc01032fba9,
    0xbc4599bdf46e997a,
    0x3feff7006bfdf99f,
    0xbc78b3b560648d5f,
    0x3fabfc6d78586dac,
    0x3c18e4fd03dbf236,
    0x3feff3c0c8103a31,
    0x3c74856dbddc0e66,
    0x3faffaaaeeed4edb,
    0xbc42d16d32684b69,
    0x3feff0015549f4d3,
    0x3c8328387b99426f,
    0x3fb1fc343d808bef,
    0xbc5f3d32e6f3be4f,
    0x3fefebc222a8ef9f,
    0x3c57934934f54c77,
    0x3fb3facb12d1755b,
    0xbc5921915299468c,
    0x3fefe7034129ef6f,
    0xbc6cbf4337c96f97,
    0x3fb5f911fd10b737,
    0xbc50184f02be9102,
    0x3fefe1c4c3c873eb,
    0xbc35a9c9057c4a02,
    0x3fb7f701032550e4,
    0x3c3afc2d1800501a,
    0x3fefdc06bf7e6b9b,
    0x3c831902b535f8db,
    0x3fb9f4902d55d1f9,
    0x3c52696d7eac1dc1,
    0x3fefd5c94b43e000,
    0xbc62e768cb4f92f9,
    0x3fbbf1b78568391d,
    0x3c5e91841dea4cc8,
    0x3fefcf0c800e99b1,
    0x3c6ea3d786d186ac,
    0x3fbdee6f16c1cce6,
    0xbc450f8e2fb71673,
    0x3fefc7d078d1bc88,
    0x3c8075d2447db685,
    0x3fbfeaaeee86ee36,
    0xbc4afcb2bcc6f03b,
    0x3fefc015527d5bd3,
    0x3c8b68f35094efb8,
    0x3fc0f3378ddd71d1,
    0x3c6d8468724f0f9e,
    0x3fefb7db2bfe0695,
    0x3c821dadf4f65ab1,
    0x3fc1f0d3d7afceaf,
    0xbc66ef95099769a5,
    0x3fefaf22263c4bd3,
    0xbc552ace133a2769,
    0x3fc2ee285e4ab88f,
    0xbc6e4d0f05dee058,
    0x3fefa5ea641c36f2,
    0x3c404da6ed17cc7c,
    0x3fc3eb312c5d66cb,
    0x3c647d666b66cb91,
    0x3fef9c340a7cc428,
    0x3c8c5b6b063b7462,
    0x3fc4e7ea4dc5f27b,
    0x3c5949db2ac072fc,
    0x3fef91ff40374d01,
    0xbc67d03f4d3a9e4c,
    0x3fc5e44fcfa126f3,
    0xbc66f443063f89b6,
    0x3fef874c2e1eecf6,
    0xbc8c6514e1332b16,
    0x3fc6e05dc05a4d4c,
    0xbbd32c5c8b81c940,
    0x3fef7c1afeffde24,
    0xbc78f55bc47540b1,
    0x3fc7dc102fbaf2b5,
    0x3c45ab50e23c97c3,
    0x3fef706bdf9ece1c,
    0xbc8698c80c36dcb4,
    0x3fc8d7632efaa944,
    0xbc620fa262cbb953,
    0x3fef643efeb82acd,
    0x3c76b00ac1fe28ac,
    0x3fc9d252d0cec312,
    0x3c59c43d80b1137d,
    0x3fef57948cff6797,
    0x3c6e3a0d3e03b1d5,
    0x3fcaccdb297a0765,
    0xbc59883b57d6cdeb,
    0x3fef4a6cbd1e3a79,
    0x3c813df0edaebb57,
    0x3fcbc6f84edc6199,
    0x3c69c1a56a7b0cab,
    0x3fef3cc7c3b3d16e,
    0xbc621a3ad28a3494,
    0x3fccc0a6588289a3,
    0xbc6868d09bc87c6b,
    0x3fef2ea5d753ffed,
    0x3c8cc4215f56d583,
    0x3fcdb9e15fb5a5d0,
    0xbc632e20d6cc6fc2,
    0x3fef20073086649f,
    0x3c7b940416c1984b,
    0x3fceb2a57f8ae5a3,
    0xbc60be06af572ceb,
    0x3fef10ec09c5873b,
    0x3c8d9072762c1283,
    0x3fcfaaeed4f31577,
    0xbc615d88508e32b8,
    0x3fef01549f7deea1,
    0x3c8d3c1e99e5cafd,
    0x3fd0515cbf65155c,
    0xbc79b8c29dfd8ec8,
    0x3feef141300d2f26,
    0xbc82aa1b08ded372,
    0x3fd0cd00cef36436,
    0xbc79fb0a0c93e2b5,
    0x3feee0b1fbc0f11c,
    0xbc4bfd2380bbc3b1,
    0x3fd14861aa94ddeb,
    0xbc6be881b5b615a4,
    0x3feecfa744d5efa1,
    0xbc556d0a4af541d0,
    0x3fd1c37d64c6b876,
    0x3c746076fe0dcff5,
    0x3feebe214f76efa8,
    0xbc802f9f12ba543e,
    0x3fd23e52111aaf36,
    0xbc74f080334eff18,
    0x3feeac2061bbaf4f,
    0x3c62c1d53e94658d,
    0x3fd2b8ddc43eb49f,
    0x3c61553899f2d807,
    0x3fee99a4c3a7cd83,
    0xbc82264b1bc53ce8,
    0x3fd3331e94049f87,
    0x3c7e0cb6b40c302c,
    0x3fee86aebf29a9ed,
    0x3c89397afdbb58a7,
    0x3fd3ad129769d3d8,
    0x3c003d5504878398,
    0x3fee733ea0193d40,
    0xbc86428b3546ce13,
    0x3fd426b7e69ee697,
    0xbc7f09c75705c59f,
    0x3fee5f54b436e9d0,
    0x3c87eb0fd02fc8bc,
    0x3fd4a00c9b0f3d20,
    0x3c7823ba6bb08ead,
    0x3fee4af14b2a449c,
    0xbc868ca02e8a6833,
    0x3fd5190ecf68a77a,
    0x3c7b357155eef0f3,
    0x3fee3614b680d6a5,
    0xbc727793aa015237,
    0x3fd591bc9fa2f597,
    0x3c67c74bac3fe0cb,
    0x3fee20bf49acd6c1,
    0xbc5660aec7ef636c,
    0x3fd60a1429078775,
    0x3c5b1fd80ba89133,
    0x3fee0af15a03dbce,
    0x3c5fe8e702771ae6,
    0x3fd682138a38d7f7,
    0xbc7d889202444aad,
    0x3fedf4ab3ebd875e,
    0xbc8e2d8a7e6736c4,
    0x3fd6f9b8e33a0255,
    0x3c742bc14ee9da0d,
    0x3feddded50f228d6,
    0xbc6e80c8d42ba2bf,
    0x3fd7710255764214,
    0xbc66ead7314bb6ce,
    0x3fedc6b7eb995912,
    0x3c54b364776dcd35,
    0x3fd7e7ee03c86d4e,
    0xbc7b63bcdabf5af2,
    0x3fedaf0b6b888e83,
    0x3c8a249e2b5e5cea,
    0x3fd85e7a12826949,
    0x3c78a40e9b5face0,
    0x3fed96e82f71a9dc,
    0x3c8ff61bd5d2039d,
    0x3fd8d4a4a774992f,
    0x3c744a02ea766326,
    0x3fed7e4e97e17b4a,
    0xbc63b770352bed94,
    0x3fd94a6be9f546c5,
    0xbc769ce13e683f58,
    0x3fed653f073e4040,
    0xbc876236434bec37,
    0x3fd9bfce02e80510,
    0x3c709e39a320b0a4,
    0x3fed4bb9e1c619e0,
    0x3c8f34bb77858f61,
    0x3fda34c91cc50cca,
    0xbc5a310e3b50cecd,
    0x3fed31bf8d8d7c06,
    0x3c7e60dd3089cbdd,
    0x3fdaa95b63a09277,
    0xbc66293eb13c0381,
    0x3fed1750727d94f0,
    0x3c80d52b1ec1a48e,
    0x3fdb1d8305321617,
    0xbc7ae242cb99f519,
    0x3fecfc6cfa52ad9f,
    0x3c88b5b5508f2a0d,
    0x3fdb913e30dbac43,
    0xbc7e38ad2f6c3ff1,
    0x3fece115909a82e5,
    0x3c81f139bb31109a,
    0x3fdc048b17b140a3,
    0x3c619fe6757e9fa7,
    0x3fecc54aa2b2972e,
    0x3c64ee162ba83a98,
    0x3fdc7767ec7fd19e,
    0xbc5eb14d1a3d5826,
    0x3feca90c9fc67d0b,
    0xbc646a81485e3462,
    0x3fdce9d2e3d4a51f,
    0xbc62fc8a12dae298,
    0x3fec8c5bf8ce1a84,
    0x3c7ab3d1a1590123,
    0x3fdd5bca34047661,
    0x3c728a44a75fc29c,
    0x3fec6f39208be53b,
    0xbc8741dbfbaadb42,
    0x3fddcd4c15329c9a,
    0x3c70d4c6e171fd9a,
    0x3fec51a48b8b175e,
    0xbc61bbb43b9aa880,
    0x3fde3e56c1582a69,
    0xbc50a4821099f88f,
    0x3fec339eb01ddd81,
    0xbc8caaf5ee82c5c0,
    0x3fdeaee8744b05f0,
    0xbc5789b43c9b027d,
    0x3fec1528065b7d50,
    0xbc8892111312e828,
    0x3fdf1eff6bc4f97b,
    0x3c717212f8a7525c,
    0x3febf641081e7536,
    0x3c8b7bd71628a9a1,
    0x3fdf8e99e76abc97,
    0x3c59d950af2d00a3,
    0x3febd6ea310294f5,
    0x3c731bbcc88c109d,
    0x3fdffdb628d2f57a,
    0x3c6f4a992e905b6a,
    0x3febb723fe630f32,
    0x3c772bd2452d0a39,
    0x3fe0362939c69955,
    0xbc82d8cd78397b01,
    0x3feb96eeef58840e,
    0x3c545a3cc78fade0,
    0x3fe06d3686946e5b,
    0x3c83f5ae4538ff1b,
    0x3feb764b84b704c2,
    0xbc8f5848c21b389b,
    0x3fe0a4021e9e1001,
    0xbc86f643a13914f6,
    0x3feb553a410c104e,
    0x3c58ff7947027a16,
    0x3fe0da8b26b5672e,
    0xbc8a58def0bee909,
    0x3feb33bba89c8948,
    0x3c8ea6a51d1f6ca9,
    0x3fe110d0c4b69c3b,
    0x3c8d918998809981,
    0x3feb11d04162a4c6,
    0x3c71dd561efbc0c2,
    0x3fe146d21f8b7f82,
    0x3c7bf9535e2739a8,
    0x3feaef78930bd275,
    0xbc7f836279746f94,
    0x3fe17c8e5f2eedb0,
    0x3c635e57102e2488,
    0x3feaccb526f69de5,
    0x3c88fb6a8dd6b6cc,
    0x3fe1b204acb02fdd,
    0xbc5f190c70cbb5ff,
    0x3feaa98688308913,
    0xbc0b83d607cd5070,
    0x3fe1e7343236574c,
    0x3c722a3fa4f41d5a,
    0x3fea85ed4373e02d,
    0x3c69be06385ec792,
    0x3fe21c1c1b0394cf,
    0x3c5e5b324b23aa31,
    0x3fea61e9e72586af,
    0x3c858330e2fd453f,
    0x3fe250bb93788bbb,
    0x3c7ea3d02457bcce,
    0x3fea3d7d0352bdcf,
    0xbc868dbaeca19669,
    0x3fe28511c917a067,
    0xbc801df1d9a16b70,
    0x3fea18a729aee445,
    0x3c395e25736c0358,
    0x3fe2b91dea88421e,
    0xbc8fa371db216ab0,
    0x3fe9f368ed912f85,
    0xbc81d200c5791606,
    0x3fe2ecdf279a3082,
    0x3c8d3557e0e7e37e,
    0x3fe9cdc2e3f25e5c,
    0x3c83f99112993f62,
    0x3fe32054b148bc4f,
    0x3c8f6b42095a135b,
    0x3fe9a7b5a36a6514,
    0x3c8722cfcc9fa7a9,
    0x3fe3537db9be0367,
    0x3c6b327e7af040f0,
    0x3fe98141c42e1310,
    0x3c8d1ff80488f08d,
    0x3fe386597456282b,
    0xbc710fada93b07a8,
    0x3fe95a67e00cb1fd,
    0xbc80befda21f862d,
    0x3fe3b8e715a2840a,
    0xbc797653a7d2f07b,
    0x3fe93328926d9e92,
    0xbc8bb77003600cda,
    0x3fe3eb25d36cd53a,
    0xbc5be570e1570fc0,
    0x3fe90b84784ddaf7,
    0xbc70feb10ab93b87,
    0x3fe41d14e4ba6790,
    0x3c84608fd287ecf5,
    0x3fe8e37c303d9ad1,
    0xbc6463a4b53d4bf8,
    0x3fe44eb381cf386b,
    0xbc83ed6c1e6a5505,
    0x3fe8bb105a5dc900,
    0x3c8863e03e9474c1,
    0x3fe48000e431159f,
    0xbc8b194a7463ed10,
    0x3fe89241985d871f,
    0x3c8c48d9c413ed84,
    0x3fe4b0fc46aab761,
    0x3c20da05738cc59a,
    0x3fe869108d77a6c6,
    0x3c7338ffe2bfe9dd,
    0x3fe4e1a4e54ed51b,
    0xbc8a492f89b7c76a,
    0x3fe83f7dde701ca0,
    0xbc4152cf609bc6e8,
    0x3fe511f9fd7b351c,
    0xbc85c0e861c48831,
    0x3fe8158a31916d5d,
    0xbc6de8b90b8228de,
    0x3fe541facddbb724,
    0x3c7232c28520d391,
    0x3fe7eb362eaa1488,
    0x3c5a1d65a4a5959f,
    0x3fe571a6966d59b3,
    0x3c5c843b4d0fb198,
    0x3fe7c0827f09e54f,
    0xbc6c73d6d72aee68,
    0x3fe5a0fc98813a12,
    0xbc8d82e2b7d4227b,
    0x3fe7956fcd7f6543,
    0xbc8ab276e9d45ae4,
    0x3fe5cffc16bf8f0d,
    0x3c896cb370eb578a,
    0x3fe769fec655211f,
    0xbc6827d5cf8c68c5,
    0x3fe5fea4552a9e57,
    0x3c80b6cef7ee20b7,
    0x3fe73e30174efba1,
    0xbc65d3ae3d94ad5f,
    0x3fe62cf49921ac79,
    0xbc8edd9855b6241a,
    0x3fe712046fa77678,
    0x3c8425b0a5029c81,
    0x3fe65aec2963e755,
    0x3c8126f96b71053c,
    0x3fe6e57c800cf55e,
    0x3c860286dedbd0a6,
    0x3fe6888a4e134b2f,
    0xbc86b7d37644d5e6,
    0x3fe6b898fa9efb5d,
    0x3c715ac786ccf4b2,
    0x3fe6b5ce50b7821a,
    0xbc65d5158f702e0f,
    0x3fe68b5a92eb6253,
    0xbc89a91ad985f89c,
    0x3fe6e2b77c40bde1,
    0xbc70e729857fad53,
    0x3fe65dc1fdeb8cba,
    0xbc597c1b47337c77,
    0x3fe70f451d0a8c40,
    0x3c697ede3885770d,
    0x3fe62fcff20191c7,
    0x3c6d9143895756ef,
    0x3fe73b7680dea578,
    0xbc72248306dc12a2,
    0x3fe6018526f563df,
    0x3c846ca5e0e432d0,
    0x3fe7674af6f7b524,
    0x3c7e9d3f94ac84a8,
    0x3fe5d2e255f1f17a,
    0x3c80314104c8892b,
    0x3fe792c1d0041d52,
    0xbc8abf05eeb354eb,
    0x3fe5a3e839824077,
    0x3c8428aa2759be62,
    0x3fe7bdda5e28b3c2,
    0x3c4ad1197ccd0393,
    0x3fe574978d8e83f2,
    0x3c8f4714af282d23,
    0x3fe7e893f5037959,
    0x3c80eefbaa650c4c,
    0x3fe544f10f592ca5,
    0xbc8e7ae8e6c7a62f,
    0x3fe812ede9ae4ba4,
    0xbc87830adf402dda,
    0x3fe514f57d7bf3da,
    0x3c747a108073c259,
];

#[inline]
fn tab(k: usize) -> f64 {
    f64::from_bits(SINCOSTAB[k])
}

/// `u.i[LOW_HALF] << 2` of `u = big + |x|`: the table row.
#[inline]
fn row(u: f64) -> usize {
    ((u.to_bits() as u32 as i32) << 2) as usize
}

/// `do_cos(x, dx)`.
fn do_cos(x: f64, dx: f64) -> f64 {
    let dx = if x < 0.0 { -dx } else { dx };
    let u = BIG + x.abs();
    let x = x.abs() - (u - BIG) + dx;
    let xx = x * x;
    let s = fma(x * xx, fma(xx, SN5, SN3), x);
    let c = xx * fma(xx, fma(xx, CS6, CS4), CS2);
    let k = row(u);
    let (sn, ssn, cs, ccs) = (tab(k), tab(k + 1), tab(k + 2), tab(k + 3));
    let cor = fma(-sn, s, fma(-cs, c, fma(-s, ssn, ccs)));
    cs + cor
}

/// `TAYLOR_SIN(xx, x, dx)`.
fn taylor_sin(xx: f64, x: f64, dx: f64) -> f64 {
    let poly = fma(fma(fma(fma(S5, xx, S4), xx, S3), xx, S2), xx, S1);
    let t = fma(fma(poly, x, -(0.5 * dx)), xx, dx);
    x + t
}

/// `do_sin(x, dx)`.
fn do_sin(x: f64, dx: f64) -> f64 {
    let xold = x;
    if x.abs() < 0.126 {
        return taylor_sin(x * x, x, dx);
    }
    let dx = if x <= 0.0 { -dx } else { dx };
    let u = BIG + x.abs();
    let x = x.abs() - (u - BIG);
    let xx = x * x;
    let s = x + fma(x * xx, fma(xx, SN5, SN3), dx);
    let c = fma(x, dx, xx * fma(xx, fma(xx, CS6, CS4), CS2));
    let k = row(u);
    let (sn, ssn, cs, ccs) = (tab(k), tab(k + 1), tab(k + 2), tab(k + 3));
    let cor = fma(cs, s, fma(-sn, c, fma(s, ccs, ssn)));
    (sn + cor).copysign(xold)
}

/// `reduce_sincos(x)` → `(n, a, da)`.
fn reduce_sincos(x: f64) -> (i32, f64, f64) {
    let t = fma(x, HPINV, TOINT);
    let xn = t - TOINT;
    let y = fma(-xn, MP2, fma(-xn, MP1, x));
    let n = (t.to_bits() as u32 & 3) as i32;
    let t2 = fma(-xn, PP3, y);
    let mut db = fma(-xn, PP3, y - t2);
    let b = fma(-xn, PP4, t2);
    db += fma(-xn, PP4, t2 - b);
    (n, b, db)
}

fn do_sincos(a: f64, da: f64, n: i32) -> f64 {
    let r = if n & 1 != 0 {
        do_cos(a, da)
    } else {
        do_sin(a, da)
    };
    if n & 2 != 0 {
        -r
    } else {
        r
    }
}

/// `__branred(x)` → `(n, a, aa)` (no FMA).
fn branred(x: f64) -> (i32, f64, f64) {
    const T576: f64 = 2.4733040147310453e+173;
    const TM600: f64 = 2.409919865102884e-181;
    const TM24: f64 = 5.960464477539063e-08;
    const BIGB: f64 = 6755399441055744.0;
    const BIG1: f64 = 2.7021597764222976e+16;
    const BHP0: f64 = 1.5707963267948966;
    const BHP1: f64 = 6.123233995736766e-17;
    const BMP1: f64 = 1.5707963407039642;
    const BMP2: f64 = -1.3909067675399456e-08;
    const SPLIT: f64 = 134217729.0;
    let x = x * TM600;
    let t = x * SPLIT;
    let x1 = t - (t - x);
    let x2 = x - x1;
    let half = |xp: f64| -> (f64, f64, f64) {
        let mut sum = 0.0f64;
        let mut k = (((xp.to_bits() >> 32) as u32 >> 20) & 2047) as i32;
        k = (k - 450) / 24;
        if k < 0 {
            k = 0;
        }
        let mut gor = f64::from_bits(T576.to_bits() - (((k * 24) as u64) << 52));
        let mut r = [0.0f64; 6];
        for (i, ri) in r.iter_mut().enumerate() {
            *ri = xp * TOVERP[k as usize + i] * gor;
            gor *= TM24;
        }
        for ri in r.iter_mut().take(3) {
            let s = (*ri + BIGB) - BIGB;
            sum += s;
            *ri -= s;
        }
        let mut t = 0.0f64;
        for i in 0..6 {
            t += r[5 - i];
        }
        let mut bb = (((((r[0] - t) + r[1]) + r[2]) + r[3]) + r[4]) + r[5];
        let s = (t + BIGB) - BIGB;
        sum += s;
        let t = t - s;
        let b = t + bb;
        bb = (t - b) + bb;
        let s = (sum + BIG1) - BIG1;
        sum -= s;
        (b, bb, sum)
    };
    let (b1, bb1, sum1) = half(x1);
    let (b2, bb2, sum2) = half(x2);
    let mut sum = sum1 + sum2;
    let mut b = b1 + b2;
    let bb = if b1.abs() > b2.abs() {
        (b1 - b) + b2
    } else {
        (b2 - b) + b1
    };
    if b > 0.5 {
        b -= 1.0;
        sum += 1.0;
    } else if b < -0.5 {
        b += 1.0;
        sum -= 1.0;
    }
    let s = b + (bb + bb1 + bb2);
    let t = ((b - s) + bb) + (bb1 + bb2);
    let bs = s * SPLIT;
    let t1 = bs - (bs - s);
    let t2 = s - t1;
    let b = s * BHP0;
    let bb = (((t1 * BMP1 - b) + t1 * BMP2) + t2 * BMP1) + (t2 * BMP2 + s * BHP1 + t * BHP0);
    let s = b + bb;
    let t = (b - s) + bb;
    ((sum as i32) & 3, s, t)
}

/// glibc `sin(x)`.
pub fn sin(x: f64) -> f64 {
    let k = ((x.to_bits() >> 32) as u32) & 0x7fff_ffff;
    if k < 0x3e50_0000 {
        x
    } else if k < 0x3feb_6000 {
        do_sin(x, 0.0)
    } else if k < 0x4003_68fd {
        let t = HP0 - x.abs();
        do_cos(t, HP1).copysign(x)
    } else if k < 0x4199_21FB {
        let (n, a, da) = reduce_sincos(x);
        do_sincos(a, da, n)
    } else if k < 0x7ff0_0000 {
        let (n, a, da) = branred(x);
        do_sincos(a, da, n)
    } else {
        x / x
    }
}

/// glibc `cos(x)`.
pub fn cos(x: f64) -> f64 {
    let k = ((x.to_bits() >> 32) as u32) & 0x7fff_ffff;
    if k < 0x3e40_0000 {
        1.0
    } else if k < 0x3feb_6000 {
        do_cos(x, 0.0)
    } else if k < 0x4003_68fd {
        let y = HP0 - x.abs();
        let a = y + HP1;
        let da = (y - a) + HP1;
        do_sin(a, da)
    } else if k < 0x4199_21FB {
        let (n, a, da) = reduce_sincos(x);
        do_sincos(a, da, n + 1)
    } else if k < 0x7ff0_0000 {
        let (n, a, da) = branred(x);
        do_sincos(a, da, n + 1)
    } else {
        x / x
    }
}

#[cfg(test)]
mod reference_tests {
    use super::{cos, sin};

    #[test]
    fn finite_values_match_independent_glibc_bits() {
        let mut count = 0;
        for line in include_str!("../../tests/golden/glibc_trig_exact.csv")
            .lines()
            .skip(2)
        {
            let values: Vec<_> = line
                .split(',')
                .map(|value| u64::from_str_radix(value, 16).unwrap())
                .collect();
            assert_eq!(values.len(), 3);
            let input = f64::from_bits(values[0]);
            assert_eq!(
                sin(input).to_bits(),
                values[1],
                "sin input {:016x}",
                values[0]
            );
            assert_eq!(
                cos(input).to_bits(),
                values[2],
                "cos input {:016x}",
                values[0]
            );
            count += 1;
        }
        assert_eq!(count, 10205);
    }
}
