use super::mock::*;
use crate::Error;
use frame_support::{assert_err, assert_ok};

#[test]
fn set_value_ok() {
	new_test_ext().execute_with(|| {
		assert_ok!(Flipper::set_value(Origin::signed(ALICE), false));
		assert_eq!(Flipper::value(), Some(false));
	});
}

#[test]
fn set_value_err_already_set() {
	new_test_ext().execute_with(|| {
		assert_ok!(Flipper::set_value(Origin::signed(ALICE), false));
		assert_err!(
			Flipper::set_value(Origin::signed(ALICE), true),
			Error::<TestRuntime>::AlreadySet
		);
	});
}

#[test]
fn flip_value_ok() {
	new_test_ext().execute_with(|| {
		assert_ok!(Flipper::set_value(Origin::signed(ALICE), true));
		assert_ok!(Flipper::flip_value(Origin::signed(ALICE)));
		assert_eq!(Flipper::value(), Some(false));
	});
}

#[test]
fn flip_value_ko() {
	new_test_ext().execute_with(|| {
		assert_err!(
			Flipper::flip_value(Origin::signed(ALICE)),
			Error::<TestRuntime>::NoneValue
		);
	});
}
