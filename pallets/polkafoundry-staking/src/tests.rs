use crate::*;
use frame_support::{assert_noop, assert_ok};
use mock::*;

#[test]
pub fn bond_work () {
	mock_test().execute_with(|| {
		give_money(&1, 1000);
		give_money(&2, 1000);
		// Account 1 is stashed and locked, and account 11 is the controller
		assert_ok!(
			Staking::bond(
				Origin::signed(1),
				11,
				1000,
				RewardDestination::Controller,
			)
		);
		// Double bond should be failed
		assert_noop!(
			Staking::bond(
				Origin::signed(1),
				11,
				1000,
				RewardDestination::Controller,
			),
			Error::<Test>::AlreadyBonded
		);
		// Staking with same controller should be failed
		assert_noop!(
			Staking::bond(
				Origin::signed(2),
				11,
				1000,
				RewardDestination::Controller,
			),
			Error::<Test>::AlreadyPaired
		);
		assert_eq!(Balances::reserved_balance(&1), 1000);
		// Staking below min should be failed
		assert_noop!(
			Staking::bond(
				Origin::signed(2),
				12,
				100,
				RewardDestination::Controller,
			),
			Error::<Test>::BondBelowMin
		);
	})
}

#[test]
pub fn bond_extra_work () {
	mock_test().execute_with(|| {
		give_money(&1, 2000);

		// bond account 1
		assert_ok!(
			Staking::bond(
				Origin::signed(1),
				11,
				1000,
				RewardDestination::Controller,
			)
		);
		// Call the bond_extra function from stash, add 100
		assert_ok!(Staking::bond_extra(Origin::signed(1), 100));
		// There should be 100 more `total` and `active` in the ledger
		let ledger = Staking::ledger(&11).unwrap();
		assert_eq!(ledger.stash, 1);
		assert_eq!(ledger.total, 1100);
		assert_eq!(ledger.active, 1100);
		assert_eq!(Balances::reserved_balance(&1), 1100);
	})
}

#[test]
pub fn bond_less_work() {
	mock_test().execute_with(|| {
		give_money(&1, 2000);
		// bond account 1
		assert_ok!(
			Staking::bond(
				Origin::signed(1),
				11,
				1000,
				RewardDestination::Controller,
			)
		);
		// bond less than min bonding
		assert_noop!(
			Staking::bond_less(
				Origin::signed(1),
				800
			),
			Error::<Test>::BondBelowMin
		);
		// bond less much than bonded balance
		assert_noop!(
			Staking::bond_less(
				Origin::signed(1),
				6500
			),
			Error::<Test>::Underflow
		);

		// Call the bond_less function from stash, unbond 100
		assert_ok!(Staking::bond_less(Origin::signed(1), 100));
		let ledger = Staking::ledger(&11).unwrap();
		assert_eq!(ledger.stash, 1);
		assert_eq!(ledger.total, 1000);
		assert_eq!(ledger.active, 900);
		assert_eq!(ledger.unbonding, vec![UnBondChunk {
			value: 100,
			era: 3
		}]);
		mock::start_active_era(3);
		let ledger = Staking::ledger(&11).unwrap();
		assert_eq!(ledger.stash, 1);
		assert_eq!(ledger.total, 900);
		assert_eq!(ledger.active, 900);
		assert_eq!(ledger.unbonding, vec![]);
	})
}

#[test]
pub fn nominate_work() {
	mock_test().execute_with(|| {
		give_money(&1, 2000);
		give_money(&2, 2000);
		give_money(&3, 2000);
		// nominate for stash account not controller
		assert_noop!(
			Staking::nominate(
				Origin::signed(1),
				100,
				1000
			),
			Error::<Test>::NotController
		);
		// nominate below min
		assert_noop!(
			Staking::nominate(
				Origin::signed(1),
				101,
				99
			),
			Error::<Test>::NominateBelowMin
		);
		// nominate is okay
		assert_ok!(
			Staking::nominate(
				Origin::signed(1),
				101,
				1000
			),
		);
		// double nominate same collator failed
		assert_noop!(
			Staking::nominate(
				Origin::signed(1),
				101,
				1000
			),
			Error::<Test>::AlreadyNominatedCollator
		);
		// nominate is okay
		assert_ok!(
			Staking::nominate(
				Origin::signed(2),
				101,
				1000
			),
		);
		let ledger = Staking::ledger(&101).unwrap();
		assert_eq!(
			ledger.nominations,
			vec![Bond {
				owner: 1,
				amount: 1000
			}, Bond {
				owner: 2,
				amount: 1000
			}]
		);
		// cannot nominate full nomination collator
		assert_noop!(
			Staking::nominate(
				Origin::signed(3),
				101,
				1000
			),
			Error::<Test>::TooManyNominations
		);
		assert_eq!(Balances::reserved_balance(&1), 1000);
		assert_eq!(Balances::reserved_balance(&2), 1000);
	})
}

#[test]
pub fn nominate_extra_work() {
	mock_test().execute_with(|| {
		give_money(&1, 2000);
		// nominate extra not work when not nominate before
		assert_noop!(
			Staking::nominate_extra(
				Origin::signed(1),
				101,
				500
			),
			Error::<Test>::NominationNotExist
		);
		Staking::nominate(
			Origin::signed(1),
			101,
			500
		).unwrap();
		// nominate extra before nominate worked
		assert_ok!(
			Staking::nominate_extra(
				Origin::signed(1),
				101,
				300
			),
		);
		assert_eq!(Balances::reserved_balance(&1), 800);
		let ledger = Staking::ledger(&101).unwrap();
		assert_eq!(
			ledger.nominations,
			vec![Bond {
				owner: 1,
				amount: 800
			}]
		);
	})
}

#[test]
pub fn nominate_less_work() {
	mock_test().execute_with(|| {
		give_money(&1, 2000);
		mock::start_active_era(1);
		// nominate less not work when not nominate before
		assert_noop!(
			Staking::nominate_extra(
				Origin::signed(1),
				101,
				500
			),
			Error::<Test>::NominationNotExist
		);
		// nominate
		Staking::nominate(
			Origin::signed(1),
			101,
			500
		).unwrap();
		// nominate less than min nomination
		assert_noop!(
			Staking::nominate_less(
				Origin::signed(1),
				101,
				450
			),
			Error::<Test>::NominateBelowMin
		);
		// nominate too much
		assert_noop!(
			Staking::nominate_less(
				Origin::signed(1),
				101,
				650
			),
			Error::<Test>::Underflow
		);
		// nominate less okay
		assert_ok!(
			Staking::nominate_less(
				Origin::signed(1),
				101,
				300
			)
		);
		let nomination = Staking::nominators(&1).unwrap();
		assert_eq!(
			nomination.total,
			200
		);
		assert_eq!(
			nomination.unbonding,
			vec![UnBondChunk {
				value: 300,
				era: 4
			}]
		);
		assert_eq!(Balances::reserved_balance(&1), 500);
		mock::start_active_era(4);
		// balance will unbond after `BondingDuration`
		assert_eq!(Balances::reserved_balance(&1), 200);
		let nomination = Staking::nominators(&1).unwrap();
		assert_eq!(
			nomination.unbonding,
			vec![]
		);
		assert_eq!(
			nomination.total,
			200
		);
		let ledger = Staking::ledger(&101).unwrap();
		assert_eq!(
			ledger.nominations,
			vec![Bond {
				owner: 1,
				amount: 200
			}]
		);
	})
}
#[test]
fn collator_unbond_work() {
	mock_test().execute_with(|| {
		give_money(&1, 2000);
		assert_ok!(
			Staking::nominate(
				Origin::signed(1),
				101,
				500
			),
		);
		assert_eq!(Balances::reserved_balance(&1), 500);
		// using controller account unbond not work
		assert_noop!(
			Staking::collator_unbond(
				Origin::signed(101),
			),
			Error::<Test>::NotStash
		);
		// using stash account unbond is okay
		assert_ok!(
			Staking::collator_unbond(
				Origin::signed(100),
			),
		);
		// double unbond failed
		assert_noop!(
			Staking::collator_unbond(
				Origin::signed(100),
			),
			Error::<Test>::NotController
		);
		assert_eq!(Balances::reserved_balance(&1), 0);
		assert_eq!(Balances::reserved_balance(&100), 1000);
		let exit = Staking::exit_queue(&100).unwrap();
		assert_eq!(
			exit.remaining,
			1000
		);
		mock::start_active_era(3);
		// balance will unbond after `BondingDuration`
		assert_eq!(Balances::reserved_balance(&100), 0);
	})
}

#[test]
fn collator_bond_less_then_unbond_work() {
	mock_test().execute_with(|| {
		give_money(&1, 2000);
		assert_ok!(
			Staking::nominate(
				Origin::signed(1),
				101,
				200
			),
		);
		// using stash account to bond less
		assert_ok!(
			Staking::bond_less(
				Origin::signed(100),
				150
			),
		);
		assert_eq!(Balances::reserved_balance(&1), 200);
		mock::start_active_era(2);
		// using stash account to unbond
		assert_ok!(
			Staking::collator_unbond(
				Origin::signed(100),
			),
		);
		// double unbond fail
		assert_noop!(
			Staking::collator_unbond(
				Origin::signed(100),
			),
			Error::<Test>::NotController
		);
		// nominator balance unlocked immediately
		assert_eq!(Balances::reserved_balance(&1), 0);
		let exit = Staking::exit_queue(&100).unwrap();
		// the active balance before unbond is correct
		assert_eq!(
			exit.remaining,
			850
		);
		// the unbonding balance before unbond is correct
		assert_eq!(
			exit.unbonding,
			vec![UnBondChunk {
				value: 150,
				era: 3
			}]
		);
		assert_eq!(
			exit.when,
			5
		);
		assert_eq!(Balances::reserved_balance(&100), 1000);
		// after bonding duration `bond_less` balances will be unlocked
		mock::start_active_era(3);
		let exit = Staking::exit_queue(&100).unwrap();
		assert_eq!(
			exit.remaining,
			850
		);
		assert_eq!(
			exit.unbonding,
			vec![]
		);
		assert_eq!(Balances::reserved_balance(&100), 850);
		// after bonding duration `collator_unbond` balances will be unlocked
		mock::start_active_era(5);
		let exit = Staking::exit_queue(&100);
		// acc will be removed from exit queue
		assert!(exit.is_none());
		assert_eq!(Balances::reserved_balance(&100), 0);
	})
}

#[test]
fn collator_bond_less_and_unbond_same_time() {
	mock_test().execute_with(|| {
		give_money(&1, 2000);
		assert_ok!(
			Staking::nominate(
				Origin::signed(1),
				101,
				200
			),
		);

		// using stash account to bond less
		assert_ok!(
			Staking::bond_less(
				Origin::signed(100),
				150
			),
		);
		// using stash account to unbond
		assert_ok!(
			Staking::collator_unbond(
				Origin::signed(100),
			),
		);

		assert_eq!(Balances::reserved_balance(&1), 0);
		let exit = Staking::exit_queue(&100).unwrap();
		assert_eq!(
			exit.remaining,
			850
		);
		assert_eq!(
			exit.unbonding,
			vec![UnBondChunk {
				value: 150,
				era: 3
			}]
		);
		assert_eq!(
			exit.when,
			3
		);
		assert_eq!(Balances::reserved_balance(&100), 1000);
		mock::start_active_era(3);
		assert_eq!(Balances::reserved_balance(&100), 0);
	})
}

#[test]
fn nominator_leave_collator_work() {
	mock_test().execute_with(|| {
		give_money(&1, 2000);
		give_money(&2, 2000);
		// 2 nominator nominate
		assert_ok!(
			Staking::nominate(
				Origin::signed(1),
				101,
				500
			),
		);
		assert_ok!(
			Staking::nominate(
				Origin::signed(2),
				101,
				1500
			),
		);
		// try to leave not controller account fail
		assert_noop!(
			Staking::nominator_leave_collator(
				Origin::signed(1),
				100
			),
			Error::<Test>::NotController
		);
		assert_ok!(
			Staking::nominator_leave_collator(
				Origin::signed(1),
				101
			),
		);
		let nominator = Staking::nominators(&1).unwrap();
		assert_eq!(
			nominator.unbonding,
			vec![UnBondChunk {
				value: 500,
				era: 3
			}]
		);
		assert_eq!(
			nominator.nominations,
			vec![]
		);
		// the weight to count as vote be 0 after leave collator
		assert_eq!(
			nominator.total,
			0
		);
		// the balance still reserve until `BondingDuration`
		assert_eq!(Balances::reserved_balance(&1), 500);
		// One nomination remain after leave
		let ledger = Staking::ledger(&101).unwrap();
		assert_eq!(
			ledger.nominations,
			vec![Bond {
				owner: 2,
				amount: 1500
			}]
		);

		mock::start_active_era(3);
		// the balance unlock after `BondingDuration`
		let nomination = Staking::nominators(&1).unwrap();
		assert_eq!(
			nomination.unbonding,
			vec![]
		);
		assert_eq!(
			nomination.total,
			0
		);
		assert_eq!(Balances::reserved_balance(&1), 0);
	})
}
//
// #[test]
// fn payout_stakers_work() {
// 	mock_test().execute_with(|| {
// 		run_to_block(11);
// 		assert_ok!(
// 			Staking::nominate(
// 				Origin::signed(20),
// 				100,
// 				400
// 			),
// 		);
// 		assert_ok!(
// 			Staking::nominate(
// 				Origin::signed(3),
// 				300,
// 				800
// 			),
// 		);
//
// 		set_author(2, 100, 3000);
// 		set_author(2, 200, 2000);
// 		set_author(2, 300, 5000);
//
// 		run_to_block(31);
// 		// total stake = 2000
// 		// 200 earn = 500/2000 * 50% + 2000/10000 * 50% = 22.5%
// 		// 300 earn = 600/1600 * 50% + 5000/10000 * 50% = 40%
// 		// the rest for nominator but not display because of minimum balance
// 		// TODO: Make test more clear
// 		let expected = vec![
// 			crate::Event::CollatorChoosen(2, 200, 500),
// 			crate::Event::CollatorChoosen(2, 300, 600),
// 			crate::Event::Nominate(100,400),
// 			crate::Event::Nominate(300,800),
// 			crate::Event::CollatorChoosen(3, 100, 900),
// 			crate::Event::CollatorChoosen(3, 300, 1400),
// 			crate::Event::Rewarded(300, 3),
// 			crate::Event::Rewarded(200, 2),
// 			crate::Event::CollatorChoosen(4, 100, 900),
// 			crate::Event::CollatorChoosen(4, 300, 1400),
// 		];
// 		assert_eq!(events(), expected);
// 	})
// }
