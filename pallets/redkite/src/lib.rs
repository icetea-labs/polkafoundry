#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;
use frame_support::pallet;

// #[cfg(test)]
// pub(crate) mod mock;
// #[cfg(test)]
// mod tests;

#[pallet]
pub mod pallet {
	use frame_support::{
		pallet_prelude::*,
		traits::{Currency, ExistenceRequirement::AllowDeath, IsType, UnixTime},
		PalletId,
	};
	use frame_system::pallet_prelude::*;
	use sp_runtime::{traits::{AccountIdConversion, CheckedSub, Zero, Saturating}, Perbill};
	use sp_std::{convert::{From}, vec::Vec};

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		type PalletId: Get<PalletId>;

		type Currency: Currency<Self::AccountId>;

		type UnixTime: UnixTime;
	}

	pub type BalanceOf<T> = <<T as Config>::Currency as Currency<
		<T as frame_system::Config>::AccountId,
	>>::Balance;

	#[derive(Default, Clone, Encode, Decode, RuntimeDebug)]
	pub struct PoolInfo<T:Config> {
		pub token: Vec<u8>, // address
		pub open_time: u64,
		pub close_time: u64,
		pub offered_currency: Vec<u8>, // address
		pub funding_wallet: T::AccountId, // address
		pub signer: T::AccountId, // address
	}

	#[derive(Default, Clone, Encode, Decode, RuntimeDebug)]
	pub struct CurrencyInfo {
		pub offered_currency_decimals: u32,
		pub offered_currency_rate: u32,
	}

	#[derive(Default, Clone, Encode, Decode, RuntimeDebug)]
	pub struct UserWinnerInfo<T:Config> {
		pub max_purchased: BalanceOf<T>,
		pub min_purchased: BalanceOf<T>,
		pub purchased: BalanceOf<T>,
		pub claimed: BalanceOf<T>,
	}

	#[derive(Default, Clone, Encode, Decode, RuntimeDebug)]
	pub struct UserInfo {
		pub staked: u32,
		pub last_staked_at: u32,
	}

	#[pallet::pallet]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

	// TODO:
	// - Stake & unstake
	// - Admin & Operations permissions (avoid sudo call)

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(0)]
		pub fn init_pool(
			origin: OriginFor<T>,
			pool_id: u32,
			token: Vec<u8>,
			duration: u64,
			open_time: u64,
			offered_currency: Vec<u8>,
			funding_wallet: T::AccountId,
			signer: T::AccountId
		) -> DispatchResultWithPostInfo {
			ensure_root(origin)?;

			let close_time = open_time.saturating_add(duration);
			Pools::<T>::insert(pool_id, PoolInfo{
				token: token.clone(),
				open_time,
				close_time,
				offered_currency: offered_currency.clone(),
				funding_wallet: funding_wallet.clone(),
				signer: signer.clone(),
			});

			Self::deposit_event(Event::PoolChanged(pool_id, token, open_time, close_time, offered_currency, funding_wallet, signer));
			Ok(Default::default())
		}

		#[pallet::weight(0)]
		pub fn set_pool_winners(
			origin: OriginFor<T>,
			pool_id: u32,
			winners: Vec<(T::AccountId, BalanceOf<T>)>, // (account,max_amount)
		) -> DispatchResultWithPostInfo {
			ensure_root(origin)?;
			// TODO: clear the old winners

			for (who, amount) in winners {
				let new_user_winner_info = UserWinnerInfo{
					min_purchased: Zero::zero(),
					max_purchased: amount,
					purchased: Zero::zero(),
					claimed: Zero::zero(),
				};

				Winners::<T>::insert(pool_id, who, new_user_winner_info);
			}

			Ok(Default::default())
		}

		#[pallet::weight(0)]
		pub fn set_offered_currency_rate_and_decimals(
			origin: OriginFor<T>,
			offered_currency: Vec<u8>,
			offered_currency_decimals: u32,
			offered_currency_rate: u32,
		) -> DispatchResultWithPostInfo {
			ensure_root(origin)?;

			OfferedCurrencies::<T>::insert(offered_currency.clone(), CurrencyInfo{
				offered_currency_decimals,
				offered_currency_rate,
			});
			Self::deposit_event(Event::OfferedCurrenciesChanged(offered_currency.clone(), offered_currency_decimals, offered_currency_rate));
			Ok(Default::default())
		}

		#[pallet::weight(0)]
		pub fn set_close_time(
			origin: OriginFor<T>,
			pool_id: u32,
			close_time: u64,
		) -> DispatchResultWithPostInfo {
			ensure_root(origin)?;

			let mut pool = Pools::<T>::get(pool_id).ok_or(Error::<T>::PoolNotFound)?;
			pool.close_time = close_time;

			Pools::<T>::insert(pool_id, pool.clone());
			Self::deposit_event(Event::PoolChanged(pool_id, pool.token, pool.open_time, pool.close_time, pool.offered_currency, pool.funding_wallet, pool.signer));
			Ok(Default::default())
		}

		#[pallet::weight(0)]
		pub fn set_open_time(
			origin: OriginFor<T>,
			pool_id: u32,
			open_time: u64,
		) -> DispatchResultWithPostInfo {
			ensure_root(origin)?;

			let mut pool = Pools::<T>::get(pool_id).ok_or(Error::<T>::PoolNotFound)?;
			pool.open_time = open_time;

			Pools::<T>::insert(pool_id, pool.clone());
			Self::deposit_event(Event::PoolChanged(pool_id, pool.token, pool.open_time, pool.close_time, pool.offered_currency, pool.funding_wallet, pool.signer));
			Ok(Default::default())
		}

		#[pallet::weight(0)]
		pub fn buy_token(
			origin: OriginFor<T>,
			pool_id: u32,
			amount: BalanceOf<T>,
		) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;

			let pool = Pools::<T>::get(pool_id).ok_or(Error::<T>::PoolNotFound)?;
			let mut winner = Winners::<T>::get(pool_id, who.clone()).ok_or(Error::<T>::WinnerNotFound)?;
			let rate = OfferedCurrencies::<T>::get(pool.token).ok_or(Error::<T>::RateNotFound)?;
			let token_amount = Perbill::from_rational(1, 10u32.saturating_pow(rate.offered_currency_decimals)) // 1/10^decimals
				.mul_floor(amount.saturating_mul(rate.offered_currency_rate.into()));

			let now = T::UnixTime::now().as_secs();

			ensure!(pool.open_time <= now && pool.close_time >= now, Error::<T>::PoolClosed);
			ensure!(winner.min_purchased < token_amount, Error::<T>::PurchaseAmountBelowMinimum);
			ensure!(winner.purchased.saturating_add(token_amount) <= winner.max_purchased, Error::<T>::PurchaseAmountAboveMaximum);

			let _ = T::Currency::transfer(&who, &Self::account_id(), amount, AllowDeath)
				.map_err(|_| Error::<T>::BuyTokenFailed)?;

			winner.purchased = winner.purchased.saturating_add(token_amount);
			Winners::<T>::insert(pool_id, who.clone(), winner);
			Self::deposit_event(Event::TokenPurchased(pool_id, who.clone(), amount));

			Ok(Default::default())
		}

		#[pallet::weight(0)]
		pub fn claim_token(origin: OriginFor<T>) -> DispatchResultWithPostInfo {
			let _ = ensure_signed(origin)?;
			// TODO: transfer token to users
			Ok(Default::default())
		}
	}

	#[pallet::storage]
	#[pallet::getter(fn pools)]
	pub type Pools<T: Config> =
	StorageMap<_, Blake2_128Concat, u32, PoolInfo<T>>;

	#[pallet::storage]
	#[pallet::getter(fn winners)]
	pub type Winners<T: Config> =
	StorageDoubleMap<_, Blake2_128Concat, u32, Blake2_128Concat, T::AccountId, UserWinnerInfo<T>>;

	#[pallet::storage]
	#[pallet::getter(fn offered_currencies)]
	pub type OfferedCurrencies<T: Config> =
	StorageMap<_, Blake2_128Concat, Vec<u8>, CurrencyInfo>;

	#[pallet::error]
	pub enum Error<T> {
		/// Pool not found
		PoolNotFound,
		/// Pool closed
		PoolClosed,
		/// Winner not found
		WinnerNotFound,
		/// Exchange rate between Native token and token is not found
		RateNotFound,
		/// The amount of purchase below the minimum
		PurchaseAmountBelowMinimum,
		/// The amount of purchase above the maximum
		PurchaseAmountAboveMaximum,
		/// Token is bought failed
		BuyTokenFailed,
	}

	#[pallet::event]
	#[pallet::generate_deposit(fn deposit_event)]
	pub enum Event<T: Config> {
		PoolChanged(u32, Vec<u8>, u64, u64, Vec<u8>, T::AccountId, T::AccountId),
		OfferedCurrenciesChanged(Vec<u8>, u32, u32),
		TokenPurchased(u32, T::AccountId, BalanceOf<T>),
		TokenClaimed(u32, Vec<u8>, u32, u32),
		RefundToken(u32, Vec<u8>, u32),
	}

	#[pallet::extra_constants]
	impl<T: Config> Pallet<T> {
		/// The account ID of the pallet.
		///
		/// This actually does computation. If you need to keep using it, then make sure you cache the
		/// value and only call this once.
		pub fn account_id() -> T::AccountId {
			T::PalletId::get().into_account()
		}

		pub fn pot() -> BalanceOf<T> {
			T::Currency::free_balance(&Self::account_id())
				.checked_sub(&T::Currency::minimum_balance()).unwrap_or_else(Zero::zero)
		}
	}

	impl<T: Config> Pallet<T> {
		pub fn get_offered_currency_rate(address: Vec<u8>) -> u32 {
			match OfferedCurrencies::<T>::get(address) {
				Some(item) => item.offered_currency_rate,
				None => 0,
			}
		}

		pub fn get_offered_currency_decimals(address: Vec<u8>) -> u32 {
			match OfferedCurrencies::<T>::get(address) {
				Some(item) => item.offered_currency_decimals,
				None => 0,
			}
		}
	}
}

