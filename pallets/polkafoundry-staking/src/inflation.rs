use sp_runtime::{Perbill, traits::AtLeast32BitUnsigned};
use crate::taylor_series::compute_inflation;
use sp_arithmetic::PerThing;
use std::ops::Mul;

pub fn compute_total_payout<N>(
	npos_token_staked: N,
	total_tokens: N,
	i_0: u32,
	i_ideal: u32,
	x_ideal: u32,
	d: u32,
	round_duration: u64
) -> N
	where N: AtLeast32BitUnsigned + Clone
{
	// Milliseconds per year for the Julian year (365.25 days).
	const MILLISECONDS_PER_YEAR: u64 = 1000 * 3600 * 24 * 36525 / 100;
	let i_0 = Perbill::from_rational(i_0, 1000);
	let i_ideal = Perbill::from_rational(i_ideal, 100);
	let x = Perbill::from_rational(npos_token_staked, total_tokens.clone());
	let x_ideal = Perbill::from_rational(x_ideal, 100);
	let d = Perbill::from_rational(d, 100);

	let portion = Perbill::from_rational(round_duration,MILLISECONDS_PER_YEAR);

	portion * (compute_i_npos(i_0, i_ideal, x, x_ideal, d)).mul(total_tokens)
}

pub fn compute_i_npos<P: PerThing> (
	i_0: P,
	i_ideal: P,
	x: P,
	x_ideal: P,
	d: P,
) -> P {
	let i_npos_ideal = i_ideal * x_ideal;
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

	#[test]
	fn compute_total_payout_work() {
		const YEAR: u64 = 365 * 24 * 60 * 60 * 1000;

		assert_eq!(super::compute_total_payout(0, 100_000u64, 2_5u32, 20u32, 50u32, 5u32, YEAR), 2498);
		assert_eq!(super::compute_total_payout(5_000, 100_000u64, 2_5u32, 20u32, 50u32, 5u32, YEAR), 3_248);
		assert_eq!(super::compute_total_payout(25_000, 100_000u64, 2_5u32, 20u32, 50u32, 5u32, YEAR), 6_246);
		assert_eq!(super::compute_total_payout(40_000, 100_000u64, 2_5u32, 20u32, 50u32, 5u32, YEAR), 8_494);
		assert_eq!(super::compute_total_payout(50_000, 100_000u64, 2_5u32, 20u32, 50u32, 5u32, YEAR), 9_993);
		assert_eq!(super::compute_total_payout(60_000, 100_000u64, 2_5u32, 20u32, 50u32, 5u32, YEAR), 4_372);
		assert_eq!(super::compute_total_payout(75_000, 100_000u64, 2_5u32, 20u32, 50u32, 5u32, YEAR), 2_732);
		assert_eq!(super::compute_total_payout(95_000, 100_000u64, 2_5u32, 20u32, 50u32, 5u32, YEAR), 2_513);
		assert_eq!(super::compute_total_payout(100_000, 100_000u64, 2_5u32, 20u32, 50u32, 5u32, YEAR), 2_505);

		const DAY: u64 = 24 * 60 * 60 * 1000;
		assert_eq!(super::compute_total_payout(0, 100_000u64, 2_5u32, 20u32, 50u32, 5u32, DAY), 7);
		assert_eq!(super::compute_total_payout(25_000, 100_000u64, 2_5u32, 20u32, 50u32, 5u32, DAY), 17);
		assert_eq!(super::compute_total_payout(50_000, 100_000u64, 2_5u32, 20u32, 50u32, 5u32, DAY), 27);
		assert_eq!(super::compute_total_payout(100_000, 100_000u64, 2_5u32, 20u32, 50u32, 5u32, DAY), 7);

		const SIX_HOURS: u64 = 6 * 60 * 60 * 1000;
		assert_eq!(super::compute_total_payout(0, 100_000u64, 2_5u32, 20u32, 50u32, 5u32, SIX_HOURS), 2);
		assert_eq!(super::compute_total_payout(25_000, 100_000u64, 2_5u32, 20u32, 50u32, 5u32, SIX_HOURS), 4);
		assert_eq!(super::compute_total_payout(50_000, 100_000u64, 2_5u32, 20u32, 50u32, 5u32, SIX_HOURS), 7);
		assert_eq!(super::compute_total_payout(100_000, 100_000u64, 2_5u32, 20u32, 50u32, 5u32, SIX_HOURS), 2);

		const HOUR: u64 = 60 * 60 * 1000;
		assert_eq!(super::compute_total_payout(2_500_000_000_000_000_000_000_000_000u128, 5_000_000_000_000_000_000_000_000_000u128, 2_5u32, 20u32, 50u32, 5u32, HOUR), 57_038_500_000_000_000_000_000);

	}
}
