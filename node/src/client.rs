use std::sync::Arc;
use runtime_primitives::{Block, AccountId, Nonce, Balance, BlakeTwo256};
use sp_runtime::{
	generic::{BlockId, SignedBlock},
	traits::{Block as BlockT},
	Justifications
};
use sp_storage::{ChildInfo, PrefixedStorageKey, StorageData, StorageKey};
use sp_api::{NumberFor};
use sp_consensus::BlockStatus;
use sc_client_api::{KeyIterator};

#[derive(Clone)]
pub enum Client {
	#[cfg(feature = "polkafoundry")]
	PolkaFoundry(Arc<crate::service::FullClient<polkafoundry_runtime::RuntimeApi, crate::service::PolkaFoundryExecutor>>),
	#[cfg(feature = "polkasmith")]
	PolkaSmith(Arc<crate::service::FullClient<polkasmith_runtime::RuntimeApi, crate::service::PolkaSmithExecutor>>),
	#[cfg(feature = "halongbay")]
	Halongbay(Arc<crate::service::FullClient<halongbay_runtime::RuntimeApi, crate::service::HalongbayExecutor>>),
}

impl sc_client_api::UsageProvider<Block> for Client {
	fn usage_info(&self) -> sc_client_api::ClientInfo<Block> {
		match self {
			#[cfg(feature = "polkafoundry")]
			Self::PolkaFoundry(client) => client.usage_info(),
			#[cfg(feature = "polkasmith")]
			Self::PolkaSmith(client) => client.usage_info(),
			#[cfg(feature = "halongbay")]
			Self::Halongbay(client) => client.usage_info(),
		}
	}
}

impl sc_client_api::StorageProvider<Block, crate::service::FullBackend> for Client {
	fn storage(&self, id: &BlockId<Block>, key: &StorageKey) -> sp_blockchain::Result<Option<StorageData>> {
		match self {
			#[cfg(feature = "polkafoundry")]
			Self::PolkaFoundry(client) => client.storage(id, key),
			#[cfg(feature = "polkasmith")]
			Self::PolkaSmith(client) => client.storage(id, key),
			#[cfg(feature = "halongbay")]
			Self::Halongbay(client) => client.storage(id, key),
		}
	}

	fn storage_keys(&self, id: &BlockId<Block>, key_prefix: &StorageKey) -> sp_blockchain::Result<Vec<StorageKey>> {
		match self {
			#[cfg(feature = "polkafoundry")]
			Self::PolkaFoundry(client) => client.storage_keys(id, key_prefix),
			#[cfg(feature = "polkasmith")]
			Self::PolkaSmith(client) => client.storage_keys(id, key_prefix),
			#[cfg(feature = "halongbay")]
			Self::Halongbay(client) => client.storage_keys(id, key_prefix),
		}
	}

	fn storage_hash(
		&self,
		id: &BlockId<Block>,
		key: &StorageKey,
	) -> sp_blockchain::Result<Option<<Block as BlockT>::Hash>> {
		match self {
			#[cfg(feature = "polkafoundry")]
			Self::PolkaFoundry(client) => client.storage_hash(id, key),
			#[cfg(feature = "polkasmith")]
			Self::PolkaSmith(client) => client.storage_hash(id, key),
			#[cfg(feature = "halongbay")]
			Self::Halongbay(client) => client.storage_hash(id, key),
		}
	}

	fn storage_pairs(
		&self,
		id: &BlockId<Block>,
		key_prefix: &StorageKey,
	) -> sp_blockchain::Result<Vec<(StorageKey, StorageData)>> {
		match self {
			#[cfg(feature = "polkafoundry")]
			Self::PolkaFoundry(client) => client.storage_pairs(id, key_prefix),
			#[cfg(feature = "polkasmith")]
			Self::PolkaSmith(client) => client.storage_pairs(id, key_prefix),
			#[cfg(feature = "halongbay")]
			Self::Halongbay(client) => client.storage_pairs(id, key_prefix),
		}
	}

	fn storage_keys_iter<'a>(
		&self,
		id: &BlockId<Block>,
		prefix: Option<&'a StorageKey>,
		start_key: Option<&StorageKey>,
	) -> sp_blockchain::Result<KeyIterator<'a, <crate::service::FullBackend as sc_client_api::Backend<Block>>::State, Block>> {
		match self {
			#[cfg(feature = "polkafoundry")]
			Self::PolkaFoundry(client) => client.storage_keys_iter(id, prefix, start_key),
			#[cfg(feature = "polkasmith")]
			Self::PolkaSmith(client) => client.storage_keys_iter(id, prefix, start_key),
			#[cfg(feature = "halongbay")]
			Self::Halongbay(client) => client.storage_keys_iter(id, prefix, start_key),
		}
	}

	fn child_storage(
		&self,
		id: &BlockId<Block>,
		child_info: &ChildInfo,
		key: &StorageKey,
	) -> sp_blockchain::Result<Option<StorageData>> {
		match self {
			#[cfg(feature = "polkafoundry")]
			Self::PolkaFoundry(client) => client.child_storage(id, child_info, key),
			#[cfg(feature = "polkasmith")]
			Self::PolkaSmith(client) => client.child_storage(id, child_info, key),
			#[cfg(feature = "halongbay")]
			Self::Halongbay(client) => client.child_storage(id, child_info, key),
		}
	}

	fn child_storage_keys(
		&self,
		id: &BlockId<Block>,
		child_info: &ChildInfo,
		key_prefix: &StorageKey,
	) -> sp_blockchain::Result<Vec<StorageKey>> {
		match self {
			#[cfg(feature = "polkafoundry")]
			Self::PolkaFoundry(client) => client.child_storage_keys(id, child_info, key_prefix),
			#[cfg(feature = "polkasmith")]
			Self::PolkaSmith(client) => client.child_storage_keys(id, child_info, key_prefix),
			#[cfg(feature = "halongbay")]
			Self::Halongbay(client) => client.child_storage_keys(id, child_info, key_prefix),
		}
	}

	fn child_storage_hash(
		&self,
		id: &BlockId<Block>,
		child_info: &ChildInfo,
		key: &StorageKey,
	) -> sp_blockchain::Result<Option<<Block as BlockT>::Hash>> {
		match self {
			#[cfg(feature = "polkafoundry")]
			Self::PolkaFoundry(client) => client.child_storage_hash(id, child_info, key),
			#[cfg(feature = "polkasmith")]
			Self::PolkaSmith(client) => client.child_storage_hash(id, child_info, key),
			#[cfg(feature = "halongbay")]
			Self::Halongbay(client) => client.child_storage_hash(id, child_info, key),
		}
	}

	fn max_key_changes_range(
		&self,
		first: NumberFor<Block>,
		last: BlockId<Block>,
	) -> sp_blockchain::Result<Option<(NumberFor<Block>, BlockId<Block>)>> {
		match self {
			#[cfg(feature = "polkafoundry")]
			Self::PolkaFoundry(client) => client.max_key_changes_range(first, last),
			#[cfg(feature = "polkasmith")]
			Self::PolkaSmith(client) => client.max_key_changes_range(first, last),
			#[cfg(feature = "halongbay")]
			Self::Halongbay(client) => client.max_key_changes_range(first, last),
		}
	}

	fn key_changes(
		&self,
		first: NumberFor<Block>,
		last: BlockId<Block>,
		storage_key: Option<&PrefixedStorageKey>,
		key: &StorageKey,
	) -> sp_blockchain::Result<Vec<(NumberFor<Block>, u32)>> {
		match self {
			#[cfg(feature = "polkafoundry")]
			Self::PolkaFoundry(client) => client.key_changes(first, last, storage_key, key),
			#[cfg(feature = "polkasmith")]
			Self::PolkaSmith(client) => client.key_changes(first, last, storage_key, key),
			#[cfg(feature = "halongbay")]
			Self::Halongbay(client) => client.key_changes(first, last, storage_key, key),
		}
	}
}

impl sc_client_api::BlockBackend<Block> for Client {
	fn block_body(&self, id: &BlockId<Block>) -> sp_blockchain::Result<Option<Vec<<Block as BlockT>::Extrinsic>>> {
		match self {
			#[cfg(feature = "polkafoundry")]
			Self::PolkaFoundry(client) => client.block_body(id),
			#[cfg(feature = "polkasmith")]
			Self::PolkaSmith(client) => client.block_body(id),
			#[cfg(feature = "halongbay")]
			Self::Halongbay(client) => client.block_body(id),
		}
	}

	fn block_indexed_body(&self, id: &BlockId<Block>) -> sp_blockchain::Result<Option<Vec<Vec<u8>>>> {
		match self {
			#[cfg(feature = "polkafoundry")]
			Self::PolkaFoundry(client) => client.block_indexed_body(id),
			#[cfg(feature = "polkasmith")]
			Self::PolkaSmith(client) => client.block_indexed_body(id),
			#[cfg(feature = "halongbay")]
			Self::Halongbay(client) => client.block_indexed_body(id),
		}
	}

	fn block(&self, id: &BlockId<Block>) -> sp_blockchain::Result<Option<SignedBlock<Block>>> {
		match self {
			#[cfg(feature = "polkafoundry")]
			Self::PolkaFoundry(client) => client.block(id),
			#[cfg(feature = "polkasmith")]
			Self::PolkaSmith(client) => client.block(id),
			#[cfg(feature = "halongbay")]
			Self::Halongbay(client) => client.block(id),
		}
	}

	fn block_status(&self, id: &BlockId<Block>) -> sp_blockchain::Result<BlockStatus> {
		match self {
			#[cfg(feature = "polkafoundry")]
			Self::PolkaFoundry(client) => client.block_status(id),
			#[cfg(feature = "polkasmith")]
			Self::PolkaSmith(client) => client.block_status(id),
			#[cfg(feature = "halongbay")]
			Self::Halongbay(client) => client.block_status(id),
		}
	}

	fn justifications(&self, id: &BlockId<Block>) -> sp_blockchain::Result<Option<Justifications>> {
		match self {
			#[cfg(feature = "polkafoundry")]
			Self::PolkaFoundry(client) => client.justifications(id),
			#[cfg(feature = "polkasmith")]
			Self::PolkaSmith(client) => client.justifications(id),
			#[cfg(feature = "halongbay")]
			Self::Halongbay(client) => client.justifications(id),
		}
	}

	fn block_hash(&self, number: NumberFor<Block>) -> sp_blockchain::Result<Option<<Block as BlockT>::Hash>> {
		match self {
			#[cfg(feature = "polkafoundry")]
			Self::PolkaFoundry(client) => client.block_hash(number),
			#[cfg(feature = "polkasmith")]
			Self::PolkaSmith(client) => client.block_hash(number),
			#[cfg(feature = "halongbay")]
			Self::Halongbay(client) => client.block_hash(number),
		}
	}

	fn indexed_transaction(&self, hash: &<Block as BlockT>::Hash) -> sp_blockchain::Result<Option<Vec<u8>>> {
		match self {
			#[cfg(feature = "polkafoundry")]
			Self::PolkaFoundry(client) => client.indexed_transaction(hash),
			#[cfg(feature = "polkasmith")]
			Self::PolkaSmith(client) => client.indexed_transaction(hash),
			#[cfg(feature = "halongbay")]
			Self::Halongbay(client) => client.indexed_transaction(hash),
		}
	}

	fn has_indexed_transaction(&self, hash: &<Block as BlockT>::Hash) -> sp_blockchain::Result<bool> {
		match self {
			#[cfg(feature = "polkafoundry")]
			Self::PolkaFoundry(client) => client.has_indexed_transaction(hash),
			#[cfg(feature = "polkasmith")]
			Self::PolkaSmith(client) => client.has_indexed_transaction(hash),
			#[cfg(feature = "halongbay")]
			Self::Halongbay(client) => client.has_indexed_transaction(hash),
		}
	}
}


/// A set of APIs that polkadot-like runtimes must implement.
pub trait RuntimeApiCollection:
sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block>
+ sp_api::ApiExt<Block>
+ sp_block_builder::BlockBuilder<Block>
+ frame_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Nonce>
+ pallet_transaction_payment_rpc_runtime_api::TransactionPaymentApi<Block, Balance>
+ pkfp_oracle_rpc::OracleRuntimeApi<Block, pkfp_primitives::DataProviderId, AccountId, pkfp_primitives::CurrencyId, runtime_common::TimeStampedPrice>
+ sp_api::Metadata<Block>
+ sp_offchain::OffchainWorkerApi<Block>
+ sp_session::SessionKeys<Block>
+ fp_rpc::EthereumRuntimeRPCApi<Block>
+ cumulus_primitives_core::CollectCollationInfo<Block>
	where
		<Self as sp_api::ApiExt<Block>>::StateBackend: sp_api::StateBackend<BlakeTwo256>,
{}

impl<Api> RuntimeApiCollection for Api
	where
		Api: sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block>
		+ sp_api::ApiExt<Block>
		+ sp_block_builder::BlockBuilder<Block>
		+ frame_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Nonce>
		+ pallet_transaction_payment_rpc_runtime_api::TransactionPaymentApi<Block, Balance>
		+ pkfp_oracle_rpc::OracleRuntimeApi<Block, pkfp_primitives::DataProviderId, AccountId, pkfp_primitives::CurrencyId, runtime_common::TimeStampedPrice>
		+ sp_api::Metadata<Block>
		+ sp_offchain::OffchainWorkerApi<Block>
		+ sp_session::SessionKeys<Block>
		+ fp_rpc::EthereumRuntimeRPCApi<Block>
		+ cumulus_primitives_core::CollectCollationInfo<Block>,
		<Self as sp_api::ApiExt<Block>>::StateBackend: sp_api::StateBackend<BlakeTwo256>,
{}
