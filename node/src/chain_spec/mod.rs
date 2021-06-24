use serde::{Deserialize, Serialize};
use sc_chain_spec::ChainSpecExtension;

#[cfg(feature = "polkafoundry")]
pub mod polkafoundry;
#[cfg(feature = "polkasmith")]
pub mod polkasmith;
#[cfg(feature = "halongbay")]
pub mod halongbay;

/// The extensions for the [`ChainSpec`].
#[derive(Default, Clone, Serialize, Deserialize, ChainSpecExtension)]
#[serde(deny_unknown_fields)]
pub struct Extensions {
	/// The relay chain of the Parachain.
	pub relay_chain: String,
	/// The id of the Parachain.
	pub para_id: u32,
}

impl Extensions {
	/// Try to get the extension from the given `ChainSpec`.
	pub fn try_get(chain_spec: &dyn sc_service::ChainSpec) -> Option<&Self> {
		sc_chain_spec::get_extension(chain_spec.extensions())
	}
}
