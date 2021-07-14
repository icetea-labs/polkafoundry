#![cfg_attr(not(feature = "std"), no_std)]
pub mod elections;

use frame_support::{
	parameter_types,
	weights::{Weight, constants::WEIGHT_PER_SECOND, DispatchClass},
	traits::{Currency, Imbalance, OnUnbalanced}
};
use frame_system::limits;
use sp_runtime::{Perbill};
pub use frame_support::weights::constants::{BlockExecutionWeight, ExtrinsicBaseWeight, RocksDbWeight};
pub use runtime_primitives::{BlockNumber, Moment};
pub use pkfp_primitives::Price;
pub use elections::{OffchainSolutionLengthLimit, OffchainSolutionWeightLimit};

/// We assume that ~10% of the block weight is consumed by `on_initalize` handlers.
/// This is used to limit the maximal weight of a single extrinsic.
const AVERAGE_ON_INITIALIZE_RATIO: Perbill = Perbill::from_percent(10);
/// We allow `Normal` extrinsics to fill up the block up to 75%, the rest can be used
/// by  Operational  extrinsics.
pub const NORMAL_DISPATCH_RATIO: Perbill = Perbill::from_percent(75);
/// We allow for 2 seconds of compute with a 6 second average block time.
pub const MAXIMUM_BLOCK_WEIGHT: Weight = 2 * WEIGHT_PER_SECOND;

pub type TimeStampedPrice = pkfp_oracle::TimestampedValue<Price, Moment>;

// Common constants used in all runtimes.
parameter_types! {
	pub const BlockHashCount: BlockNumber = 250;

	/// Maximum length of block. Up to 5MB.
	pub BlockLength: limits::BlockLength =
		limits::BlockLength::max_with_normal_ratio(5 * 1024 * 1024, NORMAL_DISPATCH_RATIO);
	/// Block weights base values and limits.
	pub BlockWeights: limits::BlockWeights = limits::BlockWeights::builder()
		.base_block(BlockExecutionWeight::get())
		.for_class(DispatchClass::all(), |weights| {
			weights.base_extrinsic = ExtrinsicBaseWeight::get();
		})
		.for_class(DispatchClass::Normal, |weights| {
			weights.max_total = Some(NORMAL_DISPATCH_RATIO * MAXIMUM_BLOCK_WEIGHT);
		})
		.for_class(DispatchClass::Operational, |weights| {
			weights.max_total = Some(MAXIMUM_BLOCK_WEIGHT);
			// Operational transactions have an extra reserved space, so that they
			// are included even if block reached `MAXIMUM_BLOCK_WEIGHT`.
			weights.reserved = Some(
				MAXIMUM_BLOCK_WEIGHT - NORMAL_DISPATCH_RATIO * MAXIMUM_BLOCK_WEIGHT,
			);
		})
		.avg_block_initialization(AVERAGE_ON_INITIALIZE_RATIO)
		.build_or_panic();
}

pub type NegativeImbalance<T> = <pallet_balances::Pallet<T> as Currency<<T as frame_system::Config>::AccountId>>::NegativeImbalance;

/// Logic for the author to get a portion of fees.
pub struct ToAuthor<R>(sp_std::marker::PhantomData<R>);
impl<R> OnUnbalanced<NegativeImbalance<R>> for ToAuthor<R>
	where
		R: pallet_balances::Config + pallet_authorship::Config,
		<R as frame_system::Config>::AccountId: From<polkadot_primitives::v1::AccountId>,
		<R as frame_system::Config>::AccountId: Into<polkadot_primitives::v1::AccountId>,
		<R as frame_system::Config>::Event: From<pallet_balances::Event<R>>,
{
	fn on_nonzero_unbalanced(amount: NegativeImbalance<R>) {
		let numeric_amount = amount.peek();
		let author = <pallet_authorship::Pallet<R>>::author();
		<pallet_balances::Pallet<R>>::resolve_creating(&<pallet_authorship::Pallet<R>>::author(), amount);
		<frame_system::Pallet<R>>::deposit_event(pallet_balances::Event::Deposit(author, numeric_amount));
	}
}

pub struct DealWithFees<R>(sp_std::marker::PhantomData<R>);
impl<R> OnUnbalanced<NegativeImbalance<R>> for DealWithFees<R>
	where
		R: pallet_balances::Config + pallet_authorship::Config + pallet_treasury::Config,
		<R as frame_system::Config>::AccountId: From<polkadot_primitives::v1::AccountId>,
		<R as frame_system::Config>::AccountId: Into<polkadot_primitives::v1::AccountId>,
		<R as frame_system::Config>::Event: From<pallet_balances::Event<R>>,
		pallet_treasury::Pallet<R>: OnUnbalanced<NegativeImbalance<R>>,
{
	fn on_unbalanceds<B>(mut fees_then_tips: impl Iterator<Item = NegativeImbalance<R>>) {
		if let Some(fees) = fees_then_tips.next() {
			// for fees, 90% are burned, 5% to the treasury, 5% to block author
			let (_, to_treasury_and_staking) = fees.ration(90, 10);
			let (treasury_part, staking_part) = to_treasury_and_staking.ration(50, 50);

			<ToAuthor<R> as OnUnbalanced<_>>::on_unbalanced(staking_part);
			<pallet_treasury::Pallet<R> as OnUnbalanced<_>>::on_unbalanced(treasury_part);
		}
	}
}

