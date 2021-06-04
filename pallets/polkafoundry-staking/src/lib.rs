#![cfg_attr(not(feature = "std"), no_std)]
pub use pallet::*;
use frame_support::pallet;
#[cfg(test)]
pub(crate) mod mock;
#[cfg(test)]
mod tests;

pub mod taylor_series;
pub mod inflation;

pub(crate) const LOG_TARGET: &'static str = "runtime::staking";

// syntactic sugar for logging.
#[macro_export]
macro_rules! log {
	($level:tt, $patter:expr $(, $values:expr)* $(,)?) => {
		log::$level!(
			target: crate::LOG_TARGET,
			concat!("[{:?}] 💸 ", $patter), <frame_system::Pallet<T>>::block_number() $(, $values)*
		)
	};
}

#[pallet]
pub mod pallet {
	use frame_support::{pallet_prelude::*, traits::{Currency, ReservableCurrency, CurrencyToVote, Imbalance}};
	use frame_system::pallet_prelude::*;
	use sp_runtime::traits::{Saturating, Zero};
	use sp_runtime::{Perbill};
	use sp_std::{convert::{From}, vec::Vec};
	use frame_support::sp_runtime::traits::{AtLeast32BitUnsigned, UnixTime, Convert};
	use frame_election_provider_support::{ElectionProvider, VoteWeight, Supports, data_provider};
	use crate::inflation::compute_total_payout;
	use sp_std::{cmp::Ordering, prelude::*, ops::{Mul, AddAssign, Add, Sub}};
	use frame_support::sp_std::fmt::Debug;
	use log::info;

	/// Counter for the number of round that have passed
	pub type RoundIndex = u32;
	/// Counter for the number of "reward" points earned by a given collator
	pub type RewardPoint = u32;
	pub type EraIndex = u32;
	pub type SessionIndex = u32;

	type BalanceOf<T> = <<T as Config>::Currency as Currency<
		<T as frame_system::Config>::AccountId,
	>>::Balance;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// Overarching event type
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		/// Time used for computing era duration.
		///
		/// It is guaranteed to start being called from the first `on_finalize`. Thus value at genesis
		/// is not used.
		type UnixTime: UnixTime;
		/// The staking balance
		type Currency: Currency<Self::AccountId> + ReservableCurrency<Self::AccountId>;
		/// Number of block per round
		type BlocksPerRound: Get<u32>;
		/// Number of collators that nominators can be nominated for
		const MAX_COLLATORS_PER_NOMINATOR: u32;
		/// Maximum number of nominations per collator
		type MaxNominationsPerCollator: Get<u32>;
		/// Number of round that staked funds must remain bonded for
		type BondDuration: Get<RoundIndex>;
		/// Minimum stake required to be reserved to be a collator
		type MinCollatorStake: Get<BalanceOf<Self>>;
		/// Minimum stake required to be reserved to be a nominator
		type MinNominatorStake: Get<BalanceOf<Self>>;
		/// Number of round per payout
		type PayoutDuration: Get<RoundIndex>;
		/// Something that provides the election functionality.
		type ElectionProvider: frame_election_provider_support::ElectionProvider<
			Self::AccountId,
			Self::BlockNumber,
			// we only accept an election provider that has staking as data provider.
			DataProvider = Pallet<Self>,
		>;
		/// Convert a balance into a number used for election calculation. This must fit into a `u64`
		/// but is allowed to be sensibly lossy. The `u64` is used to communicate with the
		/// [`sp_npos_elections`] crate which accepts u64 numbers and does operations in 128.
		/// Consequently, the backward convert is used convert the u128s from sp-elections back to a
		/// [`BalanceOf`].
		type CurrencyToVote: CurrencyToVote<BalanceOf<Self>>;
		/// Number of sessions per era.
		type SessionsPerEra: Get<SessionIndex>;
		/// Number of eras that staked funds must remain bonded for.
		type BondingDuration: Get<EraIndex>;
		/// Interface for interacting with a session module.
		type SessionInterface: self::SessionInterface<Self::AccountId>;

		type DesiredTarget: Get<u32>;
	}

	#[pallet::pallet]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_finalize(now: T::BlockNumber) {
			let mut current_round = CurrentRound::<T>::get();
			if current_round.should_goto_next_round(now) {
				let block_per_round = Settings::<T>::get().blocks_per_round;
				current_round.update(now, block_per_round);
				let round_index = current_round.index;
				// start a new round
				CurrentRound::<T>::put(current_round);
				// pay for stakers
				Self::payout_stakers(round_index);
				// onboard, unlock bond, unbond collators
				Self::update_collators(round_index);
				// unbond all nominators
				Self::update_nominators(round_index);
				// execute all delayed collator exits
				Self::execute_exit_queue(round_index);
				// select winner candidates in this round
				Self::enact_election(round_index);
				// update total stake of next round
				TotalStakedAt::<T>::insert(round_index, TotalStaked::<T>::get());
				TotalIssuanceAt::<T>::insert(round_index, T::Currency::total_issuance());

				Self::deposit_event(Event::NewRoundStart(round_index, round_index * block_per_round));
			}
		}
	}

	/// Means for interacting with a specialized version of the `session` trait.
///
/// This is needed because `Staking` sets the `ValidatorIdOf` of the `pallet_session::Config`
	pub trait SessionInterface<AccountId>: frame_system::Config {
		/// Disable a given validator by stash ID.
		///
		/// Returns `true` if new era should be forced at the end of this session.
		/// This allows preventing a situation where there is too many validators
		/// disabled and block production stalls.
		fn disable_validator(validator: &AccountId) -> Result<bool, ()>;
		/// Get the validators from session.
		fn validators() -> Vec<AccountId>;
		/// Prune historical session tries up to but not including the given index.
		fn prune_historical_up_to(up_to: SessionIndex);
	}

	impl<T: Config> SessionInterface<<T as frame_system::Config>::AccountId> for T where
		T: pallet_session::Config<ValidatorId = <T as frame_system::Config>::AccountId>,
		T: pallet_session::historical::Config<
			FullIdentification = Exposure<<T as frame_system::Config>::AccountId, BalanceOf<T>>,
			FullIdentificationOf = ExposureOf<T>,
		>,
		T::SessionHandler: pallet_session::SessionHandler<<T as frame_system::Config>::AccountId>,
		T::SessionManager: pallet_session::SessionManager<<T as frame_system::Config>::AccountId>,
		T::ValidatorIdOf:
		Convert<<T as frame_system::Config>::AccountId, Option<<T as frame_system::Config>::AccountId>>,
	{
		fn disable_validator(validator: &<T as frame_system::Config>::AccountId) -> Result<bool, ()> {
			<pallet_session::Pallet<T>>::disable(validator)
		}

		fn validators() -> Vec<<T as frame_system::Config>::AccountId> {
			<pallet_session::Pallet<T>>::validators()
		}

		fn prune_historical_up_to(up_to: SessionIndex) {
			<pallet_session::historical::Pallet<T>>::prune_up_to(up_to);
		}
	}

	#[derive(Default, PartialEq, Clone, Encode, Decode, RuntimeDebug)]
	pub struct SettingStruct {
		pub bond_duration: u32,
		pub blocks_per_round: u32,
		pub desired_target: u32,
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
	pub struct Bond<AccountId, Balance>  {
		pub owner: AccountId,
		pub amount: Balance,
	}

	impl<AccountId, Balance> PartialEq for Bond<AccountId, Balance>
		where AccountId: Ord
	{
		fn eq(&self, other: &Self) -> bool {
			self.owner == other.owner
		}
	}

	impl<AccountId, Balance> Eq for Bond<AccountId, Balance>
		where AccountId: Ord
	{}

	impl<AccountId, Balance> PartialOrd for Bond<AccountId, Balance>
		where AccountId: Ord
	{
		fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
			Some(self.cmp(&other))
		}
	}

	impl<AccountId, Balance> Ord for Bond<AccountId, Balance>
		where AccountId: Ord
	{
		fn cmp(&self, other: &Self) -> Ordering {
			self.owner.cmp(&other.owner)
		}
	}


	/// The ledger of a (bonded) stash.
	#[derive(Clone, Encode, Decode, RuntimeDebug)]
	pub struct StakingCollators<AccountId, Balance> {
		/// The total amount of the account's balance that we are currently accounting for.
		/// It's just `active` plus all the `unlocking` plus all the `nomination` balances then minus all the unbonding balances.
		pub total: Balance,
		/// The total amount of the stash's balance that will be at stake in any forthcoming
		/// rounds.
		pub active: Balance,
		/// The total amount of the nomination by nominator
		pub nominations: Vec<Bond<AccountId, Balance>>,
		/// Any balance that is becoming free, which may eventually be transferred out
		/// of the stash (assuming it doesn't get slashed first).
		pub unlocking: Vec<UnlockChunk<Balance>>,
		/// Any balance that is becoming free, which may eventually be transferred out
		/// of the stash (assuming it doesn't get slashed first).
		pub unbonding: Vec<UnBondChunk<Balance>>,
		/// Status of staker
		pub status: StakerStatus,
		/// List of eras for which the stakers behind a validator have claimed rewards. Only updated
		/// for validators.
		pub claimed_rewards: Vec<RoundIndex>,
	}

	impl <AccountId, Balance> StakingCollators<AccountId, Balance>
	where
		AccountId: Ord + Clone,
		Balance: Ord + Copy + Debug + Saturating + AtLeast32BitUnsigned + AddAssign + From<u32>
	{
		pub fn new (amount: Balance, next_round: RoundIndex) -> Self {
			StakingCollators {
				total: amount,
				active: 0u32.into(),
				nominations: vec![],
				unlocking: vec![UnlockChunk {
					value: amount,
					round: next_round
				}],
				unbonding: vec![],
				status: StakerStatus::default(),
				claimed_rewards: vec![]
			}
		}

		pub fn is_active(&self) -> bool { self.status == StakerStatus::Active }

		/// Active the onboarding collator
		pub fn active_onboard(&mut self) {
			if self.status == StakerStatus::Onboarding {
				self.status = StakerStatus::Active
			}
		}
		/// Bond extra for collator
		/// Active in next round
		pub fn bond_extra (&mut self, extra: Balance, next_round: RoundIndex) {
			self.total += extra;
			self.unlocking.push(UnlockChunk {
				value: extra,
				round: next_round
			});
		}
		/// Bond less for collator
		/// Unbonding amount delay of `BondDuration` round
		pub fn bond_less (&mut self, less: Balance, can_withdraw_round: RoundIndex) -> Option<Balance> {
			if self.active > less {
				self.active -= less;
				self.unbonding.push(UnBondChunk {
					value: less,
					round: can_withdraw_round
				});

				Some(self.active)
			} else {
				None
			}
		}
		/// Unlocking all the bond be locked in the previous round
		fn consolidate_active(self, current_round: RoundIndex) -> Self {
			let mut active = self.active;
			let unlocking = self.unlocking.into_iter()
				.filter(|chunk| if chunk.round > current_round {
					true
				} else {
					active += chunk.value;
					false
				})
				.collect();

			Self {
				total: self.total,
				active,
				nominations: self.nominations,
				unlocking,
				unbonding: self.unbonding,
				status: self.status,
				claimed_rewards: self.claimed_rewards
			}
		}
		/// Remove all the locked bond after `BondDuration`
		pub fn consolidate_unbonded(self, current_round: RoundIndex) -> Self {
			let mut total = self.total;
			let unbonding = self.unbonding.into_iter()
				.filter(|chunk| if chunk.round > current_round  {
					true
				} else {
					total -= chunk.value;
					false
				})
				.collect();

			Self {
				total,
				active: self.active,
				nominations: self.nominations,
				unlocking: self.unlocking,
				unbonding,
				status: self.status,
				claimed_rewards: self.claimed_rewards
			}
		}

		/// Add nomination for collator
		/// Will be count as vote weight for collator
		pub fn add_nomination(&mut self, nomination: Bond<AccountId, Balance>) -> bool {
			match self.nominations.binary_search(&nomination) {
				Ok(_) => false,
				Err(_) => {
					self.nominations.push(nomination);
					true
				}
			}
		}
		/// Nominate extra for exist nomination
		pub fn nominate_extra(&mut self, extra: Bond<AccountId, Balance>) -> Option<Balance> {
			for bond in &mut self.nominations {
				if bond.owner == extra.owner {
					bond.amount += extra.amount;
					return Some(bond.amount)
				}
			}
			None
		}
		/// Nominate less for exist nomination
		pub fn nominate_less(&mut self, less: Bond<AccountId, Balance>) -> Option<Option<Balance>> {
			for bond in &mut self.nominations {
				if bond.owner == less.owner {
					if bond.amount > less.amount {
						bond.amount -= less.amount;
						return Some(Some(bond.amount))
					} else {
						return Some(None)
					}
				}
			}
			None
		}
		/// Active the onboarding collator
		pub fn force_bond(&mut self) {
			self.active = self.total;
			self.unlocking = vec![];
			self.status = StakerStatus::Active
		}

		pub fn rm_nomination(&mut self, nominator: AccountId) -> Option<Balance> {
			let mut less: Option<Balance> = None;
			let nominations = self.nominations.clone()
				.into_iter()
				.filter_map(|n| {
					if n.owner == nominator {
						less = Some(n.amount);
						None
					} else {
						Some(n.clone())
					}
				}
				)
				.collect();
			if let Some(_) = less {
				self.nominations = nominations;
				Some(self.active)
			} else {
				None
			}
		}
	}

	#[derive(Clone, Encode, Decode, RuntimeDebug)]
	pub struct Leaving<Balance> {
		/// The `active` amount of collator before leaving.
		pub remaining: Balance,
		/// Any balance that is becoming free, which may eventually be transferred out
		/// of the stash (assuming it doesn't get slashed first).
		pub unbonding: Vec<UnBondChunk<Balance>>,
		/// Leaving in
		pub when: RoundIndex,
	}

	impl <Balance>Leaving <Balance>
		where Balance: Ord + Copy + Debug + Saturating + AtLeast32BitUnsigned + AddAssign + From<u32>
	{
		pub fn new(remaining: Balance, unbonding: Vec<UnBondChunk<Balance>>, when: RoundIndex) -> Self {
			Self {
				remaining,
				unbonding,
				when
			}
		}
	}

	#[derive(Clone, PartialEq, Copy, Encode, Decode, RuntimeDebug)]
	pub enum StakerStatus {
		/// Declared desire in validating or already participating in it.
		Validator,
		/// Nominating for a group of other stakers.
		Nominator,
		/// Ready for produce blocks/nominate.
		Active,
		/// Onboarding to candidates pool in next round
		Onboarding,
		/// Chilling.
		Idle,
		/// Leaving.
		Leaving,
	}

	impl Default for StakerStatus {
		fn default() -> Self {
			StakerStatus::Onboarding
		}
	}

	#[derive(Default, Clone, Encode, Decode, RuntimeDebug)]
	pub struct StakingNominators<AccountId, Balance> {
		pub nominations: Vec<Bond<AccountId, Balance>>,
		/// The total amount of the account's balance that we are currently accounting for.
		pub total: Balance,
		/// Any balance that is becoming free, which may eventually be transferred out
		/// of the stash (assuming it doesn't get slashed first).
		pub unbonding: Vec<UnBondChunk<Balance>>,
		/// List of eras for which the stakers behind a validator have claimed rewards. Only updated
		/// for validators.
		pub claimed_rewards: Vec<RoundIndex>,
	}

	impl <AccountId, Balance> StakingNominators<AccountId, Balance>
		where
			AccountId: Clone + PartialEq + Ord,
			Balance: Copy + Debug + Saturating + AtLeast32BitUnsigned {
		pub fn new (nominations: Vec<Bond<AccountId, Balance>>, amount: Balance) -> Self {
			StakingNominators {
				nominations,
				total: amount,
				unbonding: vec![],
				claimed_rewards: vec![]
			}
		}
		/// Add nomination
		/// Plus `total` will be count as vote weight for nominator
		pub fn add_nomination(&mut self, nomination: Bond<AccountId, Balance>) -> bool {
			match self.nominations.binary_search(&nomination) {
				Ok(_) => false,
				Err(_) => {
					self.total += nomination.amount;
					self.nominations.push(nomination);
					true
				}
			}
		}
		/// Nominate extra for exist nomination
		pub fn nominate_extra(&mut self, extra: Bond<AccountId, Balance>) -> Option<Balance> {
			for nominate in &mut self.nominations {
				if nominate.owner == extra.owner {
					self.total += extra.amount;
					nominate.amount += extra.amount;

					return Some(nominate.amount);
				}
			}
			None
		}
		/// Nominate less for exist nomination
		/// The amount unbond will be locked due to `BondDuration`
		pub fn nominate_less(&mut self, less: Bond<AccountId, Balance>, can_withdraw_round: RoundIndex) -> Option<Option<Balance>> {
			for nominate in &mut self.nominations {
				if nominate.owner == less.owner {
					if nominate.amount > less.amount {
						nominate.amount -= less.amount;
						self.unbonding.push(UnBondChunk {
							value: less.amount,
							round: can_withdraw_round
						});

						return Some(Some(nominate.amount));
					} else {
						return Some(None);
					}
				}
			}
			None
		}
		/// Remove all locked bond after `BondDuration`
		pub fn consolidate_unbonded(self, current_round: RoundIndex) -> Self {
			let mut total = self.total;
			let unbonding = self.unbonding.into_iter()
				.filter(|chunk| if chunk.round > current_round {
					true
				} else {
					total -= chunk.value;
					false
				}).collect();

			Self {
				nominations: self.nominations,
				total,
				unbonding,
				claimed_rewards: self.claimed_rewards
			}
		}

		pub fn rm_nomination(&mut self, candidate: AccountId, can_withdraw_round: RoundIndex) -> Option<Balance> {
			let mut less: Option<Balance> = None;
			let nominations = self.nominations
				.clone()
				.into_iter()
				.filter_map(|n| {
					if n.owner == candidate {
						less = Some(n.amount);
						None
					} else {
						Some(n.clone())
					}
				})
				.collect();
			if let Some(less) = less {
				self.nominations = nominations;
				self.unbonding.push(UnBondChunk {
					value: less,
					round: can_withdraw_round
				});
				Some(self.total)
			} else {
				None
			}
		}
	}

	/// Information regarding the active era (era in used in session).
	#[derive(Encode, Decode, RuntimeDebug)]
	pub struct ActiveEraInfo {
		/// Index of era.
		pub index: EraIndex,
		/// Moment of start expressed as millisecond from `$UNIX_EPOCH`.
		///
		/// Start can be none if start hasn't been set for the era yet,
		/// Start is set on the first on_finalize of the era to guarantee usage of `Time`.
		start: Option<u64>,
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
		+ Add<Output = BlockNumber>
	 	+ Sub<Output = BlockNumber>
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
		pub fn update(&mut self, now: BlockNumber, length: u32) {
			self.index += 1u32;
			self.start_in = now;
			self.length = length;
		}

		pub fn should_goto_next_round (&self, now: BlockNumber) -> bool {
			now - self.start_in >= self.length.into()
		}

		pub fn next_election_prediction (&self) -> BlockNumber {
			self.start_in + self.length.into()
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

	/// The amount of exposure (to slashing) than an individual nominator has.
	#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Encode, Decode, RuntimeDebug)]
	pub struct IndividualExposure<AccountId, Balance> {
		/// The stash account of the nominator in question.
		pub who: AccountId,
		/// Amount of funds exposed.
		pub value: Balance,
	}

	/// A snapshot of the stake backing a single validator in the system.
	#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Encode, Decode, Default, RuntimeDebug)]
	pub struct Exposure<AccountId, Balance> {
		/// The total balance backing this validator.
		pub total: Balance,
		/// The validator's own stash that is exposed.
		pub own: Balance,
		/// The portions of nominators stashes that are exposed.
		pub others: Vec<IndividualExposure<AccountId, Balance>>,
	}


	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		pub stakers: Vec<(T::AccountId, BalanceOf<T>)>,
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
			let mut total_staked: BalanceOf<T> = Zero::zero();
			for &(ref staker, balance) in &self.stakers {
				assert!(
					T::Currency::free_balance(&staker) >= balance,
					"Account does not have enough balance to bond."
				);

				total_staked += balance.clone();
				Pallet::<T>::bond(
					T::Origin::from(Some(staker.clone()).into()),
					balance.clone(),
				);
			}
			TotalStaked::<T>::put(total_staked);

			// Start Round 1 at Block 0
			let round: RoundInfo<T::BlockNumber> =
				RoundInfo::new(1u32, 0u32.into(), T::BlocksPerRound::get());
			CurrentRound::<T>::put(round);
			TotalStakedAt::<T>::insert(1u32, TotalStaked::<T>::get());
			TotalIssuanceAt::<T>::insert(1u32, T::Currency::total_issuance());
			Settings::<T>::put(SettingStruct {
				bond_duration: T::BondDuration::get(),
				blocks_per_round: T::BlocksPerRound::get(),
				desired_target: T::DesiredTarget::get()
			});
			<Pallet<T>>::deposit_event(Event::NewRoundStart(1u32, 1u32 + T::BlocksPerRound::get() as u32));
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(0)]
		pub fn config(
			origin: OriginFor<T>,
			settings: SettingStruct
		) -> DispatchResultWithPostInfo {
			ensure_root(origin)?;
			Settings::<T>::put(settings.clone());

			Self::deposit_event(Event::SettingChanged(
				settings,
			));
			Ok(Default::default())
		}

		#[pallet::weight(0)]
		pub fn bond(
			origin: OriginFor<T>,
			amount: BalanceOf<T>
		) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;

			ensure!(
				Collators::<T>::get(&who).is_none(),
				Error::<T>::AlreadyBonded
			);

			if amount < T::MinCollatorStake::get() {
				Err(Error::<T>::BondBelowMin)?
			}

			let current_round = CurrentRound::<T>::get();
			let staker = StakingCollators::new(amount, current_round.next_round_index());

			Collators::<T>::insert(&who, staker);
			let current_staked = TotalStaked::<T>::get();
			TotalStaked::<T>::put(current_staked + amount);

			T::Currency::reserve(
				&who,
				amount,
			);
			Self::deposit_event(Event::Bonded(
				who,
				amount,
			));
			Ok(Default::default())
		}

		#[pallet::weight(0)]
		pub fn force_onboard(
			origin: OriginFor<T>,
			candidate: T::AccountId
		) -> DispatchResultWithPostInfo {
			ensure_root(origin)?;

			let mut collator = Collators::<T>::get(&candidate).ok_or(Error::<T>::CandidateNotExist)?;

			ensure!(
				!collator.is_active(),
				Error::<T>::CandidateNotActive
			);
			collator.force_bond();
			Collators::<T>::insert(&candidate, collator);

			Self::deposit_event(Event::CandidateOnboard(
				candidate,
			));
			Ok(Default::default())
		}

		#[pallet::weight(0)]
		pub fn bond_extra(
			origin: OriginFor<T>,
			extra: BalanceOf<T>
		) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;
			let mut collator = Collators::<T>::get(&who).ok_or(Error::<T>::BondNotExist)?;
			ensure!(
				collator.is_active(),
				Error::<T>::CandidateNotActive
			);
			let current_round = CurrentRound::<T>::get();

			collator.bond_extra(extra, current_round.next_round_index());
			Collators::<T>::insert(&who, collator);
			let current_staked = TotalStaked::<T>::get();
			TotalStaked::<T>::put(current_staked + extra);

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
			let mut collator = Collators::<T>::get(&who).ok_or(Error::<T>::BondNotExist)?;
			ensure!(
				collator.is_active(),
				Error::<T>::CandidateNotActive
			);
			let current_round = CurrentRound::<T>::get();
			let after = collator.bond_less(less, current_round.index + Settings::<T>::get().bond_duration).ok_or(Error::<T>::Underflow)?;

			ensure!(
					after >= T::MinCollatorStake::get(),
					Error::<T>::BondBelowMin
			);

			Collators::<T>::insert(&who, collator);

			Self::deposit_event(Event::BondLess(
				who,
				less,
			));

			Ok(Default::default())
		}

		#[pallet::weight(0)]
		pub fn collator_unbond(
			origin: OriginFor<T>,
		) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;
			let collator = Collators::<T>::get(&who).ok_or(Error::<T>::BondNotExist)?;

			let current_round = CurrentRound::<T>::get();
			let when = current_round.index + Settings::<T>::get().bond_duration;

			// leave all nominations
			for nomination in collator.nominations {
				T::Currency::unreserve(&nomination.owner, nomination.amount);
			}

			let exit = Leaving::new(collator.active, collator.unbonding, when);

			ExitQueue::<T>::insert(&who, exit);
			Collators::<T>::remove(&who);

			Self::deposit_event(Event::CandidateLeaving(
				who,
				when,
			));
			Ok(Default::default())
		}

		#[pallet::weight(0)]
		pub fn nominate(
			origin: OriginFor<T>,
			candidate: T::AccountId,
			amount: BalanceOf<T>
		) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;
			ensure!(
				amount >= T::MinNominatorStake::get(),
				Error::<T>::NominateBelowMin
			);
			let mut collator = Collators::<T>::get(&candidate).ok_or(Error::<T>::CandidateNotExist)?;
			ensure!(
				collator.is_active(),
				Error::<T>::CandidateNotActive
			);

			ensure!(
					collator.nominations.len() < T::MaxNominationsPerCollator::get() as usize,
					Error::<T>::TooManyNominations
			);

			if let Some(mut nominator) = Nominators::<T>::get(&who) {
				ensure!(
					nominator.add_nomination(Bond {
						owner: candidate.clone(),
						amount,
					}),
					Error::<T>::AlreadyNominatedCollator
				);
				Nominators::<T>::insert(&who, nominator)
			} else {
				let nominator = StakingNominators::new(vec![Bond {
					owner: candidate.clone(), amount
				}], amount);

				Nominators::<T>::insert(&who, nominator)
			}
			ensure!(
					collator.add_nomination(Bond {
						owner: who.clone(),
						amount
					}),
					Error::<T>::NominationNotExist
			);

			Collators::<T>::insert(&candidate, collator);
			T::Currency::reserve(&who, amount);

			let current_staked = TotalStaked::<T>::get();
			TotalStaked::<T>::put(current_staked + amount);

			Self::deposit_event(Event::Nominate(candidate, amount));

			Ok(Default::default())
		}

		#[pallet::weight(0)]
		pub fn nominate_extra(
			origin: OriginFor<T>,
			candidate: T::AccountId,
			extra: BalanceOf<T>
		) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;
			let mut collator = Collators::<T>::get(&candidate).ok_or(Error::<T>::CandidateNotExist)?;
			ensure!(
				collator.is_active(),
				Error::<T>::CandidateNotActive
			);
			let mut nominator = Nominators::<T>::get(&who).ok_or(Error::<T>::NominationNotExist)?;
			nominator.nominate_extra(Bond {
				owner: candidate.clone(),
				amount: extra
			}).ok_or(Error::<T>::CandidateNotExist)?;

			collator.nominate_extra(Bond {
				owner: who.clone(),
				amount: extra
			}).ok_or(Error::<T>::NominationNotExist)?;

			Collators::<T>::insert(&candidate, collator);
			Nominators::<T>::insert(&who, nominator);
			T::Currency::reserve(&who, extra);

			let current_staked = TotalStaked::<T>::get();
			TotalStaked::<T>::put(current_staked + extra);

			Self::deposit_event(Event::NominateExtra(
				candidate,
				extra,
			));

			Ok(Default::default())
		}

		#[pallet::weight(0)]
		pub fn nominate_less(
			origin: OriginFor<T>,
			candidate: T::AccountId,
			less: BalanceOf<T>
		) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;
			let mut collator = Collators::<T>::get(&candidate).ok_or(Error::<T>::CandidateNotExist)?;
			ensure!(
				collator.is_active(),
				Error::<T>::CandidateNotActive
			);
			let mut nominator = Nominators::<T>::get(&who).ok_or(Error::<T>::NominationNotExist)?;
			let current_round = CurrentRound::<T>::get();

			let after = nominator.nominate_less(Bond {
				owner: candidate.clone(),
				amount: less
			}, current_round.index + Settings::<T>::get().bond_duration)
				.ok_or(Error::<T>::CandidateNotExist)?
				.ok_or(Error::<T>::Underflow)?;

			ensure!(
				after >= T::MinNominatorStake::get(),
				Error::<T>::NominateBelowMin
			);

			let after = collator.nominate_less(Bond {
				owner: who.clone(),
				amount: less
			})
				.ok_or(Error::<T>::NominationNotExist)?
				.ok_or(Error::<T>::Underflow)?;

			ensure!(
				after >= T::MinNominatorStake::get(),
				Error::<T>::NominateBelowMin
			);

			Collators::<T>::insert(&candidate, collator);
			Nominators::<T>::insert(&who, nominator);
			Self::deposit_event(Event::NominateLess(
				candidate,
				less,
			));

			Ok(Default::default())
		}

		#[pallet::weight(0)]
		pub fn nominator_leave_collator(
			origin: OriginFor<T>,
			candidate: T::AccountId,
		) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;
			let mut nomination = Nominators::<T>::get(&who).ok_or(Error::<T>::NominationNotExist)?;
			let mut collator = Collators::<T>::get(&candidate).ok_or(Error::<T>::CandidateNotExist)?;
			let current_round = CurrentRound::<T>::get();

			nomination.rm_nomination(candidate.clone(), current_round.index + Settings::<T>::get().bond_duration)
				.ok_or(Error::<T>::CandidateNotExist)?;

			collator.rm_nomination(who.clone())
				.ok_or(Error::<T>::NominationNotExist)?;

			Collators::<T>::insert(&candidate, collator);
			Nominators::<T>::insert(&who, nomination);
			Self::deposit_event(Event::NominatorLeaveCollator(
				who,
				candidate,
			));
			Ok(Default::default())
		}
	}

	impl <T: Config> Pallet<T> {
		fn payout_stakers(current_round: RoundIndex) {
			let mint = |amount: BalanceOf<T>, to: T::AccountId| {
				if amount > T::Currency::minimum_balance() {
					if let Ok(imb) = T::Currency::deposit_into_existing(&to, amount) {
						Self::deposit_event(Event::Rewarded(to.clone(), imb.peek()));
					}
				}
			};

			let duration = T::PayoutDuration::get();
			if current_round > duration {
				let payout_round = current_round - duration;
				let total_stake = TotalStakedAt::<T>::get(payout_round);
				let total_issuance = TotalIssuanceAt::<T>::get(payout_round);

				let payout = compute_total_payout(
					total_stake,
					total_issuance,
					25u32,
					20u32,
					50u32,
					5u32,
					(T::BlocksPerRound::get() * 6000) as u64);

				let commission_point = Perbill::from_rational(
					50u32,
					100
				);
				let stake_point = Perbill::from_rational(
					50u32,
					100
				);
				let total_points = TotalPoints::<T>::get(payout_round);
				for (acc, exposure) in RoundStakerClipped::<T>::drain_prefix(payout_round) {
					let point = CollatorPoints::<T>::get(&payout_round, &acc);
					let collator_exposure_part = stake_point * Perbill::from_rational(
						exposure.own,
						total_stake,
					);
					let collator_commission_part = commission_point * Perbill::from_rational(
						point,
						total_points
					);
					mint(
						collator_exposure_part.mul(payout) + collator_commission_part.mul(payout),
						acc.clone()
					);

					for nominator in exposure.others.iter() {
						let nominator_exposure_part = Perbill::from_rational(
							nominator.value,
							total_stake,
						);
						mint(
							nominator_exposure_part.mul(payout),
							nominator.who.clone()
						);
					}
				}

			}
		}

		pub fn enact_election(current_round: RoundIndex) -> Option<Vec<T::AccountId>> {
			T::ElectionProvider::elect()
				.map_err(|e| {
					log!(warn, "election provider failed due to {:?}", e)
				})
				.and_then(|(res, weight)| {
					<frame_system::Pallet<T>>::register_extra_weight_unchecked(
						weight,
						frame_support::weights::DispatchClass::Mandatory,
					);
					Self::process_election(res, current_round)
				})
				.ok()
		}

		pub fn process_election(
			flat_supports: frame_election_provider_support::Supports<T::AccountId>,
			current_round: RoundIndex,
		) -> Result<Vec<T::AccountId>, ()> {
			let exposures = Self::collect_exposures(flat_supports);
			let elected_stashes = exposures.iter().cloned().map(|(x, _)| x).collect::<Vec<_>>();

			exposures.into_iter().for_each(|(stash, exposure)| {
				RoundStakerClipped::<T>::insert(current_round, stash.clone(), exposure.clone());
				Self::deposit_event(Event::CollatorChoosen(current_round, stash, exposure.total));
			});

			Ok(elected_stashes)
		}

		/// Consume a set of [`Supports`] from [`sp_npos_elections`] and collect them into a
		/// [`Exposure`].
		fn collect_exposures(
			supports: Supports<T::AccountId>,
		) -> Vec<(T::AccountId, Exposure<T::AccountId, BalanceOf<T>>)> {
			let total_issuance = T::Currency::total_issuance();
			let to_currency = |e: frame_election_provider_support::ExtendedBalance| {
				T::CurrencyToVote::to_currency(e, total_issuance)
			};

			supports
				.into_iter()
				.map(|(validator, support)| {
					// build `struct exposure` from `support`
					let mut others = Vec::with_capacity(support.voters.len());
					let mut own: BalanceOf<T> = Zero::zero();
					let mut total: BalanceOf<T> = Zero::zero();
					support
						.voters
						.into_iter()
						.map(|(nominator, weight)| (nominator, to_currency(weight)))
						.for_each(|(nominator, stake)| {
							if nominator == validator {
								own = own.saturating_add(stake);
							} else {
								others.push(IndividualExposure { who: nominator, value: stake });
							}
							total = total.saturating_add(stake);
						});

					let exposure = Exposure { own, others, total };
					(validator, exposure)
				})
				.collect::<Vec<(T::AccountId, Exposure<_, _>)>>()
		}

		fn update_collators(current_round: RoundIndex) {
			for (acc, mut collator) in  Collators::<T>::iter() {
				// active onboarding collator
				collator.active_onboard();
				// locked bond become active bond
				collator = collator.consolidate_active(current_round.clone());

				let before_total = collator.total;
				// executed unbonding after delay BondDuration
				collator = collator.consolidate_unbonded(current_round.clone());

				T::Currency::unreserve(&acc, before_total - collator.total);
				Collators::<T>::insert(&acc, collator)
			}
		}

		fn update_nominators(current_round: RoundIndex) {
			for (acc, mut nominations) in Nominators::<T>::iter() {
				let before_total = nominations.total;
				// executed unbonding after delay BondDuration
				nominations = nominations.consolidate_unbonded(current_round.clone());
				let current_staked = TotalStaked::<T>::get();
				let unbonded = before_total - nominations.total;
				TotalStaked::<T>::put(current_staked - unbonded);

				T::Currency::unreserve(&acc, unbonded);

				Nominators::<T>::insert(&acc, nominations)
			}
		}

		fn execute_exit_queue(current_round: RoundIndex) {
			for (acc, mut exit) in ExitQueue::<T>::iter() {
				let current_staked = TotalStaked::<T>::get();

				if exit.when > current_round {
					let unbonding = exit.unbonding.into_iter()
						.filter(|chunk| if chunk.round > current_round {
							true
						} else {
							T::Currency::unreserve(&acc, chunk.value);
							TotalStaked::<T>::put(current_staked - chunk.value);

							false
						}).collect();

					exit.unbonding = unbonding;
					ExitQueue::<T>::insert(&acc, exit);
				} else {
					T::Currency::unreserve(&acc, exit.remaining);
					TotalStaked::<T>::put(current_staked - exit.remaining);

					for unbond in exit.unbonding {
						T::Currency::unreserve(&acc, unbond.value);
						TotalStaked::<T>::put(current_staked - unbond.value);
					}
					ExitQueue::<T>::remove(&acc);
				}
			}
		}

		/// The total balance that can be slashed from a stash account as of right now.
		pub fn slashable_balance_of(stash: &T::AccountId, status: StakerStatus) -> BalanceOf<T> {
			// Weight note: consider making the stake accessible through stash.
			match status {
				StakerStatus::Validator => Self::collators(stash).filter(|c| c.is_active()).map(|c| c.active).unwrap_or_default(),
				StakerStatus::Nominator => Self::nominators(stash).map(|l| l.total).unwrap_or_default(),
				_ => Default::default(),
			}
		}

		/// Internal impl of [`Self::slashable_balance_of`] that returns [`VoteWeight`].
		pub fn slashable_balance_of_vote_weight(
			stash: &T::AccountId,
			issuance: BalanceOf<T>,
			status: StakerStatus
		) -> VoteWeight {
			T::CurrencyToVote::to_vote(Self::slashable_balance_of(stash, status), issuance)
		}

		/// Returns a closure around `slashable_balance_of_vote_weight` that can be passed around.
		///
		/// This prevents call sites from repeatedly requesting `total_issuance` from backend. But it is
		/// important to be only used while the total issuance is not changing.
		pub fn slashable_balance_of_fn(status: StakerStatus) -> Box<dyn Fn(&T::AccountId) -> VoteWeight> {
			// NOTE: changing this to unboxed `impl Fn(..)` return type and the module will still
			// compile, while some types in mock fail to resolve.
			let issuance = T::Currency::total_issuance();
			Box::new(move |who: &T::AccountId| -> VoteWeight {
				Self::slashable_balance_of_vote_weight(who, issuance, status)
			})
		}

		/// Get all of the voters that are eligible for the npos election.
		///
		/// This will use all on-chain nominators, and all the validators will inject a self vote.
		///
		/// ### Slashing
		///
		/// All nominations that have been submitted before the last non-zero slash of the validator are
		/// auto-chilled.
		///
		/// Note that this is VERY expensive. Use with care.
		fn get_npos_voters() -> Vec<(T::AccountId, VoteWeight, Vec<T::AccountId>)> {
			let weight_of_validator = Self::slashable_balance_of_fn(StakerStatus::Validator);
			let weight_of_nominator = Self::slashable_balance_of_fn(StakerStatus::Nominator);
			let mut all_voters = Vec::new();

			for (validator, _) in <Collators<T>>::iter() {
				// append self vote
				let self_vote = (validator.clone(), weight_of_validator(&validator), vec![validator.clone()]);
				all_voters.push(self_vote);
			}

			for (nominator, nominations) in Nominators::<T>::iter() {
				let StakingNominators { nominations, .. } = nominations;
				let mut targets = vec![];
				for bond in nominations {
					targets.push(bond.owner.clone())
				}

				let vote_weight = weight_of_nominator(&nominator);
				all_voters.push((nominator, vote_weight, targets))
			}

			all_voters
		}

		pub fn get_npos_targets() -> Vec<T::AccountId> {
			<Collators<T>>::iter().map(|(v, _)| v).collect::<Vec<_>>()
		}

		pub fn can_author(account: &T::AccountId) -> bool {
			Collators::<T>::get(&account).is_some()
		}
	}

	impl<T: Config> frame_election_provider_support::ElectionDataProvider<T::AccountId, T::BlockNumber>
	for Pallet<T>
	{
		const MAXIMUM_VOTES_PER_VOTER: u32 = T::MAX_COLLATORS_PER_NOMINATOR;

		fn targets(maybe_max_len: Option<usize>) -> data_provider::Result<(Vec<T::AccountId>, Weight)> {
			let target_count = <Collators<T>>::iter().filter(|c| c.1.is_active()).count();

			if maybe_max_len.map_or(false, |max_len| target_count > max_len) {
				return Err("Target snapshot too big");
			}

			let weight = <T as frame_system::Config>::DbWeight::get().reads(target_count as u64);
			Ok((Self::get_npos_targets(), weight))
		}

		fn voters(maybe_max_len: Option<usize>) -> data_provider::Result<(Vec<(T::AccountId, VoteWeight, Vec<T::AccountId>)>, Weight)> {
			let nominator_count = Nominators::<T>::iter().count();
			let validator_count = <Collators<T>>::iter().filter(|c| c.1.is_active()).count();
			let voter_count = nominator_count.saturating_add(validator_count);

			if maybe_max_len.map_or(false, |max_len| voter_count > max_len) {
				return Err("Voter snapshot too big");
			}
			let weight = <T as frame_system::Config>::DbWeight::get().reads(voter_count as u64);

			Ok((Self::get_npos_voters(), weight))
		}

		fn desired_targets() -> data_provider::Result<(u32, Weight)> {
			Ok((Settings::<T>::get().desired_target, <T as frame_system::Config>::DbWeight::get().reads(1)))
		}

		fn next_election_prediction(_: T::BlockNumber) -> T::BlockNumber {
			let current_round = Self::current_round();
			current_round.next_election_prediction()
		}
	}

	#[pallet::storage]
	#[pallet::getter(fn current_round)]
	pub type CurrentRound<T: Config> =
	StorageValue<_, RoundInfo<T::BlockNumber>, ValueQuery>;
	/// The current era index.
	///
	/// This is the latest planned era, depending on how the Session pallet queues the validator
	/// set, it might be active or not.
	#[pallet::storage]
	#[pallet::getter(fn current_round)]
	pub type CurrentEra<T: Config> =
	StorageValue<_, Option<EraIndex>, ValueQuery>;

	/// The active era information, it holds index and start.
	///
	/// The active era is the era being currently rewarded. Validator set of this era must be
	/// equal to [`SessionInterface::validators`].
	#[pallet::storage]
	#[pallet::getter(fn active_era)]
	pub type ActiveEra<T: Config> =
	StorageValue<_, Option<ActiveEraInfo>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn bonded_eras)]
	pub type BondedEras<T: Config> =
	StorageValue<_, Vec<(EraIndex, SessionIndex)>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn collators)]
	pub type Collators<T: Config> =
	StorageMap<_, Twox64Concat, T::AccountId, StakingCollators<T::AccountId, BalanceOf<T>>>;

	#[pallet::storage]
	#[pallet::getter(fn exit_queue)]
	pub type ExitQueue<T: Config> =
	StorageMap<_, Twox64Concat, T::AccountId, Leaving<BalanceOf<T>>>;

	#[pallet::storage]
	#[pallet::getter(fn total_staked)]
	pub type TotalStaked<T: Config> =
	StorageValue<_, BalanceOf<T>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn settings)]
	pub type Settings<T: Config> =
	StorageValue<_, SettingStruct, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn total_staked_at)]
	pub type TotalStakedAt<T: Config> =
	StorageMap<_, Twox64Concat, RoundIndex, BalanceOf<T>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn total_issuance_at)]
	pub type TotalIssuanceAt<T: Config> =
	StorageMap<_, Twox64Concat, RoundIndex, BalanceOf<T>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn nominators)]
	pub type Nominators<T: Config> =
	StorageMap<_, Twox64Concat, T::AccountId, StakingNominators<T::AccountId, BalanceOf<T>>>;

	#[pallet::storage]
	#[pallet::getter(fn round_staker_clipped)]
	pub type RoundStakerClipped<T: Config> = StorageDoubleMap<
		_,
		Twox64Concat,
		RoundIndex,
		Twox64Concat,
		T::AccountId,
		Exposure<T::AccountId, BalanceOf<T>>,
		ValueQuery,
	>;

	#[pallet::storage]
	#[pallet::getter(fn storage_version)]
	pub type StorageVersion<T: Config> =
	StorageValue<_, Releases, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn total_points)]
	pub type TotalPoints<T: Config> = StorageMap<_, Twox64Concat, RoundIndex, RewardPoint, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn awarded_pts)]
	pub type CollatorPoints<T: Config> = StorageDoubleMap<
		_,
		Twox64Concat,
		RoundIndex,
		Twox64Concat,
		T::AccountId,
		RewardPoint,
		ValueQuery,
	>;

	#[pallet::error]
	pub enum Error<T> {
		/// Candidate already bonded
		AlreadyBonded,
		/// Candidate already in queue
		AlreadyInQueue,
		/// Bond not exist
		BondNotExist,
		/// Value under flow
		Underflow,
		/// Bond less than minimum value
		BondBelowMin,
		/// Bond less than minimum value
		NominateBelowMin,
		/// Nominate not exist candidate
		CandidateNotExist,
		/// Too many candidates supplied
		TooManyCandidates,
		/// Nomination not exist
		NominationNotExist,
		/// Already nominated collator
		AlreadyNominatedCollator,
		/// Too many nomination candidates supplied
		TooManyNominations,
		/// Candidate not active
		CandidateNotActive,
		/// Candidate is leaving
		AlreadyLeaving,
	}

	#[pallet::event]
	#[pallet::generate_deposit(fn deposit_event)]
	pub enum Event<T: Config> {
		Bonded(T::AccountId, BalanceOf<T>),
		BondExtra(T::AccountId, BalanceOf<T>),
		BondLess(T::AccountId, BalanceOf<T>),
		Nominate(T::AccountId, BalanceOf<T>),
		NominateExtra(T::AccountId, BalanceOf<T>),
		NominateLess(T::AccountId, BalanceOf<T>),
		CandidateOnboard(T::AccountId),
		CandidateLeaving(T::AccountId, RoundIndex),
		NominatorLeaveCollator(T::AccountId, T::AccountId),
		CollatorChoosen(RoundIndex, T::AccountId, BalanceOf<T>),
		Rewarded(T::AccountId, BalanceOf<T>),
		SettingChanged(SettingStruct),
		NewRoundStart(RoundIndex, RoundIndex)
	}

	/// Add reward points to block authors:
	/// * 20 points to the block producer for producing a block in
	impl<T> pallet_authorship::EventHandler<T::AccountId, T::BlockNumber> for Pallet<T>
		where
			T: Config + pallet_authorship::Config + pallet_session::Config,
	{
		fn note_author(author: T::AccountId) {
			let now = <CurrentRound<T>>::get().index;
			let score_plus_20 = <CollatorPoints<T>>::get(now, &author) + 20;
			<CollatorPoints<T>>::insert(now, author, score_plus_20);
			<TotalPoints<T>>::mutate(now, |x| *x += 20);
		}

		// just ignore it
		fn note_uncle(_author: T::AccountId, _age: T::BlockNumber) {

		}
	}


	/// A typed conversion from stash account ID to the active exposure of nominators
	/// on that account.
	///
	/// Active exposure is the exposure of the validator set currently validating, i.e. in
	/// `active_era`. It can differ from the latest planned exposure in `current_era`.
	pub struct ExposureOf<T>(sp_std::marker::PhantomData<T>);
}
