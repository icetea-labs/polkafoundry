use std::collections::BTreeMap;
use sc_service::ChainType;
use sp_core::{crypto::UncheckedInto};
use sc_chain_spec::{Properties};

use polkasmith_runtime as polkasmith;
use crate::chain_spec::{Extensions};
use hex_literal::hex;

/// The `ChainSpec` parametrised for the polkafoundry runtime.
pub type PolkaSmithChainSpec = sc_service::GenericChainSpec<polkasmith::GenesisConfig, Extensions>;

pub fn polkasmith_config() -> Result<PolkaSmithChainSpec, String> {
	PolkaSmithChainSpec::from_json_bytes(&include_bytes!("../../res/polkasmith.json")[..])
}

fn polkasmith_staging_testnet_config_genesis(wasm_binary: &[u8]) -> polkasmith::GenesisConfig {
	const ENDOWMENT: polkasmith::Balance = 70_000_000 * polkasmith::PKS;
	let endowed_accounts = vec![
		// 5GmjsxF8L3fhSNyS4nVRHQ5NzLAtZkKUARuWKJEVE88RdoMt
		hex!["d03ccd7399930a85aa99eacf62332488bc3fd78e2bf5e063e2b8197814334a0b"].into(),
	];

	polkasmith::GenesisConfig {
		frame_system: polkasmith::SystemConfig {
			code: wasm_binary.to_vec(),
			changes_trie_config: Default::default(),
		},
		pallet_balances: polkasmith::BalancesConfig {
			balances: endowed_accounts
				.iter()
				.cloned()
				.map(|k| (k, ENDOWMENT))
				.collect(),
		},
		pallet_sudo: polkasmith::SudoConfig { key: endowed_accounts[0].clone() },
		parachain_info: polkasmith::ParachainInfoConfig { parachain_id: 2009.into() },
		pallet_evm: polkasmith::EVMConfig {
			accounts: BTreeMap::new(),
		},
		pallet_ethereum: polkasmith::EthereumConfig {},
		pallet_aura: polkasmith::AuraConfig {
			authorities: vec![
				hex!["d03ccd7399930a85aa99eacf62332488bc3fd78e2bf5e063e2b8197814334a0b"]
				.unchecked_into(),
				hex!["e0c50f050110813fcd53ac4478256f3e0e438d93065f4bd0a19a043d93c7cf3c"]
				.unchecked_into(),
			]
		},
		cumulus_pallet_aura_ext: Default::default(),
	}
}

pub fn polkasmith_staging_testnet_config() ->  Result<PolkaSmithChainSpec, String>  {
	let wasm_binary = polkasmith::WASM_BINARY.ok_or("PolkaSmith development wasm not available")?;
	let boot_nodes = vec![];

	Ok(PolkaSmithChainSpec::from_genesis(
		"PolkaSmith PC1",
		"polkasmith_staging_testnet",
		ChainType::Local,
		move || polkasmith_staging_testnet_config_genesis(wasm_binary),
		boot_nodes,
		None,
		None,
		chain_properties(),
		Extensions {
			relay_chain: "kusama-local".into(),
			para_id: 2009u32.into(),
		},
	))
}

fn chain_properties() -> Option<Properties> {
	let mut p = Properties::new();

	p.insert("tokenSymbol".into(), "PKS".into());
	p.insert("tokenDecimals".into(), 18.into());
	p.insert("ss58Format".into(), 98.into());

	Some(p)
}
