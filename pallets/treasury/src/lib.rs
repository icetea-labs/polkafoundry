#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::pallet;
pub use pallet::*;

#[cfg(test)]
pub(crate) mod mock;
#[cfg(test)]
mod tests;

#[pallet]
pub mod pallet {
    use codec::{Decode, Encode};
    use frame_support::dispatch::DispatchResultWithPostInfo;
    use frame_support::storage::types::{StorageMap, StorageValue, ValueQuery};
    use frame_support::traits::{Currency, ExistenceRequirement::AllowDeath, Get, Hooks, IsType};
    use frame_support::{pallet_prelude::*, Blake2_128Concat, PalletId};
    use frame_system::pallet_prelude::*;
    use frame_system::{ensure_root, ensure_signed};
    use sp_runtime::traits::{AccountIdConversion, Saturating};
    use sp_runtime::RuntimeDebug;
    use sp_std::prelude::*;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// The treasury's module id, used for deriving its sovereign account ID.
        type PalletId: Get<PalletId>;

        /// The staking balance.
        type Currency: Currency<Self::AccountId>;

        /// The overarching event type.
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
    }

    /// An index of a proposal. Just a `u32`.
    pub type ProposalIndex = u32;
    pub type BalanceOf<T> =
        <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

    /// A spending proposal.
    #[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
    #[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug)]
    pub struct Proposal<T: Config> {
        /// The account to whom the payment should be made if the proposal is accepted.
        user: T::AccountId,
        /// The (total) amount that should be paid if the proposal is accepted.
        amount: BalanceOf<T>,
    }

    /// Error for the treasury module.
    #[pallet::error]
    pub enum Error<T> {
        /// Balance is too low.
        InsufficientBalance,
        /// The amount is lower than the minimum balance
        ScantyAmount,
        /// Donation is not successful
        FailedDonation,
        /// Allocation is not successful
        FailedAllocation,
    }

    #[pallet::event]
    #[pallet::generate_deposit(fn deposit_event)]
    pub enum Event<T: Config> {
        /// Donor has made a donation to the Treasury
        DonationReceived(T::AccountId, BalanceOf<T>, BalanceOf<T>),
        /// Treasury has allocated funds to a cause
        FundsAllocated(T::AccountId, BalanceOf<T>, BalanceOf<T>),
    }

    #[pallet::pallet]
    pub struct Pallet<T>(PhantomData<T>);

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Donate some funds to the Treasury
        #[pallet::weight(0)]
        pub fn donate(origin: OriginFor<T>, amount: BalanceOf<T>) -> DispatchResultWithPostInfo {
            let sender = ensure_signed(origin)?;

            ensure!(
                amount >= T::Currency::minimum_balance(),
                Error::<T>::ScantyAmount,
            );

            T::Currency::transfer(&sender, &Self::account_id(), amount, AllowDeath)
                .map_err(|_| Error::<T>::FailedDonation)?;
            Self::deposit_event(Event::DonationReceived(sender, amount, Self::pot()));

            Ok(Default::default())
        }

        /// Allocate the Treasury's pot
        ///
        /// Take funds from the Treasury's pot and send them somewhere. This call requires root origin,
        /// which means it must come from a governance mechanism such as Substrate's Democracy pallet.
        #[pallet::weight(0)]
        pub fn allocate(
            origin: OriginFor<T>,
            dest: T::AccountId,
            amount: BalanceOf<T>,
        ) -> DispatchResultWithPostInfo {
            ensure_root(origin)?;

            ensure!(
                amount >= T::Currency::minimum_balance(),
                Error::<T>::ScantyAmount,
            );

            T::Currency::transfer(&Self::account_id(), &dest, amount, AllowDeath)
                .map_err(|_| Error::<T>::FailedAllocation)?;

            Self::deposit_event(Event::FundsAllocated(dest, amount, Self::pot()));
            Ok(Default::default())
        }
    }

    #[pallet::extra_constants]
    impl<T: Config> Pallet<T> {
        /// The account ID of the treasury pot.
        ///
        /// This actually does computation. If you need to keep using it, then make sure you cache the
        /// value and only call this once.
        pub fn account_id() -> T::AccountId {
            T::PalletId::get().into_account()
        }

        /// Return the amount of money in the pot.
        // The existential deposit is not part of the pot so treasury account never gets deleted.
        pub fn pot() -> BalanceOf<T> {
            T::Currency::free_balance(&Self::account_id())
                // Must never be less than 0 but better be safe.
                .saturating_sub(T::Currency::minimum_balance())
        }
    }
}
