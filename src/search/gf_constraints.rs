//! WebGeocalc RANGE scheduling and GF window postprocessing.
//!
//! The callback performs a native GF relation on the supplied confinement
//! window. RANGE searches reuse the first result as the second confinement;
//! separately searching both bounds changes the source solver's endpoint bits.

use crate::finder::{
    contract_windows, expand_windows, filter_windows, normalize_windows, Condition, Interval,
    ScalarConstraint,
};
use crate::GeoError;

/// Native coordinate branch, in the input reference units. For WGC angles use
/// degrees here, and convert to native radians inside the search callback.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ScalarDomain {
    /// Nonperiodic quantities: reversed RANGE bounds are exchanged.
    Linear,
    /// Periodic coordinate branch. Signed longitudes retain both -pi and +pi
    /// (`upper_inclusive=true`); right ascension and cylindrical longitudes use
    /// the half-open [0,2*pi) branch (`upper_inclusive=false`).
    Periodic {
        origin: f64,
        period: f64,
        upper_inclusive: bool,
    },
}

/// Units accepted by the WebGeocalc GF duration parameters.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DurationUnits {
    #[default]
    Seconds,
    Minutes,
    Hours,
    Days,
}

impl DurationUnits {
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_uppercase().as_str() {
            "SECONDS" => Some(Self::Seconds),
            "MINUTES" => Some(Self::Minutes),
            "HOURS" => Some(Self::Hours),
            "DAYS" => Some(Self::Days),
            _ => None,
        }
    }

    fn scale(self) -> f64 {
        match self {
            Self::Seconds => 1.0,
            Self::Minutes => 60.0,
            Self::Hours => 3600.0,
            Self::Days => 86400.0,
        }
    }
}

/// An amount with explicit duration units.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WindowDuration {
    pub amount: f64,
    pub units: DurationUnits,
}

impl WindowDuration {
    pub fn seconds(self) -> Result<f64, GeoError> {
        let seconds = self.amount * self.units.scale();
        if !seconds.is_finite() {
            return Err(GeoError::BadSearch("window duration must be finite"));
        }
        Ok(seconds)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum IntervalAdjustment {
    #[default]
    None,
    Expand(WindowDuration),
    Contract(WindowDuration),
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum IntervalFiltering {
    #[default]
    None,
    /// Remove durations less than or equal to the threshold (WNFLTD).
    Filter(WindowDuration),
}

/// Operations run in order: complement, adjustment, filtering.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct WindowPostprocessing {
    pub complement: bool,
    pub adjustment: IntervalAdjustment,
    pub filtering: IntervalFiltering,
    pub output_duration_units: DurationUnits,
}

/// Compose native scalar searches, retaining their exact confinement schedule.
///
/// Bounds and domain use the original input units. Convert units inside the
/// callback, after RANGE scheduling. Converting 10 and 370 degrees to radians
/// before choosing an arc can reverse their normalized order through rounding.
pub fn apply_scalar_constraint<F>(
    mut native_finder: F,
    original_confinement: &[Interval],
    constraint: ScalarConstraint,
    domain: ScalarDomain,
) -> Result<Vec<Interval>, GeoError>
where
    F: FnMut(Condition, f64, &[Interval]) -> Result<Vec<Interval>, GeoError>,
{
    let original = normalize_windows(original_confinement)?;
    let (lower, upper, periodic) = match constraint {
        ScalarConstraint::Relation {
            condition,
            reference,
        } => {
            finite_bounds(reference, reference)?;
            if original.is_empty() {
                return Ok(original);
            }
            return normalize_windows(&native_finder(condition, reference, &original)?);
        }
        ScalarConstraint::Range { lower, upper } => (lower, upper, false),
        ScalarConstraint::AngularRange {
            lower,
            upper,
            period,
        } => {
            let ScalarDomain::Periodic {
                period: native_period,
                ..
            } = domain
            else {
                return Err(GeoError::BadSearch(
                    "angular RANGE requires a periodic native domain",
                ));
            };
            if period != native_period {
                return Err(GeoError::BadSearch(
                    "constraint period differs from native domain period",
                ));
            }
            (lower, upper, true)
        }
    };
    finite_bounds(lower, upper)?;
    let branch = if periodic {
        let ScalarDomain::Periodic {
            origin,
            period,
            upper_inclusive,
        } = domain
        else {
            unreachable!()
        };
        if !origin.is_finite()
            || !period.is_finite()
            || period <= 0.0
            || !(origin + period).is_finite()
        {
            return Err(GeoError::BadSearch(
                "periodic domain must have a finite origin and positive period",
            ));
        }
        Some((
            origin,
            origin + period,
            wrap_bound(lower, origin, period, upper_inclusive)?,
            wrap_bound(upper, origin, period, upper_inclusive)?,
        ))
    } else {
        None
    };
    if original.is_empty() {
        return Ok(original);
    }
    if lower == upper {
        return normalize_windows(&native_finder(Condition::Equal, lower, &original)?);
    }

    let (first, second, skip_first, skip_second, complement) =
        if let Some((min, max, a, b)) = branch {
            if a > b {
                (upper, lower, b == min, a == max, true)
            } else {
                (lower, upper, a == min, b == max, false)
            }
        } else if lower > upper {
            (upper, lower, false, false, false)
        } else {
            (lower, upper, false, false, false)
        };
    let first_result = if skip_first {
        original.clone()
    } else {
        normalize_windows(&native_finder(Condition::GreaterThan, first, &original)?)?
    };
    let result = if skip_second || first_result.is_empty() {
        first_result
    } else {
        normalize_windows(&native_finder(Condition::LessThan, second, &first_result)?)?
    };
    if complement {
        relative_complement(&result, &original)
    } else {
        Ok(result)
    }
}

fn finite_bounds(lower: f64, upper: f64) -> Result<(), GeoError> {
    if !lower.is_finite() || !upper.is_finite() {
        return Err(GeoError::BadSearch("constraint references must be finite"));
    }
    Ok(())
}

fn wrap_bound(
    value: f64,
    origin: f64,
    period: f64,
    upper_inclusive: bool,
) -> Result<f64, GeoError> {
    // Keep both exact endpoints: -pi..pi is a full longitude window, while
    // pi..-pi is its empty complement. Native GF handles the actual reference
    // normalization, so use this result only to select the RANGE schedule.
    if value >= origin && (value < origin + period || (upper_inclusive && value == origin + period))
    {
        return Ok(value);
    }
    let offset = value - origin;
    if !offset.is_finite() {
        return Err(GeoError::BadSearch(
            "periodic reference cannot be normalized",
        ));
    }
    let result = origin + offset.rem_euclid(period);
    if upper_inclusive && result == origin && value > origin {
        Ok(origin + period)
    } else {
        Ok(result)
    }
}

pub(crate) fn relative_complement(
    result: &[Interval],
    original: &[Interval],
) -> Result<Vec<Interval>, GeoError> {
    let result = normalize_windows(result)?;
    let original = normalize_windows(original)?;
    // N0067 WNDIFD. Unlike complementing each bounding interval with WNCOMD,
    // this removes a singleton in A when it is contained in B.
    let mut output = Vec::new();
    let mut j = 0;
    for confinement in &original {
        let (mut first, mut last) = (confinement.start, confinement.end);
        let mut keep = true;
        while j < result.len() {
            let b = result[j];
            if last < b.start {
                break;
            } else if first > b.end {
                j += 1;
            } else if b.start <= first && last <= b.end {
                keep = false;
                break;
            } else if b.start <= first {
                first = b.end;
                j += 1;
            } else if first <= b.start && last >= b.end && b.start < b.end {
                output.push(Interval {
                    start: first,
                    end: b.start,
                });
                first = b.end;
                if first == last {
                    keep = false;
                    break;
                }
                j += 1;
            } else if first <= b.start && last >= b.end && b.start == b.end {
                j += 1;
            } else {
                last = b.start;
                break;
            }
        }
        if keep {
            output.push(Interval {
                start: first,
                end: last,
            });
        }
    }
    Ok(output)
}

/// Apply WGC window operations relative to every original confinement segment.
pub fn postprocess_windows(
    result: &[Interval],
    original_confinement: &[Interval],
    parameters: WindowPostprocessing,
) -> Result<Vec<Interval>, GeoError> {
    let original = normalize_windows(original_confinement)?;
    let mut windows = normalize_windows(result)?;
    if parameters.complement {
        windows = relative_complement(&windows, &original)?;
    }
    windows = match parameters.adjustment {
        IntervalAdjustment::None => windows,
        IntervalAdjustment::Expand(amount) => {
            let seconds = amount.seconds()?;
            expand_windows(&windows, seconds, seconds)?
        }
        IntervalAdjustment::Contract(amount) => {
            let seconds = amount.seconds()?;
            contract_windows(&windows, seconds, seconds)?
        }
    };
    if let IntervalFiltering::Filter(threshold) = parameters.filtering {
        windows = filter_windows(&windows, threshold.seconds()?)?;
    }
    Ok(windows)
}

/// Durations in requested units, preserving interval order.
pub fn window_durations(windows: &[Interval], units: DurationUnits) -> Result<Vec<f64>, GeoError> {
    // CONVRT: first convert input to seconds, then divide by output scale.
    Ok(normalize_windows(windows)?
        .iter()
        .map(|w| (w.end - w.start) / units.scale())
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn w(a: f64, b: f64) -> Interval {
        Interval { start: a, end: b }
    }

    #[test]
    fn range_searches_are_serial_and_reversed_linear_bounds_are_swapped() {
        for (lower, upper) in [(2., 7.), (7., 2.)] {
            let mut calls = Vec::new();
            let actual = apply_scalar_constraint(
                |c, r, cf| {
                    calls.push((c, r, cf.to_vec()));
                    Ok(if c == Condition::GreaterThan {
                        vec![w(3., 9.)]
                    } else {
                        vec![w(3., 6.)]
                    })
                },
                &[w(0., 10.)],
                ScalarConstraint::Range { lower, upper },
                ScalarDomain::Linear,
            )
            .unwrap();
            assert_eq!(actual, vec![w(3., 6.)]);
            assert_eq!(
                calls,
                vec![
                    (Condition::GreaterThan, 2., vec![w(0., 10.)]),
                    (Condition::LessThan, 7., vec![w(3., 9.)])
                ]
            );
        }
    }

    #[test]
    fn angular_wrap_complements_serial_opposite_arc_without_branch_gaps() {
        let mut calls = Vec::new();
        let actual = apply_scalar_constraint(
            |c, r, cf| {
                calls.push((c, r, cf.to_vec()));
                Ok(if c == Condition::GreaterThan {
                    vec![w(0., 30.), w(33., 86.)]
                } else {
                    vec![w(0., 27.), w(33., 86.)]
                })
            },
            &[w(0., 86.)],
            ScalarConstraint::AngularRange {
                lower: 170.,
                upper: -170.,
                period: 360.,
            },
            ScalarDomain::Periodic {
                origin: -180.,
                period: 360.,
                upper_inclusive: true,
            },
        )
        .unwrap();
        assert_eq!(
            calls,
            vec![
                (Condition::GreaterThan, -170., vec![w(0., 86.)]),
                (Condition::LessThan, 170., vec![w(0., 30.), w(33., 86.)])
            ]
        );
        assert_eq!(actual, vec![w(27., 33.)]);
    }

    #[test]
    fn equality_is_a_native_equal_search() {
        let mut calls = Vec::new();
        let result = apply_scalar_constraint(
            |c, r, cf| {
                calls.push((c, r, cf.to_vec()));
                Ok(vec![w(4., 4.)])
            },
            &[w(0., 10.)],
            ScalarConstraint::Range {
                lower: 4.,
                upper: 4.,
            },
            ScalarDomain::Linear,
        )
        .unwrap();
        assert_eq!(calls, vec![(Condition::Equal, 4., vec![w(0., 10.)])]);
        assert_eq!(result, vec![w(4., 4.)]);
    }

    #[test]
    fn nasa_complement_adjust_filter_order_and_units() {
        let cnfine = [w(0., 100.), w(200., 300.)];
        let mut p = WindowPostprocessing {
            complement: true,
            ..Default::default()
        };
        assert_eq!(postprocess_windows(&[], &cnfine, p).unwrap(), cnfine);
        p.adjustment = IntervalAdjustment::Expand(WindowDuration {
            amount: 10.,
            units: DurationUnits::Seconds,
        });
        assert_eq!(
            postprocess_windows(&[], &cnfine, p).unwrap(),
            vec![w(-10., 110.), w(190., 310.)]
        );
        p.filtering = IntervalFiltering::Filter(WindowDuration {
            amount: 2.,
            units: DurationUnits::Minutes,
        });
        assert!(postprocess_windows(&[], &cnfine, p).unwrap().is_empty());
        p.adjustment = IntervalAdjustment::Contract(WindowDuration {
            amount: 1.,
            units: DurationUnits::Minutes,
        });
        p.filtering = IntervalFiltering::None;
        assert!(postprocess_windows(&[], &cnfine, p).unwrap().is_empty());
        assert_eq!(
            window_durations(&cnfine, DurationUnits::Hours).unwrap(),
            vec![100. / 3600.; 2]
        );
    }

    #[test]
    fn complement_keeps_the_gaps_in_multisegment_confinement() {
        assert_eq!(
            postprocess_windows(
                &[w(5., 20.), w(35., 35.)],
                &[w(0., 10.), w(30., 40.)],
                WindowPostprocessing {
                    complement: true,
                    ..Default::default()
                }
            )
            .unwrap(),
            vec![w(0., 5.), w(30., 40.)]
        );
    }

    #[test]
    fn nasa_accepts_signed_adjustments_and_filter_thresholds() {
        let cf = [w(0., 100.), w(200., 300.)];
        let p = WindowPostprocessing {
            complement: true,
            adjustment: IntervalAdjustment::Expand(WindowDuration {
                amount: -10.,
                units: DurationUnits::Seconds,
            }),
            filtering: IntervalFiltering::Filter(WindowDuration {
                amount: -1.,
                units: DurationUnits::Seconds,
            }),
            ..Default::default()
        };
        assert_eq!(
            postprocess_windows(&[], &cf, p).unwrap(),
            vec![w(10., 90.), w(210., 290.)]
        );
    }

    #[test]
    fn signed_and_positive_angular_branch_boundaries_differ() {
        let cf = [w(0., 86400.)];
        let mut calls = Vec::new();
        let result = apply_scalar_constraint(
            |c, r, w| {
                calls.push((c, r, w.to_vec()));
                Ok(vec![])
            },
            &cf,
            ScalarConstraint::AngularRange {
                lower: -180.,
                upper: 180.,
                period: 360.,
            },
            ScalarDomain::Periodic {
                origin: -180.,
                period: 360.,
                upper_inclusive: true,
            },
        )
        .unwrap();
        assert_eq!(result, cf);
        assert!(calls.is_empty());
        let result = apply_scalar_constraint(
            |c, r, w| {
                calls.push((c, r, w.to_vec()));
                Ok(vec![])
            },
            &cf,
            ScalarConstraint::AngularRange {
                lower: 0.,
                upper: 360.,
                period: 360.,
            },
            ScalarDomain::Periodic {
                origin: 0.,
                period: 360.,
                upper_inclusive: false,
            },
        )
        .unwrap();
        assert!(result.is_empty());
        assert_eq!(calls, vec![(Condition::LessThan, 360., cf.to_vec())]);
    }

    #[test]
    fn signed_multiturn_endpoints_retain_their_branch_side() {
        assert_eq!(wrap_bound(540., -180., 360., true).unwrap(), 180.);
        assert_eq!(wrap_bound(-540., -180., 360., true).unwrap(), -180.);
    }

    #[test]
    fn complement_removes_a_covered_singleton_confinement() {
        let p = WindowPostprocessing {
            complement: true,
            ..Default::default()
        };
        assert!(postprocess_windows(&[w(5., 5.)], &[w(5., 5.)], p)
            .unwrap()
            .is_empty());
        assert!(postprocess_windows(&[w(0., 10.)], &[w(5., 5.)], p)
            .unwrap()
            .is_empty());
        assert_eq!(
            postprocess_windows(&[], &[w(5., 5.)], p).unwrap(),
            vec![w(5., 5.)]
        );
    }
}
