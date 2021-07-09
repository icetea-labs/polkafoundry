//! A collection of node-specific RPC methods.

use std::{sync::Arc};
use std::collections::BTreeMap;

use sp_api::ProvideRuntimeApi;
use sp_transaction_pool::TransactionPool;
use sp_blockchain::{Error as BlockChainError, HeaderMetadata, HeaderBackend};
use sp_runtime::traits::BlakeTwo256;
use sp_block_builder::BlockBuilder;

use sc_rpc_api::DenyUnsafe;
use sc_client_api::{
	backend::{StorageProvider, Backend, StateBackend, AuxStore},
	client::BlockchainEvents
};
use sc_rpc::SubscriptionTaskExecutor;
use sc_network::NetworkService;
use sc_consensus_manual_seal::rpc::{ManualSeal, ManualSealApi};
use substrate_frame_rpc_system::{FullSystem, SystemApi};
use pallet_transaction_payment_rpc::{TransactionPayment, TransactionPaymentApi};
use pallet_ethereum::EthereumStorageSchema;

use fc_rpc::{
	EthApi, EthApiServer, EthFilterApi, EthFilterApiServer, EthPubSubApi, EthPubSubApiServer, HexEncodedIdProvider, NetApi, NetApiServer, OverrideHandle, RuntimeApiStorageOverride,
	SchemaV1Override, StorageOverride, Web3Api, Web3ApiServer,
};
use fc_rpc_core::types::{PendingTransactions, FilterPool};
use jsonrpc_pubsub::manager::SubscriptionManager;
use runtime_primitives::{Hash, AccountId, Index, Block, Balance};
use crate::cli;

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
	/// EthFilterApi pool.
	pub filter_pool: Option<FilterPool>,
	/// Manual seal command sink
	pub command_sink: Option<futures::channel::mpsc::Sender<sc_consensus_manual_seal::rpc::EngineCommand<Hash>>>,
	/// Frontier Backend.
	pub frontier_backend: Arc<fc_db::Backend<Block>>,
	/// Maximum number of logs in a query.
	pub max_past_logs: u32,
}

/// Instantiate all Full RPC extensions.
pub fn create_full<C, P, BE>(
	deps: FullDeps<C, P>,
	subscription_task_executor: SubscriptionTaskExecutor,
	runtime: Option<cli::ForceChain>
) -> jsonrpc_core::IoHandler<sc_rpc::Metadata> where
	BE: Backend<Block> + 'static,
	BE::State: StateBackend<BlakeTwo256>,
	C: ProvideRuntimeApi<Block> + StorageProvider<Block, BE> + AuxStore,
	C: BlockchainEvents<Block>,
	C: HeaderBackend<Block> + HeaderMetadata<Block, Error=BlockChainError>,
	C: Send + Sync + 'static,
	C::Api: substrate_frame_rpc_system::AccountNonceApi<Block, AccountId, Index>,
	C::Api: pkfp_oracle_rpc::OracleRuntimeApi<Block, AccountId, pkfp_primitives::CurrencyId, runtime_common::TimeStampedPrice>,
	C::Api: BlockBuilder<Block>,
	C::Api: pallet_transaction_payment_rpc::TransactionPaymentRuntimeApi<Block, Balance>,
	C::Api: fp_rpc::EthereumRuntimeRPCApi<Block>,
	P: TransactionPool<Block=Block> + 'static,
{
	use pkfp_oracle_rpc::{Oracle, OracleApi};

	let mut io = jsonrpc_core::IoHandler::default();
	let FullDeps {
		client,
		pool,
		deny_unsafe,
		is_authority,
		network,
		pending_transactions,
		filter_pool,
		command_sink,
		frontier_backend,
		max_past_logs,
	} = deps;

	let mut overrides_map = BTreeMap::new();
	overrides_map.insert(
		EthereumStorageSchema::V1,
		Box::new(SchemaV1Override::new(client.clone())) as Box<dyn StorageOverride<_> + Send + Sync>
	);

	let overrides = Arc::new(OverrideHandle {
		schemas: overrides_map,
		fallback: Box::new(RuntimeApiStorageOverride::new(client.clone())),
	});

	io.extend_with(
		SystemApi::to_delegate(FullSystem::new(client.clone(), pool.clone(), deny_unsafe))
	);
	io.extend_with(
		TransactionPaymentApi::to_delegate(TransactionPayment::new(client.clone()))
	);

	let signers = Vec::new();

	match runtime {
		Some(cli::ForceChain::PolkaFoundry) => {
			#[cfg(feature = "polkafoundry")]
				{
					io.extend_with(EthApiServer::to_delegate(EthApi::new(
						client.clone(),
						pool.clone(),
						polkafoundry_runtime::TransactionConverter,
						network.clone(),
						pending_transactions,
						signers,
						overrides.clone(),
						frontier_backend,
						is_authority,
						max_past_logs,
					)));
				}
			#[cfg(not(feature = "polkafoundry"))]
			panic!("PolkaFoundry runtime is not available. Please compile the node with `--features polkafoundry` to enable it.");
		}
		Some(cli::ForceChain::PolkaSmith) => {
			#[cfg(feature = "polkasmith")]
				{
					io.extend_with(EthApiServer::to_delegate(EthApi::new(
						client.clone(),
						pool.clone(),
						polkasmith_runtime::TransactionConverter,
						network.clone(),
						pending_transactions,
						signers,
						overrides.clone(),
						frontier_backend,
						is_authority,
						max_past_logs,
					)));
				}
			#[cfg(not(feature = "polkasmith"))]
			panic!("PolkaSmith runtime is not available. Please compile the node with `--features polkasmith` to enable it.");
		},
		_ => {
			#[cfg(feature = "halongbay")]
				{
					io.extend_with(OracleApi::to_delegate(Oracle::new(client.clone())));
					io.extend_with(EthApiServer::to_delegate(EthApi::new(
						client.clone(),
						pool.clone(),
						halongbay_runtime::TransactionConverter,
						network.clone(),
						pending_transactions,
						signers,
						overrides.clone(),
						frontier_backend,
						is_authority,
						max_past_logs,
					)));
				}
			#[cfg(not(feature = "halongbay"))]
			panic!("Halongbay runtime is not available. Please compile the node with `--features halongbay` to enable it.");
		}
	}

	if let Some(filter_pool) = filter_pool {
		io.extend_with(
			EthFilterApiServer::to_delegate(EthFilterApi::new(
				client.clone(),
				filter_pool.clone(),
				500 as usize, // max stored filters
				overrides.clone(),
				max_past_logs,
			))
		);
	}

	io.extend_with(
		NetApiServer::to_delegate(NetApi::new(
			client.clone(),
			network.clone(),
			// Whether to format the `peer_count` response as Hex (default) or not.
			true,
		))
	);

	io.extend_with(
		Web3ApiServer::to_delegate(Web3Api::new(
			client.clone(),
		))
	);

	io.extend_with(
		EthPubSubApiServer::to_delegate(EthPubSubApi::new(
			pool.clone(),
			client.clone(),
			network.clone(),
			SubscriptionManager::<HexEncodedIdProvider>::with_id_provider(
				HexEncodedIdProvider::default(),
				Arc::new(subscription_task_executor)
			),
			overrides
		))
	);

	match command_sink {
		Some(command_sink) => {
			io.extend_with(
				// We provide the rpc handler with the sending end of the channel to allow the rpc
				// send EngineCommands to the background block authorship task.
				ManualSealApi::to_delegate(ManualSeal::new(command_sink)),
			);
		}
		_ => {}
	}

	io
}
