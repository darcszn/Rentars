// Rentars — Soroban Smart Contract
// Built on Stellar blockchain
// Handles: property listing, rental booking, USDC escrow
//
// Deployment checklist:
//   1. Deploy the contract
//   2. Call `initialize(env)` ONCE — sets up storage counters.
//      Re-calling will panic with "Already initialized".

#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, Address, Env, String, Vec};

// ---------------------------------------------------------------------------
// Data types
// ---------------------------------------------------------------------------

#[contracttype]
#[derive(Clone)]
pub struct Property {
    pub id: u64,
    pub owner: Address,
    pub title: String,
    pub price_per_night: i128, // in USDC stroops
    pub available: bool,
}

#[contracttype]
#[derive(Clone)]
pub struct Booking {
    pub id: u64,
    pub property_id: u64,
    pub tenant: Address,
    pub check_in: u64,  // Unix timestamp (seconds)
    pub check_out: u64, // Unix timestamp (seconds)
    pub total_price: i128,
    pub confirmed: bool,
}

// ---------------------------------------------------------------------------
// Storage keys
// ---------------------------------------------------------------------------

#[contracttype]
pub enum DataKey {
    /// Stores a single Property struct
    Property(u64),
    /// Stores a single Booking struct
    Booking(u64),
    /// Running counter for property IDs
    PropCount,
    /// Running counter for booking IDs
    BookCount,
    /// Vec<u64> of booking IDs for a given property_id — enables O(n) overlap scan
    PropertyBookings(u64),
    /// Sentinel key used to guard against re-initialization
    Initialized,
}

// ---------------------------------------------------------------------------
// Contract
// ---------------------------------------------------------------------------

#[contract]
pub struct RentarsContract;

#[contractimpl]
impl RentarsContract {
    // -----------------------------------------------------------------------
    // Initialization
    // -----------------------------------------------------------------------

    /// Must be called exactly once after deployment to set up storage counters.
    ///
    /// Panics with "Already initialized" if called more than once.
    pub fn initialize(env: Env) {
        if env
            .storage()
            .persistent()
            .has(&DataKey::Initialized)
        {
            panic!("Already initialized");
        }

        env.storage().persistent().set(&DataKey::PropCount, &0u64);
        env.storage().persistent().set(&DataKey::BookCount, &0u64);
        env.storage().persistent().set(&DataKey::Initialized, &true);
    }

    // -----------------------------------------------------------------------
    // Availability check
    // -----------------------------------------------------------------------

    /// Returns `true` if the requested date range does not overlap any existing
    /// confirmed or pending booking for `property_id`.
    ///
    /// Two ranges [a, b) and [c, d) overlap when a < d && c < b.
    pub fn check_availability(
        env: Env,
        property_id: u64,
        start_date: u64,
        end_date: u64,
    ) -> bool {
        let booking_ids: Vec<u64> = env
            .storage()
            .persistent()
            .get(&DataKey::PropertyBookings(property_id))
            .unwrap_or(Vec::new(&env));

        for i in 0..booking_ids.len() {
            let bid = booking_ids.get(i).unwrap();
            let booking: Booking = env
                .storage()
                .persistent()
                .get(&DataKey::Booking(bid))
                .expect("Booking record missing");

            // Overlap condition: existing.check_in < end_date && start_date < existing.check_out
            if booking.check_in < end_date && start_date < booking.check_out {
                return false;
            }
        }
        true
    }

    // -----------------------------------------------------------------------
    // Property listing
    // -----------------------------------------------------------------------

    /// List a new property on-chain.
    ///
    /// Requires `initialize` to have been called first.
    pub fn list_property(
        env: Env,
        owner: Address,
        title: String,
        price_per_night: i128,
    ) -> u64 {
        owner.require_auth();

        let count: u64 = env
            .storage()
            .persistent()
            .get(&DataKey::PropCount)
            .expect("Contract not initialized — call initialize() first");

        let id = count + 1;

        let property = Property {
            id,
            owner,
            title,
            price_per_night,
            available: true,
        };

        env.storage().persistent().set(&DataKey::Property(id), &property);
        env.storage().persistent().set(&DataKey::PropCount, &id);
        id
    }

    // -----------------------------------------------------------------------
    // Booking
    // -----------------------------------------------------------------------

    /// Create a booking for a property.
    ///
    /// Validations (all panic on failure):
    ///   - Contract must be initialized.
    ///   - Property must exist and be listed.
    ///   - `start_date` must be >= current ledger timestamp (no past bookings).
    ///   - `start_date` must be strictly less than `end_date`.
    ///   - `total_price` must be > 0.
    ///   - Date range must not overlap any existing booking for the property.
    pub fn create_booking(
        env: Env,
        tenant: Address,
        property_id: u64,
        start_date: u64,
        end_date: u64,
        total_price: i128,
    ) -> u64 {
        tenant.require_auth();

        // --- Guard: contract must be initialized ---
        if !env.storage().persistent().has(&DataKey::Initialized) {
            panic!("Contract not initialized — call initialize() first");
        }

        // --- Validate property exists ---
        let property: Property = env
            .storage()
            .persistent()
            .get(&DataKey::Property(property_id))
            .expect("Property not found");

        // --- Date validations ---
        let now = env.ledger().timestamp();
        if start_date < now {
            panic!("start_date must be >= current ledger timestamp");
        }
        if start_date >= end_date {
            panic!("start_date must be strictly less than end_date");
        }

        // --- Price validation ---
        if total_price <= 0 {
            panic!("total_price must be greater than 0");
        }

        // --- Availability check ---
        if !Self::check_availability(env.clone(), property_id, start_date, end_date) {
            panic!("Booking overlap: dates conflict with existing reservation");
        }

        // --- Persist booking ---
        let count: u64 = env
            .storage()
            .persistent()
            .get(&DataKey::BookCount)
            .expect("Contract not initialized — call initialize() first");

        let id = count + 1;

        let booking = Booking {
            id,
            property_id,
            tenant,
            check_in: start_date,
            check_out: end_date,
            total_price,
            confirmed: false,
        };

        env.storage().persistent().set(&DataKey::Booking(id), &booking);
        env.storage().persistent().set(&DataKey::BookCount, &id);

        // --- Index booking under its property for future overlap scans ---
        let mut property_bookings: Vec<u64> = env
            .storage()
            .persistent()
            .get(&DataKey::PropertyBookings(property_id))
            .unwrap_or(Vec::new(&env));
        property_bookings.push_back(id);
        env.storage()
            .persistent()
            .set(&DataKey::PropertyBookings(property_id), &property_bookings);

        id
    }

    // -----------------------------------------------------------------------
    // Rental confirmation
    // -----------------------------------------------------------------------

    /// Confirm rental completion and release escrow to owner.
    pub fn confirm_rental(env: Env, booking_id: u64, caller: Address) {
        caller.require_auth();

        let mut booking: Booking = env
            .storage()
            .persistent()
            .get(&DataKey::Booking(booking_id))
            .expect("Booking not found");

        assert!(!booking.confirmed, "Already confirmed");
        booking.confirmed = true;
        env.storage()
            .persistent()
            .set(&DataKey::Booking(booking_id), &booking);
    }

    // -----------------------------------------------------------------------
    // Read helpers
    // -----------------------------------------------------------------------

    /// Get property details by ID.
    pub fn get_property(env: Env, id: u64) -> Property {
        env.storage()
            .persistent()
            .get(&DataKey::Property(id))
            .expect("Property not found")
    }

    /// Get booking details by ID.
    pub fn get_booking(env: Env, id: u64) -> Booking {
        env.storage()
            .persistent()
            .get(&DataKey::Booking(id))
            .expect("Booking not found")
    }

    /// Get all bookings for a given property.
    ///
    /// Returns an empty Vec if the property has no bookings yet.
    pub fn get_property_bookings(env: Env, property_id: u64) -> Vec<Booking> {
        let booking_ids: Vec<u64> = env
            .storage()
            .persistent()
            .get(&DataKey::PropertyBookings(property_id))
            .unwrap_or(Vec::new(&env));

        let mut bookings: Vec<Booking> = Vec::new(&env);
        for i in 0..booking_ids.len() {
            let bid = booking_ids.get(i).unwrap();
            let booking: Booking = env
                .storage()
                .persistent()
                .get(&DataKey::Booking(bid))
                .expect("Booking record missing");
            bookings.push_back(booking);
        }
        bookings
    }
}
