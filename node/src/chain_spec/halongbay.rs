use std::collections::BTreeMap;
use sc_service::ChainType;
use sc_chain_spec::{Properties};
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use pallet_im_online::sr25519::AuthorityId as ImOnlineId;
use runtime_primitives::AccountId;

use sp_runtime::Perbill;
use sp_core::{
	crypto::UncheckedInto,
};

use halongbay_runtime as halongbay;
use halongbay::{SessionKeys, StakerStatus};

use crate::chain_spec::{Extensions};
use hex_literal::hex;

/// The `ChainSpec` parametrised for the halongbay runtime.
pub type HalongbayChainSpec = sc_service::GenericChainSpec<halongbay::GenesisConfig, Extensions>;


pub fn halongbay_config() -> Result<HalongbayChainSpec, String> {
	HalongbayChainSpec::from_json_bytes(&include_bytes!("../../res/halongbay.json")[..])
}

/// Helper function to generate stash, controller and session key
pub fn authority_keys() -> (AccountId, AccountId, AuraId, ImOnlineId) {
	(
		hex!["ea8e9d3cfedc8afec25785703681d424e6aba10b728927b89d87a3776b47ee32"].into(),
		hex!["e0c50f050110813fcd53ac4478256f3e0e438d93065f4bd0a19a043d93c7cf3c"].into(),
		hex!["e0c50f050110813fcd53ac4478256f3e0e438d93065f4bd0a19a043d93c7cf3c"].unchecked_into(),
		hex!["e0c50f050110813fcd53ac4478256f3e0e438d93065f4bd0a19a043d93c7cf3c"].unchecked_into(),
	)
}


fn session_keys(aura: AuraId, im_online: ImOnlineId) -> SessionKeys {
	SessionKeys { aura, im_online }
}

fn halongbay_staging_testnet_config_genesis(wasm_binary: &[u8]) -> halongbay::GenesisConfig {
	const ENDOWMENT: halongbay::Balance = 200_000_000 * halongbay::HLB;
	const STASH: u128 = 1000 * halongbay::HLB;

	let endowed_accounts = vec![
		hex!["ea8e9d3cfedc8afec25785703681d424e6aba10b728927b89d87a3776b47ee32"].into(),
	];
	let authorities = vec![authority_keys()];
	let stakers = authorities
		.iter()
		.map(|x| {
			(
				x.0.clone(),
				x.1.clone(),
				STASH,
				StakerStatus::Validator,
			)
		})
		.collect::<Vec<_>>();


	halongbay::GenesisConfig {
		system: halongbay::SystemConfig {
			code: wasm_binary.to_vec(),
			changes_trie_config: Default::default(),
		},
		balances: halongbay::BalancesConfig {
			balances: endowed_accounts
				.iter()
				.cloned()
				.map(|k| (k, ENDOWMENT))
				.collect(),
		},
		sudo: halongbay::SudoConfig { key: endowed_accounts[0].clone() },
		parachain_info: halongbay::ParachainInfoConfig { parachain_id: 2018.into() },
		evm: halongbay::EVMConfig {
			accounts: BTreeMap::new(),
		},
		ethereum: halongbay::EthereumConfig {},
		staking: halongbay::StakingConfig {
			validator_count: 20,
			minimum_validator_count: 1,
			stakers,
			invulnerables: authorities.iter().map(|x| x.0.clone()).collect(),
			force_era: Default::default(),
			slash_reward_fraction: Perbill::from_percent(10),
			..Default::default()
		},
		session: halongbay::SessionConfig {
			keys: authorities
				.iter()
				.map(|x| {
					(
						x.0.clone(),
						x.0.clone(),
						session_keys(x.2.clone(), x.3.clone()),
					)
				})
				.collect::<Vec<_>>(),
		},
		// no need to pass anything to aura, in fact it will panic if we do. Session will take care
		// of this.
		aura: Default::default(),
		aura_ext: Default::default(),
		im_online: Default::default(),
		treasury: Default::default(),
	}
}

pub fn halongbay_staging_testnet_config() ->  Result<HalongbayChainSpec, String>  {
	let wasm_binary = halongbay::WASM_BINARY.ok_or("Halongbay development wasm not available")?;
	let boot_nodes = vec![];

	Ok(HalongbayChainSpec::from_genesis(
		"Halongbay PC1",
		"halongbay_staging_testnet",
		ChainType::Local,
		move || halongbay_staging_testnet_config_genesis(wasm_binary),
		boot_nodes,
		None,
		None,
		chain_properties(),
		Extensions {
			relay_chain: "rococo-local".into(),
			para_id: 2018_u32.into(),
		},
	))
}

fn chain_properties() -> Option<Properties> {
	let mut p = Properties::new();

	p.insert("tokenSymbol".into(), "HLB".into());
	p.insert("tokenDecimals".into(), 18.into());
	p.insert("ss58Format".into(), 42.into());

	Some(p)
}
