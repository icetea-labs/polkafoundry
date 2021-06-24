//! Staking pallet benchmarking.

use super::*;
use crate::Pallet as Staking;
use testing_utils::*;

use sp_runtime::traits::One;
use frame_system::RawOrigin;
pub use frame_benchmarking::{
	benchmarks, account, whitelisted_caller, whitelist_account, impl_benchmark_test_suite,
};

const SEED: u32 = 0;
const MAX_SPANS: u32 = 100;
const MAX_VALIDATORS: u32 = 1000;
const MAX_NOMINATORS: u32 = 1000;
const MAX_SLASHES: u32 = 1000;

const USER_SEED: u32 = 999666;

benchmarks! {
	bond {
		let stash = create_funded_user::<T>("stash", USER_SEED, 100);
		let controller = create_funded_user::<T>("controller", USER_SEED, 100);
		let reward_destination = RewardDestination::Staked;
		let amount = T::Currency::minimum_balance() * 10u32.into();
		whitelist_account!(stash);
	}: _(RawOrigin::Signed(stash.clone()), controller, amount, reward_destination)
	verify {
		assert!(Bonded::<T>::contains_key(stash));
		assert!(Ledger::<T>::contains_key(controller));
	}

}

impl_benchmark_test_suite!(
	Staking,
	crate::mock::ExtBuilder::default(),
	crate::mock::Test,
	exec_name = build_and_execute
);
