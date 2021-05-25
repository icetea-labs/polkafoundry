use std::{io::Write, net::SocketAddr};

use sp_core::hexdisplay::HexDisplay;
use sp_runtime::traits::Block as BlockT;
use sp_runtime::AccountId32;

use sc_cli::{
	ChainSpec, CliConfiguration, DefaultConfigurationValues, ImportParams,
	KeystoreParams, NetworkParams, Result, RuntimeVersion, SharedParams, SubstrateCli,
};
use sc_service::{
	config::{BasePath, PrometheusConfig},
};

use codec::Encode;
use cumulus_client_service::genesis::generate_genesis_block;
use cumulus_primitives_core::ParaId;
use log::info;
use runtime_primitives::Block;
use polkadot_parachain::primitives::AccountIdConversion;

use crate::{cli::{Cli, RelayChainCli, Subcommand}, chain_spec};
use crate::service;
use crate::service::IdentifyVariant;
use crate::service::halongbay_runtime::AccountId;
use std::str::FromStr;

fn chain_name() -> String {
	#[cfg(feature = "polkafoundry")]
		return "PolkaFoundry".into();
	#[cfg(feature = "polkasmith")]
		return "PolkaSmith".into();
	#[cfg(feature = "halongbay")]
		return "Halongbay".into();
}

impl SubstrateCli for Cli {
	fn impl_name() -> String {
		format!("{} Parachain Collator", chain_name())
	}

	fn impl_version() -> String {
		env!("SUBSTRATE_CLI_IMPL_VERSION").into()
	}

	fn description() -> String {
		format!(
			"{} Parachain Collator\n\nThe command-line arguments provided first will be \
		passed to the parachain node, while the arguments provided after -- will be passed \
		to the relaychain node.\n\n\
		{} [parachain-args] -- [relaychain-args]",
			chain_name(),
			Self::executable_name()
		)
	}

	fn author() -> String {
		env!("CARGO_PKG_AUTHORS").into()
	}

	fn support_url() -> String {
		"https://github.com/polkafoundry/newpolka/issues/new".into()
	}

	fn copyright_start_year() -> i32 {
		2019
	}

	fn load_spec(&self, id: &str) -> std::result::Result<Box<dyn sc_service::ChainSpec>, String> {
		Ok(match id {
			#[cfg(feature = "polkafoundry")]
			"polkafoundry" => Box::new(chain_spec::polkafoundry::polkafoundry_config()?),
			#[cfg(feature = "polkafoundry")]
			"polkafoundry-dev" => Box::new(chain_spec::polkafoundry::polkafoundry_staging_testnet_config()?),
			#[cfg(feature = "polkasmith")]
			"polkasmith" => Box::new(chain_spec::polkasmith::polkasmith_config()?),
			#[cfg(feature = "polkasmith")]
			"polkasmith-dev" => Box::new(chain_spec::polkasmith::polkasmith_staging_testnet_config()?),
			#[cfg(feature = "halongbay")]
			"halongbay" => Box::new(chain_spec::halongbay::halongbay_config()?),
			#[cfg(feature = "halongbay")]
			"" => Box::new(chain_spec::halongbay::halongbay_staging_testnet_config()?),
			path => {
				let path = std::path::PathBuf::from(path);
				let starts_with = |prefix: &str| {
					path.file_name()
						.map(|f| f.to_str().map(|s| s.starts_with(&prefix)))
						.flatten()
						.unwrap_or(false)
				};
				if starts_with("polkafoundry") {
					#[cfg(feature = "polkafoundry")]
						{
							Box::new(chain_spec::polkafoundry::PolkaFoundryChainSpec::from_json_file(path)?)
						}

					#[cfg(not(feature = "polkafoundry"))]
						return Err("PolkaFoundry runtime is not available. Please compile the node with `--features polkafoundry` to enable it.".into());
				} else if starts_with("polkasmith") {
					#[cfg(feature = "polkasmith")]
						{
							Box::new(chain_spec::polkasmith::PolkaSmithChainSpec::from_json_file(path)?)
						}
					#[cfg(not(feature = "polkasmith"))]
						return Err("PolkaSmith runtime is not available. Please compile the node with `--features polkasmith` to enable it.".into());
				} else {
					#[cfg(feature = "halongbay")]
						{
							Box::new(chain_spec::halongbay::HalongbayChainSpec::from_json_file(path)?)
						}
					#[cfg(not(feature = "halongbay"))]
						return Err("Halongbay runtime is not available. Please compile the node with `--features halongbay` to enable it.".into());
				}
			},
		})
	}

	fn native_runtime_version(chain_spec: &Box<dyn ChainSpec>) -> &'static RuntimeVersion {
		if chain_spec.is_polkafoundry() {
			#[cfg(feature = "polkafoundry")]
				return &polkafoundry_runtime::VERSION;
			#[cfg(not(feature = "polkafoundry"))]
			panic!("PolkaFoundry runtime is not available. Please compile the node with `--features polkafoundry` to enable it.");
		} else if chain_spec.is_polkasmith() {
			#[cfg(feature = "polkasmith")]
				return &polkasmith_runtime::VERSION;
			#[cfg(not(feature = "polkasmith"))]
			panic!("PolkaSmith runtime is not available. Please compile the node with `--features polkasmith` to enable it.");
		} else {
			#[cfg(feature = "halongbay")]
				return &halongbay_runtime::VERSION;
			#[cfg(not(feature = "halongbay"))]
			panic!("Halongbay runtime is not available. Please compile the node with `--features halongbay` to enable it.");
		}
	}
}

impl SubstrateCli for RelayChainCli {
	fn impl_name() -> String {
		format!(
			"{} Parachain Collator",
			chain_name()
		)
	}

	fn impl_version() -> String {
		env!("SUBSTRATE_CLI_IMPL_VERSION").into()
	}

	fn description() -> String {
		format!(
			"{} Parachain Collator\n\nThe command-line arguments provided first will be \
			passed to the parachain node, while the arguments provided after -- will be passed \
			to the relaychain node.\n\n\
			rococo-collator [parachain-args] -- [relaychain-args]"
			,
			chain_name()
		)
	}

	fn author() -> String {
		env!("CARGO_PKG_AUTHORS").into()
	}

	fn support_url() -> String {
		"https://github.com/polkafoundry/newpolka/issues/new".into()
	}

	fn copyright_start_year() -> i32 {
		2019
	}

	fn load_spec(&self, id: &str) -> std::result::Result<Box<dyn sc_service::ChainSpec>, String> {
		polkadot_cli::Cli::from_iter([RelayChainCli::executable_name()].iter()).load_spec(id)
	}

	fn native_runtime_version(chain_spec: &Box<dyn ChainSpec>) -> &'static RuntimeVersion {
		polkadot_cli::Cli::native_runtime_version(chain_spec)
	}
}

fn extract_genesis_wasm(chain_spec: &Box<dyn sc_service::ChainSpec>) -> Result<Vec<u8>> {
	let mut storage = chain_spec.build_storage()?;

	storage
		.top
		.remove(sp_core::storage::well_known_keys::CODE)
		.ok_or_else(|| "Could not find wasm file in genesis state!".into())
}

/// Parse command line arguments into service configuration.
pub fn run() -> Result<()> {
	let cli = Cli::from_args();

	match &cli.subcommand {
		Some(Subcommand::BuildSpec(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.sync_run(|config| cmd.run(config.chain_spec, config.network))
		}
		Some(Subcommand::CheckBlock(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.async_run(|mut config| {
				let (client, _, import_queue, task_manager) = service::new_chain_ops(&mut config)?;
				Ok((cmd.run(client, import_queue), task_manager))
			})
		}
		Some(Subcommand::ExportBlocks(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.async_run(|mut config| {
				let (client, _, _, task_manager) = service::new_chain_ops(&mut config)?;
				Ok((cmd.run(client, config.database), task_manager))
			})
		}
		Some(Subcommand::ExportState(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.async_run(|mut config| {
				let (client, _, _, task_manager) = service::new_chain_ops(&mut config)?;
				Ok((cmd.run(client, config.chain_spec), task_manager))
			})
		}
		Some(Subcommand::ImportBlocks(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.async_run(|mut config| {
				let (client, _, import_queue, task_manager) = service::new_chain_ops(&mut config)?;
				Ok((cmd.run(client, import_queue), task_manager))
			})
		}
		Some(Subcommand::PurgeChain(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.sync_run(|config| cmd.run(config.database))
		}
		Some(Subcommand::Revert(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.async_run(|mut config| {
				let (client, backend, _, task_manager) = service::new_chain_ops(&mut config)?;
				Ok((cmd.run(client, backend), task_manager))
			})
		}
		Some(Subcommand::ExportGenesisState(params)) => {
			let mut builder = sc_cli::LoggerBuilder::new("");
			builder.with_profiling(sc_tracing::TracingReceiver::Log, "");
			let _ = builder.init();

			let block: Block = generate_genesis_block(&cli.load_spec(&params.chain.clone().unwrap_or_default())?)?;
			let raw_header = block.header().encode();
			let output_buf = if params.raw {
				raw_header
			} else {
				format!("0x{:?}", HexDisplay::from(&block.header().encode())).into_bytes()
			};

			if let Some(output) = &params.output {
				std::fs::write(output, output_buf)?;
			} else {
				std::io::stdout().write_all(&output_buf)?;
			}

			Ok(())
		}
		Some(Subcommand::ExportGenesisWasm(params)) => {
			let mut builder = sc_cli::LoggerBuilder::new("");
			builder.with_profiling(sc_tracing::TracingReceiver::Log, "");
			let _ = builder.init();

			let raw_wasm_blob =
				extract_genesis_wasm(&cli.load_spec(&params.chain.clone().unwrap_or_default())?)?;
			let output_buf = if params.raw {
				raw_wasm_blob
			} else {
				format!("0x{:?}", HexDisplay::from(&raw_wasm_blob)).into_bytes()
			};

			if let Some(output) = &params.output {
				std::fs::write(output, output_buf)?;
			} else {
				std::io::stdout().write_all(&output_buf)?;
			}

			Ok(())
		}
		None => {
			let runner = cli.create_runner(&*cli.run)?;
			let collator = cli.run.base.validator || cli.collator;
			let author_id: Option<AccountId32> = cli.run.author_id.clone();
			if collator && author_id.is_none() {
				return Err("Collator nodes must specify an author account id".into());
			}

			runner.run_node_until_exit(|config| async move {
				let key = sp_core::Pair::generate().0;
				if cli.run.start_dev {
					// If no author id was supplied, use the one that is staked at genesis
					// in the default development spec.
					let author_id = author_id.or_else(|| {
						Some(
							AccountId::from_str("ea8e9d3cfedc8afec25785703681d424e6aba10b728927b89d87a3776b47ee32")
								.expect("Tung is a valid account"),
						)
					});

					#[cfg(feature = "halongbay")]
						{
							return service::start_dev(
								config,
								author_id.unwrap(),
								cli.run.sealing,
								collator
							);
						}
					#[cfg(not(feature = "halongbay"))]
						return Err("Halongbay runtime is not available. Please compile the node with `--features halongbay` to enable it.".into());
				}

				let extension = chain_spec::Extensions::try_get(&*config.chain_spec);
				let relay_chain_id = extension.map(|e| e.relay_chain.clone());
				let para_id = extension.map(|e| e.para_id);

				let polkadot_cli = RelayChainCli::new(
					config.base_path.as_ref().map(|x| x.path().join("polkadot")),
					relay_chain_id,
					[RelayChainCli::executable_name().to_string()]
						.iter()
						.chain(cli.relaychain_args.iter()),
				);

				let id = ParaId::from(cli.run.parachain_id.or(para_id).unwrap_or(1111));

				let parachain_account =
					AccountIdConversion::<polkadot_primitives::v0::AccountId>::into_account(&id);

				let block: Block =
					generate_genesis_block(&config.chain_spec).map_err(|e| format!("Error generate genesis block {:?}", e))?;
				let genesis_state = format!("0x{:?}", HexDisplay::from(&block.header().encode()));

				let task_executor = config.task_executor.clone();
				let polkadot_config = SubstrateCli::create_configuration(
					&polkadot_cli,
					&polkadot_cli,
					task_executor,
				).map_err(|err| format!("Relay chain argument error: {}", err))?;

				info!("Parachain id: {:?}", id);
				info!("Parachain Account: {}", parachain_account);
				info!("Parachain genesis state: {}", genesis_state);
				info!("Is collating: {}", if collator { "yes" } else { "no" });
				info!("Runtime {:?}", cli.run.force_chain);

				match cli.run.force_chain {
					crate::cli::ForceChain::PolkaFoundry => {
						#[cfg(feature = "polkafoundry")]
							{
								return service::start_node::<service::polkafoundry_runtime::RuntimeApi, service::PolkaFoundryExecutor>(
									config,
									key,
									author_id,
									polkadot_config,
									id,
									cli.run.force_chain
								)
									.await
									.map(|r| r.0)
									.map_err(Into::into);
							}
						#[cfg(not(feature = "polkafoundry"))]
						panic!("PolkaFoundry runtime is not available. Please compile the node with `--features polkafoundry` to enable it.");
					}
					crate::cli::ForceChain::PolkaSmith => {
						#[cfg(feature = "polkasmith")]
							{
								return service::start_node::<service::polkasmith_runtime::RuntimeApi, service::PolkaSmithExecutor>(
									config,
									key,
									author_id,
									polkadot_config,
									id,
									cli.run.force_chain
								)
									.await
									.map(|r| r.0)
									.map_err(Into::into);

							}
						#[cfg(not(feature = "polkasmith"))]
						panic!("PolkaSmith runtime is not available. Please compile the node with `--features polkasmith` to enable it.");
					}
					crate::cli::ForceChain::Halongbay => {
						#[cfg(feature = "halongbay")]
							{
								return service::start_node::<service::halongbay_runtime::RuntimeApi, service::HalongbayExecutor>(
									config,
									key,
									author_id,
									polkadot_config,
									id,
									cli.run.force_chain
								)
									.await
									.map(|r| r.0)
									.map_err(Into::into)
							}
						#[cfg(not(feature = "halongbay"))]
						panic!("Halongbay runtime is not available. Please compile the node with `--features halongbay` to enable it.");
					}
				};
			}).map_err(Into::into)
		}
	}
}

impl DefaultConfigurationValues for RelayChainCli {
	fn p2p_listen_port() -> u16 {
		30334
	}

	fn rpc_ws_listen_port() -> u16 {
		9945
	}

	fn rpc_http_listen_port() -> u16 {
		9934
	}

	fn prometheus_listen_port() -> u16 {
		9616
	}
}

impl CliConfiguration<Self> for RelayChainCli {
	fn shared_params(&self) -> &SharedParams {
		self.base.base.shared_params()
	}

	fn import_params(&self) -> Option<&ImportParams> {
		self.base.base.import_params()
	}

	fn network_params(&self) -> Option<&NetworkParams> {
		self.base.base.network_params()
	}

	fn keystore_params(&self) -> Option<&KeystoreParams> {
		self.base.base.keystore_params()
	}

	fn base_path(&self) -> Result<Option<BasePath>> {
		Ok(self
			.shared_params()
			.base_path()
			.or_else(|| self.base_path.clone().map(Into::into)))
	}

	fn rpc_http(&self, default_listen_port: u16) -> Result<Option<SocketAddr>> {
		self.base.base.rpc_http(default_listen_port)
	}

	fn rpc_ipc(&self) -> Result<Option<String>> {
		self.base.base.rpc_ipc()
	}

	fn rpc_ws(&self, default_listen_port: u16) -> Result<Option<SocketAddr>> {
		self.base.base.rpc_ws(default_listen_port)
	}

	fn prometheus_config(&self, default_listen_port: u16) -> Result<Option<PrometheusConfig>> {
		self.base.base.prometheus_config(default_listen_port)
	}

	fn init<C: SubstrateCli>(&self) -> Result<()> {
		unreachable!("PolkadotCli is never initialized; qed");
	}

	fn chain_id(&self, is_dev: bool) -> Result<String> {
		let chain_id = self.base.base.chain_id(is_dev)?;

		Ok(if chain_id.is_empty() {
			self.chain_id.clone().unwrap_or_default()
		} else {
			chain_id
		})
	}

	fn role(&self, is_dev: bool) -> Result<sc_service::Role> {
		self.base.base.role(is_dev)
	}

	fn transaction_pool(&self) -> Result<sc_service::config::TransactionPoolOptions> {
		self.base.base.transaction_pool()
	}

	fn state_cache_child_ratio(&self) -> Result<Option<usize>> {
		self.base.base.state_cache_child_ratio()
	}

	fn rpc_methods(&self) -> Result<sc_service::config::RpcMethods> {
		self.base.base.rpc_methods()
	}

	fn rpc_ws_max_connections(&self) -> Result<Option<usize>> {
		self.base.base.rpc_ws_max_connections()
	}

	fn rpc_cors(&self, is_dev: bool) -> Result<Option<Vec<String>>> {
		self.base.base.rpc_cors(is_dev)
	}

	fn telemetry_external_transport(&self) -> Result<Option<sc_service::config::ExtTransport>> {
		self.base.base.telemetry_external_transport()
	}

	fn default_heap_pages(&self) -> Result<Option<u64>> {
		self.base.base.default_heap_pages()
	}

	fn force_authoring(&self) -> Result<bool> {
		self.base.base.force_authoring()
	}

	fn disable_grandpa(&self) -> Result<bool> {
		self.base.base.disable_grandpa()
	}

	fn max_runtime_instances(&self) -> Result<Option<usize>> {
		self.base.base.max_runtime_instances()
	}

	fn announce_block(&self) -> Result<bool> {
		self.base.base.announce_block()
	}
}
