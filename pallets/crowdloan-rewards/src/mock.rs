use crate::{self as pallet_crowdloan_rewards, Config};
use frame_support::{construct_runtime, parameter_types};
use sp_core::{ed25519, Pair, H256};
use sp_io;
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup},
};
use sp_std::convert::{From, TryInto};

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
	type AccountId = u64;
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

impl Config for Test {
	type Event = Event;
	type RewardCurrency = Balances;
	type RelayChainAccountId = [u8; 32];
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
		Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
		Crowdloan: pallet_crowdloan_rewards::{Pallet, Call, Storage, Event<T>},
		Utility: pallet_utility::{Pallet, Call, Storage, Event},
	}
);

pub struct ExtBuilder;

impl ExtBuilder {
	pub fn build(contributions: Vec<([u8; 32], u32)>) -> sp_io::TestExternalities {
		let mut storage = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();
		let mut ext = sp_io::TestExternalities::from(storage);
		ext.execute_with(|| {
			Crowdloan::initialize_reward(
				Origin::root(),
				contributions.clone(),
				10,
				10
			).unwrap();
			System::set_block_number(1)
		});

		ext
	}
}

pub(crate) fn get_ed25519_pairs(num: u32) -> Vec<ed25519::Pair> {
	let seed: u128 = 12345678901234567890123456789012;
	let mut pairs = Vec::new();
	for i in 0..num {
		pairs.push(ed25519::Pair::from_seed(
			(seed.clone() + i as u128)
				.to_string()
				.as_bytes()
				.try_into()
				.unwrap(),
		))
	}
	pairs
}

pub(crate) fn mock_test() -> sp_io::TestExternalities {
	let pairs = get_ed25519_pairs(3);
	ExtBuilder::build(vec![
		([1u8; 32].into(), 500),
		([2u8; 32].into(), 500),
		(pairs[0].public().into(), 500),
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

