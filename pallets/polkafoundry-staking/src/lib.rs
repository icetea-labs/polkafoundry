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
	use frame_support::{
		pallet_prelude::*,
		traits::{Currency, ReservableCurrency, CurrencyToVote, Imbalance, OnUnbalanced, UnixTime, EstimateNextNewSession}
	};
	use frame_system::pallet_prelude::*;
	use sp_runtime::traits::{Saturating, Zero, AtLeast32BitUnsigned, Convert, SaturatedConversion};
	use sp_runtime::{Perbill};
	use sp_std::{convert::{From}, vec::Vec};
	use frame_election_provider_support::{ElectionProvider, VoteWeight, Supports, data_provider};
	use crate::inflation::{compute_total_payout, INposInput};
	use sp_std::{cmp::Ordering, prelude::*, ops::{Mul, AddAssign, Add, Sub}};
	use log::info;
	use pallet_session::historical;
	use serde::{Serialize, Deserialize};
	use frame_support::sp_std::fmt::Debug;

	/// Counter for the number of round that have passed
	pub type RoundIndex = u32;
	/// Counter for the number of "reward" points earned by a given collator
	pub type RewardPoint = u32;
	pub type EraIndex = u32;
	pub type SessionIndex = u32;

	type BalanceOf<T> = <<T as Config>::Currency as Currency<
		<T as frame_system::Config>::AccountId,
	>>::Balance;

	type PositiveImbalanceOf<T> = <<T as Config>::Currency as Currency<
		<T as frame_system::Config>::AccountId,
	>>::PositiveImbalance;

	pub type NegativeImbalanceOf<T> = <<T as Config>::Currency as Currency<
		<T as frame_system::Config>::AccountId,
	>>::NegativeImbalance;

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
		/// Number of collators that nominators can be nominated for
		const MAX_COLLATORS_PER_NOMINATOR: u32;
		/// Maximum number of nominations per collator
		type MaxNominationsPerCollator: Get<u32>;
		/// Minimum stake required to be reserved to be a collator
		type MinCollatorStake: Get<BalanceOf<Self>>;
		/// Minimum stake required to be reserved to be a nominator
		type MinNominatorStake: Get<BalanceOf<Self>>;
		/// Number of era per payout
		type PayoutDuration: Get<EraIndex>;
		/// Tokens have been minted and are unused for validator-reward.
		type RewardRemainder: OnUnbalanced<NegativeImbalanceOf<Self>>;

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
		/// Something that can estimate the next session change, accurately or as a best effort guess.
		type NextNewSession: EstimateNextNewSession<Self::BlockNumber>;

		type DesiredTarget: Get<u32>;
	}

	#[pallet::pallet]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(_now: T::BlockNumber) -> Weight {
			// just return the weight of the on_finalize.
			T::DbWeight::get().reads(1)
		}

		fn on_finalize(now: T::BlockNumber) {
			if let Some(mut active_era) = Self::active_era() {
				if active_era.start.is_none() {
					let now_as_millis_u64 = T::UnixTime::now().as_millis().saturated_into::<u64>();
					active_era.start = Some(now_as_millis_u64);
					// This write only ever happens once, we don't include it in the weight in general
					ActiveEra::<T>::put(Some(active_era));
				}
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

	/// Mode of era-forcing.
	#[derive(Copy, Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug)]
	#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
	pub enum Forcing {
		/// Not forcing anything - just let whatever happen.
		NotForcing,
		/// Force a new era, then reset to `NotForcing` as soon as it is done.
		ForceNew,
		/// Avoid a new era indefinitely.
		ForceNone,
		/// Force a new era at the end of all sessions indefinitely.
		ForceAlways,
	}

	impl Default for Forcing {
		fn default() -> Self { Forcing::NotForcing }
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
		/// era number at which point it'll be unbonded.
		pub era: EraIndex,
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
		/// The stash account whose balance is actually locked and at stake.
		pub stash: AccountId,
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
	}

	impl <AccountId, Balance> StakingCollators<AccountId, Balance>
	where
		AccountId: Ord + Clone,
		Balance: Ord + Copy + Debug + Saturating + AtLeast32BitUnsigned + AddAssign + From<u32>
	{
		pub fn new (stash: AccountId, amount: Balance, next_era: EraIndex) -> Self {
			StakingCollators {
				stash,
				total: amount,
				active: amount,
				nominations: vec![],
				unlocking: vec![UnlockChunk {
					value: amount,
					round: next_era
				}],
				unbonding: vec![],
				status: StakerStatus::default(),
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
		pub fn bond_extra (&mut self, extra: Balance) {
			self.total += extra;
			self.active += extra;
		}

		/// Bond less for collator
		/// Unbonding amount delay of `BondDuration` round
		pub fn bond_less (&mut self, less: Balance, era: EraIndex) -> Option<Balance> {
			if self.active > less {
				self.active -= less;
				self.unbonding.push(UnBondChunk {
					value: less,
					era
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
				stash: self.stash,
				total: self.total,
				active,
				nominations: self.nominations,
				unlocking,
				unbonding: self.unbonding,
				status: self.status,
			}
		}
		/// Remove all the locked bond after `BondDuration`
		/// Update `total` `active` updated immediately when call `bond_less`
		pub fn remove_unbond(self, active_era: EraIndex) -> Self {
			let mut total = self.total;
			let unbonding = self.unbonding.into_iter()
				.filter(|chunk| if chunk.era > active_era  {
					true
				} else {
					total -= chunk.value;
					false
				})
				.collect();

			Self {
				stash: self.stash,
				total,
				active: self.active,
				nominations: self.nominations,
				unlocking: self.unlocking,
				unbonding,
				status: self.status,
			}
		}

		/// Add nomination for collator
		/// Will be counted as vote weight for collator
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

	/// A destination account for payment.
	#[derive(PartialEq, Eq, Copy, Clone, Encode, Decode, RuntimeDebug)]
	pub enum RewardDestination<AccountId> {
		/// Pay into the stash account, increasing the amount at stake accordingly.
		Staked,
		/// Pay into the stash account, not increasing the amount at stake.
		Stash,
		/// Pay into the controller account.
		Controller,
		/// Pay into a specified account.
		Account(AccountId),
		/// Receive no reward.
		None,
	}

	impl<AccountId> Default for RewardDestination<AccountId> {
		fn default() -> Self {
			RewardDestination::Staked
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
		pub fn nominate_less(&mut self, less: Bond<AccountId, Balance>, era: EraIndex) -> Option<Option<Balance>> {
			for nominate in &mut self.nominations {
				if nominate.owner == less.owner {
					if nominate.amount > less.amount {
						nominate.amount -= less.amount;
						self.unbonding.push(UnBondChunk {
							value: less.amount,
							era
						});
						self.total -= less.amount;
						return Some(Some(nominate.amount));
					} else {
						return Some(None);
					}
				}
			}
			None
		}
		/// Remove all locked bond after `BondDuration`
		pub fn remove_unbond(&mut self, active_era: EraIndex) -> Balance {
			let mut unbonded = Zero::zero();
			let unbonding = self.unbonding.clone().into_iter()
				.filter(|chunk| if chunk.era > active_era {
					true
				} else {
					unbonded += chunk.value;
					false
				}).collect();

			self.unbonding = unbonding;

			unbonded
		}

		pub fn rm_nomination(&mut self, candidate: AccountId, era: EraIndex) -> Option<Balance> {
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
					era
				});
				self.total -= less;
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
		pub stakers: Vec<(T::AccountId, T::AccountId, BalanceOf<T>)>,
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
			for &(ref stash, ref controller, balance) in &self.stakers {
				assert!(
					T::Currency::free_balance(&stash) >= balance,
					"Account does not have enough balance to bond."
				);

				let _ = <Pallet<T>>::bond(
					T::Origin::from(Some(stash.clone()).into()),
					controller.clone(),
					balance,
					RewardDestination::Staked,
				);
			}
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(0)]
		pub fn bond(
			origin: OriginFor<T>,
			controller: T::AccountId,
			amount: BalanceOf<T>,
			payee: RewardDestination<T::AccountId>,
		) -> DispatchResultWithPostInfo {
			let stash = ensure_signed(origin)?;

			if <Bonded<T>>::contains_key(&stash) {
				Err(Error::<T>::AlreadyBonded)?
			}

			if <Ledger<T>>::contains_key(&controller) {
				Err(Error::<T>::AlreadyPaired)?
			}

			if amount < T::MinCollatorStake::get() {
				Err(Error::<T>::BondBelowMin)?
			}

			frame_system::Pallet::<T>::inc_consumers(&stash).map_err(|_| Error::<T>::BadState)?;
			// You're auto-bonded forever, here. We might improve this by only bonding when
			// you actually validate/nominate and remove once you unbond __everything__.
			<Bonded<T>>::insert(&stash, &controller);
			<Payee<T>>::insert(&stash, payee);

			let current_era = CurrentEra::<T>::get().unwrap_or(0);
			let staker = StakingCollators::new(stash.clone(), amount, current_era + 1);

			Ledger::<T>::insert(&controller, staker);

			T::Currency::reserve(
				&stash,
				amount,
			);
			Self::deposit_event(Event::Bonded(
				stash,
				amount,
			));
			Ok(Default::default())
		}

		#[pallet::weight(0)]
		pub fn bond_extra(
			origin: OriginFor<T>,
			extra: BalanceOf<T>
		) -> DispatchResultWithPostInfo {
			let stash = ensure_signed(origin)?;
			let controller = Bonded::<T>::get(&stash).ok_or(Error::<T>::NotStash)?;
			let mut ledger = Ledger::<T>::get(&controller).ok_or(Error::<T>::NotController)?;

			ledger.bond_extra(extra);
			Ledger::<T>::insert(&controller, ledger);

			T::Currency::reserve(
				&stash,
				extra,
			);

			Self::deposit_event(Event::BondExtra(
				stash,
				extra,
			));

			Ok(Default::default())
		}

		#[pallet::weight(0)]
		pub fn bond_less(
			origin: OriginFor<T>,
			less: BalanceOf<T>
		) -> DispatchResultWithPostInfo {
			let stash = ensure_signed(origin)?;
			let controller = Bonded::<T>::get(&stash).ok_or(Error::<T>::NotStash)?;
			let mut ledger = Ledger::<T>::get(&controller).ok_or(Error::<T>::NotController)?;

			let era = CurrentEra::<T>::get().unwrap_or(0) + T::BondingDuration::get();
			let after = ledger.bond_less(less, era).ok_or(Error::<T>::Underflow)?;

			ensure!(
					after >= T::MinCollatorStake::get(),
					Error::<T>::BondBelowMin
			);

			Ledger::<T>::insert(&controller, ledger);

			Self::deposit_event(Event::BondLess(
				stash,
				less,
			));

			Ok(Default::default())
		}

		#[pallet::weight(0)]
		pub fn collator_unbond(
			origin: OriginFor<T>,
		) -> DispatchResultWithPostInfo {
			let stash = ensure_signed(origin)?;
			let controller = Bonded::<T>::get(&stash).ok_or(Error::<T>::NotStash)?;
			let mut ledger = Ledger::<T>::get(&controller).ok_or(Error::<T>::NotController)?;

			let current_era = CurrentEra::<T>::get().unwrap_or(0);
			let when = current_era + T::BondingDuration::get();

			// leave all nominations
			for nomination in ledger.nominations {
				T::Currency::unreserve(&nomination.owner, nomination.amount);
			}

			let exit = Leaving::new(ledger.active, ledger.unbonding, when);

			ExitQueue::<T>::insert(&stash, exit);
			Ledger::<T>::remove(&controller);

			Self::deposit_event(Event::CandidateLeaving(
				stash,
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
			let mut ledger = Self::ledger(&candidate).ok_or(Error::<T>::NotController)?;

			ensure!(
					ledger.nominations.len() < T::MaxNominationsPerCollator::get() as usize,
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
				ledger.add_nomination(Bond {
					owner: who.clone(),
					amount
				}),
				Error::<T>::NominationNotExist
			);

			Ledger::<T>::insert(&candidate, ledger);
			T::Currency::reserve(&who, amount);

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
			let mut ledger = Self::ledger(&candidate).ok_or(Error::<T>::NotController)?;

			let mut nominator = Nominators::<T>::get(&who).ok_or(Error::<T>::NominationNotExist)?;

			nominator.nominate_extra(Bond {
				owner: candidate.clone(),
				amount: extra
			}).ok_or(Error::<T>::CandidateNotExist)?;

			ledger.nominate_extra(Bond {
				owner: who.clone(),
				amount: extra
			}).ok_or(Error::<T>::NominationNotExist)?;

			Ledger::<T>::insert(&candidate, ledger);
			Nominators::<T>::insert(&who, nominator);
			T::Currency::reserve(&who, extra);

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
			let mut ledger = Self::ledger(&candidate).ok_or(Error::<T>::NotController)?;

			let mut nominator = Nominators::<T>::get(&who).ok_or(Error::<T>::NominationNotExist)?;
			let current_era = CurrentEra::<T>::get().unwrap_or(0);

			let after = nominator.nominate_less(Bond {
				owner: candidate.clone(),
				amount: less
			}, current_era + T::BondingDuration::get())
				.ok_or(Error::<T>::CandidateNotExist)?
				.ok_or(Error::<T>::Underflow)?;

			ensure!(
				after >= T::MinNominatorStake::get(),
				Error::<T>::NominateBelowMin
			);

			let after = ledger.nominate_less(Bond {
				owner: who.clone(),
				amount: less
			})
				.ok_or(Error::<T>::NominationNotExist)?
				.ok_or(Error::<T>::Underflow)?;

			ensure!(
				after >= T::MinNominatorStake::get(),
				Error::<T>::NominateBelowMin
			);

			Ledger::<T>::insert(&candidate, ledger);
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
			let mut ledger = Self::ledger(&candidate).ok_or(Error::<T>::NotController)?;
			let mut nominator = Nominators::<T>::get(&who).ok_or(Error::<T>::NominationNotExist)?;
			let current_era = CurrentEra::<T>::get().unwrap_or(0);

			nominator.rm_nomination(candidate.clone(), current_era + T::BondingDuration::get())
				.ok_or(Error::<T>::CandidateNotExist)?;

			ledger.rm_nomination(who.clone())
				.ok_or(Error::<T>::NominationNotExist)?;

			Ledger::<T>::insert(&candidate, ledger);
			Nominators::<T>::insert(&who, nominator);
			Self::deposit_event(Event::NominatorLeaveCollator(
				who,
				candidate,
			));
			Ok(Default::default())
		}
	}

	impl <T: Config> Pallet<T> {
		fn new_session(session_index: SessionIndex) -> Option<Vec<T::AccountId>> {
			if let Some(current_era) = CurrentEra::<T>::get() {
				// Initial era has been set.
				let current_era_start_session_index = Self::eras_start_session_index(current_era)
					.unwrap_or_else(|| {
						frame_support::print("Error: start_session_index must be set for current_era");
						0u32
					});

				let era_length = session_index.checked_sub(current_era_start_session_index)
					.unwrap_or(0); // Must never happen.

				match ForceEra::<T>::get() {
					// Will set to default again, which is `NotForcing`.
					Forcing::ForceNew => ForceEra::<T>::kill(),
					// Short circuit to `new_era`.
					Forcing::ForceAlways => (),
					// Only go to `new_era` if deadline reached.
					Forcing::NotForcing if era_length >= T::SessionsPerEra::get() => (),
					_ => {
						// either `Forcing::ForceNone`,
						// or `Forcing::NotForcing if era_length >= T::SessionsPerEra::get()`.
						return None
					},
				}

				CurrentSession::<T>::put(&session_index);
				// new era.
				Self::new_era(session_index)
			} else {
				log!(debug, "Starting the first era.");

				CurrentSession::<T>::put(&session_index);
				Self::new_era(session_index)
			}
		}

		/// Start a session potentially starting an era.
		fn start_session(start_session: SessionIndex) {
			let next_active_era = Self::active_era().map(|e| e.index + 1).unwrap_or(0);
			// This is only `Some` when current era has already progressed to the next era, while the
			// active era is one behind (i.e. in the *last session of the active era*, or *first session
			// of the new current era*, depending on how you look at it).
			if let Some(next_active_era_start_session_index) =
			Self::eras_start_session_index(next_active_era)
			{
				if next_active_era_start_session_index == start_session {
					Self::start_era(start_session);
				} else if next_active_era_start_session_index < start_session {
					// This arm should never happen, but better handle it than to stall the staking
					// pallet.
					frame_support::print("Warning: A session appears to have been skipped.");
					Self::start_era(start_session);
				}
			}
		}

		/// End a session potentially ending an era.
		fn end_session(session_index: SessionIndex) {
			if let Some(active_era) = Self::active_era() {
				if let Some(next_active_era_start_session_index) =
				Self::eras_start_session_index(active_era.index + 1)
				{
					if next_active_era_start_session_index == session_index + 1 {
						Self::end_era(active_era, session_index);
					}
				}
			}
		}

		/// Plan a new era. Return the potential new staking set.
		fn new_era(start_session_index: SessionIndex) -> Option<Vec<T::AccountId>> {
			// Increment or set current era.
			let current_era = CurrentEra::<T>::mutate(|s| {
				*s = Some(s.map(|s| s + 1).unwrap_or(0));
				s.unwrap()
			});
			ErasStartSessionIndex::<T>::insert(&current_era, &start_session_index);

			// Clean old era information.
			if let Some(old_era) = current_era.checked_sub(Self::history_depth() + 1) {
				Self::clear_era_information(old_era);
			}

			// Set staking information for new era.
			let maybe_new_validators = Self::enact_election(current_era);

			maybe_new_validators
		}

		/// * Increment `active_era.index`,
		/// * reset `active_era.start`,
		fn start_era(start_session: SessionIndex) {
			let active_era = ActiveEra::<T>::mutate(|active_era| {
				let new_index = active_era.as_ref().map(|info| info.index + 1).unwrap_or(0);
				*active_era = Some(ActiveEraInfo {
					index: new_index,
					// Set new active era start in next `on_finalize`. To guarantee usage of `Time`
					start: None,
				});
				new_index
			});
			let bonding_duration = T::BondingDuration::get();

			BondedEras::<T>::mutate(|bonded| {
				bonded.push((active_era, start_session));

				if active_era > bonding_duration {
					let first_kept = active_era - bonding_duration;

					// prune out everything that's from before the first-kept index.
					let n_to_prune = bonded.iter()
						.take_while(|&&(era_idx, _)| era_idx < first_kept)
						.count();

					// kill slashing metadata.
					for (pruned_era, _) in bonded.drain(..n_to_prune) {
						// slashing::clear_era_metadata::<T>(pruned_era);
					}

					if let Some(&(_, first_session)) = bonded.first() {
						T::SessionInterface::prune_historical_up_to(first_session);
					}
				}
			});

			Self::update_collators(active_era);
			Self::update_nominators(active_era);
			Self::execute_exit_queue(active_era);
		}

		/// Clear all era information for given era.
		fn clear_era_information(era_index: EraIndex) {
			ErasStartSessionIndex::<T>::remove(era_index);
		}

		/// Compute payout for era.
		fn end_era(active_era: ActiveEraInfo, _session_index: SessionIndex) {
			// Note: active_era_start can be None if end era is called during genesis config.
			if let Some(active_era_start) = active_era.start {
				let now_as_millis_u64 = T::UnixTime::now().as_millis().saturated_into::<u64>();

				let era_duration = (now_as_millis_u64 - active_era_start).saturated_into::<u64>();
				let staked = Self::eras_total_stake(&active_era.index);
				let issuance = T::Currency::total_issuance();
				let validator_payout = Self::era_payout(staked, issuance, era_duration);

				Self::deposit_event(Event::EraPayout(active_era.index, validator_payout));

				// Set ending era reward.
				<ErasValidatorReward<T>>::insert(&active_era.index, validator_payout);

				Self::payout_stakers(active_era.index)
			}
		}

		fn era_payout(staked: BalanceOf<T>, issuance: BalanceOf<T>, era_duration: u64) -> BalanceOf<T> {
			let payout = compute_total_payout(
				INposInput {
					i_0: 25u32,
					i_ideal: 20u32,
					x_ideal: 50u32,
					d: 5u32
				},
				staked,
				 issuance,
				era_duration);

			payout
		}

		fn payout_stakers(current_era: EraIndex) {
			let mint = |amount: BalanceOf<T>, to: T::AccountId| {
				if amount > T::Currency::minimum_balance() {
					if let Some(imb) = Self::make_payout(&to, amount) {
						Self::deposit_event(Event::Rewarded(to.clone(), imb.peek()));
					}
				}
			};

			let duration = T::PayoutDuration::get();
			if current_era > duration {
				let payout_era = current_era - duration;
				let total_stake = ErasTotalStake::<T>::take(&payout_era);

				let payout = ErasValidatorReward::<T>::get(&payout_era);
				let mut rest = payout.clone();

				let commission_point = Perbill::from_rational(
					50u32,
					100
				);
				let stake_point = Perbill::from_rational(
					50u32,
					100
				);
				let total_points = TotalPoints::<T>::take(&payout_era);
				for (acc, exposure) in ErasStakersClipped::<T>::drain_prefix(payout_era) {
					let point = CollatorPoints::<T>::take(&payout_era, &acc);
					let collator_exposure_part = stake_point * Perbill::from_rational(
						exposure.own,
						total_stake,
					);
					let collator_commission_part = commission_point * Perbill::from_rational(
						point,
						total_points
					);
					let collator_part = collator_exposure_part.mul(payout) + collator_commission_part.mul(payout);
					rest -= collator_part;

					mint(
						collator_part,
						acc.clone()
					);

					for nominator in exposure.others.iter() {
						let nominator_exposure_part = Perbill::from_rational(
							nominator.value,
							total_stake,
						);
						let nominator_part = nominator_exposure_part.mul(payout);
						rest -= nominator_part;

						mint(
							nominator_part,
							nominator.who.clone()
						);
					}
				}
				T::RewardRemainder::on_unbalanced(T::Currency::issue(rest));
			}
		}

		/// Actually make a payment to a staker. This uses the currency's reward function
		/// to pay the right payee for the given staker account.
		fn make_payout(stash: &T::AccountId, amount: BalanceOf<T>) -> Option<PositiveImbalanceOf<T>> {
			let dest = Self::payee(stash);
			match dest {
				RewardDestination::Controller => Self::bonded(stash)
					.and_then(|controller|
						Some(T::Currency::deposit_creating(&controller, amount))
					),
				RewardDestination::Stash =>
					T::Currency::deposit_into_existing(stash, amount).ok(),
				RewardDestination::Staked => Self::bonded(stash)
					.and_then(|c| Self::ledger(&c).map(|l| (c, l)))
					.and_then(|(controller, mut l)| {
						l.active += amount;
						l.total += amount;
						let r = T::Currency::deposit_into_existing(stash, amount).ok();
						T::Currency::reserve(&controller, l.total);
						r
					}),
				RewardDestination::Account(dest_account) => {
					Some(T::Currency::deposit_creating(&dest_account, amount))
				},
				RewardDestination::None => None,
			}
		}

		pub fn enact_election(current_era: EraIndex) -> Option<Vec<T::AccountId>> {
			T::ElectionProvider::elect()
				.map_err(|e| {
					log!(warn, "election provider failed due to {:?}", e)
				})
				.and_then(|(res, weight)| {
					<frame_system::Pallet<T>>::register_extra_weight_unchecked(
						weight,
						frame_support::weights::DispatchClass::Mandatory,
					);
					Self::process_election(res, current_era)
				})
				.ok()
		}

		pub fn process_election(
			flat_supports: frame_election_provider_support::Supports<T::AccountId>,
			current_era: EraIndex,
		) -> Result<Vec<T::AccountId>, ()> {
			let exposures = Self::collect_exposures(flat_supports);
			let elected_stashes = exposures.iter().cloned().map(|(x, _)| x).collect::<Vec<_>>();

			// Populate stakers, exposures, and the snapshot of validator prefs.
			let mut total_stake: BalanceOf<T> = Zero::zero();
			exposures.into_iter().for_each(|(stash, exposure)| {
				total_stake = total_stake.saturating_add(exposure.total);

				ErasStakersClipped::<T>::insert(current_era, stash.clone(), exposure.clone());
				Self::deposit_event(Event::CollatorChoosen(current_era, stash, exposure.total));
			});
			<ErasTotalStake<T>>::insert(&current_era, total_stake);

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

		fn update_collators(active_era: EraIndex) {
			for (acc, mut collator) in  Ledger::<T>::iter() {
				let before_total = collator.total;
				// executed unbonding after delay BondDuration
				collator = collator.remove_unbond(active_era.clone());

				T::Currency::unreserve(&acc, before_total - collator.total);
				Ledger::<T>::insert(&acc, collator)
			}
		}

		fn update_nominators(active_era: EraIndex) {
			for (acc, mut nominations) in Nominators::<T>::iter() {
				// executed unbonding after delay BondDuration
				let unbonded = nominations.remove_unbond(active_era.clone());

				T::Currency::unreserve(&acc, unbonded);

				Nominators::<T>::insert(&acc, nominations)
			}
		}

		fn execute_exit_queue(active_era: EraIndex) {
			for (acc, mut exit) in ExitQueue::<T>::iter() {
				// if now > active era unreserve the balance and remove collator
				if exit.when > active_era {
					let unbonding = exit.unbonding.into_iter()
						.filter(|chunk| if chunk.era > active_era {
							true
						} else {
							T::Currency::unreserve(&acc, chunk.value);

							false
						}).collect();

					exit.unbonding = unbonding;
					ExitQueue::<T>::insert(&acc, exit);
				} else {
					// unbond all remaining balance and unbond balance then remove to queue
					T::Currency::unreserve(&acc, exit.remaining);

					for unbond in exit.unbonding {
						T::Currency::unreserve(&acc, unbond.value);
					}
					ExitQueue::<T>::remove(&acc);
				}
			}
		}

		/// The total balance that can be slashed from a stash account as of right now.
		pub fn slashable_balance_of(stash: &T::AccountId, status: StakerStatus) -> BalanceOf<T> {
			// Weight note: consider making the stake accessible through stash.
			match status {
				StakerStatus::Validator => Self::ledger(stash).map(|c| c.active).unwrap_or_default(),
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

			for (validator, _) in <Ledger<T>>::iter() {
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
			let t = <Ledger<T>>::iter().map(|(v, _)| v).collect::<Vec<_>>();
			<Ledger<T>>::iter().map(|(v, _)| v).collect::<Vec<_>>()
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
			let target_count = <Ledger<T>>::iter().count();

			if maybe_max_len.map_or(false, |max_len| target_count > max_len) {
				return Err("Target snapshot too big");
			}

			let weight = <T as frame_system::Config>::DbWeight::get().reads(target_count as u64);
			Ok((Self::get_npos_targets(), weight))
		}

		fn voters(maybe_max_len: Option<usize>) -> data_provider::Result<(Vec<(T::AccountId, VoteWeight, Vec<T::AccountId>)>, Weight)> {
			let nominator_count = Nominators::<T>::iter().count();
			let validator_count = <Ledger<T>>::iter().count();
			let voter_count = nominator_count.saturating_add(validator_count);

			if maybe_max_len.map_or(false, |max_len| voter_count > max_len) {
				return Err("Voter snapshot too big");
			}
			let weight = <T as frame_system::Config>::DbWeight::get().reads(voter_count as u64);

			Ok((Self::get_npos_voters(), weight))
		}

		fn desired_targets() -> data_provider::Result<(u32, Weight)> {
			Ok((10u32, <T as frame_system::Config>::DbWeight::get().reads(1)))
		}

		fn next_election_prediction(now: T::BlockNumber) -> T::BlockNumber {
			let current_era = Self::current_era().unwrap_or(0);
			let current_session = Self::current_planned_session();
			let current_era_start_session_index =
				Self::eras_start_session_index(current_era).unwrap_or(0);
			let era_length = current_session
				.saturating_sub(current_era_start_session_index)
				.min(T::SessionsPerEra::get());

			let session_length = T::NextNewSession::average_session_length();

			let until_this_session_end = T::NextNewSession::estimate_next_new_session(now)
				.0
				.unwrap_or_default()
				.saturating_sub(now);

			let sessions_left: T::BlockNumber = T::SessionsPerEra::get()
				.saturating_sub(era_length)
				// one session is computed in this_session_end.
				.saturating_sub(1)
				.into();

			now.saturating_add(
				until_this_session_end.saturating_add(sessions_left.saturating_mul(session_length)),
			)
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
	#[pallet::getter(fn current_era)]
	pub type CurrentEra<T: Config> =
	StorageValue<_, Option<EraIndex>, ValueQuery>;

	/// The current session index.
	#[pallet::storage]
	#[pallet::getter(fn current_session)]
	pub type CurrentSession<T: Config> =
	StorageValue<_, SessionIndex, ValueQuery>;

	/// Mode of era forcing.
	#[pallet::storage]
	#[pallet::getter(fn force_era)]
	pub type ForceEra<T: Config> =
	StorageValue<_, Forcing, ValueQuery>;

	/// Mode of era forcing.
	#[pallet::storage]
	#[pallet::getter(fn history_depth)]
	pub type HistoryDepth<T: Config> =
	StorageValue<_, u32, ValueQuery>;

	/// The active era information, it holds index and start.
	///
	/// The active era is the era being currently rewarded. Validator set of this era must be
	/// equal to [`SessionInterface::validators`].
	#[pallet::storage]
	#[pallet::getter(fn active_era)]
	pub type ActiveEra<T: Config> =
	StorageValue<_, Option<ActiveEraInfo>, ValueQuery>;

	/// A mapping from still-bonded eras to the first session index of that era.
	///
	/// Must contains information for eras for the range:
	/// `[active_era - bounding_duration; active_era]`
	#[pallet::storage]
	#[pallet::getter(fn bonded_eras)]
	pub type BondedEras<T: Config> =
	StorageValue<_, Vec<(EraIndex, SessionIndex)>, ValueQuery>;

	/// Map from all locked "stash" accounts to the controller account.
	#[pallet::storage]
	#[pallet::getter(fn bonded)]
	pub type Bonded<T: Config> =
	StorageMap<_, Twox64Concat, T::AccountId, T::AccountId>;

	/// Where the reward payment should be made. Keyed by stash.
	#[pallet::storage]
	#[pallet::getter(fn payee)]
	pub type Payee<T: Config> =
	StorageMap<_, Twox64Concat, T::AccountId, RewardDestination<T::AccountId>, ValueQuery>;

	/// Map from all (unlocked) "controller" accounts to the info regarding the staking.
	#[pallet::storage]
	#[pallet::getter(fn ledger)]
	pub type Ledger<T: Config> =
	StorageMap<_, Twox64Concat, T::AccountId, StakingCollators<T::AccountId, BalanceOf<T>>>;

	#[pallet::storage]
	#[pallet::getter(fn collators)]
	pub type Collators<T: Config> =
	StorageMap<_, Twox64Concat, T::AccountId, StakingCollators<T::AccountId, BalanceOf<T>>>;

	/// The session index at which the era start for the last `HISTORY_DEPTH` eras.
	///
	/// Note: This tracks the starting session (i.e. session index when era start being active)
	/// for the eras in `[CurrentEra - HISTORY_DEPTH, CurrentEra]`.
	#[pallet::storage]
	#[pallet::getter(fn eras_start_session_index)]
	pub type ErasStartSessionIndex<T: Config> =
	StorageMap<_, Twox64Concat, EraIndex, SessionIndex>;
	/// The session index at which the era start for the last `HISTORY_DEPTH` eras.
	///
	/// Note: This tracks the starting session (i.e. session index when era start being active)
	/// for the eras in `[CurrentEra - HISTORY_DEPTH, CurrentEra]`.
	#[pallet::storage]
	#[pallet::getter(fn eras_total_stake)]
	pub type ErasTotalStake<T: Config> =
	StorageMap<_, Twox64Concat, EraIndex, BalanceOf<T>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn exit_queue)]
	pub type ExitQueue<T: Config> =
	StorageMap<_, Twox64Concat, T::AccountId, Leaving<BalanceOf<T>>>;
	/// The last planned session scheduled by the session pallet.
	///
	/// This is basically in sync with the call to [`SessionManager::new_session`].
	#[pallet::storage]
	#[pallet::getter(fn current_planned_session)]
	pub type CurrentPlannedSession<T: Config> =
	StorageValue<_, SessionIndex, ValueQuery>;
	/// The total validator era payout for the last `HISTORY_DEPTH` eras.
	///
	/// Eras that haven't finished yet or has been removed doesn't have reward.
	#[pallet::storage]
	#[pallet::getter(fn eras_validator_reward)]
	pub type ErasValidatorReward<T: Config> =
	StorageMap<_, Twox64Concat, EraIndex, BalanceOf<T>, ValueQuery>;
	#[pallet::storage]
	#[pallet::getter(fn nominators)]
	pub type Nominators<T: Config> =
	StorageMap<_, Twox64Concat, T::AccountId, StakingNominators<T::AccountId, BalanceOf<T>>>;
	/// Clipped Exposure of validator at era.
	///
	/// This is similar to [`ErasStakers`] but number of nominators exposed is reduced to the
	/// `T::MaxNominatorRewardedPerValidator` biggest stakers.
	/// (Note: the field `total` and `own` of the exposure remains unchanged).
	/// This is used to limit the i/o cost for the nominator payout.
	///
	/// This is keyed fist by the era index to allow bulk deletion and then the stash account.
	///
	/// Is it removed after `HISTORY_DEPTH` eras.
	/// If stakers hasn't been set or has been removed then empty exposure is returned.
	#[pallet::storage]
	#[pallet::getter(fn eras_stakers_clipped)]
	pub type ErasStakersClipped<T: Config> = StorageDoubleMap<
		_,
		Twox64Concat,
		EraIndex,
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
	pub type TotalPoints<T: Config> = StorageMap<_, Twox64Concat, EraIndex, RewardPoint, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn awarded_pts)]
	pub type CollatorPoints<T: Config> = StorageDoubleMap<
		_,
		Twox64Concat,
		EraIndex,
		Twox64Concat,
		T::AccountId,
		RewardPoint,
		ValueQuery,
	>;

	#[pallet::storage]
	#[pallet::getter(fn eras_stakers)]
	pub type ErasStakers<T: Config> = StorageDoubleMap<
		_,
		Twox64Concat,
		EraIndex,
		Twox64Concat,
		T::AccountId,
		Exposure<T::AccountId, BalanceOf<T>>,
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
		/// Controller is already paired.
		AlreadyPaired,
		/// Internal state has become somehow corrupted and the operation cannot continue.
		BadState,
		/// Not a controller account.
		NotController,
		/// Not a stash account.
		NotStash,
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
		NewRoundStart(RoundIndex, RoundIndex),
		EraPayout(EraIndex, BalanceOf<T>),
}

	/// In this implementation `new_session(session)` must be called before `end_session(session-1)`
	/// i.e. the new session must be planned before the ending of the previous session.
	///
	/// Once the first new_session is planned, all session must start and then end in order, though
	/// some session can lag in between the newest session planned and the latest session started.
	impl<T: Config> pallet_session::SessionManager<T::AccountId> for Pallet<T> {
		fn new_session(new_index: SessionIndex) -> Option<Vec<T::AccountId>> {
			log!(trace, "planning new_session({})", new_index);
			CurrentPlannedSession::<T>::put(new_index);
			Self::new_session(new_index)
		}
		fn start_session(start_index: SessionIndex) {
			log!(trace, "starting start_session({})", start_index);
			Self::start_session(start_index)
		}
		fn end_session(end_index: SessionIndex) {
			log!(trace, "ending end_session({})", end_index);
			Self::end_session(end_index)
		}
	}

	impl<T: Config> historical::SessionManager<T::AccountId, Exposure<T::AccountId, BalanceOf<T>>>
	for Pallet<T>
	{
		fn new_session(
			new_index: SessionIndex,
		) -> Option<Vec<(T::AccountId, Exposure<T::AccountId, BalanceOf<T>>)>> {
			<Self as pallet_session::SessionManager<_>>::new_session(new_index).map(|validators| {
				let current_era = Self::current_era()
					// Must be some as a new era has been created.
					.unwrap_or(0);

				validators.into_iter().map(|v| {
					let exposure = Self::eras_stakers(current_era, &v);
					(v, exposure)
				}).collect()
			})
		}
		fn start_session(start_index: SessionIndex) {
			<Self as pallet_session::SessionManager<_>>::start_session(start_index)
		}
		fn end_session(end_index: SessionIndex) {
			<Self as pallet_session::SessionManager<_>>::end_session(end_index)
		}
	}

	/// Add reward points to block authors:
	/// * 20 points to the block producer for producing a block in
	impl<T> pallet_authorship::EventHandler<T::AccountId, T::BlockNumber> for Pallet<T>
		where
			T: Config + pallet_authorship::Config + pallet_session::Config,
	{
		fn note_author(author: T::AccountId) {
			let now = <CurrentEra<T>>::get().unwrap_or(0);
			let score_plus_20 = <CollatorPoints<T>>::get(now, &author) + 20;
			<CollatorPoints<T>>::insert(now, author, score_plus_20);
			<TotalPoints<T>>::mutate(now, |x| *x += 20);
		}

		// just ignore it
		fn note_uncle(_author: T::AccountId, _age: T::BlockNumber) {

		}
	}

	/// A `Convert` implementation that finds the stash of the given controller account,
	/// if any.
	pub struct StashOf<T>(sp_std::marker::PhantomData<T>);

	impl<T: Config> Convert<T::AccountId, Option<T::AccountId>> for StashOf<T> {
		fn convert(controller: T::AccountId) -> Option<T::AccountId> {
			<Pallet<T>>::ledger(&controller).map(|l| l.stash)
		}
	}

	/// A typed conversion from stash account ID to the active exposure of nominators
	/// on that account.
	///
	/// Active exposure is the exposure of the validator set currently validating, i.e. in
	/// `active_era`. It can differ from the latest planned exposure in `current_era`.
	pub struct ExposureOf<T>(sp_std::marker::PhantomData<T>);

	impl<T: Config> Convert<T::AccountId, Option<Exposure<T::AccountId, BalanceOf<T>>>>
	for ExposureOf<T>
	{
		fn convert(validator: T::AccountId) -> Option<Exposure<T::AccountId, BalanceOf<T>>> {
			<Pallet<T>>::active_era()
				.map(|active_era| <Pallet<T>>::eras_stakers(active_era.index, &validator))
		}
	}
}
