#![cfg_attr(not(feature = "std"), no_std)]
pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type Event: From<Event<Self, I>> + IsType<<Self as frame_system::Config>::Event>;

		type RewardCurrency: Currency<Self::AccountId>;

		type RelayChainAccountId:
		Parameter
		+ Member
		+ MaybeSerializeDeserialize
		+ Debug
		+ MaybeDisplay
		+ Ord
		+ Default
		+ Into<AccountId32>;

		type RewardPeriod: Get<Self::BlockNumber>;
	}


	type BalanceOf<T> = <<T as Config>::RewardCurrency as Currency<
		<T as frame_system::Config>::AccountId,
	>>::Balance;

	#[derive(Default, Clone, Encode, Decode, RuntimeDebug)]
	pub struct RewardInfo<T: Config> {
		pub total_reward: BalanceOf<T>,
		pub claimed_reward: BalanceOf<T>,
		pub last_paid: T::BlockNumber,
	}

}

