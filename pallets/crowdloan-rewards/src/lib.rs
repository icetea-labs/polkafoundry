#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;
use frame_support::pallet;

#[cfg(test)]
pub(crate) mod mock;
#[cfg(test)]
mod tests;

#[pallet]
pub mod pallet {
	use frame_support::{
		dispatch::fmt::Debug,
		pallet_prelude::*,
		traits::{Currency, ExistenceRequirement::AllowDeath, IsType},
		PalletId,
	};
	use frame_system::pallet_prelude::*;
	use sp_core::crypto::AccountId32;
	use sp_runtime::traits::{AccountIdConversion, Saturating, Zero};
	use sp_runtime::{SaturatedConversion};
	use sp_std::{
		convert::{From, TryInto},
		vec::Vec,
	};

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		/// The crowdloan's module id, used for deriving its sovereign account ID.
		type PalletId: Get<PalletId>;

		/// The reward balance.
		type RewardCurrency: Currency<Self::AccountId>;

		/// Percentage rates of token at Token generating event rate (TGE)
		const TGE_RATE: u32;

		type RelayChainAccountId:
		Parameter
		+ Member
		+ MaybeSerializeDeserialize
		+ Debug
		+ Ord
		+ Default
		+ Into<AccountId32>;
	}

	pub type BalanceOf<T> = <<T as Config>::RewardCurrency as Currency<
		<T as frame_system::Config>::AccountId,
	>>::Balance;

	#[derive(Default, Clone, Encode, Decode, RuntimeDebug)]
	pub struct RewardInfo<T: Config> {
		pub total_reward: BalanceOf<T>, // Total of rewarded token based on conversion rate
		pub init_locked: BalanceOf<T>, // The initialize locked token = total_reward - Distributed Token at TGE
		pub claimed_reward: BalanceOf<T>,
		pub last_paid: T::BlockNumber,
	}

	#[pallet::pallet]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(0)]
		pub fn initialize_reward(
			origin: OriginFor<T>,
			contributions: Vec<(T::AccountId, BalanceOf<T>)>,
			end_block: T::BlockNumber,
		) -> DispatchResultWithPostInfo {
			ensure_root(origin)?;

			let now = frame_system::Pallet::<T>::block_number();
			ensure!(&now < &end_block, Error::<T>::InvalidEndBlock);

			let current_reward_end_in = CurrentRewardEndIn::<T>::get();
			ensure!(
				&now >= &current_reward_end_in,
				Error::<T>::AlreadyInitReward
			);

			for (who, amount) in &contributions {
				let total_reward = BalanceOf::<T>::from(*amount);
				let claimed_reward = total_reward.saturating_mul(BalanceOf::<T>::from(T::TGE_RATE)) / BalanceOf::<T>::from(100u32);
				let init_locked = total_reward.saturating_sub(claimed_reward);

				// A part of token are distributed immediately at TGE.
				T::RewardCurrency::transfer(&Self::account_id(), &who, claimed_reward, AllowDeath)
					.map_err(|_| Error::<T>::RewardFailed)?;
				Self::deposit_event(Event::RewardPaid(who.clone(), claimed_reward));

				// The remaining are distributed linearly until end block
				Contributors::<T>::insert(
					who,
					RewardInfo {
						total_reward,
						init_locked,
						claimed_reward,
						last_paid: now.clone(),
					},
				);
			}
			CurrentRewardEndIn::<T>::put(&end_block);
			RewardPeriod::<T>::put(end_block - now);
			Ok(Default::default())
		}

		#[pallet::weight(0)]
		pub fn claim(
			origin: OriginFor<T>,
		) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;
			let now = frame_system::Pallet::<T>::block_number();
			let mut info = Contributors::<T>::get(&who)
				.ok_or(Error::<T>::NotContributedYet)?;

			ensure!(
				&info.total_reward > &info.claimed_reward,
				Error::<T>::AlreadyPaid
			);

			let reward_period = RewardPeriod::<T>::get()
				.saturated_into::<u128>()
				.try_into()
				.ok()
				.ok_or(Error::<T>::WrongConversionU128ToBalance)?;

			ensure!(
				reward_period > Zero::zero(),
				Error::<T>::NotReady,
			);

			let reward_per_block = info.init_locked / reward_period;
			let reward_period = now.saturating_sub(info.last_paid);

			let reward_period_as_balance: BalanceOf<T> = reward_period
				.saturated_into::<u128>()
				.try_into()
				.ok()
				.ok_or(Error::<T>::WrongConversionU128ToBalance)?;

			let amount = if reward_per_block.saturating_mul(reward_period_as_balance)
				> info.total_reward - info.claimed_reward {
				info.total_reward - info.claimed_reward
			} else {
				reward_per_block.saturating_mul(reward_period_as_balance)
			};
			info.last_paid = now;
			info.claimed_reward = info.claimed_reward.saturating_add(amount);
			Contributors::<T>::insert(&who, info);

			ensure!(
				amount >= T::RewardCurrency::minimum_balance(),
				Error::<T>::ClaimAmountBelowMinimum
			);

			T::RewardCurrency::transfer(&Self::account_id(), &who, amount, AllowDeath)
				.map_err(|_| Error::<T>::RewardFailed)?;

			Self::deposit_event(Event::RewardPaid(who, amount));
			Ok(Default::default())
		}
	}

	#[pallet::storage]
	#[pallet::getter(fn contributors)]
	pub type Contributors<T: Config> =
	StorageMap<_, Blake2_128Concat, T::AccountId, RewardInfo<T>>;

	#[pallet::storage]
	#[pallet::getter(fn current_reward_end_in)]
	pub type CurrentRewardEndIn<T: Config> =
	StorageValue<_, T::BlockNumber, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn reward_period)]
	pub type RewardPeriod<T: Config> =
	StorageValue<_, T::BlockNumber, ValueQuery>;

	#[pallet::error]
	pub enum Error<T> {
		/// The claim is not ready
		NotReady,
		/// Current block great than end block
		InvalidEndBlock,
		/// Already init a reward
		AlreadyInitReward,
		/// Already paid all reward
		AlreadyPaid,
		/// User not contribute for the crowdloan
		NotContributedYet,
		/// Invalid conversion while calculating payable amount
		WrongConversionU128ToBalance,
		/// User cannot receive a reward
		RewardFailed,
		/// The amount of claim below the minimum balance
		ClaimAmountBelowMinimum,
	}

	#[pallet::event]
	#[pallet::generate_deposit(fn deposit_event)]
	pub enum Event<T: Config> {
		RewardPaid(T::AccountId, BalanceOf<T>),
	}

	impl<T: Config> Pallet<T> {
		/// The account ID of the pallet.
		///
		/// This actually does computation. If you need to keep using it, then make sure you cache the
		/// value and only call this once.
		pub fn account_id() -> T::AccountId {
			T::PalletId::get().into_account()
		}

		pub fn pot() -> BalanceOf<T> {
			T::RewardCurrency::free_balance(&Self::account_id())
				// Must never be less than 0 but better be safe.
				.saturating_sub(T::RewardCurrency::minimum_balance())
		}
	}
}

