//! A collection of node-specific RPC methods.
//! Substrate provides the `sc-rpc` crate, which defines the core RPC layer
//! used by Substrate nodes. This file extends those RPC definitions with
//! capabilities that are specific to this project's runtime configuration.

#![warn(missing_docs)]

use std::sync::Arc;

use fc_rpc_core::types::{PendingTransactions};
use polkafoundry_runtime::{opaque::Block, AccountId, Balance, Index, Hash, TransactionConverter};
use sc_client_api::{
	backend::{StorageProvider, Backend, StateBackend, AuxStore},
};
use sc_network::NetworkService;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::{Error as BlockChainError, HeaderMetadata, HeaderBackend};
use sp_block_builder::BlockBuilder;
use sp_runtime::traits::BlakeTwo256;
pub use sc_rpc_api::DenyUnsafe;
use sp_transaction_pool::TransactionPool;


/// Full client dependencies.
pub struct FullDeps<C, P> {
	/// The client instance to use.
	pub client: Arc<C>,
	/// Transaction pool instance.
	pub pool: Arc<P>,
	/// Whether to deny unsafe calls
	pub deny_unsafe: DenyUnsafe,
	/// The Node authority flag
	pub is_authority: bool,
	/// Network service
	pub network: Arc<NetworkService<Block, Hash>>,
	/// Ethereum pending transactions.
	pub pending_transactions: PendingTransactions,
}

/// Instantiate all full RPC extensions.
pub fn create_full<C, P, BE>(
	deps: FullDeps<C, P>,
) -> jsonrpc_core::IoHandler<sc_rpc::Metadata> where
	BE: Backend<Block> + 'static,
	BE::State: StateBackend<BlakeTwo256>,
	C: ProvideRuntimeApi<Block>,
	C: StorageProvider<Block, BE>,
	C: AuxStore,
	C: HeaderBackend<Block> + HeaderMetadata<Block, Error=BlockChainError> + 'static,
	C: Send + Sync + 'static,
	C::Api: substrate_frame_rpc_system::AccountNonceApi<Block, AccountId, Index>,
	C::Api: pallet_transaction_payment_rpc::TransactionPaymentRuntimeApi<Block, Balance>,
	C::Api: BlockBuilder<Block>,
	P: TransactionPool + 'static,
	C::Api: fp_rpc::EthereumRuntimeRPCApi<Block>,
	P: TransactionPool<Block=Block> + 'static,
{
	use substrate_frame_rpc_system::{FullSystem, SystemApi};
	use pallet_transaction_payment_rpc::{TransactionPayment, TransactionPaymentApi};
	use fc_rpc::{
		EthApi, EthApiServer, EthFilterApi, EthFilterApiServer, NetApi, NetApiServer,
		EthPubSubApi, EthPubSubApiServer, Web3Api, Web3ApiServer, EthDevSigner, EthSigner,
		HexEncodedIdProvider,
	};


	let mut io = jsonrpc_core::IoHandler::default();
	let FullDeps {
		client,
		pool,
		deny_unsafe,
		is_authority,
		network,
		pending_transactions,
	} = deps;

	io.extend_with(
		SystemApi::to_delegate(FullSystem::new(client.clone(), pool.clone(), deny_unsafe))
	);

	io.extend_with(
		TransactionPaymentApi::to_delegate(TransactionPayment::new(client.clone()))
	);

	// Extend this RPC with a custom API by using the following syntax.
	// `YourRpcStruct` should have a reference to a client, which is needed
	// to call into the runtime.
	// `io.extend_with(YourRpcTrait::to_delegate(YourRpcStruct::new(ReferenceToClient, ...)));`
	let signers = Vec::new();

	io.extend_with(
		EthApiServer::to_delegate(EthApi::new(
			client.clone(),
			pool.clone(),
			polkafoundry_runtime::TransactionConverter,
			network.clone(),
			pending_transactions.clone(),
			signers,
			is_authority,
		))
	);

	io.extend_with(
		Web3ApiServer::to_delegate(Web3Api::new(
			client.clone(),
		))
	);
	io
}
