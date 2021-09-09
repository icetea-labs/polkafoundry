//! Service and ServiceFactory implementation. Specialized wrapper over substrate service.

use std::{sync::{Arc, Mutex}, collections::{HashMap, BTreeMap}, time::Duration};

use sp_core::{H256};
use sp_runtime::traits::BlakeTwo256;
use sp_trie::PrefixedMemoryDB;
use sp_inherents::{InherentIdentifier, InherentData, InherentDataProvider};
use sp_timestamp::InherentError;
use sp_consensus::SlotData;
use sp_consensus_aura::sr25519::{AuthorityId as AuraId, AuthorityPair as AuraPair};
use sp_keystore::SyncCryptoStorePtr;
use substrate_prometheus_endpoint::Registry;

pub use sc_executor::NativeExecutionDispatch;
use sc_service::{Configuration, PartialComponents, Role, TFullBackend, TFullClient, TaskManager, BasePath};
use sc_executor::native_executor_instance;
use sc_telemetry::{Telemetry, TelemetryHandle, TelemetryWorker, TelemetryWorkerHandle};
use sc_consensus_manual_seal::{run_manual_seal, EngineCommand, ManualSealParams};
use sc_consensus::LongestChain;
use sc_client_api::{BlockchainEvents, ExecutorProvider};
use sc_network::NetworkService;
pub use sc_executor::NativeExecutor;
use crate::cli;
use crate::cli::Cli;

use cumulus_client_service::{
	prepare_node_config, start_collator, start_full_node, StartCollatorParams, StartFullNodeParams,
};
use cumulus_client_network::build_block_announce_validator;
use cumulus_primitives_parachain_inherent::{ParachainInherentData, INHERENT_IDENTIFIER as PARACHAIN_INHERENT_IDENTIFIER};
use cumulus_primitives_core::PersistedValidationData;
use cumulus_test_relay_sproof_builder::RelayStateSproofBuilder;
use futures::{Stream, StreamExt};

use fc_rpc_core::types::{FilterPool, PendingTransactions};
use fc_consensus::FrontierBlockImport;
use fc_rpc::EthTask;
use fc_mapping_sync::{MappingSyncWorker, SyncStrategy};

use runtime_primitives::{Block, Hash};
use cumulus_client_consensus_aura::{build_aura_consensus, BuildAuraConsensusParams, SlotProportion};
use cumulus_client_consensus_common::ParachainConsensus;
use cumulus_primitives_core::ParaId;

use crate::cli::Sealing;

use crate::client::*;
use sc_chain_spec::ChainSpec;
use sp_api::ConstructRuntimeApi;

#[cfg(feature = "polkafoundry")]
pub use polkafoundry_runtime;

#[cfg(feature = "polkasmith")]
pub use polkasmith_runtime;

#[cfg(feature = "halongbay")]
pub use halongbay_runtime;
use codec::{Decode};
// Our native executor instance.
#[cfg(feature = "polkafoundry")]
native_executor_instance!(
	pub PolkaFoundryExecutor,
	polkafoundry_runtime::api::dispatch,
	polkafoundry_runtime::native_version,
);

#[cfg(feature = "polkasmith")]
native_executor_instance!(
	pub PolkaSmithExecutor,
	polkasmith_runtime::api::dispatch,
	polkasmith_runtime::native_version,
);

#[cfg(feature = "halongbay")]
native_executor_instance!(
	pub HalongbayExecutor,
	halongbay_runtime::api::dispatch,
	halongbay_runtime::native_version,
);


pub trait IdentifyVariant {
	/// Returns if this is a configuration for the `PolkaFoundry` network.
	fn is_polkafoundry(&self) -> bool;

	/// Returns if this is a configuration for the `PolkaSmith` network.
	fn is_polkasmith(&self) -> bool;

	/// Returns if this is a configuration for the `Halongbay` network.
	fn is_halongbay(&self) -> bool;
}

impl IdentifyVariant for Box<dyn ChainSpec> {
	fn is_polkafoundry(&self) -> bool {
		self.id().starts_with("polkafoundry]") || self.id().starts_with("pkf")
	}
	fn is_polkasmith(&self) -> bool {
		self.id().starts_with("polkasmith") || self.id().starts_with("pks")
	}
	fn is_halongbay(&self) -> bool {
		self.id().starts_with("halongbay") || self.id().starts_with("hlb")
	}
}


pub type FullClient<RuntimeApi, Executor> = TFullClient<Block, RuntimeApi, Executor>;
pub type FullBackend = TFullBackend<Block>;

pub struct MockParachainInherentDataProvider;

#[async_trait::async_trait]
impl InherentDataProvider for MockParachainInherentDataProvider {
	fn provide_inherent_data(
		&self,
		inherent_data: &mut InherentData,
	) -> Result<(), sp_inherents::Error> {
		// Use the "sproof" (spoof proof) builder to build valid mock state root and proof.
		let (relay_storage_root, proof) =
			RelayStateSproofBuilder::default().into_state_root_and_proof();

		let data = ParachainInherentData {
			validation_data: PersistedValidationData {
				parent_head: Default::default(),
				relay_parent_storage_root: relay_storage_root,
				relay_parent_number: Default::default(),
				max_pov_size: Default::default(),
			},
			downward_messages: Default::default(),
			horizontal_messages: Default::default(),
			relay_chain_state: proof,
		};

		inherent_data.put_data(PARACHAIN_INHERENT_IDENTIFIER, &data)
	}

	async fn try_handle_error(
		&self,
		identifier: &InherentIdentifier,
		error: &[u8],
	) -> Option<Result<(), sp_inherents::Error>> {
		if *identifier != PARACHAIN_INHERENT_IDENTIFIER {
			return None;
		}

		let error = InherentError::decode(&mut &error[..]).ok()?;

		Some(Err(sp_inherents::Error::Application(Box::from(format!("{:?}", error)))))
	}
}

type MaybeSelectChain = Option<LongestChain<FullBackend, Block>>;


pub fn frontier_database_dir(config: &Configuration) -> std::path::PathBuf {
	let config_dir = config.base_path.as_ref()
		.map(|base_path| base_path.config_dir(config.chain_spec.id()))
		.unwrap_or_else(|| {
			BasePath::from_project("", "", "polkafoundry")
				.config_dir(config.chain_spec.id())
		});
	config_dir.join("frontier").join("db")
}

pub fn open_frontier_backend(config: &Configuration) -> Result<Arc<fc_db::Backend<Block>>, String> {
	Ok(Arc::new(fc_db::Backend::<Block>::new(&fc_db::DatabaseSettings {
		source: fc_db::DatabaseSettingsSrc::RocksDb {
			path: frontier_database_dir(&config),
			cache_size: 0,
		}
	})?))
}

/// Starts a `ServiceBuilder` for a full service.
///
/// Use this macro if you don't actually need the full service, but just the builder in order to
/// be able to perform chain operations.
pub fn new_partial<RuntimeApi, Executor>(
	config: &Configuration,
	dev: bool,
) -> Result<
	PartialComponents<
		FullClient<RuntimeApi, Executor>,
		FullBackend,
		MaybeSelectChain,
		sp_consensus::import_queue::BasicQueue<Block, PrefixedMemoryDB<BlakeTwo256>>,
		sc_transaction_pool::FullPool<Block, FullClient<RuntimeApi, Executor>>,
		(
			PendingTransactions,
			Option<FilterPool>,
			Option<Telemetry>,
			Option<TelemetryWorkerHandle>,
			Arc<fc_db::Backend<Block>>,
		),
	>,
	sc_service::Error,
>
	where
		RuntimeApi: ConstructRuntimeApi<Block, FullClient<RuntimeApi, Executor>> + Send + Sync + 'static,
		RuntimeApi::RuntimeApi: RuntimeApiCollection<StateBackend = sc_client_api::StateBackendFor<FullBackend, Block>>,
		RuntimeApi::RuntimeApi: sp_consensus_aura::AuraApi<Block, AuraId>,
		Executor: NativeExecutionDispatch + 'static,
{

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

	let transaction_pool = sc_transaction_pool::BasicPool::new_full(
		config.transaction_pool.clone(),
		config.role.is_authority().into(),
		config.prometheus_registry(),
		task_manager.spawn_essential_handle(),
		client.clone(),
	);

	let pending_transactions: PendingTransactions = Some(Arc::new(Mutex::new(HashMap::new())));

	let filter_pool: Option<FilterPool> = Some(Arc::new(Mutex::new(BTreeMap::new())));

	let frontier_backend = open_frontier_backend(config)?;

	let frontier_block_import =
		FrontierBlockImport::new(client.clone(), client.clone(), frontier_backend.clone());

	let slot_duration = cumulus_client_consensus_aura::slot_duration(&*client)?;

	let select_chain = if dev {
		Some(sc_consensus::LongestChain::new(backend.clone()))
	} else {
		None
	};

	let import_queue = if dev {
		sc_consensus_manual_seal::import_queue(
			Box::new(frontier_block_import.clone()),
			&task_manager.spawn_essential_handle(),
			config.prometheus_registry(),
		)
	} else {
		cumulus_client_consensus_aura::import_queue::<
			sp_consensus_aura::sr25519::AuthorityPair,
			_,
			_,
			_,
			_,
			_,
			_,
		>(cumulus_client_consensus_aura::ImportQueueParams {
			block_import: frontier_block_import.clone(),
			client: client.clone(),
			create_inherent_data_providers: move |_, _| async move {
				let time = sp_timestamp::InherentDataProvider::from_system_time();

				let slot =
					sp_consensus_aura::inherents::InherentDataProvider::from_timestamp_and_duration(
						*time,
						slot_duration.slot_duration(),
					);

				Ok((time, slot))
			},
			registry: config.prometheus_registry().clone(),
			can_author_with: sp_consensus::CanAuthorWithNativeVersion::new(client.executor().clone()),
			spawner: &task_manager.spawn_essential_handle(),
			telemetry: telemetry.as_ref().map(|telemetry| telemetry.handle()),
		})?
	};

	let params = PartialComponents {
		backend,
		client,
		import_queue,
		keystore_container,
		task_manager,
		transaction_pool,
		select_chain,
		other: (pending_transactions, filter_pool, telemetry, telemetry_worker_handle, frontier_backend),
	};

	Ok(params)
}

/// Start a node with the given parachain `Configuration` and relay chain `Configuration`.
///
/// This is the actual implementation that is abstract over the executor and the runtime api.
#[sc_tracing::logging::prefix_logs_with("Parachain")]
async fn start_node_impl<RB, RuntimeApi, Executor, BIC>(
	parachain_config: Configuration,
	polkadot_config: Configuration,
	id: ParaId,
	_rpc_ext_builder: RB,
	build_consensus: BIC,
	runtime: cli::ForceChain,
	cli: &Cli,
) -> sc_service::error::Result<(TaskManager, Arc<TFullClient<Block, RuntimeApi, Executor>>)>
	where
		RB: Fn(
			Arc<TFullClient<Block, RuntimeApi, Executor>>,
		) -> jsonrpc_core::IoHandler<sc_rpc::Metadata>
		+ Send
		+ 'static,
		RuntimeApi: ConstructRuntimeApi<Block, FullClient<RuntimeApi, Executor>> + Send + Sync + 'static,
		RuntimeApi::RuntimeApi: RuntimeApiCollection<StateBackend = sc_client_api::StateBackendFor<FullBackend, Block>>,
		RuntimeApi::RuntimeApi: sp_consensus_aura::AuraApi<Block, AuraId>,
		Executor: NativeExecutionDispatch + 'static,
		BIC: FnOnce(
			Arc<TFullClient<Block, RuntimeApi, Executor>>,
			Option<&Registry>,
			Option<TelemetryHandle>,
			&TaskManager,
			&polkadot_service::NewFull<polkadot_service::Client>,
			Arc<sc_transaction_pool::FullPool<Block, TFullClient<Block, RuntimeApi, Executor>>>,
			Arc<NetworkService<Block, Hash>>,
			SyncCryptoStorePtr,
			bool,
		) -> Result<Box<dyn ParachainConsensus<Block>>, sc_service::Error>,
{
	if matches!(parachain_config.role, Role::Light) {
		return Err("Light client not supported!".into());
	}

	let parachain_config = prepare_node_config(parachain_config);
	let params = new_partial::<RuntimeApi, Executor>(&parachain_config, false)?;
	let force_authoring = parachain_config.force_authoring;
	let validator = parachain_config.role.is_authority();

	let (
		pending_transactions,
		filter_pool,
		mut telemetry,
		telemetry_worker_handle,
		frontier_backend,
	) = params.other;

	let polkadot_full_node =
		cumulus_client_service::build_polkadot_full_node(
			polkadot_config,
			telemetry_worker_handle,
		)
			.map_err(
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
	let import_queue = cumulus_client_service::SharedImportQueue::new(params.import_queue);


	let (network, system_rpc_tx, start_network) =
		sc_service::build_network(sc_service::BuildNetworkParams {
			config: &parachain_config,
			client: client.clone(),
			transaction_pool: transaction_pool.clone(),
			spawn_handle: task_manager.spawn_handle(),
			import_queue: import_queue.clone(),
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
		let frontier_backend = frontier_backend.clone();
		let max_past_logs = cli.run.max_past_logs;

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
				frontier_backend: frontier_backend.clone(),
				max_past_logs
			};

			crate::rpc::create_full(deps, subscription_task_executor.clone(), Some(runtime.clone()))
		})
	};

	task_manager.spawn_essential_handle().spawn(
		"frontier-mapping-sync-worker",
		MappingSyncWorker::new(
			client.import_notification_stream(),
			Duration::new(6, 0),
			client.clone(),
			backend.clone(),
			frontier_backend.clone(),
			SyncStrategy::Parachain,
		).for_each(|()| futures::future::ready(()))
	);

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
		system_rpc_tx,
		telemetry: telemetry.as_mut(),
	})?;

	// Spawn Frontier EthFilterApi maintenance task.
	if let Some(filter_pool) = filter_pool {
		// Each filter is allowed to stay in the pool for 100 blocks.
		const FILTER_RETAIN_THRESHOLD: u64 = 100;
		task_manager.spawn_essential_handle().spawn(
			"frontier-filter-pool",
			EthTask::filter_pool_task(
				Arc::clone(&client),
				filter_pool,
				FILTER_RETAIN_THRESHOLD,
			)
		);
	}

	// Spawn Frontier pending transactions maintenance task (as essential, otherwise we leak).
	if let Some(pending_transactions) = pending_transactions {
		const TRANSACTION_RETAIN_THRESHOLD: u64 = 5;
		task_manager.spawn_essential_handle().spawn(
			"frontier-pending-transactions",
			EthTask::pending_transaction_task(
				Arc::clone(&client),
				pending_transactions,
				TRANSACTION_RETAIN_THRESHOLD,
			)
		);
	}

	let announce_block = {
		let network = network.clone();
		Arc::new(move |hash, data| network.announce_block(hash, data))
	};

	if validator {
		let spawner = task_manager.spawn_handle();

		let parachain_consensus = build_consensus(
			client.clone(),
			prometheus_registry.as_ref(),
			telemetry.as_ref().map(|t| t.handle()),
			&task_manager,
			&polkadot_full_node,
			transaction_pool,
			network,
			params.keystore_container.sync_keystore(),
			force_authoring,
		)?;

		let params = StartCollatorParams {
			para_id: id,
			block_status: client.clone(),
			announce_block,
			client: client.clone(),
			task_manager: &mut task_manager,
			relay_chain_full_node: polkadot_full_node,
			spawner,
			parachain_consensus,
			import_queue
		};

		start_collator(params).await?;
	} else {
		let params = StartFullNodeParams {
			client: client.clone(),
			announce_block,
			task_manager: &mut task_manager,
			para_id: id,
			relay_chain_full_node: polkadot_full_node
		};

		start_full_node(params)?;
	}

	start_network.start_network();

	Ok((task_manager, client))
}

/// Start a normal parachain node.
pub async fn start_node<RuntimeApi, Executor>(
	parachain_config: Configuration,
	polkadot_config: Configuration,
	id: ParaId,
	runtime: cli::ForceChain,
	cli: &Cli,
) -> sc_service::error::Result<(TaskManager, Arc<TFullClient<Block, RuntimeApi, Executor>>)>
	where
		RuntimeApi: ConstructRuntimeApi<Block, FullClient<RuntimeApi, Executor>> + Send + Sync + 'static,
		RuntimeApi::RuntimeApi: RuntimeApiCollection<StateBackend = sc_client_api::StateBackendFor<FullBackend, Block>>,
		RuntimeApi::RuntimeApi: sp_consensus_aura::AuraApi<Block, AuraId>,
		Executor: NativeExecutionDispatch + 'static,
{
	start_node_impl(parachain_config, polkadot_config, id, |_| Default::default(), |client,
																					prometheus_registry,
																					telemetry,
																					task_manager,
																					relay_chain_node,
																					transaction_pool,
																					sync_oracle,
																					keystore,
																					force_authoring| {
		let slot_duration = cumulus_client_consensus_aura::slot_duration(&*client)?;
		let proposer_factory = sc_basic_authorship::ProposerFactory::with_proof_recording(
			task_manager.spawn_handle(),
			client.clone(),
			transaction_pool,
			prometheus_registry,
			telemetry.clone(),
		);

		let relay_chain_backend = relay_chain_node.backend.clone();
		let relay_chain_client = relay_chain_node.client.clone();
		Ok(build_aura_consensus::<AuraPair, _, _, _, _, _, _, _, _, _>(
			BuildAuraConsensusParams {
				proposer_factory,
				create_inherent_data_providers: move |_, (relay_parent, validation_data)| {
					let parachain_inherent =
						cumulus_primitives_parachain_inherent::ParachainInherentData::create_at_with_client(
							relay_parent,
							&relay_chain_client,
							&*relay_chain_backend,
							&validation_data,
							id,
						);
					async move {
						let time = sp_timestamp::InherentDataProvider::from_system_time();

						let slot = sp_consensus_aura::inherents::InherentDataProvider::from_timestamp_and_duration(
							*time,
							slot_duration.slot_duration(),
						);

						let parachain_inherent = parachain_inherent.ok_or_else(|| {
							Box::<dyn std::error::Error + Send + Sync>::from("Failed to create parachain inherent")
						})?;

						Ok((time, slot, parachain_inherent))
					}
				},
				block_import: client.clone(),
				relay_chain_client: relay_chain_node.client.clone(),
				relay_chain_backend: relay_chain_node.backend.clone(),
				para_client: client,
				backoff_authoring_blocks: Option::<()>::None,
				sync_oracle,
				keystore,
				force_authoring,
				slot_duration,
				// We got around 500ms for proposing
				block_proposal_slot_portion: SlotProportion::new(1f32 / 24f32),
				// And a maximum of 750ms if slots are skipped
				max_block_proposal_slot_portion: Some(SlotProportion::new(1f32 / 16f32)),
				telemetry,
			},
		))
	}, runtime, cli)
		.await
}

pub fn start_dev(
	config: Configuration,
	sealing: Sealing,
	validator: bool,
	cli: &Cli,
) -> sc_service::error::Result<TaskManager> {
	let sc_service::PartialComponents {
		client,
		backend,
		mut task_manager,
		import_queue,
		keystore_container,
		select_chain: _,
		transaction_pool,
		other: (
			pending_transactions,
			filter_pool,
			telemetry,
			_telemetry_worker_handle,
			frontier_backend
		),
	} = new_partial::<halongbay_runtime::RuntimeApi, HalongbayExecutor>(&config, true)?;
	let import_queue = cumulus_client_service::SharedImportQueue::new(import_queue);

	let (network, system_rpc_tx, network_starter) =
		sc_service::build_network(sc_service::BuildNetworkParams {
			config: &config,
			client: client.clone(),
			transaction_pool: transaction_pool.clone(),
			spawn_handle: task_manager.spawn_handle(),
			import_queue: import_queue.clone(),
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
				block_import: client.clone(),
				env,
				client: client.clone(),
				pool: transaction_pool.pool().clone(),
				commands_stream,
				select_chain,
				consensus_data_provider: None,
				create_inherent_data_providers: move |_, _| async move {
					let time = sp_timestamp::InherentDataProvider::from_system_time();

					Ok((time, MockParachainInherentDataProvider))
				},
			}),
		);
	};
	let subscription_task_executor =
		sc_rpc::SubscriptionTaskExecutor::new(task_manager.spawn_handle());

	task_manager.spawn_essential_handle().spawn(
		"frontier-mapping-sync-worker",
		MappingSyncWorker::new(
			client.import_notification_stream(),
			Duration::new(6, 0),
			client.clone(),
			backend.clone(),
			frontier_backend.clone(),
			SyncStrategy::Parachain,
		).for_each(|()| futures::future::ready(()))
	);

	let rpc_extensions_builder = {
		let client = client.clone();
		let pool = transaction_pool.clone();
		let network = network.clone();
		let pending = pending_transactions.clone();
		let filter_pool = filter_pool.clone();
		let frontier_backend = frontier_backend.clone();
		let max_past_logs = cli.run.max_past_logs;

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
				frontier_backend: frontier_backend.clone(),
				max_past_logs
			};
			crate::rpc::create_full(deps, subscription_task_executor.clone(), None)
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
		backend,
		system_rpc_tx,
		config,
		telemetry: None,
	})?;

	// Spawn Frontier EthFilterApi maintenance task.
	if let Some(filter_pool) = filter_pool {
		// Each filter is allowed to stay in the pool for 100 blocks.
		const FILTER_RETAIN_THRESHOLD: u64 = 100;
		task_manager.spawn_essential_handle().spawn(
			"frontier-filter-pool",
			EthTask::filter_pool_task(
				Arc::clone(&client),
				filter_pool,
				FILTER_RETAIN_THRESHOLD,
			)
		);
	}

	// Spawn Frontier pending transactions maintenance task (as essential, otherwise we leak).
	if let Some(pending_transactions) = pending_transactions {
		const TRANSACTION_RETAIN_THRESHOLD: u64 = 5;
		task_manager.spawn_essential_handle().spawn(
			"frontier-pending-transactions",
			EthTask::pending_transaction_task(
				Arc::clone(&client),
				pending_transactions,
				TRANSACTION_RETAIN_THRESHOLD,
			)
		);
	}

	network_starter.start_network();

	log::info!("Polkafoundry dev ready");

	Ok(task_manager)
}

/// Builds a new object suitable for chain operations.
pub fn new_chain_ops(
	mut config: &mut Configuration,
) -> Result<
	(
		Arc<Client>,
		Arc<FullBackend>,
		sp_consensus::import_queue::BasicQueue<Block, PrefixedMemoryDB<BlakeTwo256>>,
		TaskManager,
	),
	sc_service::error::Error
>
{
	config.keystore = sc_service::config::KeystoreConfig::InMemory;
	if config.chain_spec.is_polkafoundry() {
		#[cfg(feature = "polkafoundry")]
			{
				let PartialComponents {
					client,
					backend,
					import_queue,
					task_manager,
					..
				} = new_partial::<polkafoundry_runtime::RuntimeApi, PolkaFoundryExecutor>(config, false)?;
				Ok((Arc::new(Client::PolkaFoundry(client)), backend, import_queue, task_manager))
			}
		#[cfg(not(feature = "polkafoundry"))]
			Err("Polkafoundry runtime is not available. Please compile the node with `--features polkafoundry` to enable it.".into())
	} else if config.chain_spec.is_polkasmith() {
		#[cfg(feature = "polkasmith")]
			{
				let PartialComponents {
					client,
					backend,
					import_queue,
					task_manager,
					..
				} = new_partial::<polkasmith_runtime::RuntimeApi, PolkaSmithExecutor>(config, false)?;
				Ok((Arc::new(Client::PolkaSmith(client)), backend, import_queue, task_manager))
			}
		#[cfg(not(feature = "polkasmith"))]
			Err("PolkaSmith runtime is not available. Please compile the node with `--features polkasmith` to enable it.".into())
	} else {
		#[cfg(feature = "halongbay")]
			{
				let PartialComponents {
					client,
					backend,
					import_queue,
					task_manager,
					..
				} = new_partial::<halongbay_runtime::RuntimeApi, HalongbayExecutor>(config, true)?;
				Ok((Arc::new(Client::Halongbay(client)), backend, import_queue, task_manager))
			}
		#[cfg(not(feature = "halongbay"))]
			Err("Halongbay runtime is not available. Please compile the node with `--features halongbay` to enable it.".into())
	}
}

