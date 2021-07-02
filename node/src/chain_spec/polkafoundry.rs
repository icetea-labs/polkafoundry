use std::collections::BTreeMap;
use sc_service::ChainType;
use sc_chain_spec::{Properties};
use sp_core::{crypto::UncheckedInto};

use polkafoundry_runtime as polkafoundry;

use crate::chain_spec::{Extensions};
use hex_literal::hex;

/// The `ChainSpec` parametrised for the polkafoundry runtime.
pub type PolkaFoundryChainSpec = sc_service::GenericChainSpec<polkafoundry::GenesisConfig, Extensions>;


pub fn polkafoundry_config() -> Result<PolkaFoundryChainSpec, String> {
	PolkaFoundryChainSpec::from_json_bytes(&include_bytes!("../../res/polkafoundry.json")[..])
}

fn polkafoundry_staging_testnet_config_genesis(wasm_binary: &[u8]) -> polkafoundry::GenesisConfig {
	const ENDOWMENT: polkafoundry::Balance = 200_000_000 * polkafoundry::PKF;
	let endowed_accounts = vec![
		// 5HNFRkCYoriHQwuJbt5YgSwegRTxmSQRe51UKEEBWnUZuHf5
		hex!["ea8e9d3cfedc8afec25785703681d424e6aba10b728927b89d87a3776b47ee32"].into(),
	];

	polkafoundry::GenesisConfig {
		system: polkafoundry::SystemConfig {
			code: wasm_binary.to_vec(),
			changes_trie_config: Default::default(),
		},
		balances: polkafoundry::BalancesConfig {
			balances: endowed_accounts
				.iter()
				.cloned()
				.map(|k| (k, ENDOWMENT))
				.collect(),
		},
		sudo: polkafoundry::SudoConfig { key: endowed_accounts[0].clone() },
		parachain_info: polkafoundry::ParachainInfoConfig { parachain_id: 1111.into() },
		evm: polkafoundry::EVMConfig {
			accounts: BTreeMap::new(),
		},
		ethereum: polkafoundry::EthereumConfig {},
		aura: halongbay::AuraConfig {
			authorities: vec![hex!["e0c50f050110813fcd53ac4478256f3e0e438d93065f4bd0a19a043d93c7cf3c"]
				.unchecked_into()]
		},
		aura_ext: Default::default(),
	}
}

pub fn polkafoundry_staging_testnet_config() ->  Result<PolkafoundryChainSpec, String>  {
	let wasm_binary = polkafoundry::WASM_BINARY.ok_or("PolkaFoundry development wasm not available")?;
	let boot_nodes = vec![];

	Ok(PolkaFoundryChainSpec::from_genesis(
		"PolkaFoundry PC1",
		"polkafoundry_staging_testnet",
		ChainType::Local,
		move || polkafoundry_staging_testnet_config_genesis(wasm_binary),
		boot_nodes,
		None,
		None,
		chain_properties(),
		Extensions {
			relay_chain: "rococo-local".into(),
			para_id: 1111_u32.into(),
		},
	))
}

fn chain_properties() -> Option<Properties> {
	let mut p = Properties::new();

	p.insert("tokenSymbol".into(), "PKF".into());
	p.insert("tokenDecimals".into(), 18.into());
	p.insert("ss58Format".into(), 99.into());

	Some(p)
}
