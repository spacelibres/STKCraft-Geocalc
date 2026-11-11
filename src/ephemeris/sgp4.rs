// Preserve native numerical indexing, constants, interface shape and operation grouping.
#![allow(
    clippy::excessive_precision,
    clippy::type_complexity,
    clippy::too_many_arguments,
    clippy::needless_return,
    clippy::unnecessary_cast,
    clippy::manual_range_contains
)]
//! Source-order CSPICE N0067 SGP4 in TEME coordinates.
//! The caller supplies JDUTC-2433281.5 explicitly; no native runtime.
#![allow(
    unused_mut,
    unused_variables,
    unused_assignments,
    unreachable_code,
    unused_parens,
    non_snake_case,
    non_upper_case_globals
)]
use crate::error::GeoError;
fn f2c_mod(x: f64, y: f64) -> f64 {
    x - y * (x / y).trunc()
}
const c_b10: f64 = (-0.66666666666666663);
const c_b11: f64 = 0.33333333333333331;
const c_b16: f64 = 3.5;
const c_b22: f64 = 1.5;
const c_false: bool = false;
fn zzinil(
    geophs: &[f64; 8],
    mut opmode: i32,
    mut ecco: f64,
    mut epoch: f64,
    mut inclo: f64,
    mut no: f64,
    mut ainv: f64,
    mut ao: f64,
    mut con41: f64,
    mut con42: f64,
    mut cosio: f64,
    mut cosio2: f64,
    mut eccsq: f64,
    mut omeosq: f64,
    mut posq: f64,
    mut rp: f64,
    mut rteosq: f64,
    mut sinio: f64,
    mut gsto: f64,
) -> Result<
    (
        f64,
        f64,
        f64,
        f64,
        f64,
        f64,
        f64,
        f64,
        f64,
        f64,
        f64,
        f64,
        f64,
        f64,
    ),
    GeoError,
> {
    let mut d__1: f64 = 0.0;

    let mut adel: f64 = 0.0;
    let mut c1p2p: f64 = 0.0;
    let mut ids70: i32 = 0;
    let mut temp: f64 = 0.0;

    let mut tfrac: f64 = 0.0;
    let mut thgr70: f64 = 0.0;
    let mut c1: f64 = 0.0;
    let mut d1: f64 = 0.0;
    let mut j2: f64 = 0.0;

    let mut ak: f64 = 0.0;
    let mut po: f64 = 0.0;
    let mut radday: f64 = 0.0;

    let mut del: f64 = 0.0;
    let mut xke: f64 = 0.0;
    let mut x2o3: f64 = 0.0;
    let mut ts70: f64 = 0.0;
    let mut fk5r: f64 = 0.0;
    let mut tut1: f64 = 0.0;

    j2 = geophs[(0 as usize)];
    xke = geophs[(3 as usize)];
    x2o3 = 0.66666666666666663;
    eccsq = (ecco * ecco);
    omeosq = (1.0 - eccsq);
    rteosq = (omeosq).sqrt();
    cosio = (inclo).cos();
    cosio2 = (cosio * cosio);
    d__1 = (xke / no);
    ak = (d__1).powf(x2o3);
    d1 = (((j2 * 0.75) * ((cosio2 * 3.0) - 1.0)) / (rteosq * omeosq));
    del = (d1 / (ak * ak));
    adel = (ak
        * ((1.0 - (del * del)) - (del * ((((del * 134.0) * del) / 81.0) + 0.33333333333333331))));
    del = (d1 / (adel * adel));
    no /= (del + 1.0);
    d__1 = (xke / no);
    ao = (d__1).powf(x2o3);
    sinio = (inclo).sin();
    po = (ao * omeosq);
    con42 = (1.0 - (cosio2 * 5.0));
    con41 = (((-con42) - cosio2) - cosio2);
    ainv = (1.0 / ao);
    posq = (po * po);
    rp = (ao * (1.0 - ecco));
    if (opmode == 2) {
        radday = (std::f64::consts::TAU * 1.002737909350795);
        temp = (epoch + 2433281.5);
        d__1 = (temp - 0.5);
        tut1 = ((((d__1).trunc() + 0.5) - 2451545.0) / 36525.0);
        d__1 = (temp - 0.5);
        gsto = (((((tut1 * 628.331970688841) + 1.75336855923327)
            + ((tut1 * 6.77071394490334e-6) * tut1))
            - (((tut1 * 4.50876723431868e-10) * tut1) * tut1))
            + (radday * ((temp - 0.5) - (d__1).trunc())));
    } else {
        if (opmode == 1) {
            ts70 = (epoch - 7305.0);
            ids70 = ((ts70 + 1e-8) as i32);
            tfrac = (ts70 - (ids70 as f64));
            c1 = 0.0172027916940703639;
            thgr70 = 1.7321343856509374;
            fk5r = 5.07551419432269442e-15;
            c1p2p = (c1 + std::f64::consts::TAU);
            gsto = (((thgr70 + (c1 * (ids70 as f64))) + (c1p2p * tfrac)) + ((ts70 * ts70) * fk5r));
        } else {
            return Err(GeoError::BadSearch("SPICE(UNKNOWNMODE)"));

            return Ok((
                no, ainv, ao, con41, con42, cosio, cosio2, eccsq, omeosq, posq, rp, rteosq, sinio,
                gsto,
            ));
        }
    }
    d__1 = std::f64::consts::TAU;
    gsto = f2c_mod(gsto, d__1);
    if (gsto < 0.0) {
        gsto += std::f64::consts::TAU;
    }

    return Ok((
        no, ainv, ao, con41, con42, cosio, cosio2, eccsq, omeosq, posq, rp, rteosq, sinio, gsto,
    ));
}
fn zzdscm(
    mut epoch: f64,
    mut eccp: f64,
    mut argpp: f64,
    mut tc: f64,
    mut inclp: f64,
    mut nodep: f64,
    mut np: f64,
    mut snodm: f64,
    mut cnodm: f64,
    mut sinim: f64,
    mut cosim: f64,
    mut sinomm: f64,
    mut cosomm: f64,
    mut day: f64,
    mut e3: f64,
    mut ee2: f64,
    mut eccm: f64,
    mut emsq: f64,
    mut gam: f64,
    mut peo: f64,
    mut pgho: f64,
    mut pho: f64,
    mut pinco: f64,
    mut plo: f64,
    mut rtemsq: f64,
    mut se2: f64,
    mut se3: f64,
    mut sgh2: f64,
    mut sgh3: f64,
    mut sgh4: f64,
    mut sh2: f64,
    mut sh3: f64,
    mut si2: f64,
    mut si3: f64,
    mut sl2: f64,
    mut sl3: f64,
    mut sl4: f64,
    mut s1: f64,
    mut s2: f64,
    mut s3: f64,
    mut s4: f64,
    mut s5: f64,
    mut s6: f64,
    mut s7: f64,
    mut ss1: f64,
    mut ss2: f64,
    mut ss3: f64,
    mut ss4: f64,
    mut ss5: f64,
    mut ss6: f64,
    mut ss7: f64,
    mut sz1: f64,
    mut sz2: f64,
    mut sz3: f64,
    mut sz11: f64,
    mut sz12: f64,
    mut sz13: f64,
    mut sz21: f64,
    mut sz22: f64,
    mut sz23: f64,
    mut sz31: f64,
    mut sz32: f64,
    mut sz33: f64,
    mut xgh2: f64,
    mut xgh3: f64,
    mut xgh4: f64,
    mut xh2: f64,
    mut xh3: f64,
    mut xi2: f64,
    mut xi3: f64,
    mut xl2: f64,
    mut xl3: f64,
    mut xl4: f64,
    mut xn: f64,
    mut z1: f64,
    mut z2: f64,
    mut z3: f64,
    mut z11: f64,
    mut z12: f64,
    mut z13: f64,
    mut z21: f64,
    mut z22: f64,
    mut z23: f64,
    mut z31: f64,
    mut z32: f64,
    mut z33: f64,
    mut zmol: f64,
    mut zmos: f64,
) -> Result<
    (
        f64,
        f64,
        f64,
        f64,
        f64,
        f64,
        f64,
        f64,
        f64,
        f64,
        f64,
        f64,
        f64,
        f64,
        f64,
        f64,
        f64,
        f64,
        f64,
        f64,
        f64,
        f64,
        f64,
        f64,
        f64,
        f64,
        f64,
        f64,
        f64,
        f64,
        f64,
        f64,
        f64,
        f64,
        f64,
        f64,
        f64,
        f64,
        f64,
        f64,
        f64,
        f64,
        f64,
        f64,
        f64,
        f64,
        f64,
        f64,
        f64,
        f64,
        f64,
        f64,
        f64,
        f64,
        f64,
        f64,
        f64,
        f64,
        f64,
        f64,
        f64,
        f64,
        f64,
        f64,
        f64,
        f64,
        f64,
        f64,
        f64,
        f64,
        f64,
        f64,
        f64,
        f64,
        f64,
        f64,
        f64,
        f64,
        f64,
        f64,
        f64,
    ),
    GeoError,
> {
    let mut d__1: f64 = 0.0;
    let mut d__2: f64 = 0.0;

    let mut ctem: f64 = 0.0;
    let mut stem: f64 = 0.0;
    let mut xnoi: f64 = 0.0;

    let mut lsflg: i32 = 0;
    let mut a1: f64 = 0.0;
    let mut a2: f64 = 0.0;
    let mut a3: f64 = 0.0;
    let mut a4: f64 = 0.0;
    let mut a5: f64 = 0.0;
    let mut a6: f64 = 0.0;
    let mut a7: f64 = 0.0;
    let mut a8: f64 = 0.0;
    let mut a9: f64 = 0.0;
    let mut zcosg: f64 = 0.0;
    let mut zcosh: f64 = 0.0;
    let mut zcosi: f64 = 0.0;
    let mut zsing: f64 = 0.0;
    let mut zsinh: f64 = 0.0;
    let mut zsini: f64 = 0.0;

    let mut x1: f64 = 0.0;
    let mut x2: f64 = 0.0;
    let mut x3: f64 = 0.0;
    let mut x4: f64 = 0.0;
    let mut x5: f64 = 0.0;
    let mut x6: f64 = 0.0;
    let mut x7: f64 = 0.0;
    let mut x8: f64 = 0.0;
    let mut a10: f64 = 0.0;
    let mut cc: f64 = 0.0;
    let mut betasq: f64 = 0.0;
    let mut xnodce: f64 = 0.0;
    let mut zx: f64 = 0.0;
    let mut zy: f64 = 0.0;

    let mut c1l: f64 = 0.0;
    let mut zcosgl: f64 = 0.0;
    let mut zcoshl: f64 = 0.0;
    let mut zcosil: f64 = 0.0;
    let mut zsingl: f64 = 0.0;
    let mut zsinhl: f64 = 0.0;
    let mut zcosgs: f64 = 0.0;
    let mut zsinil: f64 = 0.0;
    let mut zcosis: f64 = 0.0;
    let mut zsings: f64 = 0.0;
    let mut zsinis: f64 = 0.0;

    let mut zel: f64 = 0.0;
    let mut zes: f64 = 0.0;
    let mut c1ss: f64 = 0.0;

    zes = 0.01675;
    zel = 0.0549;
    c1ss = 2.9864797e-6;
    c1l = 4.7968065e-7;
    zsinis = 0.39785416;
    zcosis = 0.91744867;
    zcosgs = 0.1945905;
    zsings = (-0.98088458);
    xn = np;
    eccm = eccp;
    snodm = (nodep).sin();
    cnodm = (nodep).cos();
    sinomm = (argpp).sin();
    cosomm = (argpp).cos();
    sinim = (inclp).sin();
    cosim = (inclp).cos();
    emsq = (eccm * eccm);
    betasq = (1.0 - emsq);
    rtemsq = (betasq).sqrt();
    peo = 0.0;
    pinco = 0.0;
    plo = 0.0;
    pgho = 0.0;
    pho = 0.0;
    day = ((epoch + 18261.5) + (tc / 1440.0));
    d__1 = (4.523602 - (day * 9.2422029e-4));
    d__2 = std::f64::consts::TAU;
    xnodce = f2c_mod(d__1, d__2);
    stem = (xnodce).sin();
    ctem = (xnodce).cos();
    zcosil = (0.91375164 - (ctem * 0.03568096));
    zsinil = (1.0 - (zcosil * zcosil)).sqrt();
    zsinhl = ((stem * 0.089683511) / zsinil);
    zcoshl = (1.0 - (zsinhl * zsinhl)).sqrt();
    gam = ((day * 0.001944368) + 5.8351514);
    zx = ((stem * 0.39785416) / zsinil);
    zy = ((zcoshl * ctem) + ((zsinhl * 0.91744867) * stem));
    zx = (zx).atan2(zy);
    zx = ((gam + zx) - xnodce);
    zcosgl = (zx).cos();
    zsingl = (zx).sin();
    zcosg = zcosgs;
    zsing = zsings;
    zcosi = zcosis;
    zsini = zsinis;
    zcosh = cnodm;
    zsinh = snodm;
    cc = c1ss;
    xnoi = (1.0 / xn);
    lsflg = 1;
    while (lsflg <= 2) {
        {
            a1 = ((zcosg * zcosh) + ((zsing * zcosi) * zsinh));
            a3 = (((-zsing) * zcosh) + ((zcosg * zcosi) * zsinh));
            a7 = (((-zcosg) * zsinh) + ((zsing * zcosi) * zcosh));
            a8 = (zsing * zsini);
            a9 = ((zsing * zsinh) + ((zcosg * zcosi) * zcosh));
            a10 = (zcosg * zsini);
            a2 = ((cosim * a7) + (sinim * a8));
            a4 = ((cosim * a9) + (sinim * a10));
            a5 = (((-sinim) * a7) + (cosim * a8));
            a6 = (((-sinim) * a9) + (cosim * a10));
            x1 = ((a1 * cosomm) + (a2 * sinomm));
            x2 = ((a3 * cosomm) + (a4 * sinomm));
            x3 = (((-a1) * sinomm) + (a2 * cosomm));
            x4 = (((-a3) * sinomm) + (a4 * cosomm));
            x5 = (a5 * sinomm);
            x6 = (a6 * sinomm);
            x7 = (a5 * cosomm);
            x8 = (a6 * cosomm);
            z31 = (((x1 * 12.0) * x1) - ((x3 * 3.0) * x3));
            z32 = (((x1 * 24.0) * x2) - ((x3 * 6.0) * x4));
            z33 = (((x2 * 12.0) * x2) - ((x4 * 3.0) * x4));
            z1 = ((((a1 * a1) + (a2 * a2)) * 3.0) + (z31 * emsq));
            z2 = ((((a1 * a3) + (a2 * a4)) * 6.0) + (z32 * emsq));
            z3 = ((((a3 * a3) + (a4 * a4)) * 3.0) + (z33 * emsq));
            z11 = (((a1 * (-6.0)) * a5) + (emsq * (((x1 * (-24.0)) * x7) - ((x3 * 6.0) * x5))));
            z12 = ((((a1 * a6) + (a3 * a5)) * (-6.0))
                + (emsq * ((((x2 * x7) + (x1 * x8)) * (-24.0)) - (((x3 * x6) + (x4 * x5)) * 6.0))));
            z13 = (((a3 * (-6.0)) * a6) + (emsq * (((x2 * (-24.0)) * x8) - ((x4 * 6.0) * x6))));
            z21 = (((a2 * 6.0) * a5) + (emsq * (((x1 * 24.0) * x5) - ((x3 * 6.0) * x7))));
            z22 = ((((a4 * a5) + (a2 * a6)) * 6.0)
                + (emsq * ((((x2 * x5) + (x1 * x6)) * 24.0) - (((x4 * x7) + (x3 * x8)) * 6.0))));
            z23 = (((a4 * 6.0) * a6) + (emsq * (((x2 * 24.0) * x6) - ((x4 * 6.0) * x8))));
            z1 = ((z1 + z1) + (betasq * z31));
            z2 = ((z2 + z2) + (betasq * z32));
            z3 = ((z3 + z3) + (betasq * z33));
            s3 = (cc * xnoi);
            s2 = ((s3 * (-0.5)) / rtemsq);
            s4 = (s3 * rtemsq);
            s1 = ((eccm * (-15.0)) * s4);
            s5 = ((x1 * x3) + (x2 * x4));
            s6 = ((x2 * x3) + (x1 * x4));
            s7 = ((x2 * x4) - (x1 * x3));
            if (lsflg == 1) {
                ss1 = s1;
                ss2 = s2;
                ss3 = s3;
                ss4 = s4;
                ss5 = s5;
                ss6 = s6;
                ss7 = s7;
                sz1 = z1;
                sz2 = z2;
                sz3 = z3;
                sz11 = z11;
                sz12 = z12;
                sz13 = z13;
                sz21 = z21;
                sz22 = z22;
                sz23 = z23;
                sz31 = z31;
                sz32 = z32;
                sz33 = z33;
                zcosg = zcosgl;
                zsing = zsingl;
                zcosi = zcosil;
                zsini = zsinil;
                zcosh = ((zcoshl * cnodm) + (zsinhl * snodm));
                zsinh = ((snodm * zcoshl) - (cnodm * zsinhl));
                cc = c1l;
            }
        }
        {
            lsflg += 1;
            lsflg
        };
    }
    d__1 = (((day * 0.2299715) + 4.7199672) - gam);
    d__2 = std::f64::consts::TAU;
    zmol = f2c_mod(d__1, d__2);
    d__1 = ((day * 0.017201977) + 6.2565837);
    d__2 = std::f64::consts::TAU;
    zmos = f2c_mod(d__1, d__2);
    se2 = ((ss1 * 2.0) * ss6);
    se3 = ((ss1 * 2.0) * ss7);
    si2 = ((ss2 * 2.0) * sz12);
    si3 = ((ss2 * 2.0) * (sz13 - sz11));
    sl2 = ((ss3 * (-2.0)) * sz2);
    sl3 = ((ss3 * (-2.0)) * (sz3 - sz1));
    sl4 = (((ss3 * (-2.0)) * ((-21.0) - (emsq * 9.0))) * zes);
    sgh2 = ((ss4 * 2.0) * sz32);
    sgh3 = ((ss4 * 2.0) * (sz33 - sz31));
    sgh4 = ((ss4 * (-18.0)) * zes);
    sh2 = ((ss2 * (-2.0)) * sz22);
    sh3 = ((ss2 * (-2.0)) * (sz23 - sz21));
    ee2 = ((s1 * 2.0) * s6);
    e3 = ((s1 * 2.0) * s7);
    xi2 = ((s2 * 2.0) * z12);
    xi3 = ((s2 * 2.0) * (z13 - z11));
    xl2 = ((s3 * (-2.0)) * z2);
    xl3 = ((s3 * (-2.0)) * (z3 - z1));
    xl4 = (((s3 * (-2.0)) * ((-21.0) - (emsq * 9.0))) * zel);
    xgh2 = ((s4 * 2.0) * z32);
    xgh3 = ((s4 * 2.0) * (z33 - z31));
    xgh4 = ((s4 * (-18.0)) * zel);
    xh2 = ((s2 * (-2.0)) * z22);
    xh3 = ((s2 * (-2.0)) * (z23 - z21));

    return Ok((
        snodm, cnodm, sinim, cosim, sinomm, cosomm, day, e3, ee2, eccm, emsq, gam, peo, pgho, pho,
        pinco, plo, rtemsq, se2, se3, sgh2, sgh3, sgh4, sh2, sh3, si2, si3, sl2, sl3, sl4, s1, s2,
        s3, s4, s5, s6, s7, ss1, ss2, ss3, ss4, ss5, ss6, ss7, sz1, sz2, sz3, sz11, sz12, sz13,
        sz21, sz22, sz23, sz31, sz32, sz33, xgh2, xgh3, xgh4, xh2, xh3, xi2, xi3, xl2, xl3, xl4,
        xn, z1, z2, z3, z11, z12, z13, z21, z22, z23, z31, z32, z33, zmol, zmos,
    ));
}
fn zzdsin(
    geophs: &[f64; 8],
    mut cosim: f64,
    mut emsq: f64,
    mut argpo: f64,
    mut s1: f64,
    mut s2: f64,
    mut s3: f64,
    mut s4: f64,
    mut s5: f64,
    mut sinim: f64,
    mut ss1: f64,
    mut ss2: f64,
    mut ss3: f64,
    mut ss4: f64,
    mut ss5: f64,
    mut sz1: f64,
    mut sz3: f64,
    mut sz11: f64,
    mut sz13: f64,
    mut sz21: f64,
    mut sz23: f64,
    mut sz31: f64,
    mut sz33: f64,
    mut t: f64,
    mut tc: f64,
    mut gsto: f64,
    mut mo: f64,
    mut mdot: f64,
    mut no: f64,
    mut nodeo: f64,
    mut nodedot: f64,
    mut xpidot: f64,
    mut z1: f64,
    mut z3: f64,
    mut z11: f64,
    mut z13: f64,
    mut z21: f64,
    mut z23: f64,
    mut z31: f64,
    mut z33: f64,
    mut ecco: f64,
    mut eccsq: f64,
    mut eccm: f64,
    mut argpm: f64,
    mut inclm: f64,
    mut mm: f64,
    mut xn: f64,
    mut nodem: f64,
    mut irez: i32,
    mut atime: f64,
    mut d2201: f64,
    mut d2211: f64,
    mut d3210: f64,
    mut d3222: f64,
    mut d4410: f64,
    mut d4422: f64,
    mut d5220: f64,
    mut d5232: f64,
    mut d5421: f64,
    mut d5433: f64,
    mut dedt: f64,
    mut didt: f64,
    mut dmdt: f64,
    mut dndt: f64,
    mut dnodt: f64,
    mut domdt: f64,
    mut del1: f64,
    mut del2: f64,
    mut del3: f64,
    mut xfact: f64,
    mut xlamo: f64,
    mut xli: f64,
    mut xni: f64,
) -> Result<
    (
        f64,
        f64,
        f64,
        f64,
        f64,
        f64,
        f64,
        i32,
        f64,
        f64,
        f64,
        f64,
        f64,
        f64,
        f64,
        f64,
        f64,
        f64,
        f64,
        f64,
        f64,
        f64,
        f64,
        f64,
        f64,
        f64,
        f64,
        f64,
        f64,
        f64,
        f64,
        f64,
    ),
    GeoError,
> {
    let mut d__1: f64 = 0.0;
    let mut d__2: f64 = 0.0;

    let mut sghl: f64 = 0.0;
    let mut aonv: f64 = 0.0;
    let mut sghs: f64 = 0.0;
    let mut temp: f64 = 0.0;
    let mut ainv2: f64 = 0.0;
    let mut sini2: f64 = 0.0;
    let mut temp1: f64 = 0.0;

    let mut theta: f64 = 0.0;
    let mut emsqo: f64 = 0.0;
    let mut root22: f64 = 0.0;
    let mut root32: f64 = 0.0;
    let mut root52: f64 = 0.0;
    let mut root44: f64 = 0.0;
    let mut root54: f64 = 0.0;
    let mut rptim: f64 = 0.0;

    let mut q22: f64 = 0.0;
    let mut q31: f64 = 0.0;
    let mut q33: f64 = 0.0;

    let mut g200: f64 = 0.0;
    let mut f220: f64 = 0.0;
    let mut f221: f64 = 0.0;
    let mut f311: f64 = 0.0;
    let mut f321: f64 = 0.0;
    let mut f322: f64 = 0.0;
    let mut f330: f64 = 0.0;
    let mut cosisq: f64 = 0.0;
    let mut f441: f64 = 0.0;
    let mut f442: f64 = 0.0;
    let mut f522: f64 = 0.0;
    let mut f523: f64 = 0.0;
    let mut f542: f64 = 0.0;
    let mut f543: f64 = 0.0;
    let mut g201: f64 = 0.0;
    let mut g211: f64 = 0.0;
    let mut g300: f64 = 0.0;
    let mut g310: f64 = 0.0;
    let mut g322: f64 = 0.0;
    let mut g410: f64 = 0.0;
    let mut g422: f64 = 0.0;
    let mut g520: f64 = 0.0;
    let mut g521: f64 = 0.0;
    let mut g532: f64 = 0.0;
    let mut g533: f64 = 0.0;

    let mut eoc: f64 = 0.0;
    let mut emo: f64 = 0.0;
    let mut shl: f64 = 0.0;
    let mut xke: f64 = 0.0;
    let mut ses: f64 = 0.0;
    let mut x2o3: f64 = 0.0;
    let mut sgs: f64 = 0.0;
    let mut shs: f64 = 0.0;
    let mut sis: f64 = 0.0;
    let mut sls: f64 = 0.0;
    let mut znl: f64 = 0.0;
    let mut zns: f64 = 0.0;
    let mut xno2: f64 = 0.0;

    q22 = 1.7891679e-6;
    q31 = 2.1460748e-6;
    q33 = 2.2123015e-7;
    root22 = 1.7891679e-6;
    root44 = 7.3636953e-9;
    root54 = 2.1765803e-9;
    rptim = 0.00437526908801129966;
    root32 = 3.7393792e-7;
    root52 = 1.1428639e-7;
    x2o3 = 0.66666666666666663;
    znl = 1.5835218e-4;
    zns = 1.19459e-5;
    xke = geophs[(3 as usize)];
    irez = 0;
    if ((xn < 0.0052359877) && (xn > 0.0034906585)) {
        irez = 1;
    }
    if (((xn >= 0.00826) && (xn <= 0.00924)) && (eccm >= 0.5)) {
        irez = 2;
    }
    ses = ((ss1 * zns) * ss5);
    sis = ((ss2 * zns) * (sz11 + sz13));
    sls = (((-zns) * ss3) * (((sz1 + sz3) - 14.0) - (emsq * 6.0)));
    sghs = ((ss4 * zns) * ((sz31 + sz33) - 6.0));
    shs = (((-zns) * ss2) * (sz21 + sz23));
    if ((inclm < 0.052359877) || (inclm > (std::f64::consts::PI - 0.052359877))) {
        shs = 0.0;
    }
    if (sinim != 0.0) {
        shs /= sinim;
    }
    sgs = (sghs - (cosim * shs));
    dedt = (ses + ((s1 * znl) * s5));
    didt = (sis + ((s2 * znl) * (z11 + z13)));
    dmdt = (sls - ((znl * s3) * (((z1 + z3) - 14.0) - (emsq * 6.0))));
    sghl = ((s4 * znl) * ((z31 + z33) - 6.0));
    shl = (((-znl) * s2) * (z21 + z23));
    if ((inclm < 0.052359877) || (inclm > (std::f64::consts::PI - 0.052359877))) {
        shl = 0.0;
    }
    domdt = (sgs + sghl);
    dnodt = shs;
    if (sinim != 0.0) {
        domdt -= ((cosim / sinim) * shl);
        dnodt += (shl / sinim);
    }
    dndt = 0.0;
    d__1 = (gsto + (tc * rptim));
    d__2 = std::f64::consts::TAU;
    theta = f2c_mod(d__1, d__2);
    eccm += (dedt * t);
    d__1 = eccm;
    emsq = (d__1 * d__1);
    inclm += (didt * t);
    argpm += (domdt * t);
    nodem += (dnodt * t);
    mm += (dmdt * t);
    if (irez != 0) {
        d__1 = (xn / xke);
        aonv = (d__1).powf(x2o3);
        if (irez == 2) {
            cosisq = (cosim * cosim);
            emo = eccm;
            emsqo = emsq;
            eccm = ecco;
            emsq = eccsq;
            eoc = (eccm * emsq);
            g201 = ((-0.306) - ((eccm - 0.64) * 0.44));
            if (eccm <= 0.65) {
                g211 = ((3.616 - (eccm * 13.247)) + (emsq * 16.29));
                g310 = ((((eccm * 117.39) - 19.302) - (emsq * 228.419)) + (eoc * 156.591));
                g322 = ((((eccm * 109.7927) - 18.9068) - (emsq * 214.6334)) + (eoc * 146.5816));
                g410 = ((((eccm * 242.694) - 41.122) - (emsq * 471.094)) + (eoc * 313.953));
                g422 = ((((eccm * 841.88) - 146.407) - (emsq * 1629.014)) + (eoc * 1083.435));
                g520 = ((((eccm * 3017.977) - 532.114) - (emsq * 5740.032)) + (eoc * 3708.276));
            } else {
                g211 = ((((eccm * 331.819) - 72.099) - (emsq * 508.738)) + (eoc * 266.724));
                g310 = ((((eccm * 1582.851) - 346.844) - (emsq * 2415.925)) + (eoc * 1246.113));
                g322 = ((((eccm * 1554.908) - 342.585) - (emsq * 2366.899)) + (eoc * 1215.972));
                g410 = ((((eccm * 4758.686) - 1052.797) - (emsq * 7193.992)) + (eoc * 3651.957));
                g422 = ((((eccm * 16178.11) - 3581.69) - (emsq * 24462.77)) + (eoc * 12422.52));
                if (eccm > 0.715) {
                    g520 = ((((eccm * 29936.92) - 5149.66) - (emsq * 54087.36)) + (eoc * 31324.56));
                } else {
                    g520 = ((1464.74 - (eccm * 4664.75)) + (emsq * 3763.64));
                }
            }
            if (eccm < 0.7) {
                g533 = ((((eccm * 4988.61) - 919.2277) - (emsq * 9064.77)) + (eoc * 5542.21));
                g521 = ((((eccm * 4568.6173) - 822.71072) - (emsq * 8491.4146)) + (eoc * 5337.524));
                g532 = ((((eccm * 4690.25) - 853.666) - (emsq * 8624.77)) + (eoc * 5341.4));
            } else {
                g533 = ((((eccm * 161616.52) - 37995.78) - (emsq * 229838.2)) + (eoc * 109377.94));
                g521 =
                    ((((eccm * 218913.95) - 51752.104) - (emsq * 309468.16)) + (eoc * 146349.42));
                g532 = ((((eccm * 170470.89) - 40023.88) - (emsq * 242699.48)) + (eoc * 115605.82));
            }
            sini2 = (sinim * sinim);
            f220 = ((((cosim * 2.0) + 1.0) + cosisq) * 0.75);
            f221 = (sini2 * 1.5);
            f321 = ((sinim * 1.875) * ((1.0 - (cosim * 2.0)) - (cosisq * 3.0)));
            f322 = ((sinim * (-1.875)) * (((cosim * 2.0) + 1.0) - (cosisq * 3.0)));
            f441 = ((sini2 * 35.0) * f220);
            f442 = ((sini2 * 39.375) * sini2);
            f522 = ((sinim * 9.84375)
                * ((sini2 * ((1.0 - (cosim * 2.0)) - (cosisq * 5.0)))
                    + ((((cosim * 4.0) - 2.0) + (cosisq * 6.0)) * 0.33333333)));
            f523 = (sinim
                * (((sini2 * 4.92187512) * (((-2.0) - (cosim * 4.0)) + (cosisq * 10.0)))
                    + ((((cosim * 2.0) + 1.0) - (cosisq * 3.0)) * 6.56250012)));
            f542 = ((sinim * 29.53125)
                * ((2.0 - (cosim * 8.0)) + (cosisq * (((cosim * 8.0) - 12.0) + (cosisq * 10.0)))));
            f543 = ((sinim * 29.53125)
                * (((-2.0) - (cosim * 8.0))
                    + (cosisq * (((cosim * 8.0) + 12.0) - (cosisq * 10.0)))));
            xno2 = (xn * xn);
            ainv2 = (aonv * aonv);
            temp1 = ((xno2 * 3.0) * ainv2);
            temp = (temp1 * root22);
            d2201 = ((temp * f220) * g201);
            d2211 = ((temp * f221) * g211);
            temp1 *= aonv;
            temp = (temp1 * root32);
            d3210 = ((temp * f321) * g310);
            d3222 = ((temp * f322) * g322);
            temp1 *= aonv;
            temp = ((temp1 * 2.0) * root44);
            d4410 = ((temp * f441) * g410);
            d4422 = ((temp * f442) * g422);
            temp1 *= aonv;
            temp = (temp1 * root52);
            d5220 = ((temp * f522) * g520);
            d5232 = ((temp * f523) * g532);
            temp = ((temp1 * 2.0) * root54);
            d5421 = ((temp * f542) * g521);
            d5433 = ((temp * f543) * g533);
            d__1 = ((((mo + nodeo) + nodeo) - theta) - theta);
            d__2 = std::f64::consts::TAU;
            xlamo = f2c_mod(d__1, d__2);
            xfact = (((mdot + dmdt) + (((nodedot + dnodt) - rptim) * 2.0)) - no);
            eccm = emo;
            emsq = emsqo;
        }
        if (irez == 1) {
            g200 = ((emsq * ((emsq * 0.8125) - 2.5)) + 1.0);
            g310 = ((emsq * 2.0) + 1.0);
            g300 = ((emsq * ((emsq * 6.60937) - 6.0)) + 1.0);
            f220 = (((cosim + 1.0) * 0.75) * (cosim + 1.0));
            f311 = ((((sinim * 0.9375) * sinim) * ((cosim * 3.0) + 1.0)) - ((cosim + 1.0) * 0.75));
            f330 = (cosim + 1.0);
            f330 = (((f330 * 1.875) * f330) * f330);
            del1 = ((((xn * 3.0) * xn) * aonv) * aonv);
            del2 = ((((del1 * 2.0) * f220) * g200) * q22);
            del3 = (((((del1 * 3.0) * f330) * g300) * q33) * aonv);
            del1 = ((((del1 * f311) * g310) * q31) * aonv);
            d__1 = (((mo + nodeo) + argpo) - theta);
            d__2 = std::f64::consts::TAU;
            xlamo = f2c_mod(d__1, d__2);
            xfact = ((((((mdot + xpidot) - rptim) + dmdt) + domdt) + dnodt) - no);
        }
        xli = xlamo;
        xni = no;
        atime = 0.0;
        xn = (no + dndt);
    }

    return Ok((
        emsq, eccm, argpm, inclm, mm, xn, nodem, irez, atime, d2201, d2211, d3210, d3222, d4410,
        d4422, d5220, d5232, d5421, d5433, dedt, didt, dmdt, dndt, dnodt, domdt, del1, del2, del3,
        xfact, xlamo, xli, xni,
    ));
}
fn zzdspr(
    mut opmode: i32,
    mut e3: f64,
    mut ee2: f64,
    mut peo: f64,
    mut pgho: f64,
    mut pho: f64,
    mut pinco: f64,
    mut plo: f64,
    mut se2: f64,
    mut se3: f64,
    mut sgh2: f64,
    mut sgh3: f64,
    mut sgh4: f64,
    mut sh2: f64,
    mut sh3: f64,
    mut si2: f64,
    mut si3: f64,
    mut sl2: f64,
    mut sl3: f64,
    mut sl4: f64,
    mut t: f64,
    mut xgh2: f64,
    mut xgh3: f64,
    mut xgh4: f64,
    mut xh2: f64,
    mut xh3: f64,
    mut xi2: f64,
    mut xi3: f64,
    mut xl2: f64,
    mut xl3: f64,
    mut xl4: f64,
    mut zmol: f64,
    mut zmos: f64,
    mut inclo: f64,
    mut doinit: bool,
    mut eccp: f64,
    mut inclp: f64,
    mut nodep: f64,
    mut argpp: f64,
    mut mp: f64,
) -> Result<(f64, f64, f64, f64, f64), GeoError> {
    let mut d__1: f64 = 0.0;

    let mut dalf: f64 = 0.0;
    let mut dbet: f64 = 0.0;
    let mut pinc: f64 = 0.0;
    let mut sghl: f64 = 0.0;
    let mut sghs: f64 = 0.0;
    let mut xnoh: f64 = 0.0;
    let mut alfdp: f64 = 0.0;

    let mut betdp: f64 = 0.0;
    let mut cosip: f64 = 0.0;
    let mut sinip: f64 = 0.0;
    let mut cosop: f64 = 0.0;
    let mut f2: f64 = 0.0;
    let mut f3: f64 = 0.0;
    let mut sinop: f64 = 0.0;
    let mut sinzf: f64 = 0.0;

    let mut pe: f64 = 0.0;
    let mut ph: f64 = 0.0;

    let mut pl: f64 = 0.0;
    let mut zf: f64 = 0.0;
    let mut zm: f64 = 0.0;

    let mut pgh: f64 = 0.0;
    let mut dls: f64 = 0.0;
    let mut sel: f64 = 0.0;
    let mut shl: f64 = 0.0;
    let mut ses: f64 = 0.0;
    let mut sil: f64 = 0.0;
    let mut shs: f64 = 0.0;
    let mut sis: f64 = 0.0;
    let mut sll: f64 = 0.0;
    let mut zel: f64 = 0.0;
    let mut sls: f64 = 0.0;
    let mut zes: f64 = 0.0;
    let mut znl: f64 = 0.0;
    let mut xls: f64 = 0.0;
    let mut zns: f64 = 0.0;

    zes = 0.01675;
    zel = 0.0549;
    zns = 1.19459e-5;
    znl = 1.5835218e-4;
    zm = (zmos + (zns * t));
    if doinit {
        zm = zmos;
    }
    zf = (zm + ((zes * 2.0) * (zm).sin()));
    sinzf = (zf).sin();
    f2 = (((sinzf * 0.5) * sinzf) - 0.25);
    f3 = ((sinzf * (-0.5)) * (zf).cos());
    ses = ((se2 * f2) + (se3 * f3));
    sis = ((si2 * f2) + (si3 * f3));
    sls = (((sl2 * f2) + (sl3 * f3)) + (sl4 * sinzf));
    sghs = (((sgh2 * f2) + (sgh3 * f3)) + (sgh4 * sinzf));
    shs = ((sh2 * f2) + (sh3 * f3));
    zm = (zmol + (znl * t));
    if doinit {
        zm = zmol;
    }
    zf = (zm + ((zel * 2.0) * (zm).sin()));
    sinzf = (zf).sin();
    f2 = (((sinzf * 0.5) * sinzf) - 0.25);
    f3 = ((sinzf * (-0.5)) * (zf).cos());
    sel = ((ee2 * f2) + (e3 * f3));
    sil = ((xi2 * f2) + (xi3 * f3));
    sll = (((xl2 * f2) + (xl3 * f3)) + (xl4 * sinzf));
    sghl = (((xgh2 * f2) + (xgh3 * f3)) + (xgh4 * sinzf));
    shl = ((xh2 * f2) + (xh3 * f3));
    pe = (ses + sel);
    pinc = (sis + sil);
    pl = (sls + sll);
    pgh = (sghs + sghl);
    ph = (shs + shl);
    if (!doinit) {
        pe -= peo;
        pinc -= pinco;
        pl -= plo;
        pgh -= pgho;
        ph -= pho;
        inclp += pinc;
        eccp += pe;
        sinip = (inclp).sin();
        cosip = (inclp).cos();
        if (inclp >= 0.2) {
            ph /= sinip;
            pgh -= (cosip * ph);
            argpp += pgh;
            nodep += ph;
            mp += pl;
        } else {
            sinop = (nodep).sin();
            cosop = (nodep).cos();
            alfdp = (sinip * sinop);
            betdp = (sinip * cosop);
            dalf = ((ph * cosop) + ((pinc * cosip) * sinop));
            dbet = (((-ph) * sinop) + ((pinc * cosip) * cosop));
            alfdp += dalf;
            betdp += dbet;
            d__1 = std::f64::consts::TAU;
            nodep = f2c_mod(nodep, d__1);
            if ((nodep < 0.0) && (opmode == 1)) {
                nodep += std::f64::consts::TAU;
            }
            xls = ((mp + argpp) + (cosip * nodep));
            dls = ((pl + pgh) - ((pinc * nodep) * sinip));
            xls += dls;
            xnoh = nodep;
            nodep = (alfdp).atan2(betdp);
            if ((nodep < 0.0) && (opmode == 1)) {
                nodep += std::f64::consts::TAU;
            }
            if ({
                d__1 = (xnoh - nodep);
                (d__1).abs()
            } > std::f64::consts::PI)
            {
                if (nodep < xnoh) {
                    nodep += std::f64::consts::TAU;
                } else {
                    nodep -= std::f64::consts::TAU;
                }
            }
            mp += pl;
            argpp = ((xls - mp) - (cosip * nodep));
        }
    }

    return Ok((eccp, inclp, nodep, argpp, mp));
}
fn zzdspc(
    mut irez: i32,
    mut d2201: f64,
    mut d2211: f64,
    mut d3210: f64,
    mut d3222: f64,
    mut d4410: f64,
    mut d4422: f64,
    mut d5220: f64,
    mut d5232: f64,
    mut d5421: f64,
    mut d5433: f64,
    mut dedt: f64,
    mut del1: f64,
    mut del2: f64,
    mut del3: f64,
    mut didt: f64,
    mut dmdt: f64,
    mut dnodt: f64,
    mut domdt: f64,
    mut argpo: f64,
    mut argpdot: f64,
    mut t: f64,
    mut tc: f64,
    mut gsto: f64,
    mut xfact: f64,
    mut xlamo: f64,
    mut no: f64,
    mut atime: f64,
    mut eccm: f64,
    mut argpm: f64,
    mut inclm: f64,
    mut xli: f64,
    mut mm: f64,
    mut xni: f64,
    mut nodem: f64,
    mut dndt: f64,
    mut xn: f64,
) -> Result<(f64, f64, f64, f64, f64, f64, f64, f64, f64, f64), GeoError> {
    let mut d__1: f64 = 0.0;
    let mut d__2: f64 = 0.0;

    let mut delt: f64 = 0.0;
    let mut iret: i32 = 0;
    let mut xndt: f64 = 0.0;
    let mut xomi: f64 = 0.0;
    let mut fasx2: f64 = 0.0;
    let mut fasx4: f64 = 0.0;
    let mut fasx6: f64 = 0.0;
    let mut step2: f64 = 0.0;
    let mut x2omi: f64 = 0.0;

    let mut theta: f64 = 0.0;
    let mut xnddt: f64 = 0.0;
    let mut iretn: i32 = 0;
    let mut stepn: f64 = 0.0;
    let mut xldot: f64 = 0.0;
    let mut rptim: f64 = 0.0;
    let mut stepp: f64 = 0.0;

    let mut g22: f64 = 0.0;
    let mut g32: f64 = 0.0;
    let mut g52: f64 = 0.0;
    let mut g44: f64 = 0.0;
    let mut g54: f64 = 0.0;
    let mut ft: f64 = 0.0;
    let mut xl: f64 = 0.0;

    let mut x2li: f64 = 0.0;

    fasx2 = 0.13130908;
    fasx4 = 2.8843198;
    fasx6 = 0.37448087;
    g22 = 5.7686396;
    g32 = 0.95240898;
    g44 = 1.8014998;
    g52 = 1.050833;
    g54 = 4.4108898;
    rptim = 0.00437526908801129966;
    stepp = 720.0;
    stepn = (-720.0);
    step2 = 259200.0;
    dndt = 0.0;
    d__1 = (gsto + (tc * rptim));
    d__2 = std::f64::consts::TAU;
    theta = f2c_mod(d__1, d__2);
    eccm += (dedt * t);
    inclm += (didt * t);
    argpm += (domdt * t);
    nodem += (dnodt * t);
    mm += (dmdt * t);
    ft = 0.0;
    if (irez != 0) {
        if (((atime == 0.0) || ((t * atime) <= 0.0)) || ((t).abs() < (atime).abs())) {
            atime = 0.0;
            xni = no;
            xli = xlamo;
        }
        if (t > 0.0) {
            delt = stepp;
        } else {
            delt = stepn;
        }
        iretn = 381;
        iret = 0;
        while (iretn == 381) {
            if (irez != 2) {
                xndt = (((del1 * (xli - fasx2).sin()) + (del2 * ((xli - fasx4) * 2.0).sin()))
                    + (del3 * ((xli - fasx6) * 3.0).sin()));
                xldot = (xni + xfact);
                xnddt = (((del1 * (xli - fasx2).cos())
                    + ((del2 * 2.0) * ((xli - fasx4) * 2.0).cos()))
                    + ((del3 * 3.0) * ((xli - fasx6) * 3.0).cos()));
                xnddt *= xldot;
            } else {
                xomi = (argpo + (argpdot * atime));
                x2omi = (xomi + xomi);
                x2li = (xli + xli);
                xndt = ((((((((((d2201 * ((x2omi + xli) - g22).sin())
                    + (d2211 * (xli - g22).sin()))
                    + (d3210 * ((xomi + xli) - g32).sin()))
                    + (d3222 * (((-xomi) + xli) - g32).sin()))
                    + (d4410 * ((x2omi + x2li) - g44).sin()))
                    + (d4422 * (x2li - g44).sin()))
                    + (d5220 * ((xomi + xli) - g52).sin()))
                    + (d5232 * (((-xomi) + xli) - g52).sin()))
                    + (d5421 * ((xomi + x2li) - g54).sin()))
                    + (d5433 * (((-xomi) + x2li) - g54).sin()));
                xldot = (xni + xfact);
                xnddt = (((((((d2201 * ((x2omi + xli) - g22).cos())
                    + (d2211 * (xli - g22).cos()))
                    + (d3210 * ((xomi + xli) - g32).cos()))
                    + (d3222 * (((-xomi) + xli) - g32).cos()))
                    + (d5220 * ((xomi + xli) - g52).cos()))
                    + (d5232 * (((-xomi) + xli) - g52).cos()))
                    + (((((d4410 * ((x2omi + x2li) - g44).cos())
                        + (d4422 * (x2li - g44).cos()))
                        + (d5421 * ((xomi + x2li) - g54).cos()))
                        + (d5433 * (((-xomi) + x2li) - g54).cos()))
                        * 2.0));
                xnddt *= xldot;
            }
            if ({
                d__1 = (t - atime);
                (d__1).abs()
            } >= stepp)
            {
                iret = 0;
                iretn = 381;
            } else {
                ft = (t - atime);
                iretn = 0;
            }
            if (iretn == 381) {
                xli = ((xli + (xldot * delt)) + (xndt * step2));
                xni = ((xni + (xndt * delt)) + (xnddt * step2));
                atime += delt;
            }
        }
        xn = ((xni + (xndt * ft)) + (((xnddt * ft) * ft) * 0.5));
        xl = ((xli + (xldot * ft)) + (((xndt * ft) * ft) * 0.5));
        if (irez != 1) {
            mm = ((xl - (nodem * 2.0)) + (theta * 2.0));
            dndt = (xn - no);
        } else {
            mm = (((xl - nodem) - argpm) + theta);
            dndt = (xn - no);
        }
        xn = (no + dndt);
    }

    return Ok((atime, eccm, argpm, inclm, xli, mm, xni, nodem, dndt, xn));
}
pub(crate) fn propagate(
    geophs: &[f64; 8],
    elems: &[f64; 10],
    mut opmode: i32,
    mut t: f64,
    epoch_days: f64,
) -> Result<[f64; 6], GeoError> {
    let mut state = [0.0; 6];

    let mut d__1: f64 = 0.0;
    let mut d__2: f64 = 0.0;

    let mut eccm: f64 = 0.0;
    let mut ecco: f64 = 0.0;
    let mut eccp: f64 = 0.0;
    let mut coef: f64 = 0.0;
    let mut eeta: f64 = 0.0;
    let mut alta: f64 = 0.0;
    let mut dedt: f64 = 0.0;
    let mut cnod: f64 = 0.0;
    let mut con41: f64 = 0.0;
    let mut con42: f64 = 0.0;
    let mut delm: f64 = 0.0;
    let mut didt: f64 = 0.0;
    let mut dmdt: f64 = 0.0;
    let mut dndt: f64 = 0.0;
    let mut pgho: f64 = 0.0;
    let mut ainv: f64 = 0.0;
    let mut cosi: f64 = 0.0;
    let mut altp: f64 = 0.0;
    let mut axnl: f64 = 0.0;
    let mut mdot: f64 = 0.0;
    let mut aynl: f64 = 0.0;
    let mut emsq: f64 = 0.0;
    let mut j3oj2: f64 = 0.0;
    let mut sini: f64 = 0.0;
    let mut snod: f64 = 0.0;
    let mut cosu: f64 = 0.0;
    let mut temp: f64 = 0.0;
    let mut gsto: f64 = 0.0;
    let mut sinu: f64 = 0.0;
    let mut tvec: [f64; 8] = [0.0; 8];
    let mut xinc: f64 = 0.0;
    let mut zmol: f64 = 0.0;
    let mut posq: f64 = 0.0;
    let mut xmdf: f64 = 0.0;
    let mut iter: i32 = 0;
    let mut irez: i32 = 0;
    let mut zmos: f64 = 0.0;
    let mut coef1: f64 = 0.0;
    let mut cc1sq: f64 = 0.0;
    let mut t2cof: f64 = 0.0;
    let mut t3cof: f64 = 0.0;
    let mut t4cof: f64 = 0.0;
    let mut t5cof: f64 = 0.0;
    let mut temp1: f64 = 0.0;
    let mut temp2: f64 = 0.0;
    let mut temp3: f64 = 0.0;
    let mut temp4: f64 = 0.0;
    let mut cos2u: f64 = 0.0;
    let mut sin2u: f64 = 0.0;
    let mut a: f64 = 0.0;
    let mut betal: f64 = 0.0;
    let mut u: f64 = 0.0;

    let mut eccsq: f64 = 0.0;
    let mut atime: f64 = 0.0;
    let mut aycof: f64 = 0.0;
    let mut cnodm: f64 = 0.0;
    let mut inclo: f64 = 0.0;
    let mut xfact: f64 = 0.0;
    let mut pinco: f64 = 0.0;
    let mut argpm: f64 = 0.0;
    let mut argpo: f64 = 0.0;
    let mut xlcof: f64 = 0.0;
    let mut xmcof: f64 = 0.0;
    let mut argpp: f64 = 0.0;
    let mut bstar: f64 = 0.0;
    let mut cosim: f64 = 0.0;
    let mut xlamo: f64 = 0.0;
    let mut cosio: f64 = 0.0;
    let mut x1mth2: f64 = 0.0;
    let mut cosip: f64 = 0.0;
    let mut delmo: f64 = 0.0;
    let mut d2: f64 = 0.0;
    let mut d3: f64 = 0.0;
    let mut x7thm1: f64 = 0.0;
    let mut e3: f64 = 0.0;
    let mut d4: f64 = 0.0;
    let mut dnodt: f64 = 0.0;
    let mut cossu: f64 = 0.0;
    let mut domdt: f64 = 0.0;
    let mut ecose: f64 = 0.0;
    let mut epoch: f64 = 0.0;
    let mut esine: f64 = 0.0;
    let mut etasq: f64 = 0.0;
    let mut inclm: f64 = 0.0;
    let mut j2: f64 = 0.0;
    let mut j3: f64 = 0.0;
    let mut j4: f64 = 0.0;
    let mut nodem: f64 = 0.0;
    let mut nodeo: f64 = 0.0;
    let mut nodep: f64 = 0.0;
    let mut psisq: f64 = 0.0;
    let mut qzms24: f64 = 0.0;
    let mut rdotl: f64 = 0.0;
    let mut rvdot: f64 = 0.0;
    let mut s1: f64 = 0.0;
    let mut s2: f64 = 0.0;
    let mut s3: f64 = 0.0;
    let mut s4: f64 = 0.0;
    let mut s5: f64 = 0.0;
    let mut s6: f64 = 0.0;
    let mut s7: f64 = 0.0;
    let mut sfour: f64 = 0.0;
    let mut sinim: f64 = 0.0;
    let mut sinio: f64 = 0.0;
    let mut sinip: f64 = 0.0;
    let mut coseo1: f64 = 0.0;
    let mut sinsu: f64 = 0.0;
    let mut snodm: f64 = 0.0;
    let mut t2: f64 = 0.0;
    let mut t3: f64 = 0.0;
    let mut cosio2: f64 = 0.0;
    let mut sineo1: f64 = 0.0;
    let mut cosio4: f64 = 0.0;
    let mut t4: f64 = 0.0;
    let mut tempa: f64 = 0.0;
    let mut tempe: f64 = 0.0;
    let mut templ: f64 = 0.0;
    let mut tumin: f64 = 0.0;
    let mut tzero: f64 = 0.0;
    let mut xhdot1: f64 = 0.0;
    let mut xincp: f64 = 0.0;
    let mut xnode: f64 = 0.0;
    let mut z1: f64 = 0.0;
    let mut z2: f64 = 0.0;
    let mut z3: f64 = 0.0;

    let mut am: f64 = 0.0;

    let mut ao: f64 = 0.0;

    let mut er: f64 = 0.0;
    let mut tc: f64 = 0.0;

    let mut mm: f64 = 0.0;
    let mut mo: f64 = 0.0;
    let mut mp: f64 = 0.0;
    let mut no: f64 = 0.0;
    let mut mr: f64 = 0.0;
    let mut omgadf: f64 = 0.0;
    let mut pl: f64 = 0.0;
    let mut mv: f64 = 0.0;
    let mut qzms2t: f64 = 0.0;
    let mut rl: f64 = 0.0;
    let mut delomg: f64 = 0.0;
    let mut rp: f64 = 0.0;
    let mut omgcof: f64 = 0.0;
    let mut perige: f64 = 0.0;
    let mut ss: f64 = 0.0;
    let mut su: f64 = 0.0;
    let mut ux: f64 = 0.0;
    let mut uy: f64 = 0.0;
    let mut xnodcf: f64 = 0.0;
    let mut uz: f64 = 0.0;
    let mut cc1: f64 = 0.0;
    let mut sinmao: f64 = 0.0;
    let mut cc2: f64 = 0.0;
    let mut cc4: f64 = 0.0;
    let mut cc5: f64 = 0.0;
    let mut ee2: f64 = 0.0;
    let mut cc3: f64 = 0.0;
    let mut cosomm: f64 = 0.0;
    let mut vx: f64 = 0.0;
    let mut cosisq: f64 = 0.0;
    let mut el2: f64 = 0.0;
    let mut eo1: f64 = 0.0;
    let mut omeosq: f64 = 0.0;
    let mut sinomm: f64 = 0.0;
    let mut vy: f64 = 0.0;
    let mut vz: f64 = 0.0;
    let mut rvdotl: f64 = 0.0;
    let mut rtemsq: f64 = 0.0;
    let mut se2: f64 = 0.0;
    let mut rteosq: f64 = 0.0;
    let mut se3: f64 = 0.0;
    let mut sh2: f64 = 0.0;
    let mut pinvsq: f64 = 0.0;
    let mut sh3: f64 = 0.0;
    let mut xh2: f64 = 0.0;
    let mut xh3: f64 = 0.0;
    let mut xi2: f64 = 0.0;
    let mut xi3: f64 = 0.0;
    let mut xl2: f64 = 0.0;
    let mut xl3: f64 = 0.0;
    let mut xl4: f64 = 0.0;
    let mut si2: f64 = 0.0;
    let mut si3: f64 = 0.0;
    let mut sl2: f64 = 0.0;
    let mut sl3: f64 = 0.0;
    let mut sl4: f64 = 0.0;
    let mut ss1: f64 = 0.0;
    let mut ss2: f64 = 0.0;
    let mut ss3: f64 = 0.0;
    let mut ss4: f64 = 0.0;
    let mut ss5: f64 = 0.0;
    let mut ss6: f64 = 0.0;
    let mut ss7: f64 = 0.0;
    let mut sz1: f64 = 0.0;
    let mut sz2: f64 = 0.0;
    let mut sz3: f64 = 0.0;
    let mut xl: f64 = 0.0;
    let mut d2201: f64 = 0.0;
    let mut d2211: f64 = 0.0;
    let mut d3210: f64 = 0.0;
    let mut xn: f64 = 0.0;
    let mut d3222: f64 = 0.0;
    let mut d4410: f64 = 0.0;
    let mut d5220: f64 = 0.0;
    let mut d4422: f64 = 0.0;
    let mut d5232: f64 = 0.0;
    let mut d5421: f64 = 0.0;
    let mut d5433: f64 = 0.0;
    let mut xnoddf: f64 = 0.0;
    let mut gam: f64 = 0.0;
    let mut xpidot: f64 = 0.0;
    let mut z11: f64 = 0.0;
    let mut z12: f64 = 0.0;
    let mut z13: f64 = 0.0;
    let mut eta: f64 = 0.0;
    let mut z21: f64 = 0.0;
    let mut z22: f64 = 0.0;
    let mut z23: f64 = 0.0;
    let mut day: f64 = 0.0;
    let mut z31: f64 = 0.0;
    let mut z32: f64 = 0.0;
    let mut z33: f64 = 0.0;
    let mut svmode: i32 = 0;
    let mut doinit: bool = false;
    let mut peo: f64 = 0.0;
    let mut dosimp: bool = false;
    let mut dodeep: bool = false;
    let mut pho: f64 = 0.0;
    let mut xke: f64 = 0.0;
    let mut plo: f64 = 0.0;
    let mut x2o3: f64 = 0.0;
    let mut xli: f64 = 0.0;
    let mut kps: f64 = 0.0;
    let mut xni: f64 = 0.0;
    let mut sz11: f64 = 0.0;
    let mut sz12: f64 = 0.0;
    let mut sz13: f64 = 0.0;
    let mut sz21: f64 = 0.0;
    let mut sz22: f64 = 0.0;
    let mut sz23: f64 = 0.0;
    let mut sz31: f64 = 0.0;
    let mut sz32: f64 = 0.0;
    let mut sz33: f64 = 0.0;
    let mut tsi: f64 = 0.0;
    let mut xlm: f64 = 0.0;

    let mut nodedot: f64 = 0.0;
    let mut xmx: f64 = 0.0;
    let mut xmy: f64 = 0.0;

    let mut argpdot: f64 = 0.0;

    let mut del1: f64 = 0.0;
    let mut del2: f64 = 0.0;
    let mut del3: f64 = 0.0;

    let mut sgh2: f64 = 0.0;
    let mut sgh3: f64 = 0.0;
    let mut sgh4: f64 = 0.0;
    let mut xgh2: f64 = 0.0;
    let mut xgh3: f64 = 0.0;
    let mut xgh4: f64 = 0.0;
    let mut tem5: f64 = 0.0;

    dodeep = false;
    dosimp = false;
    svmode = opmode;
    bstar = elems[(2 as usize)];
    inclo = elems[(3 as usize)];
    nodeo = elems[(4 as usize)];
    ecco = elems[(5 as usize)];
    argpo = elems[(6 as usize)];
    mo = elems[(7 as usize)];
    no = elems[(8 as usize)];

    epoch = epoch_days;

    j2 = geophs[(0 as usize)];
    j3 = geophs[(1 as usize)];
    j4 = geophs[(2 as usize)];
    er = geophs[(6 as usize)];
    xke = geophs[(3 as usize)];
    tumin = (1.0 / geophs[(3 as usize)]);
    j3oj2 = (j3 / j2);
    d__1 = (no * tumin);
    a = (d__1).powf(c_b10);
    if ({
        d__1 = (ecco - 1.0);
        (d__1).abs()
    } > 1e-6)
    {
        altp = ((a * (1.0 - ecco)) - 1.0);
        alta = ((a * (ecco + 1.0)) - 1.0);
    } else {
        alta = 999999.9;
        d__1 = (no * no);
        altp = ((4.0 / (d__1).powf(c_b11)) * 2.0);
    }
    ss = ((78.0 / er) + 1.0);
    d__1 = (42.0 / er);
    d__1 *= d__1;
    qzms2t = (d__1 * d__1);
    x2o3 = 0.66666666666666663;
    temp4 = 1.5e-12;
    tzero = 0.0;
    doinit = true;
    (
        no, ainv, ao, con41, con42, cosio, cosio2, eccsq, omeosq, posq, rp, rteosq, sinio, gsto,
    ) = zzinil(
        geophs, opmode, ecco, epoch, inclo, no, ainv, ao, con41, con42, cosio, cosio2, eccsq,
        omeosq, posq, rp, rteosq, sinio, gsto,
    )?;

    if (rp < 1.0) {
        return Err(GeoError::BadSearch("SPICE(SUBORBITAL)"));

        return Ok(state);
    }
    if ((omeosq >= 0.0) || (no >= 0.0)) {
        dosimp = false;
        if (rp < ((220.0 / er) + 1.0)) {
            dosimp = true;
        }
        sfour = ss;
        qzms24 = qzms2t;
        perige = ((rp - 1.0) * er);
        if (perige < 156.0) {
            sfour = (perige - 78.0);
            if (perige <= 98.0) {
                sfour = 20.0;
            }
            d__1 = ((120.0 - sfour) / er);
            d__1 *= d__1;
            qzms24 = (d__1 * d__1);
            sfour = ((sfour / er) + 1.0);
        }
        pinvsq = (1.0 / posq);
        tsi = (1.0 / (ao - sfour));
        eta = ((ao * ecco) * tsi);
        etasq = (eta * eta);
        eeta = (ecco * eta);
        psisq = {
            d__1 = (1.0 - etasq);
            (d__1).abs()
        };
        d__1 = tsi;
        d__1 *= d__1;
        coef = (qzms24 * (d__1 * d__1));
        coef1 = (coef / (psisq).powf(c_b16));
        cc2 = ((coef1 * no)
            * ((ao * (((etasq * 1.5) + 1.0) + (eeta * (etasq + 4.0))))
                + (((((j2 * 0.375) * tsi) / psisq) * con41)
                    * (((etasq * 3.0) * (etasq + 8.0)) + 8.0))));
        cc1 = (bstar * cc2);
        cc3 = 0.0;
        if (ecco > 1e-4) {
            cc3 = ((((((coef * (-2.0)) * tsi) * j3oj2) * no) * sinio) / ecco);
        }
        x1mth2 = (1.0 - cosio2);
        cc4 = (((((no * 2.0) * coef1) * ao) * omeosq)
            * (((eta * ((etasq * 0.5) + 2.0)) + (ecco * ((etasq * 2.0) + 0.5)))
                - (((j2 * tsi) / (ao * psisq))
                    * (((con41 * (-3.0))
                        * ((1.0 - (eeta * 2.0)) + (etasq * (1.5 - (eeta * 0.5)))))
                        + (((x1mth2 * 0.75) * ((etasq * 2.0) - (eeta * (etasq + 1.0))))
                            * (argpo * 2.0).cos())))));
        cc5 =
            ((((coef1 * 2.0) * ao) * omeosq) * ((((etasq + eeta) * 2.75) + 1.0) + (eeta * etasq)));
        cosio4 = (cosio2 * cosio2);
        temp1 = (((j2 * 1.5) * pinvsq) * no);
        temp2 = (((temp1 * 0.5) * j2) * pinvsq);
        temp3 = ((((j4 * (-0.46875)) * pinvsq) * pinvsq) * no);
        mdot = ((no + (((temp1 * 0.5) * rteosq) * con41))
            + (((temp2 * 0.0625) * rteosq) * ((13.0 - (cosio2 * 78.0)) + (cosio4 * 137.0))));
        argpdot = ((((temp1 * (-0.5)) * con42)
            + ((temp2 * 0.0625) * ((7.0 - (cosio2 * 114.0)) + (cosio4 * 395.0))))
            + (temp3 * ((3.0 - (cosio2 * 36.0)) + (cosio4 * 49.0))));
        xhdot1 = ((-temp1) * cosio);
        nodedot = (xhdot1
            + ((((temp2 * 0.5) * (4.0 - (cosio2 * 19.0)))
                + ((temp3 * 2.0) * (3.0 - (cosio2 * 7.0))))
                * cosio));
        xpidot = (argpdot + nodedot);
        omgcof = ((bstar * cc3) * (argpo).cos());
        xmcof = 0.0;
        if (ecco > 1e-4) {
            xmcof = ((((-x2o3) * coef) * bstar) / eeta);
        }
        xnodcf = (((omeosq * 3.5) * xhdot1) * cc1);
        t2cof = (cc1 * 1.5);
        if ({
            d__1 = (cosio + 1.0);
            (d__1).abs()
        } > 1.5e-12)
        {
            xlcof = ((((j3oj2 * (-0.25)) * sinio) * ((cosio * 5.0) + 3.0)) / (cosio + 1.0));
        } else {
            xlcof = ((((j3oj2 * (-0.25)) * sinio) * ((cosio * 5.0) + 3.0)) / temp4);
        }
        aycof = ((j3oj2 * (-0.5)) * sinio);
        d__1 = ((eta * (mo).cos()) + 1.0);
        delmo = (d__1 * (d__1 * d__1));
        sinmao = (mo).sin();
        x7thm1 = ((cosio2 * 7.0) - 1.0);
        if ((std::f64::consts::TAU / no) >= 225.0) {
            dodeep = true;
            dosimp = true;
            tc = 0.0;
            inclm = inclo;
            (
                snodm, cnodm, sinim, cosim, sinomm, cosomm, day, e3, ee2, eccm, emsq, gam, peo,
                pgho, pho, pinco, plo, rtemsq, se2, se3, sgh2, sgh3, sgh4, sh2, sh3, si2, si3, sl2,
                sl3, sl4, s1, s2, s3, s4, s5, s6, s7, ss1, ss2, ss3, ss4, ss5, ss6, ss7, sz1, sz2,
                sz3, sz11, sz12, sz13, sz21, sz22, sz23, sz31, sz32, sz33, xgh2, xgh3, xgh4, xh2,
                xh3, xi2, xi3, xl2, xl3, xl4, xn, z1, z2, z3, z11, z12, z13, z21, z22, z23, z31,
                z32, z33, zmol, zmos,
            ) = zzdscm(
                epoch, ecco, argpo, tc, inclo, nodeo, no, snodm, cnodm, sinim, cosim, sinomm,
                cosomm, day, e3, ee2, eccm, emsq, gam, peo, pgho, pho, pinco, plo, rtemsq, se2,
                se3, sgh2, sgh3, sgh4, sh2, sh3, si2, si3, sl2, sl3, sl4, s1, s2, s3, s4, s5, s6,
                s7, ss1, ss2, ss3, ss4, ss5, ss6, ss7, sz1, sz2, sz3, sz11, sz12, sz13, sz21, sz22,
                sz23, sz31, sz32, sz33, xgh2, xgh3, xgh4, xh2, xh3, xi2, xi3, xl2, xl3, xl4, xn,
                z1, z2, z3, z11, z12, z13, z21, z22, z23, z31, z32, z33, zmol, zmos,
            )?;
            (ecco, inclo, nodeo, argpo, mo) = zzdspr(
                opmode, e3, ee2, peo, pgho, pho, pinco, plo, se2, se3, sgh2, sgh3, sgh4, sh2, sh3,
                si2, si3, sl2, sl3, sl4, tzero, xgh2, xgh3, xgh4, xh2, xh3, xi2, xi3, xl2, xl3,
                xl4, zmol, zmos, inclm, doinit, ecco, inclo, nodeo, argpo, mo,
            )?;
            argpm = 0.0;
            nodem = 0.0;
            mm = 0.0;
            (
                emsq, eccm, argpm, inclm, mm, xn, nodem, irez, atime, d2201, d2211, d3210, d3222,
                d4410, d4422, d5220, d5232, d5421, d5433, dedt, didt, dmdt, dndt, dnodt, domdt,
                del1, del2, del3, xfact, xlamo, xli, xni,
            ) = zzdsin(
                geophs, cosim, emsq, argpo, s1, s2, s3, s4, s5, sinim, ss1, ss2, ss3, ss4, ss5,
                sz1, sz3, sz11, sz13, sz21, sz23, sz31, sz33, tzero, tc, gsto, mo, mdot, no, nodeo,
                nodedot, xpidot, z1, z3, z11, z13, z21, z23, z31, z33, ecco, eccsq, eccm, argpm,
                inclm, mm, xn, nodem, irez, atime, d2201, d2211, d3210, d3222, d4410, d4422, d5220,
                d5232, d5421, d5433, dedt, didt, dmdt, dndt, dnodt, domdt, del1, del2, del3, xfact,
                xlamo, xli, xni,
            )?;
        }
        if (!dosimp) {
            cc1sq = (cc1 * cc1);
            d2 = (((ao * 4.0) * tsi) * cc1sq);
            temp = (((d2 * tsi) * cc1) / 3.0);
            d3 = (((ao * 17.0) + sfour) * temp);
            d4 = (((((temp * 0.5) * ao) * tsi) * ((ao * 221.0) + (sfour * 31.0))) * cc1);
            t3cof = (d2 + (cc1sq * 2.0));
            t4cof = (((d3 * 3.0) + (cc1 * ((d2 * 12.0) + (cc1sq * 10.0)))) * 0.25);
            t5cof = (((((d4 * 3.0) + ((cc1 * 12.0) * d3)) + ((d2 * 6.0) * d2))
                + ((cc1sq * 15.0) * ((d2 * 2.0) + cc1sq)))
                * 0.2);
        }
    }
    doinit = false;

    x2o3 = 0.66666666666666663;
    mr = 0.0;
    coseo1 = 1.0;
    sineo1 = 0.0;
    temp4 = 1.5e-12;
    kps = ((er * xke) / 60.0);
    xmdf = (mo + (mdot * t));
    omgadf = (argpo + (argpdot * t));
    xnoddf = (nodeo + (nodedot * t));
    argpm = omgadf;
    mm = xmdf;
    t2 = (t * t);
    nodem = (xnoddf + (xnodcf * t2));
    tempa = (1.0 - (cc1 * t));
    tempe = ((bstar * cc4) * t);
    templ = (t2cof * t2);
    if (!dosimp) {
        delomg = (omgcof * t);
        d__1 = ((eta * (xmdf).cos()) + 1.0);
        delm = (xmcof * ((d__1 * (d__1 * d__1)) - delmo));
        temp = (delomg + delm);
        mm = (xmdf + temp);
        argpm = (omgadf - temp);
        t3 = (t2 * t);
        t4 = (t3 * t);
        tempa = (((tempa - (d2 * t2)) - (d3 * t3)) - (d4 * t4));
        tempe += ((bstar * cc5) * ((mm).sin() - sinmao));
        templ = ((templ + (t3cof * t3)) + (t4 * (t4cof + (t * t5cof))));
    }
    xn = no;
    eccm = ecco;
    inclm = inclo;
    if dodeep {
        tc = t;
        (atime, eccm, argpm, inclm, xli, mm, xni, nodem, dndt, xn) = zzdspc(
            irez, d2201, d2211, d3210, d3222, d4410, d4422, d5220, d5232, d5421, d5433, dedt, del1,
            del2, del3, didt, dmdt, dnodt, domdt, argpo, argpdot, t, tc, gsto, xfact, xlamo, no,
            atime, eccm, argpm, inclm, xli, mm, xni, nodem, dndt, xn,
        )?;
    }
    if (xn <= 0.0) {
        return Err(GeoError::BadSearch("SPICE(BADMEANMOTION)"));

        return Ok(state);
    }
    d__1 = (xke / xn);
    d__2 = tempa;
    am = ((d__1).powf(x2o3) * (d__2 * d__2));
    xn = (xke / (am).powf(c_b22));
    eccm -= tempe;
    if ((eccm >= 1.0) || (eccm < (-0.001))) {
        return Err(GeoError::BadSearch("SPICE(BADMECCENTRICITY)"));

        return Ok(state);
    }
    if (am < 0.95) {
        return Err(GeoError::BadSearch("SPICE(BADMSEMIMAJOR)"));

        return Ok(state);
    }
    if (eccm < 1e-6) {
        eccm = 1e-6;
    }
    mm += (no * templ);
    xlm = ((mm + argpm) + nodem);
    emsq = (eccm * eccm);
    temp = (1.0 - emsq);
    d__1 = std::f64::consts::TAU;
    nodem = f2c_mod(nodem, d__1);
    d__1 = std::f64::consts::TAU;
    argpm = f2c_mod(argpm, d__1);
    d__1 = std::f64::consts::TAU;
    xlm = f2c_mod(xlm, d__1);
    d__1 = ((xlm - argpm) - nodem);
    d__2 = std::f64::consts::TAU;
    mm = f2c_mod(d__1, d__2);
    sinim = (inclm).sin();
    cosim = (inclm).cos();
    eccp = eccm;
    xincp = inclm;
    argpp = argpm;
    nodep = nodem;
    mp = mm;
    sinip = sinim;
    cosip = cosim;
    if dodeep {
        (eccp, xincp, nodep, argpp, mp) = zzdspr(
            svmode, e3, ee2, peo, pgho, pho, pinco, plo, se2, se3, sgh2, sgh3, sgh4, sh2, sh3, si2,
            si3, sl2, sl3, sl4, t, xgh2, xgh3, xgh4, xh2, xh3, xi2, xi3, xl2, xl3, xl4, zmol, zmos,
            inclo, c_false, eccp, xincp, nodep, argpp, mp,
        )?;
        if (xincp < 0.0) {
            xincp = (-xincp);
            nodep += std::f64::consts::PI;
            argpp -= std::f64::consts::PI;
        }
        if ((eccp < 0.0) || (eccp > 1.0)) {
            return Err(GeoError::BadSearch("SPICE(BADPECCENTRICITY)"));

            return Ok(state);
        }
    }
    if dodeep {
        sinip = (xincp).sin();
        cosip = (xincp).cos();
        aycof = ((j3oj2 * (-0.5)) * sinip);
        if ({
            d__1 = (cosip + 1.0);
            (d__1).abs()
        } > 1.5e-12)
        {
            xlcof = ((((j3oj2 * (-0.25)) * sinip) * ((cosip * 5.0) + 3.0)) / (cosip + 1.0));
        } else {
            xlcof = ((((j3oj2 * (-0.25)) * sinip) * ((cosip * 5.0) + 3.0)) / temp4);
        }
    }
    axnl = (eccp * (argpp).cos());
    temp = (1.0 / (am * (1.0 - (eccp * eccp))));
    aynl = ((eccp * (argpp).sin()) + (temp * aycof));
    xl = (((mp + argpp) + nodep) + ((temp * xlcof) * axnl));
    d__1 = (xl - nodep);
    d__2 = std::f64::consts::TAU;
    u = f2c_mod(d__1, d__2);
    eo1 = u;
    iter = 0;
    temp = 9999.9;
    while ((temp >= 1e-12) && (iter < 10)) {
        {
            iter += 1;
            iter
        };
        sineo1 = (eo1).sin();
        coseo1 = (eo1).cos();
        tem5 = ((1.0 - (coseo1 * axnl)) - (sineo1 * aynl));
        tem5 = ((((u - (aynl * coseo1)) + (axnl * sineo1)) - eo1) / tem5);
        temp = (tem5).abs();
        if (temp > 1.0) {
            tem5 /= temp;
        }
        eo1 += tem5;
    }
    ecose = ((axnl * coseo1) + (aynl * sineo1));
    esine = ((axnl * sineo1) - (aynl * coseo1));
    el2 = ((axnl * axnl) + (aynl * aynl));
    pl = (am * (1.0 - el2));
    if (pl < 0.0) {
        return Err(GeoError::BadSearch("SPICE(BADSEMILATUS)"));

        return Ok(state);
    }
    rl = (am * (1.0 - ecose));
    rdotl = (((am).sqrt() * esine) / rl);
    rvdotl = ((pl).sqrt() / rl);
    betal = (1.0 - el2).sqrt();
    temp = (esine / (betal + 1.0));
    sinu = ((am / rl) * ((sineo1 - aynl) - (axnl * temp)));
    cosu = ((am / rl) * ((coseo1 - axnl) + (aynl * temp)));
    su = (sinu).atan2(cosu);
    sin2u = ((cosu + cosu) * sinu);
    cos2u = (1.0 - ((sinu * 2.0) * sinu));
    temp = (1.0 / pl);
    temp1 = ((j2 * 0.5) * temp);
    temp2 = (temp1 * temp);
    if dodeep {
        cosisq = (cosip * cosip);
        con41 = ((cosisq * 3.0) - 1.0);
        x1mth2 = (1.0 - cosisq);
        x7thm1 = ((cosisq * 7.0) - 1.0);
    }
    mr = ((rl * (1.0 - (((temp2 * 1.5) * betal) * con41))) + (((temp1 * 0.5) * x1mth2) * cos2u));
    su -= (((temp2 * 0.25) * x7thm1) * sin2u);
    xnode = (nodep + (((temp2 * 1.5) * cosip) * sin2u));
    xinc = (xincp + ((((temp2 * 1.5) * cosip) * sinip) * cos2u));
    mv = (rdotl - ((((xn * temp1) * x1mth2) * sin2u) / xke));
    rvdot = (rvdotl + (((xn * temp1) * ((x1mth2 * cos2u) + (con41 * 1.5))) / xke));
    sinsu = (su).sin();
    cossu = (su).cos();
    snod = (xnode).sin();
    cnod = (xnode).cos();
    sini = (xinc).sin();
    cosi = (xinc).cos();
    xmx = ((-snod) * cosi);
    xmy = (cnod * cosi);
    ux = ((xmx * sinsu) + (cnod * cossu));
    uy = ((xmy * sinsu) + (snod * cossu));
    uz = (sini * sinsu);
    vx = ((xmx * cossu) - (cnod * sinsu));
    vy = ((xmy * cossu) - (snod * sinsu));
    vz = (sini * cossu);
    state[(0 as usize)] = ((mr * ux) * er);
    state[(1 as usize)] = ((mr * uy) * er);
    state[(2 as usize)] = ((mr * uz) * er);
    state[(3 as usize)] = (((mv * ux) + (rvdot * vx)) * kps);
    state[(4 as usize)] = (((mv * uy) + (rvdot * vy)) * kps);
    state[(5 as usize)] = (((mv * uz) + (rvdot * vz)) * kps);
    if (mr < 1.0) {
        return Err(GeoError::BadSearch("SPICE(ORBITDECAY)"));

        return Ok(state);
    }

    return Ok(state);
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_sgp4_teme_states_exact() {
        let mut count = 0;
        let mut errors = Vec::new();
        for line in include_str!("../../tests/golden/sgp4_teme.csv")
            .lines()
            .skip(1)
        {
            let f: Vec<_> = line.split(',').collect();
            let g: [f64; 8] = std::array::from_fn(|i| f[i].parse().unwrap());
            let e: [f64; 10] = std::array::from_fn(|i| f[8 + i].parse().unwrap());
            let mode = f[18].parse().unwrap();
            let t = f[19].parse().unwrap();
            let epoch = f[20].parse().unwrap();
            let actual = propagate(&g, &e, mode, t, epoch);
            if f[21] == "OK" {
                let expected: [f64; 6] = std::array::from_fn(|i| f[22 + i].parse().unwrap());
                if actual.as_ref().map(|x| x.map(f64::to_bits)).ok()
                    != Some(expected.map(f64::to_bits))
                {
                    errors.push(format!(
                        "case{count} t={t}: {actual:?} expected{expected:?}"
                    ));
                }
            } else if !matches!(&actual,Err(GeoError::BadSearch(message)) if *message==f[21]) {
                errors.push(format!("case{count}: expected{} got{actual:?}", f[21]));
            }
            count += 1;
        }
        assert_eq!(count, 12740);
        assert!(
            errors.is_empty(),
            "{} mismatches/{count}:\n{}",
            errors.len(),
            errors
                .iter()
                .take(8)
                .cloned()
                .collect::<Vec<_>>()
                .join("\n")
        );
    }
}
