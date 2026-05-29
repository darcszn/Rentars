# Rentars Contract Overview

Soroban smart contract for the Rentars platform. The contract anchors property
listings on Stellar while keeping bulky listing content (titles, descriptions,
images, amenities) off-chain in Supabase. Integrity between the two layers is
guaranteed by a SHA-256 `data_hash` stored on-chain.

## On-chain types

### `PropertyListing`

| Field             | Type      | Notes                                                   |
| ----------------- | --------- | ------------------------------------------------------- |
| `id`              | `u64`     | Caller-supplied listing identifier (unique).            |
| `owner`           | `Address` | Stellar account permitted to mutate the listing.        |
| `data_hash`       | `String`  | Hex-encoded SHA-256 of the canonical off-chain payload. |
| `price_per_night` | `i128`    | USDC stroops; used by the booking flow on-chain.        |
| `available`       | `bool`    | Toggled by the booking flow.                            |

The full title, description, photos, and other rich fields are **not** stored
on-chain — they live in Supabase. Only `data_hash` represents them on-chain.

## Listing entry points

- `create_listing(env, id, data_hash, owner, price_per_night)` — creates a new
  listing. Requires `owner.require_auth()`. Fails if `id` already exists.
- `update_listing(env, id, data_hash, owner)` — replaces the `data_hash` for an
  existing listing. Requires `owner.require_auth()` and verifies that the
  supplied `owner` matches the on-chain owner recorded at creation time.
- `get_listing(env, id) -> PropertyListing` — returns the full on-chain
  listing record.

## Hash generation algorithm

Clients (backend / web) compute `data_hash` as follows before calling
`create_listing` or `update_listing`:

1. Build a **canonical** JSON object containing the off-chain property fields
   that must be tamper-evident. The canonical field set is:

   ```json
   {
     "id": "<listing id, as string>",
     "title": "<string>",
     "description": "<string>",
     "address": "<string>",
     "city": "<string>",
     "country": "<string>",
     "latitude": <number>,
     "longitude": <number>,
     "bedrooms": <integer>,
     "bathrooms": <integer>,
     "max_guests": <integer>,
     "amenities": ["<string>", ...],   // sorted alphabetically
     "images":    ["<url>", ...],      // preserved in display order
     "price_per_night": "<i128 as string>",
     "owner": "<Stellar address>"
   }
   ```

2. Serialise the object as **canonical JSON**:
   - UTF-8 encoded.
   - Object keys sorted lexicographically.
   - No insignificant whitespace (no spaces after `:` or `,`, no trailing
     newline).
   - Numeric values emitted without redundant precision; `i128` price values
     emitted as decimal strings to avoid floating-point loss.
   - Arrays preserve the ordering rules above (amenities sorted, images in
     display order).

3. Compute `SHA-256` over the canonical JSON byte string.

4. Encode the 32-byte digest as **lowercase hexadecimal** (64 ASCII chars) and
   pass it as the `data_hash` argument to the contract.

### Reference (Node.js / TypeScript)

```ts
import { createHash } from "node:crypto";

export function hashProperty(property: CanonicalProperty): string {
  const canonical = JSON.stringify(property, Object.keys(property).sort());
  return createHash("sha256").update(canonical, "utf8").digest("hex");
}
```

The backend recomputes the hash on every read of the Supabase row and compares
it against `get_listing(id).data_hash`; a mismatch indicates tampering with
the off-chain record and the listing is rejected from API responses.

## Ownership & authorisation

`update_listing` enforces two checks:

1. `owner.require_auth()` — the caller must have signed for `owner`.
2. `listing.owner == owner` — the on-chain record's owner must equal the
   supplied address.

Both checks must pass; otherwise the call aborts and no state is written.

## Storage layout

All listing and booking records live in **instance storage** keyed by the
`DataKey` enum:

- `DataKey::Property(u64)` → `PropertyListing`
- `DataKey::Booking(u64)`  → `Booking`
- `DataKey::BookingCount`  → `u64` (monotonic booking id counter)

Listing ids are caller-supplied and not counted on-chain; the backend is the
source of truth for id allocation (typically the Supabase row id).

---

## Network-specific configuration

### Supported networks

| Network  | Passphrase                                      | RPC endpoint (default)                          |
| -------- | ----------------------------------------------- | ----------------------------------------------- |
| Testnet  | `Test SDF Network ; September 2015`             | `https://soroban-testnet.stellar.org`           |
| Mainnet  | `Public Global Stellar Network ; September 2015`| `https://soroban-mainnet.stellar.org`           |
| Localnet | `Standalone Network ; February 2017`            | `http://localhost:8000/soroban/rpc`             |

### Ledger settings

Soroban contracts are network-agnostic at the source level. The differences
between networks are entirely in the **host environment** (ledger sequence,
timestamp, and network passphrase). The test suite simulates testnet conditions
by advancing the mock ledger before contract registration:

```rust
env.ledger().with_mut(|li| {
    li.sequence_number = 1_000_000;  // representative testnet sequence
    li.timestamp = 1_700_000_000;    // ~Nov 2023 Unix timestamp
});
```

This pattern is used in `test_deployment_validation_networks_testnet_init` in
each contract's test file to verify that contracts initialise and operate
correctly under non-default ledger conditions.

### Deterministic contract IDs

Soroban derives a contract address deterministically from the deployer address
and a caller-supplied **salt** (a 32-byte value). Given the same deployer and
salt, the resulting contract address is identical on every network. This
property is verified in `test_deployment_validation_networks_deterministic_contract_id`
in each contract's test file.

**Deployment script pattern (Stellar CLI):**

```bash
# Deploy with an explicit salt so the address is reproducible
stellar contract deploy \
  --wasm target/wasm32-unknown-unknown/release/<contract>.wasm \
  --source <deployer-keypair> \
  --network testnet \
  --salt 0000000000000000000000000000000000000000000000000000000000000001
```

The salt `0x01…01` (32 bytes) maps to the `[0x01u8; 32]` byte array used in
the determinism tests. Use a unique salt per contract to avoid address
collisions.

### Per-contract deployment addresses

Each contract uses a distinct salt in its determinism test to guarantee unique
addresses even when deployed by the same account:

| Contract           | Test salt byte | Purpose                              |
| ------------------ | -------------- | ------------------------------------ |
| `booking`          | `0x01`         | Booking lifecycle management         |
| `property-listing` | `0x02`         | On-chain property listing registry   |
| `review-contract`  | `0x03`         | Tenant review and reputation system  |

### Testnet faucet and funding

Before deploying to Stellar Testnet, fund the deployer account via the
Friendbot faucet:

```bash
curl "https://friendbot.stellar.org?addr=<deployer-public-key>"
```

### Environment variables

The backend (`apps/backend/.env`) must be configured with the correct network
values before invoking any contract:

```env
STELLAR_NETWORK=testnet                          # or mainnet / localnet
STELLAR_RPC_URL=https://soroban-testnet.stellar.org
STELLAR_NETWORK_PASSPHRASE="Test SDF Network ; September 2015"
BOOKING_CONTRACT_ID=<deployed-booking-contract-address>
PROPERTY_LISTING_CONTRACT_ID=<deployed-property-listing-contract-address>
REVIEW_CONTRACT_ID=<deployed-review-contract-address>
```
