#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

#[frame_support::pallet]
pub mod pallet {
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;

    use codec::alloc::collections::BTreeMap;
    use core::convert::TryInto;
    use fractal_token_distribution::TokenDistribution;
    use frame_support::{
        traits::{Currency, Get, Imbalance},
        weights::Weight,
    };
    use frame_system::ensure_signed;
    use sp_runtime::traits::{Bounded, CheckedSub};

    pub type FractalId = u64;

    pub const HOLDER_REWARDS_PURPOSE: u8 = 1;

    type BalanceOf<T> = <<T as fractal_token_distribution::Config>::Currency as Currency<
        <T as frame_system::Config>::AccountId,
    >>::Balance;

    #[pallet::config]
    pub trait Config:
        frame_system::Config + fractal_token_distribution::Config + pallet_balances::Config
    {
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        type MintEveryNBlocks: Get<Self::BlockNumber>;

        type TokenDistribution: TokenDistribution<Self>;
    }

    #[pallet::storage]
    pub type CoinBlockShares<T: Config> =
        StorageMap<_, Blake2_128Concat, BlockNumberFor<T>, u32, OptionQuery>;

    #[pallet::storage]
    pub type BlockBalances<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        BlockNumberFor<T>,
        Blake2_128Concat,
        T::AccountId,
        BalanceOf<T>,
        ValueQuery,
    >;

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(_);

    #[pallet::event]
    #[pallet::metadata(BalanceOf<T> = "Balance")]
    pub enum Event<T: Config> {}

    #[pallet::error]
    pub enum Error<T> {}

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::weight((
            10_000 + T::DbWeight::get().reads_writes(0, 1),
            DispatchClass::Normal,
            Pays::No
        ))]
        pub fn set_hold_shares(
            origin: OriginFor<T>,
            coin_block_shares: BTreeMap<BlockNumberFor<T>, u32>,
        ) -> DispatchResult {
            ensure_root(origin)?;

            CoinBlockShares::<T>::remove_all();
            for (coin_block, shares) in coin_block_shares {
                CoinBlockShares::<T>::insert(coin_block, shares);
            }

            Ok(())
        }
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T>
    where
        BalanceOf<T>: core::iter::Sum,
    {
        fn on_finalize(block_number: BlockNumberFor<T>) {
            let is_minting_block =
                |n: BlockNumberFor<T>| n % T::MintEveryNBlocks::get() == 0u32.into();

            for (block_delta, _) in CoinBlockShares::<T>::iter() {
                if !is_minting_block(block_number + block_delta) {
                    continue;
                }

                for (id, _) in frame_system::pallet::Account::<T>::iter() {
                    let balance = T::Currency::free_balance(&id);
                    BlockBalances::<T>::insert(block_number, id, balance);
                }
            }

            if !is_minting_block(block_number) {
                return;
            }

            let coin_block_shares = CoinBlockShares::<T>::iter().collect::<BTreeMap<_, _>>();
            let account_shares = frame_system::pallet::Account::<T>::iter()
                .map(|(id, _)| {
                    let mut effective_balance = BalanceOf::<T>::max_value();
                    let balance = coin_block_shares
                        .iter()
                        .filter_map(|(&delta, &shares)| {
                            effective_balance = core::cmp::min(
                                BlockBalances::<T>::get(block_number.checked_sub(&delta)?, &id),
                                effective_balance,
                            );
                            Some(effective_balance * shares.into())
                        })
                        .sum();

                    (id, balance)
                })
                .collect::<BTreeMap<_, _>>();

            let total_shares = account_shares.values().cloned().sum();

            let amount = T::TokenDistribution::take_from(HOLDER_REWARDS_PURPOSE);
            for (id, shares) in account_shares {
                let to_this = amount * shares / total_shares;
                T::Currency::deposit_creating(&id, to_this);
            }
        }
    }

    impl<T: Config> Pallet<T> {}
}
