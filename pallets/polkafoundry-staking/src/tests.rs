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
			Error::<Test>::AlreadyBonded
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
		assert_noop!(
			Staking::bond_extra(
				Origin::signed(1),
				100
			),
			Error::<Test>::CandidateNotActive
		);
		let collator = Staking::collators(&1).unwrap();
		assert_eq!(
			collator.unlocking,
			vec![UnlockChunk {
				value: 500,
				round: 2
			}]
		);
		// run to next round to update all locked bond
		run_to_block(11);
		assert_ok!(
			Staking::bond_extra(
				Origin::signed(1),
				300
			),
		);
		let collator = Staking::collators(&1).unwrap();
		assert_eq!(
			collator.total,
			800
		);
		assert_eq!(
			collator.active,
			500
		);
		// bond extra token will be locked till the next round
		assert_eq!(
			collator.unlocking,
			vec![UnlockChunk {
				value: 300,
				round: 3
			}]
		);
		run_to_block(31);
		let collator = Staking::collators(&1).unwrap();
		assert_eq!(
			collator.active,
			800
		);
		assert_eq!(
			collator.unlocking,
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
		assert_noop!(
			Staking::bond_less(
				Origin::signed(1),
				800
			),
			Error::<Test>::CandidateNotActive
		);
		run_to_block(11);
		assert_ok!(
			Staking::bond_less(
				Origin::signed(1),
				150
			),
		);
		let collator = Staking::collators(&1).unwrap();
		assert_eq!(
			collator.total,
			1000
		);
		assert_eq!(
			collator.active,
			850
		);
		assert_eq!(
			collator.unbonding,
			vec![UnBondChunk {
				value: 150,
				round: 4
			}]
		);
		run_to_block(21);
		assert_noop!(
			Staking::bond_less(
				Origin::signed(1),
				800
			),
			Error::<Test>::BondBelowMin
		);

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
		let collator = Staking::collators(&1).unwrap();
		assert_eq!(
			collator.total,
			1000
		);
		assert_eq!(
			collator.active,
			750
		);
		assert_eq!(
			collator.unbonding,
			vec![UnBondChunk {
				value: 150,
				round: 4
			}, UnBondChunk {
				value: 100,
				round: 5
			}]
		);
		run_to_block(31);
		let collator = Staking::collators(&1).unwrap();
		assert_eq!(
			collator.total,
			850
		);
		assert_eq!(
			collator.active,
			750
		);
		assert_eq!(
			collator.unbonding,
			vec![UnBondChunk {
				value: 100,
				round: 5
			}]
		);
		assert_eq!(Balances::reserved_balance(&1), 850);
		run_to_block(41);
		let collator = Staking::collators(&1).unwrap();
		assert_eq!(
			collator.total,
			750
		);
		assert_eq!(
			collator.active,
			750
		);
		assert_eq!(
			collator.unbonding,
			vec![]
		);
		assert_eq!(Balances::reserved_balance(&1), 750);
	})
}

#[test]
pub fn nominate_work() {
	mock_test().execute_with(|| {
		assert_noop!(
			Staking::nominate(
				Origin::signed(10),
				100,
				1000
			),
			Error::<Test>::CandidateNotActive
		);
		run_to_block(11);
		assert_noop!(
			Staking::nominate(
				Origin::signed(10),
				2,
				1000
			),
			Error::<Test>::CandidateNotExist
		);
		assert_noop!(
			Staking::nominate(
				Origin::signed(10),
				100,
				10
			),
			Error::<Test>::NominateBelowMin
		);
		assert_ok!(
			Staking::nominate(
				Origin::signed(10),
				100,
				500
			),
		);
		assert_noop!(
			Staking::nominate(
				Origin::signed(10),
				100,
				500
			),
			Error::<Test>::AlreadyNominatedCollator
		);
		assert_ok!(
			Staking::nominate(
				Origin::signed(10),
				200,
				300
			),
		);
		assert_ok!(
			Staking::nominate(
				Origin::signed(20),
				100,
				500
			),
		);
		assert_noop!(
			Staking::nominate(
				Origin::signed(30),
				100,
				500
			),
			Error::<Test>::TooManyNominations
		);
		assert_eq!(Balances::reserved_balance(&10), 800);
	})
}

#[test]
pub fn nominate_extra_work() {
	mock_test().execute_with(|| {
		assert_noop!(
			Staking::nominate(
				Origin::signed(10),
				100,
				500
			),
			Error::<Test>::CandidateNotActive
		);

		run_to_block(11);
		Staking::nominate(
			Origin::signed(10),
			100,
			500
		).unwrap();

		assert_noop!(
			Staking::nominate_extra(
				Origin::signed(10),
				1001,
				500
			),
			Error::<Test>::CandidateNotExist
		);

		assert_ok!(
			Staking::nominate_extra(
				Origin::signed(10),
				100,
				300
			),
		);

		assert_eq!(Balances::reserved_balance(&10), 800);
	})
}

#[test]
pub fn nominate_less_work() {
	mock_test().execute_with(|| {
		assert_noop!(
			Staking::nominate(
				Origin::signed(10),
				100,
				500
			),
			Error::<Test>::CandidateNotActive
		);

		run_to_block(11);
		Staking::nominate(
			Origin::signed(10),
			100,
			500
		).unwrap();

		assert_noop!(
			Staking::nominate_less(
				Origin::signed(10),
				100,
				450
			),
			Error::<Test>::NominateBelowMin
		);

		assert_noop!(
			Staking::nominate_less(
				Origin::signed(10),
				100,
				650
			),
			Error::<Test>::Underflow
		);
		assert_ok!(
			Staking::nominate_less(
				Origin::signed(10),
				100,
				300
			)
		);
		let nomination = Staking::nominators(&10).unwrap();
		assert_eq!(
			nomination.total,
			500
		);
		assert_eq!(
			nomination.unbonding,
			vec![UnBondChunk {
				value: 300,
				round: 4
			}]
		);
		assert_eq!(Balances::reserved_balance(&10), 500);
		run_to_block(31);
		assert_eq!(Balances::reserved_balance(&10), 200);
		let nomination = Staking::nominators(&10).unwrap();
		assert_eq!(
			nomination.unbonding,
			vec![]
		);
		assert_eq!(
			nomination.total,
			200
		);

	})
}

#[test]
fn force_onboard_work() {
	mock_test().execute_with(|| {
		assert_noop!(
			Staking::bond_extra(
				Origin::signed(100),
				1000
			),
			Error::<Test>::CandidateNotActive
		);
		assert_noop!(
			Staking::force_onboard(
				Origin::root(),
				1
			),
			Error::<Test>::CandidateNotExist
		);
		assert_ok!(
			Staking::force_onboard(
				Origin::root(),
				100
			)
		);
		let collator = Staking::collators(&100).unwrap();
		assert_eq!(
			collator.active,
			500
		);
		assert_eq!(
			collator.total,
			500
		);
		assert_ok!(
			Staking::bond_extra(
				Origin::signed(100),
				1000
			),
		);
	})
}

#[test]
fn collator_unbond_work() {
	mock_test().execute_with(|| {
		run_to_block(11);

		assert_ok!(
			Staking::nominate(
				Origin::signed(10),
				100,
				500
			),
		);
		assert_eq!(Balances::reserved_balance(&10), 500);
		assert_noop!(
			Staking::collator_unbond(
				Origin::signed(1),
			),
			Error::<Test>::BondNotExist
		);
		assert_ok!(
			Staking::collator_unbond(
				Origin::signed(100),
			),
		);
		assert_noop!(
			Staking::collator_unbond(
				Origin::signed(100),
			),
			Error::<Test>::BondNotExist
		);
		assert_eq!(Balances::reserved_balance(&10), 0);
		assert_eq!(Balances::reserved_balance(&100), 500);
		let exit = Staking::exit_queue(&100).unwrap();
		assert_eq!(
			exit.remaining,
			500
		);
		run_to_block(31);
		assert_eq!(Balances::reserved_balance(&100), 0);
	})
}

#[test]
fn collator_bond_less_then_unbond_work() {
	mock_test().execute_with(|| {
		Staking::bond(
			Origin::signed(1),
			1000
		).unwrap();

		run_to_block(11);

		assert_ok!(
			Staking::bond_less(
				Origin::signed(1),
				150
			),
		);
		run_to_block(21);
		assert_ok!(
			Staking::collator_unbond(
				Origin::signed(1),
			),
		);
		let exit = Staking::exit_queue(&1).unwrap();
		assert_eq!(
			exit.remaining,
			850
		);
		assert_eq!(
			exit.unbonding,
			vec![UnBondChunk {
				value: 150,
				round: 4
			}]
		);
		assert_eq!(
			exit.when,
			5
		);
		assert_eq!(Balances::reserved_balance(&1), 1000);
		run_to_block(31);
		let exit = Staking::exit_queue(&1).unwrap();
		assert_eq!(
			exit.remaining,
			850
		);
		assert_eq!(
			exit.unbonding,
			vec![]
		);
		assert_eq!(Balances::reserved_balance(&1), 850);
		run_to_block(41);
		assert_eq!(Balances::reserved_balance(&1), 0);
	})
}

#[test]
fn collator_bond_less_and_unbond_same_time() {
	mock_test().execute_with(|| {
		Staking::bond(
			Origin::signed(1),
			1000
		).unwrap();

		run_to_block(11);
		assert_ok!(
			Staking::bond_less(
				Origin::signed(1),
				150
			),
		);
		assert_ok!(
			Staking::collator_unbond(
				Origin::signed(1),
			),
		);
		let exit = Staking::exit_queue(&1).unwrap();
		assert_eq!(
			exit.remaining,
			850
		);
		assert_eq!(
			exit.unbonding,
			vec![UnBondChunk {
				value: 150,
				round: 4
			}]
		);
		assert_eq!(
			exit.when,
			4
		);
		assert_eq!(Balances::reserved_balance(&1), 1000);
		run_to_block(31);
		assert_eq!(Balances::reserved_balance(&1), 0);
	})
}

#[test]
fn nominator_leave_collator_work() {
	mock_test().execute_with(|| {
		run_to_block(11);
		assert_ok!(
			Staking::nominate(
				Origin::signed(10),
				100,
				500
			),
		);
		assert_ok!(
			Staking::nominate(
				Origin::signed(10),
				200,
				500
			),
		);
		assert_noop!(
			Staking::nominator_leave_collator(
				Origin::signed(10),
				1000
			),
			Error::<Test>::CandidateNotExist
		);
		assert_ok!(
			Staking::nominator_leave_collator(
				Origin::signed(10),
				100
			),
		);
		let nomination = Staking::nominators(&10).unwrap();
		assert_eq!(
			nomination.unbonding,
			vec![UnBondChunk {
				value: 500,
				round: 4
			}]
		);
		assert_eq!(
			nomination.nominations,
			vec![Bond {
				owner: 200,
				amount: 500
			}]
		);
		assert_eq!(
			nomination.total,
			1000
		);

		assert_eq!(Balances::reserved_balance(&10), 1000);
		run_to_block(31);
		let nomination = Staking::nominators(&10).unwrap();
		assert_eq!(
			nomination.unbonding,
			vec![]
		);
		assert_eq!(
			nomination.total,
			500
		);
		assert_eq!(Balances::reserved_balance(&10), 500);
	})
}
