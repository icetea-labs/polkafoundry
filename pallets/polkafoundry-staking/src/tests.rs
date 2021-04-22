use crate::*;
use frame_support::{assert_noop, assert_ok};
use mock::*;
use codec::Encode;

#[test]
pub fn bond_work () {
	mock_test().execute_with(|| {
		assert_ok!(
			Staking::bond(
				Origin::signed(1),
				1000
			)
		);
		assert_eq!(Balances::reserved_balance(&1), 1000);

		assert_noop!(
			Staking::bond(
				Origin::signed(1),
				1000
			),
			Error::<Test>::AlreadyInQueue
		);
		assert_noop!(
			Staking::bond(
				Origin::signed(2),
				100
			),
			Error::<Test>::BondBelowMin
		);
	})
}

#[test]
pub fn bond_extra_work () {
	mock_test().execute_with(|| {
		assert_noop!(
			Staking::bond_extra(
				Origin::signed(1),
				1000
			),
			Error::<Test>::BondNotExist
		);
		Staking::bond(
			Origin::signed(1),
			500
		).unwrap();
		assert_ok!(
			Staking::bond_extra(
				Origin::signed(1),
				100
			),
		);
		// candidates will be pushed to the queue first
		let queue = Staking::candidate_in_queue(&1).unwrap();
		assert_eq!(queue.bond, 600);
		assert_eq!(Balances::reserved_balance(&1), 600);
		// run to next round to update all candidates in the queue
		run_to_block(11);
		let ledger = Staking::ledger(&1).unwrap();
		assert_eq!(
			ledger.total,
			600
		);
		assert_eq!(
			ledger.active,
			600
		);
		assert_ok!(
			Staking::bond_extra(
				Origin::signed(1),
				300
			),
		);
		let ledger = Staking::ledger(&1).unwrap();
		assert_eq!(
			ledger.total,
			900
		);
		assert_eq!(
			ledger.active,
			600
		);
		// bond extra token will be locked till the next round
		assert_eq!(
			ledger.unlocking,
			vec![UnlockChunk {
				value: 300,
				round: 3
			}]
		);
		run_to_block(31);
		let ledger = Staking::ledger(&1).unwrap();
		assert_eq!(
			ledger.active,
			900
		);
		assert_eq!(
			ledger.unlocking,
			vec![]
		);
	})
}

#[test]
pub fn bond_less_work() {
	mock_test().execute_with(|| {
		Staking::bond(
			Origin::signed(1),
			1000
		).unwrap();
		assert_ok!(
			Staking::bond_less(
				Origin::signed(1),
				100
			),
		);
		let queue = Staking::candidate_in_queue(&1).unwrap();
		assert_eq!(queue.bond, 900);
		assert_eq!(Balances::reserved_balance(&1), 900);
		// try to unbond less than minimum value
		assert_noop!(
			Staking::bond_less(
				Origin::signed(1),
				450
			),
			Error::<Test>::BondBelowMin
		);
		// try to unbond less than bonded value
		assert_noop!(
			Staking::bond_less(
				Origin::signed(1),
				9000
			),
			Error::<Test>::Underflow
		);
		run_to_block(11);
		assert_noop!(
			Staking::bond_less(
				Origin::signed(1),
				450
			),
			Error::<Test>::BondBelowMin
		);
		// try to unbond less than bonded value
		assert_noop!(
			Staking::bond_less(
				Origin::signed(1),
				6000
			),
			Error::<Test>::Underflow
		);
		assert_ok!(
			Staking::bond_less(
				Origin::signed(1),
				100
			),
		);
		let ledger = Staking::ledger(&1).unwrap();
		assert_eq!(
			ledger.total,
			900
		);
		assert_eq!(
			ledger.active,
			800
		);
		assert_eq!(
			ledger.unbonding,
			vec![UnBondChunk {
				value: 100,
				round: 100 // just hardcode here. will fix later
			}]
		);
	})
}
