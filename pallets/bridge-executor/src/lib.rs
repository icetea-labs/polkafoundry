#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;
use frame_support::pallet;

#[pallet]
pub mod pallet {
	use frame_system::pallet_prelude::*;
	use frame_support::pallet_prelude::*;
	use frame_support::traits::{Currency, EnsureOrigin, ExistenceRequirement::AllowDeath, Get};
	use frame_support::{decl_error, decl_event, decl_module, dispatch::DispatchResult, ensure};
	use frame_system::{self as system, ensure_signed};

	use pallet_bridge as bridge;
	use sp_arithmetic::traits::SaturatedConversion;
	use sp_core::U256;
	use sp_std::prelude::*;

	type BalanceOf<T> = <<T as Config>::Currency as Currency<
		<T as frame_system::Config>::AccountId,
	>>::Balance;
	type ResourceId = bridge::ResourceId;

	#[pallet::pallet]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

	#[pallet::config]
	pub trait Config: frame_system::Config + bridge::Config {
		/// Overarching event type
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		/// Specifies the origin check provided by the bridge for calls that can only be called by the bridge pallet
		type BridgeOrigin: EnsureOrigin<Self::Origin, Success = Self::AccountId>;

		/// The currency mechanism.
		type Currency: Currency<Self::AccountId>;

		/// Ids can be defined by the runtime and passed in, perhaps from blake2b_128 hashes.
		type HashId: Get<ResourceId>;
		type NativeTokenId: Get<ResourceId>;
	}

	#[pallet::call]
	impl <T: Config> Pallet<T> {
		/// Transfers some amount of the native token to some recipient on a (whitelisted) destination chain.
		#[pallet::weight(195_000_000)]
		pub fn transfer_native(
			origin: OriginFor<T>,
			amount: BalanceOf<T>,
			recipient: Vec<u8>,
			dest_id: bridge::ChainId
		) -> DispatchResultWithPostInfo {
			let source = ensure_signed(origin)?;
			ensure!(<bridge::Module<T>>::chain_whitelisted(dest_id), Error::<T>::InvalidTransfer);
			let bridge_id = <bridge::Module<T>>::account_id();
			T::Currency::transfer(&source, &bridge_id, amount.into(), AllowDeath)?;

			let resource_id = T::NativeTokenId::get();
			<bridge::Module<T>>::transfer_fungible(dest_id, resource_id, recipient, U256::from(amount.saturated_into::<u128>()))
		}

	}

	#[pallet::event]
	#[pallet::generate_deposit(fn deposit_event)]
	pub enum Event<T: Config>{}

	#[pallet::error]
	pub enum Error<T>{
		InvalidTransfer
	}
}
