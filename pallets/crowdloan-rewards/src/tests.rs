use crate::*;
use frame_support::{assert_noop, assert_ok};
use mock::*;
use codec::Encode;
use sp_core::Pair;
use sp_runtime::MultiSignature;

#[test]
fn init_reward_work () {
	mock_test().execute_with(|| {
		assert_noop!(
			Crowdloan::initialize_reward(
				Origin::root(),
				vec![
					([1u8; 32].into(), 500),
					([2u8; 32].into(), 500)
				],
				10,
				10
			),
			Error::<Test>::AlreadyInitReward
		);
		run_to_block(10);
		assert_noop!(
				Crowdloan::initialize_reward(
				Origin::root(),
				vec![
					([1u8; 32].into(), 500),
					([2u8; 32].into(), 500)
				],
				10,
				10
			),
			Error::<Test>::InvalidEndBlock
		);
		assert_ok!(
				Crowdloan::initialize_reward(
				Origin::root(),
				vec![
					([1u8; 32].into(), 500),
					([2u8; 32].into(), 500)
				],
				10,
				20
			),
		);
	})
}

#[test]
fn associate_account_work() {
	let pairs = get_ed25519_pairs(1);
	let proof: MultiSignature = pairs[0].sign(&1u64.encode()).into();
	mock_test().execute_with(|| {
		assert_noop!(
			Crowdloan::associate_account(
				Origin::signed(2),
				pairs[0].public().into(),
				proof.clone()
			),
			Error::<Test>::InvalidSignature
		);
		assert_ok!(
			Crowdloan::associate_account(
				Origin::signed(1),
				pairs[0].public().into(),
				proof.clone()
			)
		);
		assert_noop!(
			Crowdloan::associate_account(
				Origin::signed(1),
				pairs[0].public().into(),
				proof.clone()
			),
			Error::<Test>::AlreadyAssociated
		);
		let expected = vec![crate::Event::AssociatedAccount(
			1,
			pairs[0].public().into(),
		)];
		assert_eq!(events(), expected);
	})
}

#[test]
fn get_money_work() {
	let pairs = get_ed25519_pairs(2);
	let proof: MultiSignature = pairs[0].sign(&1u64.encode()).into();
	let proof1: MultiSignature = pairs[1].sign(&11u64.encode()).into();
	let relay_account = pairs[0].public().into();
	// 1 is contributor, 11 not
	mock_test().execute_with(|| {
		// user 100 donate fund to Treasury
		assert_ok!(Treasury::donate(Origin::signed(100), 10_000_000));

		assert_noop!(
			Crowdloan::get_money(
				Origin::signed(1),
			),
			Error::<Test>::NoAssociatedAccount
		);
		Crowdloan::associate_account(
			Origin::signed(11),
			pairs[1].public().into(),
			proof1.clone()
		).unwrap();
		assert_noop!(
			Crowdloan::get_money(
				Origin::signed(11),
			),
			Error::<Test>::NotContributedYet
		);
		Crowdloan::associate_account(
			Origin::signed(1),
			relay_account,
			proof.clone()
		).unwrap();
		run_to_block(2);
		assert_ok!(Crowdloan::get_money(
				Origin::signed(1),
		));
		assert_noop!(
			Crowdloan::get_money(
				Origin::signed(1),
			),
			Error::<Test>::ScantyReward
		);
		assert_eq!(
			Crowdloan::contributors(&relay_account).unwrap().last_paid,
			2u64
		);
		// we mock rate = 10 and period = 10 and pair1 contribute 500
		// earn (500 * 10)/10 = 500 token per block
		assert_eq!(
			Crowdloan::contributors(&relay_account).unwrap().claimed_reward,
			1000
		);
		run_to_block(8);
		assert_ok!(Crowdloan::get_money(
				Origin::signed(1),
		));
		assert_eq!(
			Crowdloan::contributors(&relay_account).unwrap().claimed_reward,
			4000
		);
		run_to_block(11);
		assert_ok!(Crowdloan::get_money(
				Origin::signed(1),
		));
		assert_eq!(
			Crowdloan::contributors(&relay_account).unwrap().claimed_reward,
			5000
		);
		assert_noop!(
			Crowdloan::get_money(
				Origin::signed(1),
			),
			Error::<Test>::AlreadyPaid
		);
		let expected = vec![
			crate::Event::AssociatedAccount(11, pairs[1].public().into()),
			crate::Event::AssociatedAccount(1, relay_account),
			crate::Event::RewardPaid(1, 1000),
			crate::Event::RewardPaid(1, 3000),
			crate::Event::RewardPaid(1, 1000),
		];
		assert_eq!(events(), expected);
	})
}

#[test]
fn update_associate_account_work() {
	let pairs = get_ed25519_pairs(2);
	let proof: MultiSignature = pairs[0].sign(&1u64.encode()).into();
	let proof1: MultiSignature = pairs[1].sign(&2u64.encode()).into();
	let proof2: MultiSignature = pairs[0].sign(&3u64.encode()).into();
	mock_test().execute_with(|| {
		// user 100 donate fund to Treasury
		assert_ok!(Treasury::donate(Origin::signed(100), 10_000_000));

		Crowdloan::associate_account(
			Origin::signed(1),
			pairs[0].public().into(),
			proof.clone()
		).unwrap();
		// user 2 try to get reward of user 1 by input address of user 1
		assert_noop!(
			Crowdloan::update_associate_account(
				Origin::signed(2),
				1,
				pairs[1].public().into(),
				proof1.clone()
			),
			Error::<Test>::BadRelayAccount
		);
		assert_noop!(
			Crowdloan::update_associate_account(
				Origin::signed(2),
				1,
				pairs[0].public().into(),
				proof1.clone()
			),
			Error::<Test>::InvalidSignature
		);
		assert_noop!(
			Crowdloan::update_associate_account(
				Origin::signed(2),
				2,
				pairs[1].public().into(),
				proof1.clone()
			),
			Error::<Test>::NoAssociatedAccount
		);
		run_to_block(5);
		Crowdloan::get_money(
			Origin::signed(1),
		).unwrap();
		// the only way to update is sign proof by own relay account and input correct associated account
		assert_ok!(
			Crowdloan::update_associate_account(
				Origin::signed(3),
				1,
				pairs[0].public().into(),
				proof2.clone()
			),
		);
		run_to_block(6);
		assert_ok!(
			Crowdloan::get_money(
				Origin::signed(3)
			)
		);
		assert_noop!(
			Crowdloan::get_money(
				Origin::signed(1)
			),
			Error::<Test>::NoAssociatedAccount
		);
	})
}
