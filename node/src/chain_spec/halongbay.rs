use std::collections::BTreeMap;
use sc_service::ChainType;
use sc_chain_spec::{Properties};

use sp_core::{crypto::UncheckedInto};

use halongbay_runtime as halongbay;

use crate::chain_spec::{Extensions};
use hex_literal::hex;

/// The `ChainSpec` parametrised for the halongbay runtime.
pub type HalongbayChainSpec = sc_service::GenericChainSpec<halongbay::GenesisConfig, Extensions>;


pub fn halongbay_config() -> Result<HalongbayChainSpec, String> {
	HalongbayChainSpec::from_json_bytes(&include_bytes!("../../res/halongbay.json")[..])
}

fn halongbay_staging_testnet_config_genesis(wasm_binary: &[u8]) -> halongbay::GenesisConfig {
	const ENDOWMENT: halongbay::Balance = 200_000_000 * halongbay::HLB;
	let endowed_accounts = vec![
		// 5HNFRkCYoriHQwuJbt5YgSwegRTxmSQRe51UKEEBWnUZuHf5
		hex!["ea8e9d3cfedc8afec25785703681d424e6aba10b728927b89d87a3776b47ee32"].into(),
	];

	halongbay::GenesisConfig {
		frame_system: halongbay::SystemConfig {
			code: wasm_binary.to_vec(),
			changes_trie_config: Default::default(),
		},
		pallet_balances: halongbay::BalancesConfig {
			balances: endowed_accounts
				.iter()
				.cloned()
				.map(|k| (k, ENDOWMENT))
				.collect(),
		},
		pallet_sudo: halongbay::SudoConfig { key: endowed_accounts[0].clone() },
		parachain_info: halongbay::ParachainInfoConfig { parachain_id: 1111.into() },
		pallet_evm: halongbay::EVMConfig {
			accounts: BTreeMap::new(),
		},
		pallet_ethereum: halongbay::EthereumConfig {},
		pallet_aura: halongbay::AuraConfig {
			authorities: vec![hex!["ea8e9d3cfedc8afec25785703681d424e6aba10b728927b89d87a3776b47ee32"]
				.unchecked_into()]
		},
		cumulus_pallet_aura_ext: Default::default(),
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
			para_id: 1111_u32.into(),
		},
	))
}

fn chain_properties() -> Option<Properties> {
	let mut p = Properties::new();

	p.insert("tokenSymbol".into(), "HLB".into());
	p.insert("tokenDecimals".into(), 12.into());
	p.insert("ss58Format".into(), 42.into());

	Some(p)
}
