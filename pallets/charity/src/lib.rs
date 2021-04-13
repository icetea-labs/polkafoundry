//! A Simple Charity which holds and governs a pot of funds.
//!
//! The Charity has a pot of funds. The Pot is unique because unlike other token-holding accounts,
//! it is not controlled by a cryptographic keypair. Rather it belongs to the pallet itself.
//! Funds can be added to the pot in two ways:
//! * Anyone can make a donation through the `donate` extrinsic.
//! * An imablance can be absorbed from somewhere else in the runtime.
//! Funds can only be allocated by a root call to the `allocate` extrinsic/
#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::pallet;

pub use pallet::*;

//
// use frame_system::{self as system, ensure_root, ensure_signed};
// use sp_runtime::{ModuleId, traits::AccountIdConversion};
// use sp_std::prelude::*;

#[cfg(test)]
mod tests;

#[pallet]
pub mod pallet {
	use frame_support::{
		dispatch::fmt::Debug, pallet_prelude::*,
		traits::{Currency, ExistenceRequirement::AllowDeath},
	};
	use frame_system::pallet_prelude::*;
	use sp_runtime::{ModuleId, traits::AccountIdConversion};

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		type Currency: Currency<Self::AccountId>;
	}

	type BalanceOf<T> = <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;
	type NegativeImbalanceOf<T> =
	<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::NegativeImbalance;

	/// Hardcoded pallet ID; used to create the special Pot Account
	/// Must be exactly 8 characters long
	const PALLET_ID: ModuleId = ModuleId(*b"Charity!");

	#[pallet::pallet]
	#[pallet::generate_store(pub (super) trait Store)]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}


	#[pallet::event]
	#[pallet::generate_deposit(fn deposit_event)]
	pub enum Event<T: Config>
	{
		/// Donor has made a charitable donation to the charity
		DonationReceived(T::AccountId, BalanceOf<T>, BalanceOf<T>),
		/// An imbalance from elsewhere in the runtime has been absorbed by the Charity
		ImbalanceAbsorbed(BalanceOf<T>, BalanceOf<T>),
		/// Charity has allocated funds to a cause
		FundsAllocated(T::AccountId, BalanceOf<T>, BalanceOf<T>),
		/// For testing purposes, to impl From<()> for TestEvent to assign `()` to balances::Event
		NullEvent(u32), // u32 could be aliases as an error code for mocking setup
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {

		/// Donate some funds to the charity
		/// WARNING: for test net the weight is 0.
		#[pallet::weight(0)]
		fn donate(
			origin: OriginFor<T>,
			amount: BalanceOf<T>,
		) -> DispatchResult {
			let donor = ensure_signed(origin)?;

			T::Currency::transfer(&donor, &Self::account_id(), amount, AllowDeath)
				.map_err(|_| DispatchError::Other("Can't make donation"))?;

			Self::deposit_event(Event::DonationReceived(donor, amount, Self::pot()));
			Ok(())
		}

		/// Allocate the Charity's funds
		///
		/// Take funds from the Charity's pot and send them somewhere. This call requires root origin,
		/// which means it must come from a governance mechanism such as Substrate's Democracy pallet.
		/// WARNING: for test net the weight is 0.
		#[pallet::weight(0)]
		fn allocate(
			origin: OriginFor<T>,
			dest: T::AccountId,
			amount: BalanceOf<T>,
		) -> DispatchResult {
			ensure_root(origin)?;

			// Make the transfer requested
			T::Currency::transfer(
				&Self::account_id(),
				&dest,
				amount,
				AllowDeath,
			).map_err(|_| DispatchError::Other("Can't make allocation"))?;

			//TODO what about errors here??

			Self::deposit_event(Event::FundsAllocated(dest, amount, Self::pot()));
			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		/// The account ID that holds the Charity's funds
		pub fn account_id() -> T::AccountId {
			PALLET_ID.into_account()
		}

		/// The Charity's balance
		fn pot() -> BalanceOf<T> {
			T::Currency::free_balance(&Self::account_id())
		}
	}


	// decl_storage! {
	// 	trait Store for Module<T: Config> as SimpleTreasury {
	// 	// No storage items of our own, but we still need decl_storage to initialize the pot
	// }
	//
	// add_extra_genesis {
	// 	build(|_config| {
	// 		// Create the charity's pot of funds, and ensure it has the minimum required deposit
	// 		let _ = T::Currency::make_free_balance_be(
	// 			&<Module<T>>::account_id(),
	// 			T::Currency::minimum_balance(),
	// 		);
	// 	});
	// }}
}






