use crate::*;
use frame_support::{assert_noop, assert_ok};
use mock::*;

#[test]
fn init_reward_work() {
    mock_test().execute_with(|| {
        let donor = Origin::signed(1);
        assert_ok!(Treasury::donate(donor, 100 * ONE_COIN_UNIT));
    })
}

#[test]
fn donate_work() {
    mock_test().execute_with(|| {
        let donor = Origin::signed(1);
        let fund = 100 * ONE_COIN_UNIT;
        assert_ok!(Treasury::donate(donor, fund));
        let expected = vec![crate::Event::DonationReceived(
            1,
            fund,
            fund - MINIMUM_BALANCE,
        )];
        assert_eq!(events(), expected);
    })
}

#[test]
fn allocate_work() {
    mock_test().execute_with(|| {
        let donor = Origin::signed(1);
        let fund = 100 * ONE_COIN_UNIT;
        assert_ok!(Treasury::donate(donor, fund));

        let reward = 50 * ONE_COIN_UNIT;
        assert_ok!(Treasury::allocate(Origin::root(), 3, reward));
        let expected = vec![
            crate::Event::DonationReceived(1, fund, fund - MINIMUM_BALANCE),
            crate::Event::FundsAllocated(3, reward, fund - reward - MINIMUM_BALANCE),
        ];
        assert_eq!(events(), expected);
    })
}

#[test]
fn allocate_error() {
    mock_test().execute_with(|| {
        let reward = 100_000 * ONE_COIN_UNIT;
        assert_noop!(
            Treasury::allocate(Origin::root(), 1, reward),
            Error::<Test>::FailedAllocation,
        );
    })
}

#[test]
fn donate_error() {
    mock_test().execute_with(|| {
        let donor = Origin::signed(1);
        let fund = 1_000_000 * ONE_COIN_UNIT;
        assert_noop!(Treasury::donate(donor, fund), Error::<Test>::FailedDonation,);
    })
}
