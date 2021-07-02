use crate::{self as pallet_crowdloan_rewards, Config, Error};
use frame_support::{construct_runtime, parameter_types, PalletId, assert_ok, assert_noop};

use sp_core::{H256};
use sp_io;
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup},
};
use sp_std::convert::{From};
use frame_support::traits::{GenesisBuild, Hooks};
use cumulus_primitives_core::{ParaId, PersistedValidationData};
use cumulus_test_relay_sproof_builder::RelayStateSproofBuilder;
use cumulus_primitives_parachain_inherent::ParachainInherentData;

pub type AccountId = u64;
pub type Balance = u128;

parameter_types! {
	pub const BlockHashCount: u64 = 250;
	pub BlockWeights: frame_system::limits::BlockWeights =
		frame_system::limits::BlockWeights::simple_max(1024);
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
	type OnSetCode = cumulus_pallet_parachain_system::ParachainSetCode<Self>;
	type SystemWeightInfo = ();
	type SS58Prefix = ();
}

parameter_types! {
	pub const ExistentialDeposit: u128 = 1;
}

impl pallet_balances::Config for Test {
	type MaxLocks = ();
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

parameter_types! {
	pub const ParachainId: ParaId = ParaId::new(200);
}
impl cumulus_pallet_parachain_system::Config for Test {
	type Event = Event;
	type OnValidationData = ();
	type SelfParaId = ParachainId;
	type OutboundXcmpMessageSource = ();
	type DmpMessageHandler = ();
	type ReservedDmpWeight = ();
	type XcmpMessageHandler = ();
	type ReservedXcmpWeight = ();
}

parameter_types! {
	pub const CrowdloanPalletId: PalletId = PalletId(*b"Crowdloa");
}

impl Config for Test {
	type Event = Event;
	type PalletId = CrowdloanPalletId;
	type RewardCurrency = Balances;
}

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;
pub const INIT_BALANCE: u128 = 100_000_000;
pub const INIT_CONTRIBUTED: u128 = 5000;
pub const MINIMUM_BALANCE: u128 = ExistentialDeposit::get();

construct_runtime!(
	pub enum Test where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
		Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
		Crowdloan: pallet_crowdloan_rewards::{Pallet, Call, Storage, Event<T>},
		Utility: pallet_utility::{Pallet, Call, Storage, Event},
		ParachainSystem: cumulus_pallet_parachain_system::{Pallet, Call, Storage, Inherent, Event<T>},
	}
);

pub struct ExtBuilder;

impl ExtBuilder {
	pub fn build(contributions: Vec<(u64, u128)>) -> sp_io::TestExternalities {
		let mut storage = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();
		// Provide some initial balances
		pallet_balances::GenesisConfig::<Test> {balances: vec![(Crowdloan::account_id(), INIT_BALANCE)]}
			.assimilate_storage(&mut storage)
			.unwrap();

		// mock: reward from block 4 to block 14
		pallet_crowdloan_rewards::GenesisConfig::<Test> {
			// reward_fund: INIT_BALANCE,
			start_block: 4,
			end_block: 14,
			tge_rate: 35,
		}.assimilate_storage(&mut storage)
			.unwrap();

		let mut ext = sp_io::TestExternalities::from(storage);
		ext.execute_with(|| {
			run_to_relay_chain_block(2);
			assert_noop!(
				Crowdloan::initialize_reward(
					Origin::root(),
					vec![
						(1u64, INIT_BALANCE + 1),
					]
				),
				Error::<Test>::InsufficientFunds
			);
			assert_ok!(Crowdloan::initialize_reward(
				Origin::root(),
				contributions.clone(),
			));
		});

		ext
	}
}

pub(crate) fn mock_test() -> sp_io::TestExternalities {
	ExtBuilder::build(vec![
		(1u64, INIT_CONTRIBUTED),
		(2u64, INIT_CONTRIBUTED),
		(3u64, INIT_CONTRIBUTED),
	])
}

pub(crate) fn events() -> Vec<super::Event<Test>> {
	System::events()
		.into_iter()
		.map(|r| r.event)
		.filter_map(|e| {
			if let Event::pallet_crowdloan_rewards(inner) = e {
				Some(inner)
			} else {
				None
			}
		})
		.collect::<Vec<_>>()
}

pub(crate) fn run_to_block(n: u64) {
	while System::block_number() < n {
		System::set_block_number(System::block_number() + 1);
	}
}

pub (crate) fn run_to_relay_chain_block(n: u64) {
	// clear state of relay chain
	let _ = ParachainSystem::on_initialize(n);
	run_to_block(n);

	let builder = RelayStateSproofBuilder::default();
	let (relay_parent_storage_root, relay_chain_state) = builder.into_state_root_and_proof();

	let vfp = PersistedValidationData {
		relay_parent_number: n as u32,
		relay_parent_storage_root,
		..Default::default()
	};

	let system_inherent_data = ParachainInherentData {
		validation_data: vfp.clone(),
		relay_chain_state,
		downward_messages: Default::default(),
		horizontal_messages: Default::default(),
	};

	assert_ok!(ParachainSystem::set_validation_data(Origin::none(), system_inherent_data));
}

