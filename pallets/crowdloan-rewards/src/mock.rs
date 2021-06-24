use crate::{self as pallet_crowdloan_rewards, Config};
use frame_support::{construct_runtime, parameter_types, PalletId, assert_ok};

use sp_core::{H256};
use sp_io;
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup},
};
use sp_std::convert::{From};

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
	type OnSetCode = ();
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
	pub const CrowdloanPalletId: PalletId = PalletId(*b"Crowdloa");
}

impl Config for Test {
	type Event = Event;
	type PalletId = CrowdloanPalletId;
	type RewardCurrency = Balances;
	const TGE_RATE: u32 = 35;
	type RelayChainAccountId = [u8; 32];
}

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;
pub const INIT_BALANCE: u128 = 100_000_000;
pub const INIT_CONTRIBUTED: u128 = 5000;

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

		let mut ext = sp_io::TestExternalities::from(storage);
		ext.execute_with(|| {
			System::set_block_number(4);
			assert_ok!(Crowdloan::initialize_reward(
				Origin::root(),
				contributions.clone(),
				14
			));
			// mock: reward from block 5 to block 14 (10 blocks)
			System::set_block_number(5);
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

