use crate::{mock::*, *};
use frame_support::{
    assert_noop, assert_ok,
    traits::{Currency, OnFinalize, OnInitialize},
};

#[cfg(test)]
mod register_identity {
    use super::*;
    use fractal_token_distribution::TokenDistribution;
    use frame_support::dispatch::PostDispatchInfo;
    use frame_support::pallet_prelude::Pays;

    fn run_test(f: impl FnOnce()) {
        new_test_ext().execute_with(|| {
            step_block();
            assert_ok!(FractalHolderRewards::set_hold_shares(
                Origin::root(),
                maplit::btreemap! {
                    0 => 1,
                }
            ));

            f();
        });
    }

    fn step_block() {
        FractalHolderRewards::on_finalize(System::block_number());
        System::on_finalize(System::block_number());
        System::set_block_number(System::block_number() + 1);
        System::on_initialize(System::block_number());
        FractalHolderRewards::on_initialize(System::block_number());
    }

    fn run_to_next_minting() {
        let mint_every_n = <Test as crate::Config>::MintEveryNBlocks::get();

        loop {
            step_block();

            if System::block_number() % mint_every_n == 1 {
                break;
            }
        }
    }

    #[test]
    fn distributes_to_single_user() {
        run_test(|| {
            FractalTokenDistribution::return_to(HOLDER_REWARDS_PURPOSE, 100_000);
            let _ = Balances::deposit_creating(&1, 100_000);

            run_to_next_minting();

            assert_eq!(Balances::free_balance(1), 200_000);
        });
    }

    #[test]
    fn distributes_among_many_users() {
        run_test(|| {
            FractalTokenDistribution::return_to(HOLDER_REWARDS_PURPOSE, 100_000);
            let _ = Balances::deposit_creating(&1, 100_000);
            let _ = Balances::deposit_creating(&2, 100_000);
            let _ = Balances::deposit_creating(&3, 100_000);

            run_to_next_minting();

            assert_eq!(Balances::free_balance(1), 100_000 + 100_000 / 3);
            assert_eq!(Balances::free_balance(2), 100_000 + 100_000 / 3);
            assert_eq!(Balances::free_balance(3), 100_000 + 100_000 / 3);
        });
    }

    #[test]
    fn distributes_proportional_to_balance() {
        run_test(|| {
            FractalTokenDistribution::return_to(HOLDER_REWARDS_PURPOSE, 100_000);
            let _ = Balances::deposit_creating(&1, 100_000);
            let _ = Balances::deposit_creating(&2, 50_000);
            let _ = Balances::deposit_creating(&3, 50_000);

            run_to_next_minting();

            assert_eq!(Balances::free_balance(1), 100_000 + 100_000 / 2);
            assert_eq!(Balances::free_balance(2), 50_000 + 100_000 / 4);
            assert_eq!(Balances::free_balance(3), 50_000 + 100_000 / 4);
        });
    }

    #[test]
    fn older_coins_receive_more() {
        run_test(|| {
            assert_ok!(FractalHolderRewards::set_hold_shares(
                Origin::root(),
                maplit::btreemap! {
                    0 => 1,
                    <Test as crate::Config>::MintEveryNBlocks::get() => 1,
                },
            ));

            let _ = Balances::deposit_creating(&1, 100_000);

            run_to_next_minting();

            let _ = Balances::deposit_creating(&2, 100_000);
            let _ = Balances::deposit_creating(&3, 100_000);

            FractalTokenDistribution::return_to(HOLDER_REWARDS_PURPOSE, 100_000);
            run_to_next_minting();

            assert_eq!(Balances::free_balance(1), 100_000 + 100_000 / 2);
            assert_eq!(Balances::free_balance(2), 100_000 + 100_000 / 4);
            assert_eq!(Balances::free_balance(3), 100_000 + 100_000 / 4);
        });
    }

    #[test]
    fn older_coins_after_transfer() {
        run_test(|| {
            assert_ok!(FractalHolderRewards::set_hold_shares(
                Origin::root(),
                maplit::btreemap! {
                    0 => 1,
                    <Test as crate::Config>::MintEveryNBlocks::get() => 1,
                },
            ));

            let _ = Balances::deposit_creating(&1, 100_000);

            run_to_next_minting();

            let _ = Balances::transfer(Origin::signed(1), 2, 50_000);
            let _ = Balances::deposit_creating(&3, 50_000);

            FractalTokenDistribution::return_to(HOLDER_REWARDS_PURPOSE, 100_000);
            run_to_next_minting();

            let expected_shares = 4;
            let share_amount = 100_000 / expected_shares;
            assert_eq!(Balances::free_balance(1), 50_000 + share_amount * 2);
            assert_eq!(Balances::free_balance(2), 50_000 + share_amount);
            assert_eq!(Balances::free_balance(3), 50_000 + share_amount);
        });
    }

    #[test]
    fn older_coins_intermittent_drop() {
        run_test(|| {
            assert_ok!(FractalHolderRewards::set_hold_shares(
                Origin::root(),
                maplit::btreemap! {
                    0 => 1,
                    <Test as crate::Config>::MintEveryNBlocks::get() => 1,
                    <Test as crate::Config>::MintEveryNBlocks::get() * 2 => 1,
                },
            ));

            let _ = Balances::make_free_balance_be(&1, 100_000);
            run_to_next_minting();

            let _ = Balances::make_free_balance_be(&1, 50_000);
            run_to_next_minting();

            let _ = Balances::make_free_balance_be(&1, 100_000);
            let _ = Balances::make_free_balance_be(&2, 50_000);

            FractalTokenDistribution::return_to(HOLDER_REWARDS_PURPOSE, 100_000);
            run_to_next_minting();

            let expected_shares = 5;
            let share_amount = 100_000 / expected_shares;
            assert_eq!(Balances::free_balance(1), 100_000 + share_amount * 4);
            assert_eq!(Balances::free_balance(2), 50_000 + share_amount);
        });
    }

    // Weighs based on coin-days
    // Returns to purpose
    // Ignore specific addresses
    //
    // Multiply overflow
    // Split across many blocks
}
