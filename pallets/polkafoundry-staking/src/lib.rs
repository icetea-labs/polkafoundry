#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;
use frame_support::pallet;
#[cfg(test)]
pub(crate) mod mock;
#[cfg(test)]
mod tests;

#[pallet]
pub mod pallet {
	use frame_support::{pallet_prelude::*, traits::{Currency, LockIdentifier, ReservableCurrency}};
	use frame_system::pallet_prelude::*;
	use sp_runtime::traits::{Saturating, Verify};
	use sp_runtime::{MultiSignature, SaturatedConversion};
	use sp_core::crypto::AccountId32;
	use sp_std::{convert::{From, TryInto}, vec::Vec};
	use frame_support::sp_runtime::traits::{Bounded, AtLeast32BitUnsigned};
	use std::fmt::Debug;

	/// Counter for the number of round that have passed
	pub type RoundIndex = u32;
	/// Counter for the number of "reward" points earned by a given collator
	pub type RewardPoint = u32;

	type BalanceOf<T> = <<T as Config>::Currency as Currency<
		<T as frame_system::Config>::AccountId,
	>>::Balance;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// Overarching event type
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		/// The staking balance
		type Currency: Currency<Self::AccountId> + ReservableCurrency<Self::AccountId>;
		/// Number of block per round
		type BlocksPerRound: Get<u32>;
		/// Number of collators that nominators can be nominated for
		type MaxCollatorsPerNominator: Get<u32>;
		/// Number of round that staked funds must remain bonded for
		type BondDuration: Get<RoundIndex>;
		/// Minimum stake required to be reserved to be a collator
		type MinCollatorStake: Get<BalanceOf<Self>>;
		/// Minimum stake required to be reserved to be a nominator
		type MinNominatorStake: Get<BalanceOf<Self>>;
		/// Number of round per payout
		type VestingAfter: Get<RoundIndex>;
	}

	#[pallet::pallet]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_finalize(now: T::BlockNumber) {
			let mut current_round = CurrentRound::<T>::get();
			let round_index = current_round.index;

			if current_round.should_goto_next_round(now) {
				current_round = current_round.update(now, T::BlocksPerRound::get());
				CurrentRound::<T>::put(current_round);
				Self::update_ledger(round_index);
				Self::update_candidate_pool(round_index);
			}
		}
	}

	/// Just a Balance/BlockNumber tuple to encode when a chunk of funds will be unlocked.
	#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
	pub struct UnlockChunk<Balance> {
		/// Amount of funds to be unlocked.
		pub value: Balance,
		/// Round number at which point it'll be unlocked.
		pub round: RoundIndex,
	}
	/// Just a Balance/BlockNumber tuple to encode when a chunk of funds will be unbonded.
	#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
	pub struct UnBondChunk<Balance> {
		/// Amount of funds to be unbonded.
		pub value: Balance,
		/// Round number at which point it'll be unbonded.
		pub round: RoundIndex,
	}

	#[derive(Default, Clone, Encode, Decode, RuntimeDebug)]
	pub struct Bond<AccountId, Balance> {
		pub owner: AccountId,
		pub amount: Balance,
	}

	/// The ledger of a (bonded) stash.
	#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
	pub struct StakingLedger<Balance> {
		/// The total amount of the account's balance that we are currently accounting for.
		/// It's just `active` plus all the `unlocking` balances then minus all the unbonding balances.
		pub total: Balance,
		/// The total amount of the stash's balance that will be at stake in any forthcoming
		/// rounds.
		pub active: Balance,
		/// Any balance that is becoming free, which may eventually be transferred out
		/// of the stash (assuming it doesn't get slashed first).
		pub unlocking: Vec<UnlockChunk<Balance>>,
		/// Any balance that is becoming free, which may eventually be transferred out
		/// of the stash (assuming it doesn't get slashed first).
		pub unbonding: Vec<UnBondChunk<Balance>>,
		/// List of eras for which the stakers behind a validator have claimed rewards. Only updated
		/// for validators.
		pub claimed_rewards: Vec<RoundIndex>,
	}

	impl <Balance> StakingLedger<Balance>
	where Balance: Copy + Debug + Saturating + AtLeast32BitUnsigned
	{
		pub fn new (total: Balance, active: Balance) -> Self {
			StakingLedger {
				total,
				active,
				unlocking: vec![],
				unbonding: vec![],
				claimed_rewards: vec![]
			}
		}

		pub fn bond_extra (self, extra: Balance, next_round: RoundIndex) -> Self {
			let mut total = self.total;
			let mut unlocking = self.unlocking;
			total = total.saturating_add(extra);
			unlocking.push(UnlockChunk {
				value: extra,
				round: next_round
			});
			Self {
				total,
				active: self.active,
				unlocking,
				unbonding: self.unbonding,
				claimed_rewards: self.claimed_rewards
			}
		}

		pub fn bond_less (mut self, less: Balance, can_withdraw_round: RoundIndex) -> Option<Self> {
			let mut unbonding = self.unbonding;
			let mut active = self.active;
			if active > less {
				active = active.saturating_sub(less);
				unbonding.push(UnBondChunk {
					value: less,
					round: can_withdraw_round
				});

				Some(Self {
					total: self.total,
					active,
					unlocking: self.unlocking,
					unbonding,
					claimed_rewards: self.claimed_rewards
				})
			} else {
				None
			}
		}

		fn consolidate_active(self, current_round: RoundIndex) -> Self {
			let mut active = self.active;
			let unlocking = self.unlocking.into_iter()
				.filter(|chunk| if chunk.round > current_round {
					true
				} else {
					active = active.saturating_add(chunk.value);
					false
				})
				.collect();

			Self {
				total: self.total,
				active,
				unlocking,
				unbonding: self.unbonding,
				claimed_rewards: self.claimed_rewards
			}
		}

	}

	/// Chilling, onboarding, leaving candidates
	#[derive(Default, Clone, Encode, Decode, RuntimeDebug)]
	pub struct CandidatePool<Balance> {
		pub active_at: RoundIndex,
		pub bond: Balance,
		pub status: CandidateStatus
	}

	impl <Balance> CandidatePool<Balance>
	where Balance: PartialOrd
	+ Copy
	+ std::ops::AddAssign
	+ std::ops::Sub<Output = Balance>
	+ sp_std::ops::SubAssign
	{
		pub fn new (next_round: RoundIndex, amount: Balance) -> Self {
			CandidatePool {
				active_at: next_round,
				bond: amount,
				status: CandidateStatus::default()
			}
		}

		pub fn bond_extra(&mut self, extra: Balance) {
			self.bond += extra;
		}

		pub fn bond_less(&mut self, less: Balance) -> Option<Balance> {
			if self.bond > less {
				self.bond -= less;
				Some(self.bond)
			} else {
				None
			}
		}
	}

	#[derive(Clone, Encode, Decode, RuntimeDebug)]
	pub enum CandidateStatus {
		/// Committed to be online and producing valid blocks (not equivocating)
		Active,
		/// Onboarding to candidates pool in next round
		Onboarding,
		/// Chilling.
		Idle,
		/// Leaving in round
		Leaving(RoundIndex)
	}

	impl Default for CandidateStatus {
		fn default() -> Self {
			CandidateStatus::Onboarding
		}
	}

	#[derive(Default, Clone, Encode, Decode, RuntimeDebug)]
	pub struct Nominators<AccountId, Balance> {
		pub nominations: Vec<Bond<AccountId, Balance>>,
		pub bond: Balance
	}

	/// A nominator pool, a set of onboarding, leaving nominators
	#[derive(Default, Clone, Encode, Decode, RuntimeDebug)]
	pub struct NominatorPool<AccountId, Balance> {
		pub active_at: RoundIndex,
		pub nominations: Vec<Bond<AccountId, Balance>>,
		pub status: NominatorStatus
	}

	#[derive(Clone, Encode, Decode, RuntimeDebug)]
	pub enum NominatorStatus {
		/// Committed to be online and nominate to candidates
		Active,
		/// Onboarding to nominate in the next round
		Onboarding,
		/// Leaving in round
		Leaving(RoundIndex),
	}

	impl Default for NominatorStatus {
		fn default() -> Self {
			NominatorStatus::Onboarding
		}
	}

	impl<A, B> NominatorPool<A, B> {
		pub fn new(next_round: RoundIndex, nominations: Vec<Bond<A, B>>) -> Self {
			NominatorPool {
				active_at: next_round,
				nominations,
				status: NominatorStatus::default()
			}
		}

		// pub fn bond_extra(&mut self, candidate: A, amount: B) -> Self::nominations {
		// 	for nominate in &mut self.nominations {
		// 		if nominate.owner == &candidate {
		// 			nominate.amount.saturated_into::<u128>().saturating_add(*amount)
		// 		}
		// 	}
		// 	&self.nominations
		// }
		//
		// pub fn bond_less(&mut self, candidate: A, amount: B) -> Self::nominations {
		// 	for nominate in &mut self.nominations {
		// 		if nominate.owner == &candidate {
		// 			nominate.amount.saturated_into::<u128>().saturating_sub(*amount)
		// 		}
		// 	}
		// 	&self.nominations
		// }
	}


	#[derive(Default, Copy, Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug)]
	pub struct RoundInfo<BlockNumber> {
		/// Index of current round
		index: RoundIndex,
		/// Block where round to be started
		start_in: BlockNumber,
		/// Length of current round
		length: u32
	}

	impl<BlockNumber> RoundInfo<BlockNumber>
	where BlockNumber: PartialOrd
		+ Copy
		+ Debug
		+ From<u32>
		+ std::ops::Add<Output = BlockNumber>
	 	+ std::ops::Sub<Output = BlockNumber>
	{
		pub fn new(index: u32, start_in: BlockNumber, length: u32) -> Self {
			RoundInfo {
				index,
				start_in,
				length
			}
		}
		pub fn next_round_index(&self) -> u32 {
			&self.index + 1u32
		}

		/// New round
		pub fn update(&mut self, now: BlockNumber, length: u32) -> Self {
			let mut index = self.index;
			let mut start_in = self.start_in;
			index += 1u32;
			start_in = now;

			Self {
				index,
				start_in,
				length
			}
		}

		pub fn should_goto_next_round (&self, now: BlockNumber) -> bool {
			now - self.start_in >= self.length.into()
		}
	}

	// A value placed in storage that represents the current version of the Staking storage. This value
	// is used by the `on_runtime_upgrade` logic to determine whether we run storage migration logic.
	// This should match directly with the semantic versions of the Rust crate.
	#[derive(Encode, Decode, Clone, Copy, PartialEq, Eq, RuntimeDebug)]
	pub enum Releases {
		V1_0_0,
	}

	impl Default for Releases {
		fn default() -> Self {
			Releases::V1_0_0
		}
	}

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		pub stakers: Vec<(T::AccountId)>,
	}

	#[cfg(feature = "std")]
	impl <T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			Self {
				stakers: vec![],
			}
		}
	}


	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
		fn build(&self) {
			// Start Round 1 at Block 0
			let round: RoundInfo<T::BlockNumber> =
				RoundInfo::new(1u32, 0u32.into(), T::BlocksPerRound::get());
			CurrentRound::<T>::put(round);
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(0)]
		pub fn bond(
			origin: OriginFor<T>,
			amount: BalanceOf<T>
		) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;

			ensure!(
				Ledger::<T>::get(&who).is_none(),
				Error::<T>::AlreadyBond
			);
			ensure!(
				CandidateQueue::<T>::get(&who).is_none(),
				Error::<T>::AlreadyInQueue
			);

			if amount < T::MinCollatorStake::get() {
				Err(Error::<T>::BondBelowMin)?
			}

			let current_round = CurrentRound::<T>::get();
			let candidate = CandidatePool::new( current_round.next_round_index(), amount);

			CandidateQueue::<T>::insert(&who, candidate);

			T::Currency::reserve(
				&who,
				amount,
			);
			Self::deposit_event(Event::BondInQueue(
				who,
				amount,
			));
			Ok(Default::default())
		}

		#[pallet::weight(0)]
		pub fn bond_extra(
			origin: OriginFor<T>,
			extra: BalanceOf<T>
		) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;
			let ledger = Ledger::<T>::get(&who);
			let candidate_in_queue = CandidateQueue::<T>::get(&who);

			if ledger.is_none() && candidate_in_queue.is_none() {
				Err(Error::<T>::BondNotExist)?
			}

			if candidate_in_queue.is_some() {
				let mut candidate = candidate_in_queue.unwrap();
				candidate.bond_extra(extra);
				CandidateQueue::<T>::insert(&who, candidate);
			} else {
				let current_round = CurrentRound::<T>::get();
				let mut unwrap_ledger = ledger.unwrap();

				unwrap_ledger = unwrap_ledger.bond_extra(extra, current_round.next_round_index());
				Ledger::<T>::insert(&who, unwrap_ledger);
			}

			T::Currency::reserve(
				&who,
				extra,
			);

			Self::deposit_event(Event::BondExtra(
				who,
				extra,
			));

			Ok(Default::default())
		}

		#[pallet::weight(0)]
		pub fn bond_less(
			origin: OriginFor<T>,
			less: BalanceOf<T>
		) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;
			let ledger = Ledger::<T>::get(&who);
			let candidate_in_queue = CandidateQueue::<T>::get(&who);

			if ledger.is_none() && candidate_in_queue.is_none() {
				Err(Error::<T>::BondNotExist)?
			}

			if candidate_in_queue.is_some() {
				let mut candidate = candidate_in_queue.unwrap();
				let after = candidate.bond_less(less).ok_or(Error::<T>::Underflow)?;

				ensure!(
					after >= T::MinCollatorStake::get(),
					Error::<T>::BondBelowMin
				);

				CandidateQueue::<T>::insert(&who, candidate);

				T::Currency::unreserve(
					&who,
					less,
				);
			} else {
				let _current_round = CurrentRound::<T>::get();
				let unwrap_ledger = ledger.unwrap();
				let after_ledger = unwrap_ledger.bond_less(less, 100u32).ok_or(Error::<T>::Underflow)?;

				ensure!(
					after_ledger.active >= T::MinCollatorStake::get(),
					Error::<T>::BondBelowMin
				);
				Ledger::<T>::insert(&who, after_ledger);
			}

			Self::deposit_event(Event::BondLess(
				who,
				less,
			));

			Ok(Default::default())
		}
	}

	impl <T: Config> Pallet<T> {
		fn update_ledger(current_round: RoundIndex) {
			for (acc, mut staker) in  Ledger::<T>::iter() {
				staker = staker.consolidate_active(current_round.clone());
				Ledger::<T>::insert(acc, staker)
			}
		}

		fn update_candidate_pool(current_round: RoundIndex) {
			for (acc, candidate) in CandidateQueue::<T>::iter() {
				if candidate.active_at >= current_round {
					Ledger::<T>::insert(&acc, StakingLedger::new(
						candidate.bond,
						candidate.bond,
					));
					CandidateQueue::<T>::remove(&acc)
				}
			}
		}
	}

	#[pallet::storage]
	#[pallet::getter(fn current_round)]
	pub type CurrentRound<T: Config> =
	StorageValue<_, RoundInfo<T::BlockNumber>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn ledger)]
	pub type Ledger<T: Config> =
	StorageMap<_, Twox64Concat, T::AccountId, StakingLedger<BalanceOf<T>>>;

	#[pallet::storage]
	#[pallet::getter(fn candidate_in_queue)]
	pub type CandidateQueue<T: Config> =
	StorageMap<_, Twox64Concat, T::AccountId, CandidatePool<BalanceOf<T>>>;

	#[pallet::storage]
	#[pallet::getter(fn storage_version)]
	pub type StorageVersion<T: Config> =
	StorageValue<_, Releases, ValueQuery>;


	#[pallet::error]
	pub enum Error<T> {
		/// Candidate already bonded
		AlreadyBond,
		/// Candidate already in queue
		AlreadyInQueue,
		/// Bond not exist
		BondNotExist,
		/// Value under flow
		Underflow,
		/// Bond less than minimum value
		BondBelowMin,
	}

	#[pallet::event]
	#[pallet::generate_deposit(fn deposit_event)]
	pub enum Event<T: Config> {
		BondInQueue(T::AccountId, BalanceOf<T>),
		BondExtra(T::AccountId, BalanceOf<T>),
		BondLess(T::AccountId, BalanceOf<T>),
	}
}
