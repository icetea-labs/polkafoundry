//! Service and ServiceFactory implementation. Specialized wrapper over substrate service.

use std::{sync::{Arc, Mutex}, cell::RefCell, collections::{HashMap, BTreeMap}};

use sp_core::{Pair, H256};
use sp_runtime::traits::BlakeTwo256;
use sp_trie::PrefixedMemoryDB;
use sp_inherents::{ProvideInherentData, InherentIdentifier, InherentData};
use sp_timestamp::InherentError;

use sc_service::{Configuration, PartialComponents, Role, TFullBackend, TFullClient, TaskManager};
use sc_executor::native_executor_instance;
use sc_telemetry::{Telemetry, TelemetryWorker, TelemetryWorkerHandle};
use sc_consensus_manual_seal::{run_manual_seal, EngineCommand, ManualSealParams};
use sc_client_api::{BlockchainEvents};
pub use sc_executor::NativeExecutor;

use cumulus_client_service::{
	prepare_node_config, start_collator, start_full_node, StartCollatorParams, StartFullNodeParams,
};
use cumulus_client_network::build_block_announce_validator;
use cumulus_client_consensus_relay_chain::{
	build_relay_chain_consensus, BuildRelayChainConsensusParams,
};
use cumulus_primitives_parachain_inherent::{ParachainInherentData, INHERENT_IDENTIFIER as PARACHAIN_INHERENT_IDENTIFIER};
use cumulus_primitives_core::PersistedValidationData;
use cumulus_test_relay_sproof_builder::RelayStateSproofBuilder;
use futures::{Stream, StreamExt};
use fc_rpc_core::types::{FilterPool, PendingTransactions};
use fc_consensus::FrontierBlockImport;
use polkafoundry_runtime::{self, opaque::Block, RuntimeApi, SLOT_DURATION};
use polkadot_primitives::v0::CollatorPair;

use crate::cli::Sealing;

// Our native executor instance.
native_executor_instance!(
	pub Executor,
	polkafoundry_runtime::api::dispatch,
	polkafoundry_runtime::native_version,
);

type FullClient = TFullClient<Block, RuntimeApi, Executor>;
type FullBackend = TFullBackend<Block>;

/// Provide a mock duration starting at 0 in millisecond for timestamp inherent.
/// Each call will increment timestamp by slot_duration making Aura think time has passed.
pub struct MockTimestampInherentDataProvider;

pub const INHERENT_IDENTIFIER: InherentIdentifier = *b"timstap0";

thread_local!(static TIMESTAMP: RefCell<u64> = RefCell::new(0));


impl ProvideInherentData for MockTimestampInherentDataProvider {
	fn inherent_identifier(&self) -> &'static InherentIdentifier {
		&INHERENT_IDENTIFIER
	}

	fn provide_inherent_data(
		&self,
		inherent_data: &mut InherentData,
	) -> Result<(), sp_inherents::Error> {
		TIMESTAMP.with(|x| {
			*x.borrow_mut() += SLOT_DURATION;
			inherent_data.put_data(INHERENT_IDENTIFIER, &*x.borrow())
		})
	}

	fn error_to_string(&self, error: &[u8]) -> Option<String> {
		InherentError::try_from(&INHERENT_IDENTIFIER, error).map(|e| format!("{:?}", e))
	}
}

pub struct MockParachainInherentDataProvider;

impl ProvideInherentData for MockParachainInherentDataProvider {
	fn inherent_identifier(&self) -> &'static InherentIdentifier {
		&PARACHAIN_INHERENT_IDENTIFIER
	}

	fn provide_inherent_data(
		&self,
		inherent_data: &mut InherentData,
	) -> Result<(), sp_inherents::Error> {
		let (relay_storage_root, proof) =
			RelayStateSproofBuilder::default().into_state_root_and_proof();

		let data = ParachainInherentData {
			validation_data: PersistedValidationData {
				parent_head: Default::default(),
				relay_parent_number: Default::default(),
				relay_parent_storage_root: relay_storage_root,
				max_pov_size: 0
			},
			relay_chain_state: proof,
			downward_messages: vec![],
			horizontal_messages: Default::default()
		};
		inherent_data.put_data(PARACHAIN_INHERENT_IDENTIFIER, &data )
	}

	fn error_to_string(&self, error: &[u8]) -> Option<String> {
		InherentError::try_from(&PARACHAIN_INHERENT_IDENTIFIER, error).map(|e| format!("{:?}", e))
	}
}

/// Starts a `ServiceBuilder` for a full service.
///
/// Use this macro if you don't actually need the full service, but just the builder in order to
/// be able to perform chain operations.
pub fn new_partial(
	config: &Configuration,
) -> Result<
	PartialComponents<
		FullClient,
		FullBackend,
		(),
		sp_consensus::import_queue::BasicQueue<Block, PrefixedMemoryDB<BlakeTwo256>>,
		sc_transaction_pool::FullPool<Block, FullClient>,
		(
			FrontierBlockImport<Block, Arc<FullClient>, FullClient>,
			PendingTransactions,
			Option<FilterPool>,
			Option<Telemetry>,
			Option<TelemetryWorkerHandle>,
		),
	>,
	sc_service::Error,
> {
	let inherent_data_providers = sp_inherents::InherentDataProviders::new();

	let telemetry = config.telemetry_endpoints.clone()
		.filter(|x| !x.is_empty())
		.map(|endpoints| -> Result<_, sc_telemetry::Error> {
			let worker = TelemetryWorker::new(16)?;
			let telemetry = worker.handle().new_telemetry(endpoints);
			Ok((worker, telemetry))
		})
		.transpose()?;

	let (client, backend, keystore_container, task_manager) =
		sc_service::new_full_parts::<Block, RuntimeApi, Executor>(&config, telemetry.as_ref().map(|(_, telemetry)| telemetry.handle()), )?;

	let client = Arc::new(client);

	let telemetry_worker_handle = telemetry
		.as_ref()
		.map(|(worker, _)| worker.handle());

	let telemetry = telemetry
		.map(|(worker, telemetry)| {
			task_manager.spawn_handle().spawn("telemetry", worker.run());
			telemetry
		});

	let registry = config.prometheus_registry();

	let transaction_pool = sc_transaction_pool::BasicPool::new_full(
		config.transaction_pool.clone(),
		config.role.is_authority().into(),
		config.prometheus_registry(),
		task_manager.spawn_handle(),
		client.clone(),
	);

	let pending_transactions: PendingTransactions = Some(Arc::new(Mutex::new(HashMap::new())));

	let filter_pool: Option<FilterPool> = Some(Arc::new(Mutex::new(BTreeMap::new())));

	let frontier_block_import = FrontierBlockImport::new(client.clone(), client.clone(), true);

	let import_queue = cumulus_client_consensus_relay_chain::import_queue(
		client.clone(),
		frontier_block_import.clone(),
		inherent_data_providers.clone(),
		&task_manager.spawn_essential_handle(),
		registry,
	)?;

	let params = PartialComponents {
		backend,
		client,
		import_queue,
		keystore_container,
		task_manager,
		transaction_pool,
		inherent_data_providers,
		select_chain: (),
		other: (frontier_block_import, pending_transactions, filter_pool, telemetry, telemetry_worker_handle),
	};

	Ok(params)
}

/// Start a node with the given parachain `Configuration` and relay chain `Configuration`.
///
/// This is the actual implementation that is abstract over the executor and the runtime api.
#[sc_tracing::logging::prefix_logs_with("Parachain")]
async fn start_node_impl<RB>(
	parachain_config: Configuration,
	collator_key: CollatorPair,
	polkadot_config: Configuration,
	id: polkadot_primitives::v0::Id,
	validator: bool,
	_rpc_ext_builder: RB,
) -> sc_service::error::Result<(TaskManager, Arc<TFullClient<Block, RuntimeApi, Executor>>)>
	where
		RB: Fn(
			Arc<TFullClient<Block, RuntimeApi, Executor>>,
		) -> jsonrpc_core::IoHandler<sc_rpc::Metadata>
		+ Send
		+ 'static,
{
	if matches!(parachain_config.role, Role::Light) {
		return Err("Light client not supported!".into());
	}

	let parachain_config = prepare_node_config(parachain_config);
	let params = new_partial(&parachain_config)?;
	params
		.inherent_data_providers
		.register_provider(sp_timestamp::InherentDataProvider)
		.unwrap();

	let (
		block_import,
		pending_transactions,
		filter_pool,
		mut telemetry,
		telemetry_worker_handle,
	) = params.other;

	let polkadot_full_node =
		cumulus_client_service::build_polkadot_full_node(polkadot_config, collator_key.clone(), telemetry_worker_handle).map_err(
			|e| match e {
				polkadot_service::Error::Sub(x) => x,
				s => format!("{}", s).into(),
			},
		)?;


	let client = params.client.clone();
	let backend = params.backend.clone();
	let block_announce_validator = build_block_announce_validator(
		polkadot_full_node.client.clone(),
		id,
		Box::new(polkadot_full_node.network.clone()),
		polkadot_full_node.backend.clone(),
	);

	let prometheus_registry = parachain_config.prometheus_registry().cloned();
	let transaction_pool = params.transaction_pool.clone();
	let mut task_manager = params.task_manager;
	let import_queue = params.import_queue;


	let (network, network_status_sinks, system_rpc_tx, start_network) =
		sc_service::build_network(sc_service::BuildNetworkParams {
			config: &parachain_config,
			client: client.clone(),
			transaction_pool: transaction_pool.clone(),
			spawn_handle: task_manager.spawn_handle(),
			import_queue,
			on_demand: None,
			block_announce_validator_builder: Some(Box::new(|_| block_announce_validator)),
		})?;

	let subscription_task_executor =
		sc_rpc::SubscriptionTaskExecutor::new(task_manager.spawn_handle());

	let rpc_extensions_builder = {
		let client = client.clone();
		let pool = transaction_pool.clone();
		let network = network.clone();
		let pending = pending_transactions.clone();
		let filter_pool = filter_pool.clone();
		Box::new(move |deny_unsafe, _| {
			let deps = crate::rpc::FullDeps {
				client: client.clone(),
				pool: pool.clone(),
				deny_unsafe,
				is_authority: validator,
				network: network.clone(),
				pending_transactions: pending.clone(),
				filter_pool: filter_pool.clone(),
				command_sink: None,
			};

			crate::rpc::create_full(deps, subscription_task_executor.clone())
		})
	};

	sc_service::spawn_tasks(sc_service::SpawnTasksParams {
		on_demand: None,
		remote_blockchain: None,
		rpc_extensions_builder,
		client: client.clone(),
		transaction_pool: transaction_pool.clone(),
		task_manager: &mut task_manager,
		config: parachain_config,
		keystore: params.keystore_container.sync_keystore(),
		backend: backend.clone(),
		network: network.clone(),
		network_status_sinks,
		system_rpc_tx,
		telemetry: telemetry.as_mut(),
	})?;
	// Spawn Frontier EthFilterApi maintenance task.
	if filter_pool.is_some() {
		// Each filter is allowed to stay in the pool for 100 blocks.
		const FILTER_RETAIN_THRESHOLD: u64 = 100;
		task_manager.spawn_essential_handle().spawn(
			"frontier-filter-pool",
			client.import_notification_stream().for_each(move |notification| {
				if let Ok(locked) = &mut filter_pool.clone().unwrap().lock() {
					let imported_number: u64 = notification.header.number as u64;
					for (k, v) in locked.clone().iter() {
						let lifespan_limit = v.at_block + FILTER_RETAIN_THRESHOLD;
						if lifespan_limit <= imported_number {
							locked.remove(&k);
						}
					}
				}
				futures::future::ready(())
			})
		);
	}

	// Spawn Frontier pending transactions maintenance task (as essential, otherwise we leak).
	if pending_transactions.is_some() {
		use fp_consensus::{FRONTIER_ENGINE_ID, ConsensusLog};
		use sp_runtime::generic::OpaqueDigestItemId;

		const TRANSACTION_RETAIN_THRESHOLD: u64 = 5;
		task_manager.spawn_essential_handle().spawn(
			"frontier-pending-transactions",
			client.import_notification_stream().for_each(move |notification| {

				if let Ok(locked) = &mut pending_transactions.clone().unwrap().lock() {
					// As pending transactions have a finite lifespan anyway
					// we can ignore MultiplePostRuntimeLogs error checks.
					let mut frontier_log: Option<_> = None;
					for log in notification.header.digest.logs {
						let log = log.try_to::<ConsensusLog>(OpaqueDigestItemId::Consensus(&FRONTIER_ENGINE_ID));
						if let Some(log) = log {
							frontier_log = Some(log);
						}
					}

					let imported_number: u64 = notification.header.number as u64;

					if let Some(ConsensusLog::EndBlock {
									block_hash: _, transaction_hashes,
								}) = frontier_log {
						// Retain all pending transactions that were not
						// processed in the current block.
						locked.retain(|&k, _| !transaction_hashes.contains(&k));
					}
					locked.retain(|_, v| {
						// Drop all the transactions that exceeded the given lifespan.
						let lifespan_limit = v.at_block + TRANSACTION_RETAIN_THRESHOLD;
						lifespan_limit > imported_number
					});
				}
				futures::future::ready(())
			})
		);
	}

	let announce_block = {
		let network = network.clone();
		Arc::new(move |hash, data| network.announce_block(hash, data))
	};

	if validator {
		let proposer_factory = sc_basic_authorship::ProposerFactory::with_proof_recording(
			task_manager.spawn_handle(),
			client.clone(),
			transaction_pool,
			prometheus_registry.as_ref(),
			telemetry.as_ref().map(|x| x.handle()),
		);
		let spawner = task_manager.spawn_handle();

		let parachain_consensus = build_relay_chain_consensus(BuildRelayChainConsensusParams {
			para_id: id,
			proposer_factory,
			inherent_data_providers: params.inherent_data_providers,
			block_import,
			relay_chain_client: polkadot_full_node.client.clone(),
			relay_chain_backend: polkadot_full_node.backend.clone(),
		});

		let params = StartCollatorParams {
			para_id: id,
			block_status: client.clone(),
			announce_block,
			client: client.clone(),
			task_manager: &mut task_manager,
			collator_key,
			spawner,
			backend,
			relay_chain_full_node: polkadot_full_node,
			parachain_consensus,
		};

		start_collator(params).await?;
	} else {
		let params = StartFullNodeParams {
			client: client.clone(),
			announce_block,
			task_manager: &mut task_manager,
			para_id: id,
			polkadot_full_node,
		};

		start_full_node(params)?;
	}

	start_network.start_network();

	Ok((task_manager, client))
}

/// Start a normal parachain node.
pub async fn start_node(
	parachain_config: Configuration,
	collator_key: CollatorPair,
	polkadot_config: Configuration,
	id: polkadot_primitives::v0::Id,
	validator: bool,
) -> sc_service::error::Result<(TaskManager, Arc<TFullClient<Block, RuntimeApi, Executor>>)> {
	start_node_impl(
		parachain_config,
		collator_key,
		polkadot_config,
		id,
		validator,
		|_| Default::default(),
	)
		.await
}

pub fn start_dev(
	config: Configuration,
	sealing: Sealing,
	validator: bool
) -> sc_service::error::Result<TaskManager> {
	let sc_service::PartialComponents {
		client,
		backend,
		mut task_manager,
		import_queue,
		keystore_container,
		select_chain: _,
		transaction_pool,
		inherent_data_providers,
		other: (
			block_import,
			pending_transactions,
			filter_pool,
			telemetry,
			_telemetry_worker_handle,
		),
	} = new_partial(&config)?;

	inherent_data_providers
		.register_provider(MockTimestampInherentDataProvider)
		.map_err(Into::into)
		.map_err(sp_consensus::error::Error::InherentData)?;

	inherent_data_providers
		.register_provider(MockParachainInherentDataProvider)
		.map_err(Into::into)
		.map_err(sp_consensus::error::Error::InherentData)?;

	let (network, network_status_sinks, system_rpc_tx, network_starter) =
		sc_service::build_network(sc_service::BuildNetworkParams {
			config: &config,
			client: client.clone(),
			transaction_pool: transaction_pool.clone(),
			spawn_handle: task_manager.spawn_handle(),
			import_queue,
			on_demand: None,
			block_announce_validator_builder: None,
		})?;
	let mut command_sink = None;

	if config.offchain_worker.enabled {
		sc_service::build_offchain_workers(
			&config, task_manager.spawn_handle(), client.clone(), network.clone(),
		);
	};
	let prometheus_registry = config.prometheus_registry().cloned();

	if validator {
		let env = sc_basic_authorship::ProposerFactory::new(
			task_manager.spawn_handle(),
			client.clone(),
			transaction_pool.clone(),
			prometheus_registry.as_ref(),
			telemetry.as_ref().map(|x| x.handle()),
		);
		let commands_stream: Box<dyn Stream<Item = EngineCommand<H256>> + Send + Sync + Unpin> =
			match sealing {
				Sealing::Instant => {
					Box::new(
						transaction_pool
							.pool()
							.validated_pool()
							.import_notification_stream()
							.map(|_| EngineCommand::SealNewBlock {
								create_empty: false,
								finalize: false,
								parent_hash: None,
								sender: None,
							}),
					)
				}
				Sealing::Manual => {
					let (sink, stream) = futures::channel::mpsc::channel(1000);
					// Keep a reference to the other end of the channel. It goes to the RPC.
					command_sink = Some(sink);
					Box::new(stream)
				}
			};

		let select_chain = sc_consensus::LongestChain::new(backend.clone());

		task_manager.spawn_essential_handle().spawn_blocking(
			"authorship_task",
			run_manual_seal(ManualSealParams {
				block_import,
				env,
				client: client.clone(),
				pool: transaction_pool.pool().clone(),
				commands_stream,
				select_chain,
				inherent_data_providers,
				consensus_data_provider: None,
			}),
		);
	};
	let subscription_task_executor =
		sc_rpc::SubscriptionTaskExecutor::new(task_manager.spawn_handle());

	let rpc_extensions_builder = {
		let client = client.clone();
		let pool = transaction_pool.clone();
		let network = network.clone();
		let pending = pending_transactions.clone();
		let filter_pool = filter_pool.clone();
		Box::new(move |deny_unsafe, _| {
			let deps = crate::rpc::FullDeps {
				client: client.clone(),
				pool: pool.clone(),
				deny_unsafe,
				is_authority: validator,
				network: network.clone(),
				pending_transactions: pending.clone(),
				filter_pool: filter_pool.clone(),
				command_sink: command_sink.clone(),
			};
			crate::rpc::create_full(deps, subscription_task_executor.clone())
		})
	};

	sc_service::spawn_tasks(sc_service::SpawnTasksParams {
		network: network.clone(),
		client: client.clone(),
		keystore: keystore_container.sync_keystore(),
		task_manager: &mut task_manager,
		transaction_pool: transaction_pool.clone(),
		rpc_extensions_builder,
		on_demand: None,
		remote_blockchain: None,
		backend, network_status_sinks, system_rpc_tx, config, telemetry: None,
	})?;

	// Spawn Frontier EthFilterApi maintenance task.
	if filter_pool.is_some() {
		// Each filter is allowed to stay in the pool for 100 blocks.
		const FILTER_RETAIN_THRESHOLD: u64 = 100;
		task_manager.spawn_essential_handle().spawn(
			"frontier-filter-pool",
			client.import_notification_stream().for_each(move |notification| {
				if let Ok(locked) = &mut filter_pool.clone().unwrap().lock() {
					let imported_number: u64 = notification.header.number as u64;
					for (k, v) in locked.clone().iter() {
						let lifespan_limit = v.at_block + FILTER_RETAIN_THRESHOLD;
						if lifespan_limit <= imported_number {
							locked.remove(&k);
						}
					}
				}
				futures::future::ready(())
			})
		);
	}

	// Spawn Frontier pending transactions maintenance task (as essential, otherwise we leak).
	if pending_transactions.is_some() {
		use fp_consensus::{FRONTIER_ENGINE_ID, ConsensusLog};
		use sp_runtime::generic::OpaqueDigestItemId;

		const TRANSACTION_RETAIN_THRESHOLD: u64 = 5;
		task_manager.spawn_essential_handle().spawn(
			"frontier-pending-transactions",
			client.import_notification_stream().for_each(move |notification| {

				if let Ok(locked) = &mut pending_transactions.clone().unwrap().lock() {
					// As pending transactions have a finite lifespan anyway
					// we can ignore MultiplePostRuntimeLogs error checks.
					let mut frontier_log: Option<_> = None;
					for log in notification.header.digest.logs {
						let log = log.try_to::<ConsensusLog>(OpaqueDigestItemId::Consensus(&FRONTIER_ENGINE_ID));
						if let Some(log) = log {
							frontier_log = Some(log);
						}
					}

					let imported_number: u64 = notification.header.number as u64;

					if let Some(ConsensusLog::EndBlock {
									block_hash: _, transaction_hashes,
								}) = frontier_log {
						// Retain all pending transactions that were not
						// processed in the current block.
						locked.retain(|&k, _| !transaction_hashes.contains(&k));
					}
					locked.retain(|_, v| {
						// Drop all the transactions that exceeded the given lifespan.
						let lifespan_limit = v.at_block + TRANSACTION_RETAIN_THRESHOLD;
						lifespan_limit > imported_number
					});
				}
				futures::future::ready(())
			})
		);
	}

	network_starter.start_network();

	log::info!("Polkafoundry dev ready");

	Ok(task_manager)
}
