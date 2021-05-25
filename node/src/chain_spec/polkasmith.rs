use std::collections::BTreeMap;
use sc_service::ChainType;
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
	const ENDOWMENT: polkasmith::Balance = 200_000_000 * polkasmith::PKS;
	let endowed_accounts = vec![
		// 5HNFRkCYoriHQwuJbt5YgSwegRTxmSQRe51UKEEBWnUZuHf5
		hex!["ea8e9d3cfedc8afec25785703681d424e6aba10b728927b89d87a3776b47ee32"].into(),
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
		parachain_info: polkasmith::ParachainInfoConfig { parachain_id: 1111.into() },
		pallet_evm: polkasmith::EVMConfig {
			accounts: BTreeMap::new(),
		},
		pallet_ethereum: polkasmith::EthereumConfig {},
		pallet_aura: Default::default(),
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
			para_id: 2009_u32.into(),
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
