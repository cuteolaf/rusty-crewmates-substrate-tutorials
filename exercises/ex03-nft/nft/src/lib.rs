#![cfg_attr(not(feature = "std"), no_std)]

/// Edit this file to define custom logic or remove it if it is not needed.
/// Learn more about FRAME and the core library of Substrate FRAME pallets:
/// <https://docs.substrate.io/v3/runtime/frame>
pub use pallet::*;

#[cfg(test)]
mod tests;
pub mod types;

use frame_support::ensure;
use types::*;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;

	#[pallet::config]
	pub trait Config: frame_system::Config + scale_info::TypeInfo {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		#[pallet::constant]
		type MaxLength: Get<u32>;
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::storage]
	#[pallet::getter(fn unique_asset)]
	pub(super) type UniqueAsset<T: Config> =
		StorageMap<_, Blake2_128Concat, UniqueAssetId, UniqueAssetDetails<T, T::MaxLength>>;

	#[pallet::storage]
	#[pallet::getter(fn account)]
	/// The holdings of a specific account for a specific asset.
	pub(super) type Account<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		UniqueAssetId,
		Blake2_128Concat,
		T::AccountId,
		u128,
		ValueQuery,
	>;

	#[pallet::storage]
	#[pallet::getter(fn nonce)]
	/// Nonce for id of the next created asset
	pub(super) type Nonce<T: Config> = StorageValue<_, UniqueAssetId, ValueQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// New unique asset created
		Created {
			creator: T::AccountId,
			asset_id: UniqueAssetId,
		},
		/// Some assets have been burned
		Burned {
			asset_id: UniqueAssetId,
			owner: T::AccountId,
			total_supply: u128,
		},
		/// Some assets have been transferred
		Transferred {
			asset_id: UniqueAssetId,
			from: T::AccountId,
			to: T::AccountId,
			amount: u128,
		},
	}

	#[pallet::error]
	pub enum Error<T> {
		/// The asset ID is unknown
		UnknownAssetId,
		/// The signing account does not own any amount of this asset
		NotOwned,
		/// Supply must be positive
		NoSupply,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(0)]
		pub fn mint(
			origin: OriginFor<T>,
			metadata: BoundedVec<u8, T::MaxLength>,
			supply: u128,
		) -> DispatchResult {
			// Ensure that the extrinsic is signed
			let owner = ensure_signed(origin)?;

			ensure!(supply > 0, Error::<T>::NoSupply);

			let asset_id = Self::nonce();
			Nonce::<T>::set(asset_id + 1);

			let details = UniqueAssetDetails::new(owner.clone(), metadata, supply);

			UniqueAsset::<T>::insert(asset_id, details);

			Account::<T>::insert(asset_id, owner.clone(), supply);

			Self::deposit_event(Event::<T>::Created {
				creator: owner,
				asset_id,
			});

			Ok(())
		}

		#[pallet::weight(0)]
		pub fn burn(origin: OriginFor<T>, asset_id: UniqueAssetId, amount: u128) -> DispatchResult {
			// Ensure the extrinsic is signed
			let owner = ensure_signed(origin)?;

			// Ensure the asset id exists
			ensure!(
				UniqueAsset::<T>::contains_key(asset_id),
				Error::<T>::UnknownAssetId
			);

			ensure!(
				Self::account(asset_id, owner.clone()) > 0,
				Error::<T>::NotOwned
			);

			let mut burnt_amount = 0;
			let mut total_supply = 0;

			// Mutate the account balance

			Account::<T>::mutate(asset_id, owner.clone(), |balance| -> DispatchResult {
				let old_balance = *balance;
				*balance = old_balance.saturating_sub(amount);
				burnt_amount = old_balance - *balance;

				Ok(())
			})?;

			UniqueAsset::<T>::mutate(asset_id, |details| -> DispatchResult {
				let details = details.as_mut().ok_or(Error::<T>::UnknownAssetId)?;

				details.supply = details.supply.saturating_sub(burnt_amount);
				total_supply = details.supply;

				Ok(())
			})?;

			Self::deposit_event(Event::<T>::Burned {
				asset_id,
				owner,
				total_supply,
			});

			Ok(())
		}

		#[pallet::weight(0)]
		pub fn transfer(
			origin: OriginFor<T>,
			asset_id: UniqueAssetId,
			amount: u128,
			to: T::AccountId,
		) -> DispatchResult {
			let from = ensure_signed(origin)?;

			ensure!(
				UniqueAsset::<T>::contains_key(asset_id) == true,
				Error::<T>::UnknownAssetId
			);
			ensure!(
				Self::account(asset_id, from.clone()) > 0,
				Error::<T>::NotOwned
			);

			let mut transferred = 0;

			Account::<T>::mutate(asset_id, from.clone(), |balance| {
				let old_balance = *balance;
				*balance = old_balance.saturating_sub(amount);
				transferred = old_balance - *balance;
			});

			Account::<T>::mutate(asset_id, to.clone(), |balance| {
				*balance += transferred;
			});

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
