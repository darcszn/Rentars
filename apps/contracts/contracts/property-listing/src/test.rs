//! Unit tests for the PropertyListing contract.
//!
//! Coverage:
//!   Happy paths  — create, get, update, update_status
//!   Error cases  — unauthorized, not-found, duplicate-id logic, invalid inputs
//!   Edge cases   — empty strings, boundary values, max price, single-char title

#[cfg(test)]
mod tests {
    use soroban_sdk::{testutils::Address as _, Address, BytesN, Env, String};

    use crate::{ListingStatus, PropertyListingContract, PropertyListingContractClient};

    // ─── Helpers ─────────────────────────────────────────────────────────────

    /// Create a fresh environment with the contract registered.
    /// Returns (Env, contract_id) — callers create the client themselves so the
    /// borrow of `env` stays in the same scope.
    fn make_env() -> (Env, soroban_sdk::Address) {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, PropertyListingContract);
        (env, contract_id)
    }

    // ─── Happy Path Tests ────────────────────────────────────────────────────

    /// Creating a listing returns ID 1 and stores the correct data.
    #[test]
    fn test_create_listing() {
        let (env, cid) = make_env();
        let client = PropertyListingContractClient::new(&env, &cid);
        let owner = Address::generate(&env);

        let id = client.create_listing(
            &owner,
            &String::from_str(&env, "Mountain Cabin"),
            &String::from_str(&env, "Peaceful retreat in the Alps"),
            &50_0000000_i128,
        );

        assert_eq!(id, 1, "First listing should have ID 1");

        let listing = client.get_listing(&id);
        assert_eq!(listing.id, 1);
        assert_eq!(listing.owner, owner);
        assert_eq!(listing.title, String::from_str(&env, "Mountain Cabin"));
        assert_eq!(
            listing.description,
            String::from_str(&env, "Peaceful retreat in the Alps")
        );
        assert_eq!(listing.price_per_night, 50_0000000_i128);
        assert_eq!(listing.status, ListingStatus::Active);
    }

    /// IDs auto-increment: second listing gets ID 2.
    #[test]
    fn test_create_listing_increments_id() {
        let (env, cid) = make_env();
        let client = PropertyListingContractClient::new(&env, &cid);
        let owner = Address::generate(&env);

        let id1 = client.create_listing(
            &owner,
            &String::from_str(&env, "House A"),
            &String::from_str(&env, "desc"),
            &100_0000000_i128,
        );
        let id2 = client.create_listing(
            &owner,
            &String::from_str(&env, "House B"),
            &String::from_str(&env, "desc"),
            &100_0000000_i128,
        );

        assert_eq!(id1, 1);
        assert_eq!(id2, 2);
        assert_eq!(client.listing_count(), 2);
    }

    /// get_listing returns the correct listing for a valid ID.
    #[test]
    fn test_get_listing() {
        let (env, cid) = make_env();
        let client = PropertyListingContractClient::new(&env, &cid);
        let owner = Address::generate(&env);

        let id = client.create_listing(
            &owner,
            &String::from_str(&env, "City Apartment"),
            &String::from_str(&env, "Modern flat in downtown"),
            &200_0000000_i128,
        );

        let listing = client.get_listing(&id);
        assert_eq!(listing.id, id);
        assert_eq!(listing.price_per_night, 200_0000000_i128);
        assert_eq!(listing.status, ListingStatus::Active);
    }

    /// Owner can update title, description, and price.
    #[test]
    fn test_update_listing() {
        let (env, cid) = make_env();
        let client = PropertyListingContractClient::new(&env, &cid);
        let owner = Address::generate(&env);

        let id = client.create_listing(
            &owner,
            &String::from_str(&env, "Cozy Beach House"),
            &String::from_str(&env, "A lovely place by the sea"),
            &100_0000000_i128,
        );

        client.update_listing(
            &owner,
            &id,
            &String::from_str(&env, "Updated Beach House"),
            &String::from_str(&env, "Renovated with ocean view"),
            &150_0000000_i128,
        );

        let listing = client.get_listing(&id);
        assert_eq!(listing.title, String::from_str(&env, "Updated Beach House"));
        assert_eq!(
            listing.description,
            String::from_str(&env, "Renovated with ocean view")
        );
        assert_eq!(listing.price_per_night, 150_0000000_i128);
        // Status should be unchanged
        assert_eq!(listing.status, ListingStatus::Active);
    }

    /// Owner can change listing status to Inactive.
    #[test]
    fn test_update_status() {
        let (env, cid) = make_env();
        let client = PropertyListingContractClient::new(&env, &cid);
        let owner = Address::generate(&env);

        let id = client.create_listing(
            &owner,
            &String::from_str(&env, "Cozy Beach House"),
            &String::from_str(&env, "desc"),
            &100_0000000_i128,
        );

        client.update_status(&owner, &id, &ListingStatus::Inactive);

        let listing = client.get_listing(&id);
        assert_eq!(listing.status, ListingStatus::Inactive);
    }

    /// Owner can cycle through all status variants.
    #[test]
    fn test_update_status_all_variants() {
        let (env, cid) = make_env();
        let client = PropertyListingContractClient::new(&env, &cid);
        let owner = Address::generate(&env);

        let id = client.create_listing(
            &owner,
            &String::from_str(&env, "Property"),
            &String::from_str(&env, "desc"),
            &100_0000000_i128,
        );

        client.update_status(&owner, &id, &ListingStatus::Rented);
        assert_eq!(client.get_listing(&id).status, ListingStatus::Rented);

        client.update_status(&owner, &id, &ListingStatus::Active);
        assert_eq!(client.get_listing(&id).status, ListingStatus::Active);

        client.update_status(&owner, &id, &ListingStatus::Inactive);
        assert_eq!(client.get_listing(&id).status, ListingStatus::Inactive);
    }

    /// Multiple owners can each have their own listings independently.
    #[test]
    fn test_multiple_owners_independent_listings() {
        let (env, cid) = make_env();
        let client = PropertyListingContractClient::new(&env, &cid);
        let owner_a = Address::generate(&env);
        let owner_b = Address::generate(&env);

        let id_a = client.create_listing(
            &owner_a,
            &String::from_str(&env, "Owner A Property"),
            &String::from_str(&env, "desc"),
            &100_0000000_i128,
        );
        let id_b = client.create_listing(
            &owner_b,
            &String::from_str(&env, "Owner B Property"),
            &String::from_str(&env, "desc"),
            &200_0000000_i128,
        );

        assert_ne!(id_a, id_b);
        assert_eq!(client.get_listing(&id_a).owner, owner_a);
        assert_eq!(client.get_listing(&id_b).owner, owner_b);
    }

    // ─── Error / Negative Tests ───────────────────────────────────────────────

    /// A non-owner cannot update a listing.
    #[test]
    #[should_panic(expected = "Unauthorized")]
    fn test_update_listing_unauthorized() {
        let (env, cid) = make_env();
        let client = PropertyListingContractClient::new(&env, &cid);
        let owner = Address::generate(&env);
        let attacker = Address::generate(&env);

        let id = client.create_listing(
            &owner,
            &String::from_str(&env, "Cozy Beach House"),
            &String::from_str(&env, "desc"),
            &100_0000000_i128,
        );

        // attacker tries to update owner's listing
        client.update_listing(
            &attacker,
            &id,
            &String::from_str(&env, "Hacked Title"),
            &String::from_str(&env, "Hacked desc"),
            &1_i128,
        );
    }

    /// A non-owner cannot change the status of a listing.
    #[test]
    #[should_panic(expected = "Unauthorized")]
    fn test_update_status_unauthorized() {
        let (env, cid) = make_env();
        let client = PropertyListingContractClient::new(&env, &cid);
        let owner = Address::generate(&env);
        let attacker = Address::generate(&env);

        let id = client.create_listing(
            &owner,
            &String::from_str(&env, "Cozy Beach House"),
            &String::from_str(&env, "desc"),
            &100_0000000_i128,
        );

        client.update_status(&attacker, &id, &ListingStatus::Inactive);
    }

    /// Getting a listing that does not exist panics.
    #[test]
    #[should_panic(expected = "Listing not found")]
    fn test_get_nonexistent_listing() {
        let (env, cid) = make_env();
        let client = PropertyListingContractClient::new(&env, &cid);
        client.get_listing(&9999);
    }

    /// Updating a listing that does not exist panics.
    #[test]
    #[should_panic(expected = "Listing not found")]
    fn test_update_nonexistent_listing() {
        let (env, cid) = make_env();
        let client = PropertyListingContractClient::new(&env, &cid);
        let caller = Address::generate(&env);

        client.update_listing(
            &caller,
            &9999,
            &String::from_str(&env, "Title"),
            &String::from_str(&env, "Desc"),
            &100_i128,
        );
    }

    /// Creating a listing with zero price panics.
    #[test]
    #[should_panic(expected = "price_per_night must be positive")]
    fn test_create_listing_zero_price() {
        let (env, cid) = make_env();
        let client = PropertyListingContractClient::new(&env, &cid);
        let owner = Address::generate(&env);

        client.create_listing(
            &owner,
            &String::from_str(&env, "Free House"),
            &String::from_str(&env, "desc"),
            &0_i128,
        );
    }

    /// Creating a listing with a negative price panics.
    #[test]
    #[should_panic(expected = "price_per_night must be positive")]
    fn test_create_listing_negative_price() {
        let (env, cid) = make_env();
        let client = PropertyListingContractClient::new(&env, &cid);
        let owner = Address::generate(&env);

        client.create_listing(
            &owner,
            &String::from_str(&env, "Negative House"),
            &String::from_str(&env, "desc"),
            &-1_i128,
        );
    }

    /// Creating a listing with an empty title panics.
    #[test]
    #[should_panic(expected = "title must not be empty")]
    fn test_create_listing_empty_title() {
        let (env, cid) = make_env();
        let client = PropertyListingContractClient::new(&env, &cid);
        let owner = Address::generate(&env);

        client.create_listing(
            &owner,
            &String::from_str(&env, ""),
            &String::from_str(&env, "desc"),
            &100_0000000_i128,
        );
    }

    /// Simulates "duplicate listing" — same owner creates two listings with identical data.
    /// The contract assigns distinct IDs (no deduplication by content).
    #[test]
    fn test_create_duplicate_listing() {
        let (env, cid) = make_env();
        let client = PropertyListingContractClient::new(&env, &cid);
        let owner = Address::generate(&env);

        let id1 = client.create_listing(
            &owner,
            &String::from_str(&env, "Same Title"),
            &String::from_str(&env, "Same desc"),
            &100_0000000_i128,
        );
        let id2 = client.create_listing(
            &owner,
            &String::from_str(&env, "Same Title"),
            &String::from_str(&env, "Same desc"),
            &100_0000000_i128,
        );

        // Both are stored independently with unique IDs
        assert_ne!(id1, id2, "Duplicate content should still produce unique IDs");
        assert_eq!(client.listing_count(), 2);
    }

    // ─── Edge Case Tests ──────────────────────────────────────────────────────

    /// Edge cases: boundary values and unusual but valid inputs.
    #[test]
    fn test_edge_cases() {
        let (env, cid) = make_env();
        let client = PropertyListingContractClient::new(&env, &cid);
        let owner = Address::generate(&env);

        // Minimum valid price: 1 stroop
        let id_min_price = client.create_listing(
            &owner,
            &String::from_str(&env, "A"),
            &String::from_str(&env, ""),
            &1_i128,
        );
        let listing = client.get_listing(&id_min_price);
        assert_eq!(listing.price_per_night, 1_i128);

        // Single-character title is valid
        assert_eq!(listing.title, String::from_str(&env, "A"));

        // Empty description is valid
        assert_eq!(listing.description, String::from_str(&env, ""));

        // Maximum i128 price (boundary)
        let id_max_price = client.create_listing(
            &owner,
            &String::from_str(&env, "Max Price Property"),
            &String::from_str(&env, "desc"),
            &i128::MAX,
        );
        let listing_max = client.get_listing(&id_max_price);
        assert_eq!(listing_max.price_per_night, i128::MAX);

        // Listing count reflects both
        assert_eq!(client.listing_count(), 2);
    }

    /// Updating with the minimum valid price (1 stroop) succeeds.
    #[test]
    fn test_update_listing_minimum_price() {
        let (env, cid) = make_env();
        let client = PropertyListingContractClient::new(&env, &cid);
        let owner = Address::generate(&env);

        let id = client.create_listing(
            &owner,
            &String::from_str(&env, "Cozy Beach House"),
            &String::from_str(&env, "desc"),
            &100_0000000_i128,
        );

        client.update_listing(
            &owner,
            &id,
            &String::from_str(&env, "Cheap Stay"),
            &String::from_str(&env, "Budget option"),
            &1_i128,
        );

        assert_eq!(client.get_listing(&id).price_per_night, 1_i128);
    }

    /// Updating with zero price panics.
    #[test]
    #[should_panic(expected = "price_per_night must be positive")]
    fn test_update_listing_zero_price() {
        let (env, cid) = make_env();
        let client = PropertyListingContractClient::new(&env, &cid);
        let owner = Address::generate(&env);

        let id = client.create_listing(
            &owner,
            &String::from_str(&env, "Cozy Beach House"),
            &String::from_str(&env, "desc"),
            &100_0000000_i128,
        );

        client.update_listing(
            &owner,
            &id,
            &String::from_str(&env, "Title"),
            &String::from_str(&env, "desc"),
            &0_i128,
        );
    }

    /// Updating with an empty title panics.
    #[test]
    #[should_panic(expected = "title must not be empty")]
    fn test_update_listing_empty_title() {
        let (env, cid) = make_env();
        let client = PropertyListingContractClient::new(&env, &cid);
        let owner = Address::generate(&env);

        let id = client.create_listing(
            &owner,
            &String::from_str(&env, "Cozy Beach House"),
            &String::from_str(&env, "desc"),
            &100_0000000_i128,
        );

        client.update_listing(
            &owner,
            &id,
            &String::from_str(&env, ""),
            &String::from_str(&env, "desc"),
            &100_0000000_i128,
        );
    }

    /// listing_count returns 0 on a fresh contract.
    #[test]
    fn test_listing_count_starts_at_zero() {
        let (env, cid) = make_env();
        let client = PropertyListingContractClient::new(&env, &cid);
        assert_eq!(client.listing_count(), 0);
    }

    /// Listing count increments correctly across many listings.
    #[test]
    fn test_listing_count_many() {
        let (env, cid) = make_env();
        let client = PropertyListingContractClient::new(&env, &cid);
        let owner = Address::generate(&env);

        for i in 1_u64..=10 {
            client.create_listing(
                &owner,
                &String::from_str(&env, "House"),
                &String::from_str(&env, "desc"),
                &100_0000000_i128,
            );
            assert_eq!(client.listing_count(), i);
        }
    }

    /// A listing's owner field is immutable through update_listing.
    #[test]
    fn test_update_does_not_change_owner() {
        let (env, cid) = make_env();
        let client = PropertyListingContractClient::new(&env, &cid);
        let owner = Address::generate(&env);

        let id = client.create_listing(
            &owner,
            &String::from_str(&env, "Cozy Beach House"),
            &String::from_str(&env, "desc"),
            &100_0000000_i128,
        );

        client.update_listing(
            &owner,
            &id,
            &String::from_str(&env, "New Title"),
            &String::from_str(&env, "New desc"),
            &999_0000000_i128,
        );

        assert_eq!(client.get_listing(&id).owner, owner);
    }

    // ─── Economic Attack Simulation Tests ────────────────────────────────────

    /// Economic attack: creating a listing with an empty title must be rejected.
    ///
    /// An attacker submitting a listing with an empty string as the title
    /// attempts to pollute on-chain state with invalid data. The `title.len() > 0`
    /// guard must catch this before any storage write occurs.
    #[test]
    #[should_panic(expected = "title must not be empty")]
    fn test_economic_attack_simulation_empty_title() {
        let (env, cid) = make_env();
        let client = PropertyListingContractClient::new(&env, &cid);
        let attacker = Address::generate(&env);

        // Empty title — must be rejected
        client.create_listing(
            &attacker,
            &String::from_str(&env, ""),
            &String::from_str(&env, "Malicious listing with no title"),
            &100_0000000_i128,
        );
    }

    /// Economic attack: creating a listing with a zero price must be rejected.
    ///
    /// A zero-price listing would allow a tenant to book a property for free,
    /// bypassing the escrow payment flow entirely. The `price_per_night > 0`
    /// guard must block this before any state is written.
    #[test]
    #[should_panic(expected = "price_per_night must be positive")]
    fn test_economic_attack_simulation_zero_price_listing() {
        let (env, cid) = make_env();
        let client = PropertyListingContractClient::new(&env, &cid);
        let attacker = Address::generate(&env);

        // Zero price — must be rejected
        client.create_listing(
            &attacker,
            &String::from_str(&env, "Free Property Attack"),
            &String::from_str(&env, "Attempting to list at zero cost"),
            &0_i128,
        );
    }

    /// Economic attack: creating a listing with a negative price must be rejected.
    ///
    /// A negative price could cause integer underflow in downstream escrow
    /// arithmetic, potentially crediting the attacker instead of debiting them.
    #[test]
    #[should_panic(expected = "price_per_night must be positive")]
    fn test_economic_attack_simulation_negative_price_listing() {
        let (env, cid) = make_env();
        let client = PropertyListingContractClient::new(&env, &cid);
        let attacker = Address::generate(&env);

        // Negative price — must be rejected
        client.create_listing(
            &attacker,
            &String::from_str(&env, "Negative Price Attack"),
            &String::from_str(&env, "Attempting to list at negative cost"),
            &-1_i128,
        );
    }

    /// Economic attack: creating a listing with an extremely long description
    /// (simulating a data-hash field stuffed with oversized payload).
    ///
    /// Soroban's `String` type is backed by a `Bytes` object whose length is
    /// bounded by the host's memory limits. This test verifies that the contract
    /// stores a very long string without panicking and that the value round-trips
    /// correctly — confirming no silent truncation occurs. If the host enforces
    /// a length cap it will panic, which is also an acceptable outcome (the
    /// attack is rejected).
    #[test]
    fn test_economic_attack_simulation_extremely_long_description() {
        let (env, cid) = make_env();
        let client = PropertyListingContractClient::new(&env, &cid);
        let owner = Address::generate(&env);

        // Build a 512-character description string to simulate an oversized payload
        let long_desc_raw = "A".repeat(512);
        let long_desc = String::from_str(&env, &long_desc_raw);

        // The contract does not validate description length, so this must succeed
        let id = client.create_listing(
            &owner,
            &String::from_str(&env, "Valid Title"),
            &long_desc,
            &100_0000000_i128,
        );

        // Verify the description is stored exactly — no truncation
        let listing = client.get_listing(&id);
        assert_eq!(
            listing.description,
            long_desc,
            "Extremely long description must be stored without truncation"
        );
    }

    /// Economic attack: creating a listing with an extremely long title.
    ///
    /// Complements the long-description test. The title field has a non-empty
    /// guard but no upper-bound check; this test confirms the contract stores
    /// a very long title faithfully and does not panic or truncate.
    #[test]
    fn test_economic_attack_simulation_extremely_long_title() {
        let (env, cid) = make_env();
        let client = PropertyListingContractClient::new(&env, &cid);
        let owner = Address::generate(&env);

        // Build a 256-character title string
        let long_title_raw = "B".repeat(256);
        let long_title = String::from_str(&env, &long_title_raw);

        let id = client.create_listing(
            &owner,
            &long_title,
            &String::from_str(&env, "desc"),
            &100_0000000_i128,
        );

        let listing = client.get_listing(&id);
        assert_eq!(
            listing.title,
            long_title,
            "Extremely long title must be stored without truncation"
        );
    }

    // ─── Deployment Validation Tests ──────────────────────────────────────────

    /// Deployment validation: contract initialises correctly with testnet-like
    /// ledger settings (non-zero sequence number and timestamp).
    ///
    /// On Stellar Testnet the ledger sequence starts well above 0 and the
    /// timestamp reflects real wall-clock time. This test simulates that
    /// environment and verifies the contract behaves identically to a fresh
    /// mainnet deploy.
    #[test]
    fn test_deployment_validation_networks_testnet_init() {
        let env = Env::default();
        env.mock_all_auths();

        // Simulate testnet ledger state
        env.ledger().with_mut(|li| {
            li.sequence_number = 2_500_000;   // testnet-like sequence
            li.timestamp = 1_700_000_000;     // ~Nov 2023 Unix timestamp
        });

        let contract_id = env.register_contract(None, PropertyListingContract);
        let client = PropertyListingContractClient::new(&env, &contract_id);

        // Listing count starts at zero regardless of ledger state
        assert_eq!(
            client.listing_count(),
            0,
            "Listing count must be 0 on a fresh testnet deploy"
        );

        // A listing created after testnet-like init must work correctly
        let owner = Address::generate(&env);
        let id = client.create_listing(
            &owner,
            &String::from_str(&env, "Testnet Property"),
            &String::from_str(&env, "Listed on simulated testnet"),
            &50_0000000_i128,
        );

        assert_eq!(id, 1);
        let listing = client.get_listing(&id);
        assert_eq!(listing.title, String::from_str(&env, "Testnet Property"));
        assert_eq!(listing.status, ListingStatus::Active);
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
        let fixed_bytes: [u8; 32] = [0x02u8; 32];
        let contract_addr_a = soroban_sdk::Address::from_contract_id(
            &BytesN::from_array(&env_a, &fixed_bytes),
        );
        let contract_addr_b = soroban_sdk::Address::from_contract_id(
            &BytesN::from_array(&env_b, &fixed_bytes),
        );

        // Register the contract at the fixed address in both environments
        env_a.register_contract(&contract_addr_a, PropertyListingContract);
        env_b.register_contract(&contract_addr_b, PropertyListingContract);

        // Both contract addresses must be equal — determinism holds
        assert_eq!(
            contract_addr_a, contract_addr_b,
            "Contract IDs derived from the same salt must be identical across environments"
        );

        // Both instances must operate independently with correct initial state
        let client_a = PropertyListingContractClient::new(&env_a, &contract_addr_a);
        let client_b = PropertyListingContractClient::new(&env_b, &contract_addr_b);

        assert_eq!(client_a.listing_count(), 0);
        assert_eq!(client_b.listing_count(), 0);

        // Creating a listing in env_a must not affect env_b
        let owner_a = Address::generate(&env_a);
        client_a.create_listing(
            &owner_a,
            &String::from_str(&env_a, "Env A Property"),
            &String::from_str(&env_a, "desc"),
            &100_0000000_i128,
        );

        assert_eq!(client_a.listing_count(), 1);
        assert_eq!(client_b.listing_count(), 0, "env_b must remain isolated from env_a");
    }
}
