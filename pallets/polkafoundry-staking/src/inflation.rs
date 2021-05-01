use sp_runtime::{Perbill, traits::AtLeast32BitUnsigned, curve::PiecewiseLinear};
use crate::taylor_series::compute_inflation;
use sp_arithmetic::PerThing;
use sp_arithmetic::traits::Saturating;

pub fn compute_total_payout<P>(
	i_0: P,
	i_ideal: P,
	x: P,
	x_ideal: P,
	d: P,
	round_duration: u64
) -> (P, P)
	where P: PerThing
{
	// Milliseconds per year for the Julian year (365.25 days).
	const MILLISECONDS_PER_YEAR: u64 = 1000 * 3600 * 24 * 36525 / 100;

	let portion = P::from_rational(round_duration as u128, MILLISECONDS_PER_YEAR as u128);
	let payout = portion * compute_i_npos(i_0, i_ideal, x, x_ideal, d);

	let maximum = portion * payout;
	(payout, maximum)
}

pub fn compute_i_npos<P: PerThing> (
	i_0: P,
	i_ideal: P,
	x: P,
	x_ideal: P,
	d: P,
) -> P {
	let i_npos_ideal = i_ideal * x_ideal;

	let compute = compute_inflation::<P>(
		x,
		x_ideal,
		d
	);
	i_0.saturating_add(i_npos_ideal.saturating_sub(i_0) * compute_inflation::<P>(
		x,
		x_ideal,
		d
	))
}

#[cfg(test)]
mod test {
	use sp_arithmetic::{PerThing, Perbill, PerU16, Percent, Perquintill};
	use super::*;

	/// This test the precision and panics if error too big error.
	///
	/// error is asserted to be less or equal to 8/accuracy or 8*f64::EPSILON
	fn test_precision<P: PerThing>(interest_min: P, interest_ideal: P, stake: P, ideal_stake: P, falloff: P) {
		let accuracy_f64 = Into::<u128>::into(P::ACCURACY) as f64;
		let res = compute_i_npos(interest_min, interest_ideal, stake, ideal_stake, falloff);
		let res = Into::<u128>::into(res.deconstruct()) as f64 / accuracy_f64;

		let expect = float_i_npos(interest_min, interest_ideal, stake, ideal_stake, falloff);

		let error = (res - expect).abs();

		if error > 8f64 / accuracy_f64 && error > 8.0 * f64::EPSILON {
			panic!(
				"stake: {:?}, ideal_stake: {:?}, falloff: {:?}, res: {}, expect: {}",
				stake, ideal_stake, falloff, res, expect
			);
		}
	}

	/// compute the inflation using floats
	fn float_i_npos<P: PerThing>(interest_min: P, interest_ideal: P, stake: P, ideal_stake: P, falloff: P) -> f64 {
		let accuracy_f64 = Into::<u128>::into(P::ACCURACY) as f64;

		let ideal_stake = Into::<u128>::into(ideal_stake.deconstruct()) as f64 / accuracy_f64;
		let stake = Into::<u128>::into(stake.deconstruct()) as f64 / accuracy_f64;
		let falloff = Into::<u128>::into(falloff.deconstruct()) as f64 / accuracy_f64;
		let interest_min = Into::<u128>::into(interest_min.deconstruct()) as f64 / accuracy_f64;
		let interest_ideal = Into::<u128>::into(interest_ideal.deconstruct()) as f64 / accuracy_f64;

		let x_ideal = ideal_stake;
		let x = stake;
		let d = falloff;
		let mut i_0 = interest_min;
		let i_ideal = interest_ideal;
		if i_0 > i_ideal * x_ideal {
			i_0 = (i_ideal * x_ideal) / f64::from(10)
		}

		if x < x_ideal {
			i_0 + (i_ideal * x_ideal - i_0) * (x / x_ideal)
		} else {
			i_0 + (i_ideal * x_ideal - i_0) * 2_f64.powf((x_ideal - x) / d)
		}
	}

	#[test]
	fn compute_inflation_works() {
		fn compute_inflation_works<P: PerThing>() {
			for stake in 0..100 {
				for ideal_stake in 1..10 {
					for falloff in 1..10 {
						for interest_ideal in 1..10 {
							for interest_min in 1..10 {
								let stake = P::from_rational(stake, 100);
								let ideal_stake = P::from_rational(ideal_stake, 10);
								let interest_ideal = P::from_rational(interest_ideal, 10);
								let interest_min = P::from_rational(interest_min, 1000);
								let falloff = P::from_rational(falloff, 100);
								test_precision(interest_min, interest_ideal, stake, ideal_stake, falloff);
							}
						}
					}
				}
			}

			compute_inflation_works::<Perquintill>();

			compute_inflation_works::<PerU16>();

			compute_inflation_works::<Perbill>();

			compute_inflation_works::<Percent>();
		}
	}
}
