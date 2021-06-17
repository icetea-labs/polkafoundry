use crate::*;
use frame_support::{
	assert_noop, assert_ok,
	traits::{Currency, ReservableCurrency},
};
use mock::*;
use substrate_test_utils::{assert_eq_uvec};
use sp_runtime::{assert_eq_error_rate, Perbill};
use sp_std::{ops::{Mul}};
use pallet_balances::Error as BalancesError;

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

#[test]
fn rewards_should_work() {
	three_collators_three_collators_count().execute_with(|| {
		let init_balance_10 = Balances::total_balance(&10);
		let init_balance_11 = Balances::total_balance(&11);
		let init_balance_20 = Balances::total_balance(&20);
		let init_balance_21 = Balances::total_balance(&21);
		let init_balance_100 = Balances::total_balance(&100);
		let init_balance_101 = Balances::total_balance(&101);

		// Set payees
		Payee::<Test>::insert(10, RewardDestination::Controller);
		Payee::<Test>::insert(20, RewardDestination::Controller);
		Payee::<Test>::insert(100, RewardDestination::Controller);

		let total_payout_0 = current_total_payout_for_duration(reward_time_per_era());

		<Pallet<Test>>::reward_by_ids(11, 20);
		<Pallet<Test>>::reward_by_ids(21, 20);
		<Pallet<Test>>::reward_by_ids(101, 20);

		start_active_era(1);

		<Pallet<Test>>::reward_by_ids(11, 20);
		<Pallet<Test>>::reward_by_ids(11, 20);
		<Pallet<Test>>::reward_by_ids(21, 20);
		<Pallet<Test>>::reward_by_ids(101, 20);
		<Pallet<Test>>::reward_by_ids(101, 20);
		<Pallet<Test>>::reward_by_ids(101, 20);

		assert_eq!(
			*events().last().unwrap(),
			crate::Event::EraPayout(0, total_payout_0)
		);

		assert_eq!(Balances::total_balance(&10), init_balance_10);
		assert_eq!(Balances::total_balance(&11), init_balance_11);
		assert_eq!(Balances::total_balance(&20), init_balance_20);
		assert_eq!(Balances::total_balance(&21), init_balance_21);
		assert_eq!(Balances::total_balance(&100), init_balance_100);
		assert_eq!(Balances::total_balance(&101), init_balance_101);

		assert_eq_uvec!(Session::validators(), vec![10, 20, 100]);

		start_active_era(2);

		// if you not set pref, the payout will be 50:50 between collator and nominator
		// that why the remain balance is /2 because we not test with nominator
		assert_eq_error_rate!(
			mock::REWARD_REMAINDER_UNBALANCED.with(|v| *v.borrow()),
			total_payout_0 / 2,
			5
		);

		let collator_payout_0 = total_payout_0 / 2;
		// its 80% stake part * 800/2700 % staking + 20% point part * % point
		let part_for_11_0 = collator_payout_0 * 32 / 135 + collator_payout_0 * 1 / 15;
		let part_for_21_0 = collator_payout_0 * 4 / 15 + collator_payout_0 * 1 / 15;
		let part_for_101_0 = collator_payout_0 * 8 / 27 + collator_payout_0 * 1 / 15;

		assert_eq!(Balances::total_balance(&10), init_balance_10);
		assert_eq_error_rate!(Balances::total_balance(&11), part_for_11_0, 3);
		assert_eq!(Balances::total_balance(&20), init_balance_20);
		assert_eq_error_rate!(Balances::total_balance(&21), part_for_21_0, 3);
		assert_eq!(Balances::total_balance(&100), init_balance_100);
		assert_eq_error_rate!(Balances::total_balance(&101),part_for_101_0, 3);

		let rest_0 = collator_payout_0 - part_for_11_0 - part_for_21_0 - part_for_101_0;
		start_active_era(3);

		// Compute total payout now for whole duration as other parameter won't change
		let total_payout_1 = current_total_payout_for_duration(reward_time_per_era());

		assert_eq_uvec!(Session::validators(), vec![10, 20, 100]);

		let collator_payout_1 = total_payout_1 / 2;
		let part_for_11_1 = collator_payout_1 * 32 / 135 + collator_payout_1 * 1 / 15;
		let part_for_21_1 = collator_payout_1 * 4 / 15 + collator_payout_1 * 1 / 30;
		let part_for_101_1 = collator_payout_1 * 8 / 27 + collator_payout_1 * 1 / 10;

		let rest_1 = collator_payout_1 - part_for_11_1 - part_for_21_1 - part_for_101_1;

		assert_eq!(Balances::total_balance(&10), init_balance_10);
		assert_eq_error_rate!(Balances::total_balance(&11), part_for_11_0 + part_for_11_1, 3);
		assert_eq!(Balances::total_balance(&20), init_balance_20);
		assert_eq_error_rate!(Balances::total_balance(&21), part_for_21_0 + part_for_21_1, 3);
		assert_eq!(Balances::total_balance(&100), init_balance_100);
		assert_eq_error_rate!(Balances::total_balance(&101), part_for_101_0 + part_for_101_1, 3);

		assert_eq_error_rate!(
			mock::REWARD_REMAINDER_UNBALANCED.with(|v| *v.borrow()),
			total_payout_0 / 2 + total_payout_1 / 2 + rest_0 + rest_1,
			5
		);
	})
}

#[test]
fn nominating_and_rewards_should_work() {
	four_collators_two_collators_count().execute_with(|| {
		// initial validators -- everyone is actually even.
		assert_eq_uvec!(validator_controllers(), vec![31, 41]);
		// Set payee to controller
		Payee::<Test>::insert(10, RewardDestination::Controller);
		Payee::<Test>::insert(20, RewardDestination::Controller);
		Payee::<Test>::insert(30, RewardDestination::Controller);

		// 10 take 80% rewards then 20% will share for nominator
		assert_ok!(
			Pallet::<Test>::validate(
				Origin::signed(11),
				CollatorPrefs {
					commission: Perbill::from_rational(80u32, 100u32),
					blocked: false
				}
			)
		);
		// 20 take all rewards and not share for nominator
		assert_ok!(
			Pallet::<Test>::validate(
				Origin::signed(21),
				CollatorPrefs {
					commission: Perbill::from_rational(100u32, 100u32),
					blocked: false
				}
			)
		);

		// give the man some money
		let initial_balance = 5000;
		for i in [1, 2, 3, 4].iter() {
			let _ = Balances::make_free_balance_be(i, initial_balance);
		}
		let init_balance_11 = Balances::total_balance(&11);

		// bond two account pairs and state interest in nomination.
		// 2 will nominate for 10, 20
		assert_ok!(Staking::bond(Origin::signed(1), 2, 1000, RewardDestination::Controller));
		assert_ok!(Staking::nominate(Origin::signed(2), 11, 1000));
		assert_ok!(Staking::nominate(Origin::signed(2), 21, 1000));
		assert_ok!(Staking::nominate(Origin::signed(2), 31, 200));
		// 4 will nominate for 10, 20, 100
		assert_ok!(Staking::bond(Origin::signed(3), 4, 1000, RewardDestination::Controller));
		assert_ok!(Staking::nominate(Origin::signed(4), 11, 1000));
		assert_ok!(Staking::nominate(Origin::signed(4), 21, 1000));
		assert_ok!(Staking::nominate(Origin::signed(4), 41, 300));

		mock::start_active_era(1);
		<Pallet<Test>>::reward_by_ids(11, 20);
		<Pallet<Test>>::reward_by_ids(21, 20);
		assert_eq!(
			Staking::eras_stakers_clipped(Staking::active_era().unwrap().index, 10),
			Exposure {
				total: 2867,
				own: 500,
				others: vec![
					IndividualExposure { who: 4, value: 1210 },
					IndividualExposure { who: 2, value: 1157 },
				]
			},
		);
		assert_eq!(
			Staking::eras_stakers_clipped(Staking::active_era().unwrap().index, 20),
			Exposure {
				total: 2633,
				own: 500,
				others: vec![
					IndividualExposure { who: 4, value: 1090 },
					IndividualExposure { who: 2, value: 1043 },
				]
			},
		);

		// 10 and 20 have more votes, they will be chosen.
		assert_eq_uvec!(validator_controllers(), vec![11, 21]);

		mock::start_active_era(2);

		// we start test in era 3 because nominate affect in next era
		mock::start_active_era(3);
		let total_payout_1 = current_total_payout_for_duration(reward_time_per_era());

		// just 80% * stake rate(2867/5500) + 20% point rate
		let part_for_11_1 = total_payout_1 * 2867 / 6875 + total_payout_1 * 1 / 10;
		// just 20% * stake rate(1210/2867) for nominate 11
		// 21 take all reward so 4 dont receive anything
		let part_for_4_1 = (part_for_11_1 * 2 / 10) * 1210 / 2867;
		assert_eq_error_rate!(Balances::total_balance(&4), 5000 + part_for_4_1, 2);
		let part_for_2_1 = (part_for_11_1 * 2 / 10) * 1157 / 2867;
		assert_eq_error_rate!(Balances::total_balance(&2), 5000 + part_for_2_1, 2);

		let part_for_21_1 = total_payout_1 * 2633 / 6875 + total_payout_1 * 1 / 10;
		assert_eq_error_rate!(Balances::total_balance(&11), part_for_11_1 * 8 / 10, 2);
		// 21 will take all reward
		assert_eq_error_rate!(Balances::total_balance(&21), part_for_21_1, 2);

	})
}

#[test]
fn double_controlling_should_fail() {
	// should test (in the same order):
	// * an account already bonded as controller CANNOT be reused as the controller of another account.
	mock_test().execute_with(|| {
		give_money(&1, 2000);
		give_money(&3, 2000);

		let arbitrary_value = 500;
		// 2 = controller, 1 stashed => ok
		assert_ok!(Staking::bond(
			Origin::signed(1),
			2,
			arbitrary_value,
			RewardDestination::default(),
		));
		// 2 = controller, 3 stashed (Note that 2 is reused.) => no-op
		assert_noop!(
			Staking::bond(Origin::signed(3), 2, arbitrary_value, RewardDestination::default()),
			Error::<Test>::AlreadyPaired,
		);
	});
}

#[test]
fn session_and_eras_work_simple() {
	mock_test().execute_with(|| {
		assert_eq!(active_era(), 0);
		assert_eq!(current_era(), 0);
		assert_eq!(Session::current_index(), 0);
		assert_eq!(System::block_number(), 1);

		// Session 1: this is basically a noop. This has already been started.
		start_session(1);
		assert_eq!(Session::current_index(), 1);
		assert_eq!(active_era(), 0);
		assert_eq!(System::block_number(), 5);

		// Session 2: No change.
		start_session(2);
		assert_eq!(Session::current_index(), 2);
		assert_eq!(active_era(), 0);
		assert_eq!(System::block_number(), 10);

		// Session 3: Era increment.
		start_session(3);
		assert_eq!(Session::current_index(), 3);
		assert_eq!(active_era(), 1);
		assert_eq!(System::block_number(), 15);

		// Session 4: No change.
		start_session(4);
		assert_eq!(Session::current_index(), 4);
		assert_eq!(active_era(), 1);
		assert_eq!(System::block_number(), 20);

		// Session 5: No change.
		start_session(5);
		assert_eq!(Session::current_index(), 5);
		assert_eq!(active_era(), 1);
		assert_eq!(System::block_number(), 25);

		// Session 6: Era increment.
		start_session(6);
		assert_eq!(Session::current_index(), 6);
		assert_eq!(active_era(), 2);
		assert_eq!(System::block_number(), 30);
	});
}

#[test]
fn forcing_new_era_works() {
	mock_test().execute_with(|| {
		// normal flow of session.
		start_session(1);
		assert_eq!(active_era(), 0);

		start_session(2);
		assert_eq!(active_era(), 0);

		start_session(3);
		assert_eq!(active_era(), 1);

		// no era change.
		ForceEra::<Test>::put(Forcing::ForceNone);

		start_session(4);
		assert_eq!(active_era(), 1);

		start_session(5);
		assert_eq!(active_era(), 1);

		start_session(6);
		assert_eq!(active_era(), 1);

		start_session(7);
		assert_eq!(active_era(), 1);

		// back to normal.
		// this immediately starts a new session.
		ForceEra::<Test>::put(Forcing::NotForcing);

		start_session(8);
		assert_eq!(active_era(), 1);

		start_session(9);
		assert_eq!(active_era(), 2);
		// forceful change
		ForceEra::<Test>::put(Forcing::ForceAlways);

		start_session(10);
		assert_eq!(active_era(), 2);

		start_session(11);
		assert_eq!(active_era(), 3);

		start_session(12);
		assert_eq!(active_era(), 4);

		// just one forceful change
		ForceEra::<Test>::put(Forcing::ForceNew);
		start_session(13);
		assert_eq!(active_era(), 5);
		assert_eq!(ForceEra::<Test>::get(), Forcing::NotForcing);

		start_session(14);
		assert_eq!(active_era(), 6);

		start_session(15);
		assert_eq!(active_era(), 6);

		start_session(16);
		assert_eq!(active_era(), 6);

		start_session(17);
		assert_eq!(active_era(), 7);
	});
}
