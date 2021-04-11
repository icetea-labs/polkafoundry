#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[cfg(test)]
pub(crate) mod mock;
#[cfg(test)]
mod tests;

#[frame_support::pallet]
pub mod pallet {
	use frame_support::{dispatch::fmt::Debug, pallet_prelude::*, traits::Currency};
	use frame_system::pallet_prelude::*;
	use sp_runtime::traits::{Saturating, Verify};
	use sp_runtime::{MultiSignature, SaturatedConversion};
	use sp_core::crypto::AccountId32;
	use sp_std::convert::{From, TryInto};

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		type RewardCurrency: Currency<Self::AccountId>;

		type RelayChainAccountId:
		Parameter
		+ Member
		+ MaybeSerializeDeserialize
		+ Debug
		+ Ord
		+ Default
		+ Into<AccountId32>;
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

	#[pallet::pallet]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(0)]
		pub fn initialize_reward(
			origin: OriginFor<T>,
			contributions: Vec<(T::RelayChainAccountId, u32)>,
			rate: u32,
			end_block: T::BlockNumber,
		) -> DispatchResultWithPostInfo {
			ensure_root(origin)?;
			let now = frame_system::Pallet::<T>::block_number();
			ensure!(
				&now < &end_block,
				Error::<T>::InvalidEndBlock
			);
			let current_reward_end_in = CurrentRewardEndIn::<T>::get();

			ensure!(
				&now >= &current_reward_end_in,
				Error::<T>::AlreadyInitReward
			);
			for (account, amount) in &contributions {
				let reward_info = RewardInfo {
					total_reward: BalanceOf::<T>::from(*amount)
						.saturating_mul(BalanceOf::<T>::from(rate)),
					claimed_reward: 0u32.into(),
					last_paid: now.clone(),
				};
				Contributors::<T>::insert(account, reward_info);
			};
			CurrentRewardEndIn::<T>::put(&end_block);
			RewardPeriod::<T>::put(end_block - now);
			Ok(Default::default())
		}

		#[pallet::weight(0)]
		pub fn associate_account(
			origin: OriginFor<T>,
			relay_account: T::RelayChainAccountId,
			proof: MultiSignature,
		) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;
			let payload = who.clone().encode();

			ensure!(
				proof.verify(payload.as_slice(), &relay_account.clone().into()),
				Error::<T>::InvalidSignature
			);

			ensure!(
				AssociatedAccount::<T>::get(&who).is_none(),
				Error::<T>::AlreadyAssociated
			);

			AssociatedAccount::<T>::insert(&who, &relay_account);

			Self::deposit_event(Event::AssociatedAccount(
				who,
				relay_account,
			));
			Ok(Default::default())
		}

		#[pallet::weight(0)]
		pub fn get_money(
			origin: OriginFor<T>,
		) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;
			let now = frame_system::Pallet::<T>::block_number();

			let relay_account =
				AssociatedAccount::<T>::get(&who).ok_or(Error::<T>::NoAssociatedClaim)?;

			let mut info =
				Contributors::<T>::get(&relay_account).ok_or(Error::<T>::NotContributedYet)?;

			ensure!(
				&info.total_reward > &info.claimed_reward,
				Error::<T>::AlreadyPaid
			);

			let reward_period = RewardPeriod::<T>::get()
				.saturated_into::<u128>()
				.try_into()
				.ok()
				.ok_or(Error::<T>::WrongConversionU128ToBalance)?;

			let reward_per_block = info.total_reward / reward_period;
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
			Contributors::<T>::insert(&relay_account, info);

			// TODO We need the charity pallet to pay for the rewards
			T::RewardCurrency::deposit_creating(&who, amount);

			Self::deposit_event(Event::RewardPaid(
				who,
				amount,
			));
			Ok(Default::default())
		}
	}

	#[pallet::storage]
	#[pallet::getter(fn contributors)]
	pub type Contributors<T: Config> =
	StorageMap<_, Blake2_128Concat, T::RelayChainAccountId, RewardInfo<T>>;

	#[pallet::storage]
	#[pallet::getter(fn associated_account)]
	pub type AssociatedAccount<T: Config> =
	StorageMap<_, Blake2_128Concat, T::AccountId, T::RelayChainAccountId>;

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
		/// Current block great than end block
		InvalidEndBlock,
		/// Already init a reward
		AlreadyInitReward,
		/// User provide wrong signature
		InvalidSignature,
		/// User already associated relay account with native account
		AlreadyAssociated,
		/// Already paid all reward
		AlreadyPaid,
		/// User not associated relay account with native account yet
		NoAssociatedClaim,
		/// User not contribute for the crowdloan
		NotContributedYet,
		/// Invalid conversion while calculating payable amount
		WrongConversionU128ToBalance,
	}

	#[pallet::event]
	#[pallet::generate_deposit(fn deposit_event)]
	pub enum Event<T: Config> {
		AssociatedAccount(T::AccountId, T::RelayChainAccountId),

		RewardPaid(T::AccountId, BalanceOf<T>),
	}
}

