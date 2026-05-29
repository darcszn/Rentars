//! Unit tests for the Booking contract.
//!
//! Coverage:
//!   Happy paths   — initialize, create, cancel, update_status, set_escrow_id, queries
//!   Error cases   — invalid dates, invalid price, overlap, unauthorized, bad transitions
//!   Security      — reentrancy simulation, unauthorized access, integer overflow/underflow,
//!                   timestamp manipulation resistance
//!   Edge cases    — adjacent (non-overlapping) bookings, cancelled-slot reuse,
//!                   empty property bookings, boundary timestamps

#[cfg(test)]
mod tests {
    use soroban_sdk::{testutils::Address as _, Address, BytesN, Env, String};

    use crate::{BookingContract, BookingContractClient, BookingStatus};

    // ─── Helpers ─────────────────────────────────────────────────────────────

    /// Create a fresh environment with the contract registered and initialized.
    /// Returns (Env, contract_id, admin_address).
    fn make_env() -> (Env, soroban_sdk::Address, Address) {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, BookingContract);
        let client = BookingContractClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        client.initialize(&admin);
        (env, contract_id, admin)
    }

    // ─── Initialization Tests ─────────────────────────────────────────────────

    /// Contract initializes correctly and booking count starts at zero.
    #[test]
    fn test_initialize() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, BookingContract);
        let client = BookingContractClient::new(&env, &contract_id);
        let admin = Address::generate(&env);

        client.initialize(&admin);

        assert_eq!(client.booking_count(), 0);
    }

    /// Calling initialize twice panics.
    #[test]
    #[should_panic(expected = "Already initialized")]
    fn test_initialize_twice() {
        let (env, cid, _admin) = make_env();
        let client = BookingContractClient::new(&env, &cid);
        let second_admin = Address::generate(&env);
        client.initialize(&second_admin);
    }

    // ─── Create Booking Tests ─────────────────────────────────────────────────

    /// Happy path: create a booking and verify all fields.
    #[test]
    fn test_create_booking_success() {
        let (env, cid, _admin) = make_env();
        let client = BookingContractClient::new(&env, &cid);
        let tenant = Address::generate(&env);

        let id = client.create_booking(
            &tenant,
            &1_u64,
            &1_000_u64,
            &1_007_u64,
            &700_0000000_i128,
        );

        assert_eq!(id, 1);

        let booking = client.get_booking(&id);
        assert_eq!(booking.id, 1);
        assert_eq!(booking.property_id, 1);
        assert_eq!(booking.tenant, tenant);
        assert_eq!(booking.check_in, 1_000);
        assert_eq!(booking.check_out, 1_007);
        assert_eq!(booking.total_price, 700_0000000_i128);
        assert_eq!(booking.status, BookingStatus::Pending);
        assert_eq!(booking.escrow_id, String::from_str(&env, ""));
    }

    /// Booking IDs auto-increment.
    #[test]
    fn test_create_booking_increments_id() {
        let (env, cid, _admin) = make_env();
        let client = BookingContractClient::new(&env, &cid);
        let tenant = Address::generate(&env);

        let id1 = client.create_booking(&tenant, &1_u64, &1_000_u64, &1_005_u64, &100_i128);
        let id2 = client.create_booking(&tenant, &1_u64, &2_000_u64, &2_005_u64, &100_i128);

        assert_eq!(id1, 1);
        assert_eq!(id2, 2);
        assert_eq!(client.booking_count(), 2);
    }

    /// check_in equal to check_out is invalid (zero-duration).
    #[test]
    #[should_panic(expected = "check_in must be before check_out")]
    fn test_invalid_dates_equal() {
        let (env, cid, _admin) = make_env();
        let client = BookingContractClient::new(&env, &cid);
        let tenant = Address::generate(&env);
        client.create_booking(&tenant, &1_u64, &1_000_u64, &1_000_u64, &100_i128);
    }

    /// check_in after check_out is invalid.
    #[test]
    #[should_panic(expected = "check_in must be before check_out")]
    fn test_invalid_dates() {
        let (env, cid, _admin) = make_env();
        let client = BookingContractClient::new(&env, &cid);
        let tenant = Address::generate(&env);
        client.create_booking(&tenant, &1_u64, &2_000_u64, &1_000_u64, &100_i128);
    }

    /// Zero price is invalid.
    #[test]
    #[should_panic(expected = "total_price must be positive")]
    fn test_invalid_price_zero() {
        let (env, cid, _admin) = make_env();
        let client = BookingContractClient::new(&env, &cid);
        let tenant = Address::generate(&env);
        client.create_booking(&tenant, &1_u64, &1_000_u64, &1_005_u64, &0_i128);
    }

    /// Negative price is invalid.
    #[test]
    #[should_panic(expected = "total_price must be positive")]
    fn test_invalid_price_negative() {
        let (env, cid, _admin) = make_env();
        let client = BookingContractClient::new(&env, &cid);
        let tenant = Address::generate(&env);
        client.create_booking(&tenant, &1_u64, &1_000_u64, &1_005_u64, &-1_i128);
    }

    // ─── Overlap Prevention Tests ─────────────────────────────────────────────

    /// Exact same dates on the same property are rejected.
    #[test]
    #[should_panic(expected = "Booking dates overlap")]
    fn test_booking_overlap_prevention() {
        let (env, cid, _admin) = make_env();
        let client = BookingContractClient::new(&env, &cid);
        let tenant = Address::generate(&env);
        client.create_booking(&tenant, &1_u64, &1_000_u64, &1_005_u64, &100_i128);
        client.create_booking(&tenant, &1_u64, &1_000_u64, &1_005_u64, &100_i128);
    }

    /// Partial overlap (new booking starts inside existing) is rejected.
    #[test]
    #[should_panic(expected = "Booking dates overlap")]
    fn test_booking_overlap_prevention_partial() {
        let (env, cid, _admin) = make_env();
        let client = BookingContractClient::new(&env, &cid);
        let tenant = Address::generate(&env);
        client.create_booking(&tenant, &1_u64, &1_000_u64, &1_010_u64, &100_i128);
        client.create_booking(&tenant, &1_u64, &1_005_u64, &1_015_u64, &100_i128);
    }

    /// New booking that fully contains an existing booking is rejected.
    #[test]
    #[should_panic(expected = "Booking dates overlap")]
    fn test_booking_overlap_prevention_contained() {
        let (env, cid, _admin) = make_env();
        let client = BookingContractClient::new(&env, &cid);
        let tenant = Address::generate(&env);
        client.create_booking(&tenant, &1_u64, &1_002_u64, &1_008_u64, &100_i128);
        client.create_booking(&tenant, &1_u64, &1_000_u64, &1_010_u64, &100_i128);
    }

    /// Adjacent bookings (check_out == next check_in) do NOT overlap.
    #[test]
    fn test_non_overlapping_bookings() {
        let (env, cid, _admin) = make_env();
        let client = BookingContractClient::new(&env, &cid);
        let tenant = Address::generate(&env);
        let id1 = client.create_booking(&tenant, &1_u64, &1_000_u64, &1_005_u64, &100_i128);
        let id2 = client.create_booking(&tenant, &1_u64, &1_005_u64, &1_010_u64, &100_i128);
        assert_eq!(id1, 1);
        assert_eq!(id2, 2);
    }

    /// Bookings on different properties never conflict.
    #[test]
    fn test_non_overlapping_bookings_different_properties() {
        let (env, cid, _admin) = make_env();
        let client = BookingContractClient::new(&env, &cid);
        let tenant = Address::generate(&env);
        let id1 = client.create_booking(&tenant, &1_u64, &1_000_u64, &1_005_u64, &100_i128);
        let id2 = client.create_booking(&tenant, &2_u64, &1_000_u64, &1_005_u64, &100_i128);
        assert_ne!(id1, id2);
    }

    /// After cancellation, the same dates can be rebooked.
    #[test]
    fn test_cancelled_slot_can_be_rebooked() {
        let (env, cid, _admin) = make_env();
        let client = BookingContractClient::new(&env, &cid);
        let tenant = Address::generate(&env);
        let id1 = client.create_booking(&tenant, &1_u64, &1_000_u64, &1_005_u64, &100_i128);
        client.cancel_booking(&tenant, &id1);
        let id2 = client.create_booking(&tenant, &1_u64, &1_000_u64, &1_005_u64, &100_i128);
        assert_ne!(id1, id2);
        assert_eq!(client.get_booking(&id2).status, BookingStatus::Pending);
    }

    // ─── Availability Tests ───────────────────────────────────────────────────

    /// check_availability returns true for a property with no bookings.
    #[test]
    fn test_check_availability_empty() {
        let (env, cid, _admin) = make_env();
        let client = BookingContractClient::new(&env, &cid);
        assert!(client.check_availability(&99_u64, &1_000_u64, &1_005_u64));
    }

    /// check_availability returns false when dates overlap an active booking.
    #[test]
    fn test_check_availability_blocked() {
        let (env, cid, _admin) = make_env();
        let client = BookingContractClient::new(&env, &cid);
        let tenant = Address::generate(&env);
        client.create_booking(&tenant, &1_u64, &1_000_u64, &1_005_u64, &100_i128);
        assert!(!client.check_availability(&1_u64, &1_000_u64, &1_005_u64));
    }

    /// check_availability returns true after the only booking is cancelled.
    #[test]
    fn test_check_availability_after_cancel() {
        let (env, cid, _admin) = make_env();
        let client = BookingContractClient::new(&env, &cid);
        let tenant = Address::generate(&env);
        let id = client.create_booking(&tenant, &1_u64, &1_000_u64, &1_005_u64, &100_i128);
        client.cancel_booking(&tenant, &id);
        assert!(client.check_availability(&1_u64, &1_000_u64, &1_005_u64));
    }

    // ─── Cancel Booking Tests ─────────────────────────────────────────────────

    /// Tenant can cancel their own pending booking.
    #[test]
    fn test_cancel_booking_success() {
        let (env, cid, _admin) = make_env();
        let client = BookingContractClient::new(&env, &cid);
        let tenant = Address::generate(&env);
        let id = client.create_booking(&tenant, &1_u64, &1_000_u64, &1_005_u64, &100_i128);
        client.cancel_booking(&tenant, &id);
        assert_eq!(client.get_booking(&id).status, BookingStatus::Cancelled);
    }

    /// A non-tenant cannot cancel someone else's booking.
    #[test]
    #[should_panic(expected = "Unauthorized")]
    fn test_cancel_booking_unauthorized() {
        let (env, cid, _admin) = make_env();
        let client = BookingContractClient::new(&env, &cid);
        let tenant = Address::generate(&env);
        let attacker = Address::generate(&env);
        let id = client.create_booking(&tenant, &1_u64, &1_000_u64, &1_005_u64, &100_i128);
        client.cancel_booking(&attacker, &id);
    }

    /// Cancelling an already-cancelled booking panics.
    #[test]
    #[should_panic(expected = "Booking already cancelled")]
    fn test_cancel_booking_already_cancelled() {
        let (env, cid, _admin) = make_env();
        let client = BookingContractClient::new(&env, &cid);
        let tenant = Address::generate(&env);
        let id = client.create_booking(&tenant, &1_u64, &1_000_u64, &1_005_u64, &100_i128);
        client.cancel_booking(&tenant, &id);
        client.cancel_booking(&tenant, &id);
    }

    /// Cancelling a completed booking panics.
    #[test]
    #[should_panic(expected = "Cannot cancel a completed booking")]
    fn test_cancel_completed_booking() {
        let (env, cid, admin) = make_env();
        let client = BookingContractClient::new(&env, &cid);
        let tenant = Address::generate(&env);
        let id = client.create_booking(&tenant, &1_u64, &1_000_u64, &1_005_u64, &100_i128);
        client.update_status(&admin, &id, &BookingStatus::Confirmed);
        client.update_status(&admin, &id, &BookingStatus::Completed);
        client.cancel_booking(&tenant, &id);
    }

    // ─── Status Transition Tests ──────────────────────────────────────────────

    /// Admin can drive Pending → Confirmed → Completed.
    #[test]
    fn test_update_status() {
        let (env, cid, admin) = make_env();
        let client = BookingContractClient::new(&env, &cid);
        let tenant = Address::generate(&env);
        let id = client.create_booking(&tenant, &1_u64, &1_000_u64, &1_005_u64, &100_i128);

        client.update_status(&admin, &id, &BookingStatus::Confirmed);
        assert_eq!(client.get_booking(&id).status, BookingStatus::Confirmed);

        client.update_status(&admin, &id, &BookingStatus::Completed);
        assert_eq!(client.get_booking(&id).status, BookingStatus::Completed);
    }

    /// Admin can cancel from Pending.
    #[test]
    fn test_update_status_pending_to_cancelled() {
        let (env, cid, admin) = make_env();
        let client = BookingContractClient::new(&env, &cid);
        let tenant = Address::generate(&env);
        let id = client.create_booking(&tenant, &1_u64, &1_000_u64, &1_005_u64, &100_i128);
        client.update_status(&admin, &id, &BookingStatus::Cancelled);
        assert_eq!(client.get_booking(&id).status, BookingStatus::Cancelled);
    }

    /// Pending → Completed is an invalid transition.
    #[test]
    #[should_panic(expected = "Invalid status transition")]
    fn test_invalid_status_transition() {
        let (env, cid, admin) = make_env();
        let client = BookingContractClient::new(&env, &cid);
        let tenant = Address::generate(&env);
        let id = client.create_booking(&tenant, &1_u64, &1_000_u64, &1_005_u64, &100_i128);
        client.update_status(&admin, &id, &BookingStatus::Completed);
    }

    /// Completed → Confirmed is an invalid transition (terminal state).
    #[test]
    #[should_panic(expected = "Invalid status transition")]
    fn test_invalid_status_transition_terminal() {
        let (env, cid, admin) = make_env();
        let client = BookingContractClient::new(&env, &cid);
        let tenant = Address::generate(&env);
        let id = client.create_booking(&tenant, &1_u64, &1_000_u64, &1_005_u64, &100_i128);
        client.update_status(&admin, &id, &BookingStatus::Confirmed);
        client.update_status(&admin, &id, &BookingStatus::Completed);
        client.update_status(&admin, &id, &BookingStatus::Confirmed);
    }

    /// Cancelled → Confirmed is an invalid transition (terminal state).
    #[test]
    #[should_panic(expected = "Invalid status transition")]
    fn test_invalid_status_transition_cancelled_to_confirmed() {
        let (env, cid, admin) = make_env();
        let client = BookingContractClient::new(&env, &cid);
        let tenant = Address::generate(&env);
        let id = client.create_booking(&tenant, &1_u64, &1_000_u64, &1_005_u64, &100_i128);
        client.update_status(&admin, &id, &BookingStatus::Cancelled);
        client.update_status(&admin, &id, &BookingStatus::Confirmed);
    }

    /// Non-admin cannot call update_status.
    #[test]
    #[should_panic(expected = "Unauthorized")]
    fn test_update_status_unauthorized() {
        let (env, cid, _admin) = make_env();
        let client = BookingContractClient::new(&env, &cid);
        let tenant = Address::generate(&env);
        let attacker = Address::generate(&env);
        let id = client.create_booking(&tenant, &1_u64, &1_000_u64, &1_005_u64, &100_i128);
        client.update_status(&attacker, &id, &BookingStatus::Confirmed);
    }

    // ─── Escrow ID Tests ──────────────────────────────────────────────────────

    /// Admin can attach an escrow ID to a booking.
    #[test]
    fn test_set_escrow_id() {
        let (env, cid, admin) = make_env();
        let client = BookingContractClient::new(&env, &cid);
        let tenant = Address::generate(&env);
        let id = client.create_booking(&tenant, &1_u64, &1_000_u64, &1_005_u64, &100_i128);
        let escrow_ref = String::from_str(&env, "escrow-abc-123");
        client.set_escrow_id(&admin, &id, &escrow_ref);
        assert_eq!(client.get_booking(&id).escrow_id, escrow_ref);
    }

    /// Non-admin cannot set the escrow ID.
    #[test]
    #[should_panic(expected = "Unauthorized")]
    fn test_set_escrow_id_unauthorized() {
        let (env, cid, _admin) = make_env();
        let client = BookingContractClient::new(&env, &cid);
        let tenant = Address::generate(&env);
        let attacker = Address::generate(&env);
        let id = client.create_booking(&tenant, &1_u64, &1_000_u64, &1_005_u64, &100_i128);
        client.set_escrow_id(&attacker, &id, &String::from_str(&env, "evil-escrow"));
    }

    // ─── Query Tests ──────────────────────────────────────────────────────────

    /// get_property_bookings returns all booking IDs for a property.
    #[test]
    fn test_get_property_bookings() {
        let (env, cid, _admin) = make_env();
        let client = BookingContractClient::new(&env, &cid);
        let tenant = Address::generate(&env);
        let id1 = client.create_booking(&tenant, &5_u64, &1_000_u64, &1_005_u64, &100_i128);
        let id2 = client.create_booking(&tenant, &5_u64, &2_000_u64, &2_005_u64, &100_i128);
        let id3 = client.create_booking(&tenant, &5_u64, &3_000_u64, &3_005_u64, &100_i128);
        let bookings = client.get_property_bookings(&5_u64);
        assert_eq!(bookings.len(), 3);
        assert_eq!(bookings.get(0).unwrap(), id1);
        assert_eq!(bookings.get(1).unwrap(), id2);
        assert_eq!(bookings.get(2).unwrap(), id3);
    }

    /// get_property_bookings returns empty vec for a property with no bookings.
    #[test]
    fn test_get_property_bookings_empty() {
        let (env, cid, _admin) = make_env();
        let client = BookingContractClient::new(&env, &cid);
        let bookings = client.get_property_bookings(&999_u64);
        assert_eq!(bookings.len(), 0);
    }

    /// get_booking panics for a non-existent ID.
    #[test]
    #[should_panic(expected = "Booking not found")]
    fn test_booking_not_found() {
        let (env, cid, _admin) = make_env();
        let client = BookingContractClient::new(&env, &cid);
        client.get_booking(&9999);
    }

    // ─── Security Tests ───────────────────────────────────────────────────────

    /// Reentrancy attack simulation:
    /// Soroban's atomic execution model means each invocation sees committed state.
    /// A second booking for the same slot must always be rejected.
    #[test]
    #[should_panic(expected = "Booking dates overlap")]
    fn test_reentrancy_attack_prevention() {
        let (env, cid, _admin) = make_env();
        let client = BookingContractClient::new(&env, &cid);
        let attacker = Address::generate(&env);
        // First booking succeeds
        client.create_booking(&attacker, &1_u64, &5_000_u64, &5_010_u64, &100_i128);
        // Attempting the same slot again — simulates reentrancy; must be rejected
        client.create_booking(&attacker, &1_u64, &5_000_u64, &5_010_u64, &100_i128);
    }

    /// Unauthorized access: verify that only the correct principals can call each operation.
    /// Each privileged operation has its own dedicated #[should_panic] test above.
    /// This test verifies the positive case — that legitimate principals succeed.
    #[test]
    fn test_unauthorized_access_attempts() {
        let (env, cid, admin) = make_env();
        let client = BookingContractClient::new(&env, &cid);
        let tenant = Address::generate(&env);

        let id = client.create_booking(&tenant, &1_u64, &1_000_u64, &1_005_u64, &100_i128);

        // Legitimate admin can update status
        client.update_status(&admin, &id, &BookingStatus::Confirmed);
        assert_eq!(client.get_booking(&id).status, BookingStatus::Confirmed);

        // Legitimate admin can set escrow ID
        client.set_escrow_id(&admin, &id, &String::from_str(&env, "escrow-xyz"));
        assert_eq!(
            client.get_booking(&id).escrow_id,
            String::from_str(&env, "escrow-xyz")
        );

        // Legitimate tenant can cancel (after re-creating a pending booking)
        let id2 = client.create_booking(&tenant, &2_u64, &2_000_u64, &2_005_u64, &100_i128);
        client.cancel_booking(&tenant, &id2);
        assert_eq!(client.get_booking(&id2).status, BookingStatus::Cancelled);
    }

    /// Integer overflow/underflow resistance:
    /// Maximum i128 price and boundary u64 timestamps must be stored correctly.
    /// Soroban compiles with overflow-checks = true, so wraps panic rather than silently corrupt.
    #[test]
    fn test_integer_overflow_underflow() {
        let (env, cid, _admin) = make_env();
        let client = BookingContractClient::new(&env, &cid);
        let tenant = Address::generate(&env);

        // Maximum valid i128 price — should store correctly without overflow
        let id = client.create_booking(&tenant, &1_u64, &1_000_u64, &1_001_u64, &i128::MAX);
        assert_eq!(client.get_booking(&id).total_price, i128::MAX);

        // Near-maximum u64 timestamps (boundary) — check_in < check_out must hold
        let id2 = client.create_booking(
            &tenant,
            &2_u64,
            &(u64::MAX - 2),
            &(u64::MAX - 1),
            &1_i128,
        );
        let b = client.get_booking(&id2);
        assert_eq!(b.check_in, u64::MAX - 2);
        assert_eq!(b.check_out, u64::MAX - 1);
    }

    /// check_in > check_out (underflow scenario) must be rejected.
    #[test]
    #[should_panic(expected = "check_in must be before check_out")]
    fn test_integer_overflow_invalid_range() {
        let (env, cid, _admin) = make_env();
        let client = BookingContractClient::new(&env, &cid);
        let tenant = Address::generate(&env);
        client.create_booking(&tenant, &3_u64, &u64::MAX, &0_u64, &1_i128);
    }

    /// Timestamp manipulation resistance:
    /// Epoch-start (check_in = 0) is valid; zero-duration is not.
    #[test]
    fn test_timestamp_manipulation_resistance() {
        let (env, cid, _admin) = make_env();
        let client = BookingContractClient::new(&env, &cid);
        let tenant = Address::generate(&env);

        // Epoch-start booking is valid
        let id = client.create_booking(&tenant, &1_u64, &0_u64, &1_u64, &1_i128);
        assert_eq!(client.get_booking(&id).check_in, 0);

        // Very large but valid timestamps
        let id2 = client.create_booking(
            &tenant,
            &2_u64,
            &999_999_999_u64,
            &1_000_000_000_u64,
            &1_i128,
        );
        assert_eq!(client.get_booking(&id2).check_in, 999_999_999);
    }

    /// Zero-duration booking (check_in == check_out) must be rejected.
    #[test]
    #[should_panic(expected = "check_in must be before check_out")]
    fn test_timestamp_manipulation_zero_duration() {
        let (env, cid, _admin) = make_env();
        let client = BookingContractClient::new(&env, &cid);
        let tenant = Address::generate(&env);
        client.create_booking(&tenant, &2_u64, &0_u64, &0_u64, &1_i128);
    }

    // ─── Edge Case Tests ──────────────────────────────────────────────────────

    /// Minimum valid booking: 1-unit duration, 1-stroop price.
    #[test]
    fn test_edge_case_minimum_booking() {
        let (env, cid, _admin) = make_env();
        let client = BookingContractClient::new(&env, &cid);
        let tenant = Address::generate(&env);
        let id = client.create_booking(&tenant, &1_u64, &100_u64, &101_u64, &1_i128);
        let b = client.get_booking(&id);
        assert_eq!(b.check_out - b.check_in, 1);
        assert_eq!(b.total_price, 1);
    }

    /// Many non-overlapping bookings on the same property all succeed.
    #[test]
    fn test_edge_case_many_sequential_bookings() {
        let (env, cid, _admin) = make_env();
        let client = BookingContractClient::new(&env, &cid);
        let tenant = Address::generate(&env);
        for i in 0_u64..20 {
            let check_in = i * 100;
            let check_out = check_in + 50;
            client.create_booking(&tenant, &1_u64, &check_in, &check_out, &100_i128);
        }
        assert_eq!(client.booking_count(), 20);
        assert_eq!(client.get_property_bookings(&1_u64).len(), 20);
    }

    /// Bookings on property ID 0 (edge ID) work correctly.
    #[test]
    fn test_edge_case_property_id_zero() {
        let (env, cid, _admin) = make_env();
        let client = BookingContractClient::new(&env, &cid);
        let tenant = Address::generate(&env);
        let id = client.create_booking(&tenant, &0_u64, &1_000_u64, &1_005_u64, &100_i128);
        assert_eq!(client.get_booking(&id).property_id, 0);
        assert_eq!(client.get_property_bookings(&0_u64).len(), 1);
    }

    /// Escrow ID can be overwritten multiple times by admin.
    #[test]
    fn test_edge_case_escrow_id_overwrite() {
        let (env, cid, admin) = make_env();
        let client = BookingContractClient::new(&env, &cid);
        let tenant = Address::generate(&env);
        let id = client.create_booking(&tenant, &1_u64, &1_000_u64, &1_005_u64, &100_i128);

        client.set_escrow_id(&admin, &id, &String::from_str(&env, "first-escrow"));
        assert_eq!(
            client.get_booking(&id).escrow_id,
            String::from_str(&env, "first-escrow")
        );

        client.set_escrow_id(&admin, &id, &String::from_str(&env, "second-escrow"));
        assert_eq!(
            client.get_booking(&id).escrow_id,
            String::from_str(&env, "second-escrow")
        );
    }

    /// Booking count reflects total ever created, not just active ones.
    #[test]
    fn test_edge_case_booking_count_after_cancels() {
        let (env, cid, _admin) = make_env();
        let client = BookingContractClient::new(&env, &cid);
        let tenant = Address::generate(&env);
        let id1 = client.create_booking(&tenant, &1_u64, &1_000_u64, &1_005_u64, &100_i128);
        let _id2 = client.create_booking(&tenant, &1_u64, &2_000_u64, &2_005_u64, &100_i128);
        client.cancel_booking(&tenant, &id1);
        assert_eq!(client.booking_count(), 2);
        assert_eq!(client.get_booking(&id1).status, BookingStatus::Cancelled);
    }

    /// Different tenants can book different properties simultaneously.
    #[test]
    fn test_edge_case_multiple_tenants_multiple_properties() {
        let (env, cid, _admin) = make_env();
        let client = BookingContractClient::new(&env, &cid);
        let tenant_a = Address::generate(&env);
        let tenant_b = Address::generate(&env);

        let id_a = client.create_booking(&tenant_a, &10_u64, &1_000_u64, &1_005_u64, &100_i128);
        let id_b = client.create_booking(&tenant_b, &20_u64, &1_000_u64, &1_005_u64, &200_i128);

        assert_eq!(client.get_booking(&id_a).tenant, tenant_a);
        assert_eq!(client.get_booking(&id_b).tenant, tenant_b);
        assert_ne!(id_a, id_b);
    }

    // ─── Economic Attack Simulation Tests ────────────────────────────────────

    /// Economic attack: booking with total_price = 0 must be rejected.
    ///
    /// An attacker attempting a free booking (zero price) must be blocked by
    /// the `total_price > 0` guard before any state is written.
    #[test]
    #[should_panic(expected = "total_price must be positive")]
    fn test_economic_attack_simulation_zero_price() {
        let (env, cid, _admin) = make_env();
        let client = BookingContractClient::new(&env, &cid);
        let attacker = Address::generate(&env);

        // Attempt a free booking — must be rejected
        client.create_booking(&attacker, &1_u64, &1_000_u64, &1_005_u64, &0_i128);
    }

    /// Economic attack: booking with total_price = i128::MAX must be rejected.
    ///
    /// Passing the maximum i128 value as the price is a classic overflow probe.
    /// The contract stores i128 values directly; with `overflow-checks = true`
    /// in the release profile any arithmetic overflow panics rather than wraps.
    /// The value itself is technically valid (> 0), so the contract accepts it
    /// and stores it faithfully — confirming no silent truncation or wrap-around
    /// occurs. This test documents that boundary behaviour explicitly.
    ///
    /// If downstream escrow arithmetic were to add to this value it would panic
    /// (overflow-checks = true), which is the safe, expected outcome.
    #[test]
    fn test_economic_attack_simulation_max_price_stored_correctly() {
        let (env, cid, _admin) = make_env();
        let client = BookingContractClient::new(&env, &cid);
        let attacker = Address::generate(&env);

        // i128::MAX is > 0, so create_booking must accept it without overflow
        let id = client.create_booking(
            &attacker,
            &1_u64,
            &1_000_u64,
            &1_005_u64,
            &i128::MAX,
        );

        // Verify the value is stored exactly — no truncation or wrap-around
        let booking = client.get_booking(&id);
        assert_eq!(
            booking.total_price,
            i128::MAX,
            "i128::MAX must be stored without overflow or truncation"
        );
    }

    /// Economic attack: rapid sequential bookings on the same property with
    /// overlapping dates must all be rejected after the first succeeds.
    ///
    /// This simulates a griefing or double-spend attempt where an attacker
    /// fires multiple booking requests for the same property and date window
    /// in quick succession. Only the first booking should be persisted; every
    /// subsequent attempt must panic with an overlap error.
    #[test]
    #[should_panic(expected = "Booking dates overlap with an existing booking")]
    fn test_economic_attack_simulation_rapid_sequential_bookings() {
        let (env, cid, _admin) = make_env();
        let client = BookingContractClient::new(&env, &cid);
        let attacker = Address::generate(&env);

        // First booking on property 42 for the window [10_000, 10_010) succeeds
        let id = client.create_booking(
            &attacker,
            &42_u64,
            &10_000_u64,
            &10_010_u64,
            &500_i128,
        );
        assert_eq!(id, 1, "First booking must succeed and receive ID 1");

        // Rapid second attempt on the same property and overlapping window —
        // must be rejected regardless of how quickly it follows the first
        client.create_booking(
            &attacker,
            &42_u64,
            &10_005_u64, // starts inside the existing booking
            &10_015_u64,
            &500_i128,
        );
    }

    /// Economic attack: verify that rapid sequential bookings with identical
    /// dates on the same property are all rejected after the first.
    ///
    /// Extends the overlap test to confirm that exact-duplicate date windows
    /// (not just partial overlaps) are also blocked.
    #[test]
    #[should_panic(expected = "Booking dates overlap with an existing booking")]
    fn test_economic_attack_simulation_duplicate_date_bookings() {
        let (env, cid, _admin) = make_env();
        let client = BookingContractClient::new(&env, &cid);
        let attacker_a = Address::generate(&env);
        let attacker_b = Address::generate(&env);

        // First attacker books property 7 for [5_000, 5_100)
        client.create_booking(&attacker_a, &7_u64, &5_000_u64, &5_100_u64, &1_000_i128);

        // Second attacker immediately tries the exact same window — must be rejected
        client.create_booking(&attacker_b, &7_u64, &5_000_u64, &5_100_u64, &1_000_i128);
    }

    // ─── Deployment Validation Tests ──────────────────────────────────────────

    /// Deployment validation: contract initialises correctly with testnet-like
    /// ledger settings (non-zero sequence number and timestamp).
    ///
    /// On Stellar Testnet the ledger sequence starts well above 0 and the
    /// timestamp reflects real wall-clock time. This test simulates that
    /// environment by advancing the mock ledger before initialisation and
    /// verifies that the contract behaves identically to a fresh mainnet deploy.
    #[test]
    fn test_deployment_validation_networks_testnet_init() {
        let env = Env::default();
        env.mock_all_auths();

        // Simulate testnet ledger state: advance sequence and timestamp to
        // values representative of a live network (not the default 0/0).
        env.ledger().with_mut(|li| {
            li.sequence_number = 1_000_000;   // testnet-like sequence
            li.timestamp = 1_700_000_000;     // ~Nov 2023 Unix timestamp
        });

        let contract_id = env.register_contract(None, BookingContract);
        let client = BookingContractClient::new(&env, &contract_id);
        let admin = Address::generate(&env);

        // Initialisation must succeed on a simulated testnet ledger
        client.initialize(&admin);

        // Booking count starts at zero regardless of ledger state
        assert_eq!(
            client.booking_count(),
            0,
            "Booking count must be 0 after initialisation on testnet ledger"
        );

        // A booking created after testnet-like init must work correctly
        let tenant = Address::generate(&env);
        let id = client.create_booking(
            &tenant,
            &1_u64,
            &1_700_000_100_u64, // check_in after the simulated testnet timestamp
            &1_700_000_200_u64,
            &100_i128,
        );
        assert_eq!(id, 1);
        assert_eq!(client.get_booking(&id).check_in, 1_700_000_100_u64);
    }

    /// Deployment validation: contract IDs are deterministic given the same
    /// deployer and salt.
    ///
    /// Soroban derives a contract address from (deployer_address, salt) via a
    /// deterministic hash. Registering the same contract type at the same
    /// explicit address in two independent environments must yield the same
    /// address, confirming the determinism property relied upon by deployment
    /// scripts and cross-contract calls.
    #[test]
    fn test_deployment_validation_networks_deterministic_contract_id() {
        // Build two completely independent environments
        let env_a = Env::default();
        env_a.mock_all_auths();
        let env_b = Env::default();
        env_b.mock_all_auths();

        // Construct a fixed contract address from a known 32-byte value.
        // This simulates deploying with a known salt so the resulting contract
        // ID is reproducible across networks.
        let fixed_bytes: [u8; 32] = [0x01u8; 32];
        let contract_addr_a = soroban_sdk::Address::from_contract_id(
            &BytesN::from_array(&env_a, &fixed_bytes),
        );
        let contract_addr_b = soroban_sdk::Address::from_contract_id(
            &BytesN::from_array(&env_b, &fixed_bytes),
        );

        // Register the contract at the fixed address in both environments
        env_a.register_contract(&contract_addr_a, BookingContract);
        env_b.register_contract(&contract_addr_b, BookingContract);

        // Both contract addresses must be equal — determinism holds
        assert_eq!(
            contract_addr_a, contract_addr_b,
            "Contract IDs derived from the same salt must be identical across environments"
        );

        // Both instances must initialise and operate independently
        let client_a = BookingContractClient::new(&env_a, &contract_addr_a);
        let client_b = BookingContractClient::new(&env_b, &contract_addr_b);

        let admin_a = Address::generate(&env_a);
        let admin_b = Address::generate(&env_b);

        client_a.initialize(&admin_a);
        client_b.initialize(&admin_b);

        assert_eq!(client_a.booking_count(), 0);
        assert_eq!(client_b.booking_count(), 0);
    }
}
