use crate::*;
use crate as staking;
use frame_support::{
	construct_runtime, parameter_types,
	traits::{GenesisBuild, Currency, OnFinalize, OnInitialize, Imbalance, OnUnbalanced, OneSessionHandler, Get},
};
use sp_io;
use sp_runtime::{
	Perbill,
	testing::{Header, TestXt, UintAuthorityId},
	traits::{BlakeTwo256, IdentityLookup, Zero},
};
use sp_std::convert::{From};
use sp_core::H256;
use frame_election_provider_support::onchain;
use std::{cell::RefCell, collections::HashSet};

pub type AccountId = u64;
pub type Balance = u128;
pub(crate) type BlockNumber = u64;

pub const INIT_TIMESTAMP: u64 = 30_000;
pub const BLOCK_TIME: u64 = 1000;

parameter_types! {
	pub const BlockHashCount: u64 = 250;
	pub BlockWeights: frame_system::limits::BlockWeights =
		frame_system::limits::BlockWeights::simple_max(1024);
	pub const MaxLocks: u32 = 1024;
	pub static Period: BlockNumber = 5;
	pub static SessionsPerEra: SessionIndex = 3;
	pub static Offset: BlockNumber = 0;
}

impl frame_system::Config for Test {
	type BaseCallFilter = ();
	type BlockWeights = ();
	type BlockLength = ();
	type Origin = Origin;
	type Index = u64;
	type Call = Call;
	type BlockNumber = u64;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
	type Event = Event;
	type BlockHashCount = BlockHashCount;
	type DbWeight = ();
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = pallet_balances::AccountData<Balance>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type OnSetCode = ();
	type SystemWeightInfo = ();
	type SS58Prefix = ();
}

parameter_types! {
	pub const ExistentialDeposit: u128 = 1;
}

impl pallet_balances::Config for Test {
	type MaxLocks = MaxLocks;
	type MaxReserves = ();
	type ReserveIdentifier = [u8; 8];
	type Balance = Balance;
	type Event = Event;
	type DustRemoval = ();
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = System;
	type WeightInfo = ();
}

impl pallet_utility::Config for Test {
	type Event = Event;
	type Call = Call;
	type WeightInfo = ();
}

impl onchain::Config for Test {
	type AccountId = u64;
	type BlockNumber = u64;
	type BlockWeights = BlockWeights;
	type Accuracy = Perbill;
	type DataProvider = Staking;
}

thread_local! {
	static SESSION: RefCell<(Vec<AccountId>, HashSet<AccountId>)> = RefCell::new(Default::default());
}
thread_local! {
	pub static REWARD_REMAINDER_UNBALANCED: RefCell<u128> = RefCell::new(0);
}

pub struct RewardRemainderMock;

impl OnUnbalanced<NegativeImbalanceOf<Test>> for RewardRemainderMock {
	fn on_nonzero_unbalanced(amount: NegativeImbalanceOf<Test>) {
		REWARD_REMAINDER_UNBALANCED.with(|v| {
			*v.borrow_mut() += amount.peek();
		});
		drop(amount);
	}
}

/// Another session handler struct to test on_disabled.
pub struct OtherSessionHandler;
impl OneSessionHandler<AccountId> for OtherSessionHandler {
	type Key = UintAuthorityId;

	fn on_genesis_session<'a, I: 'a>(_: I)
		where I: Iterator<Item=(&'a AccountId, Self::Key)>, AccountId: 'a {}

	fn on_new_session<'a, I: 'a>(_: bool, validators: I, _: I,)
		where I: Iterator<Item=(&'a AccountId, Self::Key)>, AccountId: 'a
	{
		SESSION.with(|x| {
			*x.borrow_mut() = (
				validators.map(|x| x.0.clone()).collect(),
				HashSet::new(),
			)
		});
	}

	fn on_disabled(validator_index: usize) {
		SESSION.with(|d| {
			let mut d = d.borrow_mut();
			let value = d.0[validator_index];
			d.1.insert(value);
		})
	}
}

impl sp_runtime::BoundToRuntimeAppPublic for OtherSessionHandler {
	type Public = UintAuthorityId;
}

parameter_types! {
	pub const DisabledValidatorsThreshold: Perbill = Perbill::from_percent(25);
	pub const BondingDuration: EraIndex = 3;
}
sp_runtime::impl_opaque_keys! {
	pub struct SessionKeys {
		pub other: OtherSessionHandler,
	}
}
impl pallet_session::Config for Test {
	type SessionManager = pallet_session::historical::NoteHistoricalRoot<Test, Staking>;
	type Keys = SessionKeys;
	type ShouldEndSession = pallet_session::PeriodicSessions<Period, Offset>;
	type SessionHandler = (OtherSessionHandler,);
	type Event = Event;
	type ValidatorId = AccountId;
	type ValidatorIdOf = crate::StashOf<Test>;
	type DisabledValidatorsThreshold = DisabledValidatorsThreshold;
	type NextSessionRotation = pallet_session::PeriodicSessions<Period, Offset>;
	type WeightInfo = ();
}

impl pallet_session::historical::Config for Test {
	type FullIdentification = crate::Exposure<AccountId, Balance>;
	type FullIdentificationOf = crate::ExposureOf<Test>;
}


parameter_types! {
	pub const MaxCollatorsPerNominator: u32 = 5;
	pub const MaxNominationsPerCollator: u32 = 2;
	pub const MinCollatorStake: u32 = 500;
	pub const MinNominatorStake: u32 = 100;
	pub const PayoutDuration: u32 = 2;
}

impl Config for Test {
	const MAX_COLLATORS_PER_NOMINATOR: u32 = 5u32;
	type Event = Event;
	type UnixTime = Timestamp;
	type Currency = Balances;
	type MaxNominationsPerCollator = MaxNominationsPerCollator;
	type MinCollatorStake = MinCollatorStake;
	type MinNominatorStake = MinNominatorStake;
	type PayoutDuration = PayoutDuration;
	type ElectionProvider = onchain::OnChainSequentialPhragmen<Self>;
	type CurrencyToVote = frame_support::traits::SaturatingCurrencyToVote;
	type SessionsPerEra = SessionsPerEra;
	type SessionInterface = Self;
	type BondingDuration = BondingDuration;
	type NextNewSession = Session;
	type RewardRemainder = RewardRemainderMock;
	type Slash = ();
}

parameter_types! {
	pub const MinimumPeriod: u64 = 5;
}
impl pallet_timestamp::Config for Test {
	type Moment = u64;
	type OnTimestampSet = ();
	type MinimumPeriod = MinimumPeriod;
	type WeightInfo = ();
}
parameter_types! {
	pub const UncleGenerations: u64 = 0;
}

impl pallet_authorship::Config for Test {
	type FindAuthor = ();
	type UncleGenerations = UncleGenerations;
	type FilterUncle = ();
	type EventHandler = Pallet<Test>;
}

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

construct_runtime!(
	pub enum Test where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
		Authorship: pallet_authorship::{Pallet, Call, Storage, Inherent},
		Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
		Timestamp: pallet_timestamp::{Pallet, Call, Storage, Inherent},
		Staking: staking::{Pallet, Call, Config<T>, Storage, Event<T>},
		Utility: pallet_utility::{Pallet, Call, Storage, Event},
		Session: pallet_session::{Pallet, Call, Storage, Event, Config<T>},
	}
);

pub struct ExtBuilder {
	collator_count: u32,
}

impl Default for ExtBuilder {
	fn default() -> Self {
		Self {
			collator_count: 2,
		}
	}
}

impl ExtBuilder {
	pub fn collator_count(mut self, count: u32) -> Self {
		self.collator_count = count;
		self
	}

	pub fn build(
		self,
		balances: Vec<(AccountId, Balance)>,
		stakers: Vec<(AccountId, AccountId, Balance)>,
	) -> sp_io::TestExternalities {
		let mut storage = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();
		pallet_balances::GenesisConfig::<Test> { balances }
			.assimilate_storage(&mut storage)
			.unwrap();

		staking::GenesisConfig::<Test> {
			stakers,
			collator_count: self.collator_count
		}.assimilate_storage(&mut storage)
			.unwrap();

		let validators = (0..1)
			.map(|x| ((x + 1) * 10 + 1) as AccountId)
			.collect::<Vec<_>>();

		let _ = pallet_session::GenesisConfig::<Test> {
			keys: validators.iter().map(|x| (
				*x,
				*x,
				SessionKeys { other: UintAuthorityId(*x as u64) }
			)).collect(),
		}.assimilate_storage(&mut storage);

		let mut ext = sp_io::TestExternalities::from(storage);
		ext.execute_with(|| {
			System::set_block_number(1);
			Session::on_initialize(1);
			Staking::on_initialize(1);
			Timestamp::set_timestamp(INIT_TIMESTAMP);
		});

		ext
	}
}

pub(crate) fn mock_test() -> sp_io::TestExternalities {
	ExtBuilder::default()
		.build(vec![
			(100, 2000),

			// This allows us to have a total_payout different from 0.
			(999, 1_000_000_000_000),
		], vec![
			(100, 101, 1000),
		])
}

pub(crate) fn three_collators_three_collators_count() -> sp_io::TestExternalities {
	ExtBuilder::default()
		.collator_count(3)
		.build(
			vec![
				(10, 2000),
				(20, 2000),
				(100, 2000),

				// This allows us to have a total_payout different from 0.
				(999, 1_000_000_000_000),
			],
			vec![
				(10, 11, 800),
				(20, 21, 900),
				(100, 101, 1000),
			]
		)
}

pub(crate) fn four_collators_two_collators_count() -> sp_io::TestExternalities {
	ExtBuilder::default()
		.collator_count(2)
		.build(
			vec![
				(10, 2000),
				(20, 2000),
				(30, 2000),
				(40, 2000),

				// This allows us to have a total_payout different from 0.
				(999, 1_000_000_000_000),
			],
			vec![
				(10, 11, 500),
				(20, 21, 500),
				(30, 31, 1000),
				(40, 41, 1000),
			]
		)
}

pub(crate) fn events() -> Vec<super::Event<Test>> {
	System::events()
		.into_iter()
		.map(|r| r.event)
		.filter_map(|e| {
			if let Event::Staking(inner) = e {
				Some(inner)
			} else {
				None
			}
		})
		.collect::<Vec<_>>()
}

/// Progress to the given block, triggering session and era changes as we progress.
///
/// This will finalize the previous block, initialize up to the given block, essentially simulating
/// a block import/propose process where we first initialize the block, then execute some stuff (not
/// in the function), and then finalize the block.
pub(crate) fn run_to_block(n: BlockNumber) {
	Staking::on_finalize(System::block_number());
	for b in (System::block_number() + 1)..=n {
		System::set_block_number(b);
		Session::on_initialize(b);
		Staking::on_initialize(b);
		Timestamp::set_timestamp(System::block_number() * BLOCK_TIME + INIT_TIMESTAMP);
		if b != n {
			Staking::on_finalize(System::block_number());
		}
	}
}

/// Time it takes to finish a session.
///
/// Note, if you see `time_per_session() - BLOCK_TIME`, it is fine. This is because we set the
/// timestamp after on_initialize, so the timestamp is always one block old.
pub(crate) fn time_per_session() -> u64 {
	Period::get() * BLOCK_TIME
}

/// Time it takes to finish an era.
///
/// Note, if you see `time_per_era() - BLOCK_TIME`, it is fine. This is because we set the
/// timestamp after on_initialize, so the timestamp is always one block old.
pub(crate) fn time_per_era() -> u64 {
	time_per_session() * SessionsPerEra::get() as u64
}

/// Time that will be calculated for the reward per era.
pub(crate) fn reward_time_per_era() -> u64 {
	time_per_era() - BLOCK_TIME
}

pub(crate) fn active_era() -> EraIndex {
	Staking::active_era().unwrap().index
}

pub(crate) fn current_era() -> EraIndex {
	Staking::current_era().unwrap()
}

pub(crate) fn balances(who: &AccountId) -> (Balance, Balance) {
	(Balances::free_balance(who), Balances::reserved_balance(who))
}

pub(crate) fn give_money(who: &AccountId, amount: Balance) {
	Balances::make_free_balance_be(who, amount);
}

/// Progresses from the current block number (whatever that may be) to the `P * session_index + 1`.
pub(crate) fn start_session(session_index: SessionIndex) {
	let end: u64 = if Offset::get().is_zero() {
		(session_index as u64) * Period::get()
	} else {
		Offset::get() + (session_index.saturating_sub(1) as u64) * Period::get()
	};
	run_to_block(end);
	// session must have progressed properly.
	assert_eq!(
		Session::current_index(),
		session_index,
		"current session index = {}, expected = {}",
		Session::current_index(),
		session_index,
	);
}

pub(crate) fn validator_controllers() -> Vec<AccountId> {
	Session::validators()
		.into_iter()
		.map(|s| Staking::bonded(&s).expect("no controller for validator"))
		.collect()
}

pub(crate) fn current_total_payout_for_duration(duration: u64) -> Balance {
	let payout = Staking::era_payout(
		Staking::eras_total_stake(active_era()),
		Balances::total_issuance(),
		duration,
	);

	assert!(payout > 0);
	payout
}

/// Progress until the given era.
pub(crate) fn start_active_era(era_index: EraIndex) {
	start_session((era_index * <SessionsPerEra as Get<u32>>::get()).into());
	assert_eq!(active_era(), era_index);
	// One way or another, current_era must have changed before the active era, so they must match
	// at this point.
	assert_eq!(current_era(), active_era());
}
