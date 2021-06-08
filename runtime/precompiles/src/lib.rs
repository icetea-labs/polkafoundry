#![cfg_attr(not(feature = "std"), no_std)]

use pallet_evm_precompile_simple::{ECRecover, Identity, Ripemd160, Sha256};
use pallet_evm_precompile_dispatch::Dispatch;
use pallet_evm_precompile_modexp::Modexp;
use pallet_evm_precompile_sha3fips::Sha3FIPS256;
use pallet_evm_precompile_sha3fips::Sha3FIPS512;
// TODO: Make other precompiles work ...
// https://ethereum.stackexchange.com/questions/15479/list-of-pre-compiled-contracts

pub type PolkafoundryPrecompiles<Runtime> = (
	ECRecover,
	Sha256,
	Ripemd160,
	Identity,
	Modexp,
	Dispatch<Runtime>,
	Sha3FIPS256,
	Sha3FIPS512,
);
