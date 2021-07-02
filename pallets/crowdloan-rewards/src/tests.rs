use crate::*;
use frame_support::{assert_noop, assert_ok};
use mock::*;

#[test]
fn init_reward_work() {
    // mock_test have already called initialize_reward successfully
    mock_test().execute_with(|| {
        assert_noop!(
            Crowdloan::initialize_reward(Origin::root(), vec![(1u64, 500), (2u64, 500)]),
            Error::<Test>::AlreadyInitReward
        );
    })
}

#[test]
fn claim_work() {
    // 1 is contributor, 11 not
    mock_test().execute_with(|| {
        // we mock tge_rate = 35%, period = 10 and account 1-2-3 contribute 5000
        // total: 5000 * 3 = 15000
        // claimed at tge each account: 5000 * 35% = 1750
        // total claimed: 3 * 1750 = 5250
        assert_eq!(
            Crowdloan::pot(),
            INIT_BALANCE - 3 * INIT_CONTRIBUTED * 35 / 100 - MINIMUM_BALANCE
        );
        assert_eq!(Crowdloan::contributors(1).unwrap().last_paid, 2u64);
        assert_noop!(
            Crowdloan::claim(Origin::signed(11)),
            Error::<Test>::NotContributedYet
        );
        assert_eq!(
            Crowdloan::settings(),
            Setting {
                tge_rate: 35,
                reward_start_block: 4,
                reward_end_block: 14
            }
        );
        assert_noop!(
            Crowdloan::claim(Origin::signed(1)),
            Error::<Test>::ClaimInLockedTime
        );
		run_to_relay_chain_block(6);
		assert_ok!(Crowdloan::claim(Origin::signed(1)));
        assert_noop!(
            Crowdloan::claim(Origin::signed(1)),
            Error::<Test>::ClaimAmountBelowMinimum
        );
        assert_eq!(Crowdloan::contributors(1).unwrap().last_paid, 6u64);
        // we mock tge_rate = 35%, period = 10 and account 1 contribute 5000
        // total: 5000
        // claimed at tge: 5000 * 35% = 1750
        // earn until block 6: ((5000 - 1750)) * (6-4)) / 10 = 650
        // total claimed: 1750 + 650 = 2400
        assert_eq!(Crowdloan::contributors(1).unwrap().claimed_reward, 2400);
		run_to_relay_chain_block(9);
		assert_ok!(Crowdloan::claim(Origin::signed(1)));
        // total: 5000
        // claimed: 2400
        // earn from block 6 to block 10: ((5000 - 1750) * (9 - 6)) / 10 = 975
        // total claimed: 2400 + 975 = 3375
        assert_eq!(Crowdloan::contributors(1).unwrap().claimed_reward, 3375);
		run_to_relay_chain_block(20);
		assert_ok!(Crowdloan::claim(Origin::signed(1)));
        assert_eq!(Crowdloan::contributors(1).unwrap().claimed_reward, 5000);
        assert_noop!(
            Crowdloan::claim(Origin::signed(1)),
            Error::<Test>::AlreadyPaid
        );
        let expected = vec![
            crate::Event::RewardPaid(1, 1750),
            crate::Event::RewardPaid(2, 1750),
            crate::Event::RewardPaid(3, 1750),
            crate::Event::RewardPaid(1, 650),
            crate::Event::RewardPaid(1, 975),
            crate::Event::RewardPaid(1, 1625),
        ];
        assert_eq!(events(), expected);
    })
}

#[test]
fn distribute_all_work() {
	mock_test().execute_with(|| {
		// we mock tge_rate = 35%, period = 10 and account 1-2-3 contribute 5000
		// total: 5000 * 3 = 15000
		// claimed at tge each account: 5000 * 35% = 1750
		// total claimed: 3 * 1750 = 5250
		assert_eq!(
			Crowdloan::pot(),
			INIT_BALANCE - 3 * INIT_CONTRIBUTED * 35 / 100 - MINIMUM_BALANCE
		);
		run_to_relay_chain_block(6);
		assert_ok!(Crowdloan::claim(Origin::signed(1)));
		// we mock tge_rate = 35%, period = 10 and account 1 contribute 5000
		// total: 5000
		// claimed at tge: 5000 * 35% = 1750
		// earn until block 6: ((5000 - 1750)) * (6-4)) / 10 = 650
		// total claimed: 1750 + 650 = 2400
		assert_eq!(Crowdloan::contributors(1).unwrap().claimed_reward, 2400);
		assert_noop!(
			Crowdloan::distribute_all(Origin::root()),
			Error::<Test>::DistributeNotReady,
		);
		run_to_relay_chain_block(20);
		assert_ok!(Crowdloan::distribute_all(Origin::root()));
		let expected = vec![
			crate::Event::RewardPaid(1, 1750),
			crate::Event::RewardPaid(2, 1750),
			crate::Event::RewardPaid(3, 1750),
			crate::Event::RewardPaid(1, 650),
			// distribute all the remaining amount after end vesting period
			crate::Event::RewardPaid(3, 3250),
			crate::Event::RewardPaid(1, 2600),
			crate::Event::RewardPaid(2, 3250),
		];
		assert_eq!(events(), expected);
		// avoid transfer 0 amount of tokens to users
		assert_ok!(Crowdloan::distribute_all(Origin::root()));
		assert_eq!(events(), expected);
	})
}

#[test]
fn update_setting_work() {
    // mock_test have already called initialize_reward successfully
    mock_test().execute_with(|| {
        assert_eq!(
            Crowdloan::settings(),
            Setting {
                tge_rate: 35,
                reward_start_block: 4,
                reward_end_block: 14
            }
        );
        assert_ok!(Crowdloan::config(
            Origin::root(),
            Setting {
                tge_rate: 0, // Don't modify if tge_rate = 0
                reward_start_block: 5,
                reward_end_block: 7
            }
        ));

        assert_eq!(
            Crowdloan::settings(),
            Setting {
                tge_rate: 35,
                reward_start_block: 5,
                reward_end_block: 7
            }
        );

		assert_ok!(Crowdloan::config(
            Origin::root(),
            Setting {
                tge_rate: 10,
                reward_start_block: 8, // Don't modify lock duration if start_block > end_block
                reward_end_block: 6
            }
        ));

		assert_eq!(
			Crowdloan::settings(),
			Setting {
				tge_rate: 10,
				reward_start_block: 5,
				reward_end_block: 7
			}
		);

		assert_ok!(Crowdloan::config(
            Origin::root(),
            Setting {
                tge_rate: 0,
                reward_start_block: 0, // Don't modify lock duration if start_block = 0
                reward_end_block: 6
            }
        ));

		assert_eq!(
			Crowdloan::settings(),
			Setting {
				tge_rate: 10,
				reward_start_block: 5,
				reward_end_block: 7
			}
		);
    })
}
