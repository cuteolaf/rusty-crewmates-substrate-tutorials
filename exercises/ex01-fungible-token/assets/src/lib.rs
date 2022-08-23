#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

pub mod types;

use frame_support::ensure;
use types::*;

#[cfg(test)]
mod tests;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		#[pallet::constant]
		type MaxLength: Get<u32>;
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	// The pallet's runtime storage items.
	// https://docs.substrate.io/v3/runtime/storage
	// Learn more about declaring storage items:
	// https://docs.substrate.io/v3/runtime/storage#declaring-storage-items
	#[pallet::storage]
	#[pallet::getter(fn asset)]
	/// Details of an asset.
	pub(super) type Asset<T: Config> =
		StorageMap<_, Blake2_128Concat, AssetId, AssetDetails<T::AccountId>>;

	#[pallet::storage]
	#[pallet::getter(fn account)]
	/// The holdings of a specific account for a specific asset.
	pub(super) type Account<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		AssetId,
		Blake2_128Concat,
		T::AccountId,
		u128,
		ValueQuery,
	>;

	#[pallet::storage]
	#[pallet::getter(fn metadata)]
	/// Details of an asset.
	pub(super) type Metadata<T: Config> =
		StorageMap<_, Blake2_128Concat, AssetId, types::AssetMetadata<T::MaxLength>>;

	#[pallet::storage]
	#[pallet::getter(fn nonce)]
	/// Nonce for id of the next created asset.
	pub(super) type Nonce<T: Config> = StorageValue<_, AssetId, ValueQuery>;

	// Pallets use events to inform users when important changes are made.
	// https://docs.substrate.io/v3/runtime/events-and-errors
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// New asset created.
		Created {
			owner: T::AccountId,
			asset_id: AssetId,
		},
		/// New metadata has been set for an asset.
		MetadataSet {
			asset_id: AssetId,
			name: BoundedVec<u8, T::MaxLength>,
			symbol: BoundedVec<u8, T::MaxLength>,
		},
		/// Some assets have been minted.
		Minted {
			asset_id: AssetId,
			owner: T::AccountId,
			total_supply: u128,
		},
		/// Some assets have been burned.
		Burned {
			asset_id: AssetId,
			owner: T::AccountId,
			total_supply: u128,
		},
		/// Some assets have been transferred.
		Transferred {
			asset_id: AssetId,
			from: T::AccountId,
			to: T::AccountId,
			amount: u128,
		},
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		/// The asset ID is unknown.
		UnknownAssetId,
		/// The signing account has no permission to do the operation.
		NoPermission,
	}

	// Dispatchable functions allow users to interact with the pallet and invoke state changes.
	// These functions materialize as "extrinsics", which are often compared to transactions.
	// Dispatchable functions must be annotated with a weight and must return a DispatchResult.
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(0)]
		pub fn create(origin: OriginFor<T>) -> DispatchResult {
			let origin = ensure_signed(origin)?;

			let id = Self::nonce();
			let details = AssetDetails::new(origin.clone());

			Asset::<T>::insert(id, details);
			Nonce::<T>::set(id.saturating_add(1));

			Self::deposit_event(Event::<T>::Created {
				owner: origin,
				asset_id: id,
			});

			Ok(())
		}

		#[pallet::weight(0)]
		pub fn set_metadata(
			origin: OriginFor<T>,
			asset_id: AssetId,
			name: BoundedVec<u8, T::MaxLength>,
			symbol: BoundedVec<u8, T::MaxLength>,
		) -> DispatchResult {
			let origin = ensure_signed(origin)?;
			Self::ensure_is_owner(asset_id, origin)?;

			// TODO:
			// - Create a new AssetMetadata instance based on the call arguments.
			// - Insert this metadata in the Metadata storage, under the asset_id key.
			Metadata::<T>::insert(
				asset_id,
				AssetMetadata {
					name: name.clone(),
					symbol: symbol.clone(),
				},
			);
			// - Deposit a `MetadataSet` event.

			Self::deposit_event(Event::<T>::MetadataSet {
				asset_id,
				name,
				symbol,
			});

			Ok(())
		}

		#[pallet::weight(0)]
		pub fn mint(
			origin: OriginFor<T>,
			asset_id: AssetId,
			amount: u128,
			to: T::AccountId,
		) -> DispatchResult {
			// TODO:
			// - Ensure the extrinsic origin is a signed transaction.
			let caller = ensure_signed(origin)?;
			// - Ensure the caller is the asset owner.

			let mut minted_amount = 0;
			let mut total_supply = 0;

			Asset::<T>::try_mutate(asset_id, |maybe_details| -> DispatchResult {
				let details = maybe_details.as_mut().ok_or(Error::<T>::UnknownAssetId)?;

				ensure!(details.owner == caller, Error::<T>::NoPermission);

				let old_supply = details.supply;
				details.supply = details.supply.saturating_add(amount);
				minted_amount = details.supply - old_supply;

				total_supply = details.supply;

				Ok(())
			})?;

			Account::<T>::mutate(asset_id, to.clone(), |balance| {
				*balance += minted_amount;
			});
			// TODO: Deposit a `Minted` event.
			Self::deposit_event(Event::<T>::Minted {
				asset_id,
				owner: to,
				total_supply,
			});
			Ok(())
		}

		#[pallet::weight(0)]
		pub fn burn(origin: OriginFor<T>, asset_id: AssetId, amount: u128) -> DispatchResult {
			// TODO:
			// - Ensure the extrinsic origin is a signed transaction.
			let caller = ensure_signed(origin)?;

			ensure!(
				Asset::<T>::contains_key(asset_id),
				Error::<T>::UnknownAssetId
			);

			// - Mutate the account balance.
			let mut burnt_amount = 0;
			Account::<T>::mutate(asset_id, caller.clone(), |balance| {
				let old_balance = *balance;
				*balance = old_balance.saturating_sub(amount);
				burnt_amount = old_balance - *balance;
			});

			// - Mutate the total supply.
			let mut total_supply = 0;

			Asset::<T>::try_mutate(asset_id, |maybe_details| -> DispatchResult {
				let details = maybe_details.as_mut().ok_or(Error::<T>::UnknownAssetId)?;
				details.supply = details.supply.saturating_sub(burnt_amount);

				total_supply = details.supply;

				Ok(())
			})?;
			// - Emit a `Burned` event.
			Self::deposit_event(Event::<T>::Burned {
				asset_id,
				owner: caller,
				total_supply,
			});
			Ok(())
		}

		#[pallet::weight(0)]
		pub fn transfer(
			origin: OriginFor<T>,
			asset_id: AssetId,
			amount: u128,
			to: T::AccountId,
		) -> DispatchResult {
			// TODO:
			// - Ensure the extrinsic origin is a signed transaction.
			let from = ensure_signed(origin)?;
			// - Mutate both account balances.
			let mut transferred = 0;

			ensure!(
				Asset::<T>::contains_key(asset_id) == true,
				Error::<T>::UnknownAssetId
			);

			Account::<T>::mutate(asset_id, from.clone(), |balance| {
				let old_balance = *balance;
				*balance = old_balance.saturating_sub(amount);
				transferred = old_balance - *balance;
			});

			Account::<T>::mutate(asset_id, to.clone(), |balance| {
				let old_balance = *balance;
				*balance = old_balance.saturating_add(transferred);
				transferred = *balance - old_balance;
			});

			// - Emit a `Transferred` event.

			Self::deposit_event(Event::<T>::Transferred {
				asset_id,
				from,
				to,
				amount: transferred,
			});

			Ok(())
		}
	}
}

impl<T: Config> Pallet<T> {
	// This is not a call, so it cannot be called directly by real-world users.
	// Still it has to be generic over the runtime types, and that's why we implement it on Pallet
	// rather than just defining a local function.
	fn ensure_is_owner(asset_id: AssetId, account: T::AccountId) -> Result<(), Error<T>> {
		let details = Self::asset(asset_id).ok_or(Error::<T>::UnknownAssetId)?;
		ensure!(details.owner == account, Error::<T>::NoPermission);

		Ok(())
	}
}
