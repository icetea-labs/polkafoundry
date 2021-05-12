#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;
use frame_support::pallet;

pub type ChainId = u8;
pub type DepositNonce = u64;
pub type ResourceId = [u8; 32];


#[pallet]
pub mod pallet {
	use codec::{Decode, Encode, EncodeLike};

	use frame_system::pallet_prelude::*;
	use frame_support::pallet_prelude::*;
	use frame_support::weights::GetDispatchInfo;
	use frame_support::PalletId;
	use frame_support::sp_runtime::traits::AccountIdConversion;
	use frame_support::traits::EnsureOrigin;

	use sp_core::U256;
	use sp_runtime::traits::Dispatchable;
	use super::{ChainId, DepositNonce, ResourceId};

	const DEFAULT_RELAYER_THRESHOLD: u32 = 1;
	const PALLET_ID: PalletId = PalletId(*b"cb/bridg");

	#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
	pub enum ProposalStatus {
		Initiated,
		Approved,
		Rejected,
	}

	#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
	pub struct ProposalVotes<AccountId, BlockNumber> {
		pub votes_for: Vec<AccountId>,
		pub votes_against: Vec<AccountId>,
		pub status: ProposalStatus,
		pub expiry: BlockNumber,
	}

	impl<A: PartialEq, B: PartialOrd + Default> ProposalVotes<A, B> {
		/// Attempts to mark the proposal as approve or rejected.
		/// Returns true if the status changes from active.
		fn try_to_complete(&mut self, threshold: u32, total: u32) -> ProposalStatus {
			if self.votes_for.len() >= threshold as usize {
				self.status = ProposalStatus::Approved;
				ProposalStatus::Approved
			} else if total >= threshold && self.votes_against.len() as u32 + threshold > total {
				self.status = ProposalStatus::Rejected;
				ProposalStatus::Rejected
			} else {
				ProposalStatus::Initiated
			}
		}

		/// Returns true if the proposal has been rejected or approved, otherwise false.
		fn is_complete(&self) -> bool {
			self.status != ProposalStatus::Initiated
		}

		/// Returns true if `who` has voted for or against the proposal
		fn has_voted(&self, who: &A) -> bool {
			self.votes_for.contains(&who) || self.votes_against.contains(&who)
		}

		/// Return true if the expiry time has been reached
		fn is_expired(&self, now: B) -> bool {
			self.expiry <= now
		}
	}

	impl<AccountId, BlockNumber: Default> Default for ProposalVotes<AccountId, BlockNumber> {
		fn default() -> Self {
			Self {
				votes_for: vec![],
				votes_against: vec![],
				status: ProposalStatus::Initiated,
				expiry: BlockNumber::default(),
			}
		}
	}

	#[pallet::pallet]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// Overarching event type
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		/// Origin used to administer the pallet
		type AdminOrigin: EnsureOrigin<Self::Origin>;
		/// Proposed dispatchable call
		type Proposal: Parameter + Dispatchable<Origin = Self::Origin> + EncodeLike + GetDispatchInfo;
		/// The identifier for this chain.
		/// This must be unique and must not collide with existing IDs within a set of bridged chains.
		type ChainId: Get<ChainId>;

		type ProposalLifetime: Get<Self::BlockNumber>;
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {

		#[pallet::weight(195_000_000)]
		pub fn set_threshold(
			origin: OriginFor<T>,
			threshold: u32
		) -> DispatchResultWithPostInfo {
			Self::ensure_admin(origin)?;

			ensure!(threshold > 0, Error::<T>::InvalidThreshold);

			<RelayerThreshold<T>>::put(threshold);
			Self::deposit_event(Event::RelayerThresholdChanged(threshold));
			Ok(Default::default())
		}

		#[pallet::weight(195_000_000)]
		pub fn set_whitelist_chain(
			origin: OriginFor<T>,
			id: ChainId
		) -> DispatchResultWithPostInfo {
			Self::ensure_admin(origin)?;
			// Cannot whitelist this chain
			ensure!(id != T::ChainId::get(), Error::<T>::InvalidChainId);
			// Cannot whitelist with an existing entry
			ensure!(
				!Self::chain_whitelisted(id),
				Error::<T>::ChainAlreadyWhitelisted
      	    );

			<ChainNonces<T>>::insert(&id, Some(0));
			Self::deposit_event(Event::ChainWhitelisted(id));

			Ok(Default::default())
		}

		#[pallet::weight(195_000_000)]
		pub fn set_resource(
			origin: OriginFor<T>,
			id: ResourceId,
			method: Vec<u8>
		) -> DispatchResultWithPostInfo {
			Self::ensure_admin(origin)?;

			<Resources<T>>::insert(id, Some(method));
			Ok(Default::default())
		}

		#[pallet::weight(195_000_000)]
		pub fn unregister_resource(
			origin: OriginFor<T>,
			id: ResourceId,
		) -> DispatchResultWithPostInfo {
			Self::ensure_admin(origin)?;

			<Resources<T>>::remove(id);
			Ok(Default::default())
		}

		#[pallet::weight(195_000_000)]
		pub fn add_relayer(
			origin: OriginFor<T>,
			relayer: T::AccountId
		) -> DispatchResultWithPostInfo {
			Self::ensure_admin(origin)?;

			ensure!(
				!Self::is_relayer(relayer.clone()),
				Error::<T>::RelayerAlreadyExists
      	    );
			<Relayers<T>>::insert(&relayer, true);
			<RelayerCount<T>>::mutate(|i| *i += 1);

			Self::deposit_event(Event::RelayerAdded(relayer));

			Ok(Default::default())
		}

		#[pallet::weight(195_000_000)]
		pub fn remove_relayer(
			origin: OriginFor<T>,
			relayer: T::AccountId
		) -> DispatchResultWithPostInfo {
			Self::ensure_admin(origin)?;
			ensure!(Self::is_relayer(relayer.clone()), Error::<T>::RelayerInvalid);

			<Relayers<T>>::remove(&relayer);
			<RelayerCount<T>>::mutate(|i| *i -= 1);

			Self::deposit_event(Event::RelayerRemoved(relayer));
			Ok(Default::default())
		}

		#[pallet::weight(195_000_000)]
		pub fn acknowledge_proposal(
			origin: OriginFor<T>,
			nonce: DepositNonce,
			src_id: ChainId,
			r_id: ResourceId,
			call: Box<<T as Config>::Proposal>
		) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;
			ensure!(Self::is_relayer(who.clone()), Error::<T>::MustBeRelayer);
			ensure!(Self::chain_whitelisted(src_id), Error::<T>::ChainNotWhitelisted);
			ensure!(Self::resource_exists(r_id), Error::<T>::ResourceDoesNotExist);

			Self::vote_for(who, nonce, src_id, call)
		}

		#[pallet::weight(195_000_000)]
		pub fn reject_proposal(
			origin: OriginFor<T>,
			nonce: DepositNonce,
			src_id: ChainId,
			r_id: ResourceId,
			call: Box<<T as Config>::Proposal>
		) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;
			ensure!(Self::is_relayer(who.clone()), Error::<T>::MustBeRelayer);
			ensure!(Self::chain_whitelisted(src_id), Error::<T>::ChainNotWhitelisted);
			ensure!(Self::resource_exists(r_id), Error::<T>::ResourceDoesNotExist);

			Self::vote_against(who, nonce, src_id, call)
		}

		#[pallet::weight(195_000_000)]
		pub fn eval_vote_state(
			origin: OriginFor<T>,
			nonce: DepositNonce,
			src_id: ChainId,
			prop: Box<<T as Config>::Proposal>
		) -> DispatchResultWithPostInfo {
			ensure_signed(origin)?;

			Self::try_resolve_proposal(nonce, src_id, prop)
		}
	}


	impl<T: Config> Pallet<T> {
		pub fn account_id() -> T::AccountId {
			PALLET_ID.into_account()
		}

		pub fn ensure_admin(o: T::Origin) -> DispatchResult {
			T::AdminOrigin::try_origin(o)
				.map(|_| ())
				.or_else(ensure_root)?;
			Ok(())
		}

		pub fn is_relayer(who: T::AccountId) -> bool {
			<Relayers<T>>::get(&who).is_some()
		}

		/// Checks if a chain exists as a whitelisted destination
		pub fn chain_whitelisted(id: ChainId) -> bool {
			<ChainNonces::<T>>::get(&id).is_some()
		}

		pub fn resource_exists(id: ResourceId) -> bool {
			<Resources::<T>>::get(&id).is_some()
		}

		pub fn vote_for(
			who: T::AccountId,
			nonce: DepositNonce,
			src_id: ChainId,
			prop: Box<T::Proposal>,
		) -> DispatchResultWithPostInfo {
			Self::commit_vote(who, nonce, src_id, prop.clone(), true)?;
			Self::try_resolve_proposal(nonce, src_id, prop)
		}

		/// Commits a vote for a proposal. If the proposal doesn't exist it will be created.
		pub fn commit_vote(
			who: T::AccountId,
			nonce: DepositNonce,
			src_id: ChainId,
			prop: Box<T::Proposal>,
			in_favour: bool,
		) -> DispatchResult {
			let now = <frame_system::Pallet<T>>::block_number();
			let mut votes = match <Votes<T>>::get(src_id, (nonce, prop.clone())) {
				Some(v) => v,
				None => {
					let mut v = ProposalVotes::default();
					v.expiry = now + T::ProposalLifetime::get();
					v
				}
			};

			// Ensure the proposal isn't complete and relayer hasn't already voted
			ensure!(!votes.is_complete(), Error::<T>::ProposalAlreadyComplete);
			ensure!(!votes.is_expired(now), Error::<T>::ProposalExpired);
			ensure!(!votes.has_voted(&who), Error::<T>::RelayerAlreadyVoted);

			if in_favour {
				votes.votes_for.push(who.clone());
				Self::deposit_event(Event::VoteFor(src_id, nonce, who.clone()));
			} else {
				votes.votes_against.push(who.clone());
				Self::deposit_event(Event::VoteAgainst(src_id, nonce, who.clone()));
			}

			<Votes<T>>::insert(src_id, (nonce, prop.clone()), Some(votes.clone()));

			Ok(())
		}

		pub fn try_resolve_proposal(
			nonce: DepositNonce,
			src_id: ChainId,
			prop: Box<T::Proposal>,
		) -> DispatchResultWithPostInfo {
			if let Some(mut votes) = <Votes<T>>::get(src_id, (nonce, prop.clone())) {
				let now = <frame_system::Module<T>>::block_number();
				ensure!(!votes.is_complete(), Error::<T>::ProposalAlreadyComplete);
				ensure!(!votes.is_expired(now), Error::<T>::ProposalExpired);

				let status = votes.try_to_complete(<RelayerThreshold::<T>>::get(), <RelayerCount::<T>>::get());
				<Votes<T>>::insert(src_id, (nonce, prop.clone()), Some(votes.clone()));

				match status {
					ProposalStatus::Approved => Self::finalize_execution(src_id, nonce, prop),
					ProposalStatus::Rejected => Self::cancel_execution(src_id, nonce),
					_ => Ok(Default::default()),
				}
			} else {
				Err(Error::<T>::ProposalDoesNotExist)?
			}
		}

		/// Commits a vote against the proposal and cancels it if more than (relayers.len() - threshold)
		/// votes against exist.
		fn vote_against(
			who: T::AccountId,
			nonce: DepositNonce,
			src_id: ChainId,
			prop: Box<T::Proposal>,
		) -> DispatchResultWithPostInfo {
			Self::commit_vote(who, nonce, src_id, prop.clone(), false)?;
			Self::try_resolve_proposal(nonce, src_id, prop)
		}

		/// Execute the proposal and signals the result as an event
		fn finalize_execution(
			src_id: ChainId,
			nonce: DepositNonce,
			call: Box<T::Proposal>,
		) -> DispatchResultWithPostInfo {
			Self::deposit_event(Event::ProposalApproved(src_id, nonce));
			call.dispatch(frame_system::RawOrigin::Signed(Self::account_id()).into())
				.map(|_| ())
				.map_err(|e| e.error)?;
			Self::deposit_event(Event::ProposalSucceeded(src_id, nonce));
			Ok(Default::default())
		}

		/// Cancels a proposal.
		fn cancel_execution(src_id: ChainId, nonce: DepositNonce) -> DispatchResultWithPostInfo {
			Self::deposit_event(Event::ProposalRejected(src_id, nonce));
			Ok(Default::default())
		}
	}

	/// All whitelisted chains and their respective transaction counts
	#[pallet::storage]
	#[pallet::getter(fn nominators)]
	pub type ChainNonces<T: Config> =
	StorageMap<_, Blake2_128Concat, ChainId, Option<DepositNonce>>;

	/// Number of votes required for a proposal to execute
	#[pallet::storage]
	#[pallet::getter(fn relayer_threshold)]
	pub type RelayerThreshold<T: Config> =
	StorageValue<_, u32, ValueQuery>;

	/// Tracks current relayer set
	#[pallet::storage]
	#[pallet::getter(fn relayers)]
	pub type Relayers<T: Config> =
	StorageMap<_, Blake2_128Concat, T::AccountId, bool>;

	/// Number of relayers in set
	#[pallet::storage]
	#[pallet::getter(fn relayer_count)]
	pub type RelayerCount<T: Config> =
	StorageValue<_, u32, ValueQuery>;

	/// All known proposals.
	/// The key is the hash of the call and the deposit ID, to ensure it's unique.
	#[pallet::storage]
	#[pallet::getter(fn votes)]
	pub type Votes<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		ChainId,
		Blake2_128Concat,
		(DepositNonce, T::Proposal),
		Option<ProposalVotes<T::AccountId, T::BlockNumber>>,
		ValueQuery,
	>;

	/// Utilized by the bridge software to map resource IDs to actual methods
	#[pallet::storage]
	#[pallet::getter(fn resources)]
	pub type Resources<T: Config> =
	StorageMap<_, Blake2_128Concat, ResourceId, Option<Vec<u8>>>;

	#[pallet::event]
	#[pallet::generate_deposit(fn deposit_event)]
	pub enum Event<T: Config> {
		/// Vote threshold has changed (new_threshold)
		RelayerThresholdChanged(u32),
		/// Chain now available for transfers (chain_id)
		ChainWhitelisted(ChainId),
		/// Relayer added to set
		RelayerAdded(T::AccountId),
		/// Relayer removed from set
		RelayerRemoved(T::AccountId),
		/// FunglibleTransfer is for relaying fungibles (dest_id, nonce, resource_id, amount, recipient, metadata)
		FungibleTransfer(ChainId, DepositNonce, ResourceId, U256, Vec<u8>),
		/// NonFungibleTransfer is for relaying NFTS (dest_id, nonce, resource_id, token_id, recipient, metadata)
		NonFungibleTransfer(ChainId, DepositNonce, ResourceId, Vec<u8>, Vec<u8>, Vec<u8>),
		/// GenericTransfer is for a generic data payload (dest_id, nonce, resource_id, metadata)
		GenericTransfer(ChainId, DepositNonce, ResourceId, Vec<u8>),
		/// Vote submitted in favour of proposal
		VoteFor(ChainId, DepositNonce, T::AccountId),
		/// Vot submitted against proposal
		VoteAgainst(ChainId, DepositNonce, T::AccountId),
		/// Voting successful for a proposal
		ProposalApproved(ChainId, DepositNonce),
		/// Voting rejected a proposal
		ProposalRejected(ChainId, DepositNonce),
		/// Execution of call succeeded
		ProposalSucceeded(ChainId, DepositNonce),
		/// Execution of call failed
		ProposalFailed(ChainId, DepositNonce),
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Relayer threshold not set
		ThresholdNotSet,
		/// Provided chain Id is not valid
		InvalidChainId,
		/// Relayer threshold cannot be 0
		InvalidThreshold,
		/// Interactions with this chain is not permitted
		ChainNotWhitelisted,
		/// Chain has already been enabled
		ChainAlreadyWhitelisted,
		/// Resource ID provided isn't mapped to anything
		ResourceDoesNotExist,
		/// Relayer already in set
		RelayerAlreadyExists,
		/// Provided accountId is not a relayer
		RelayerInvalid,
		/// Protected operation, must be performed by relayer
		MustBeRelayer,
		/// Relayer has already submitted some vote for this proposal
		RelayerAlreadyVoted,
		/// A proposal with these parameters has already been submitted
		ProposalAlreadyExists,
		/// No proposal with the ID was found
		ProposalDoesNotExist,
		/// Cannot complete proposal, needs more votes
		ProposalNotComplete,
		/// Proposal has either failed or succeeded
		ProposalAlreadyComplete,
		/// Lifetime of proposal has been exceeded
		ProposalExpired,
	}


	/// Simple ensure origin for the bridge account
	pub struct EnsureBridge<T>(sp_std::marker::PhantomData<T>);
	impl<T: Config> EnsureOrigin<T::Origin> for EnsureBridge<T> {
		type Success = T::AccountId;
		fn try_origin(o: T::Origin) -> Result<Self::Success, T::Origin> {
			let bridge_id = PALLET_ID.into_account();
			o.into().and_then(|o| match o {
				frame_system::RawOrigin::Signed(who) if who == bridge_id => Ok(bridge_id),
				r => Err(T::Origin::from(r)),
			})
		}
	}
}
