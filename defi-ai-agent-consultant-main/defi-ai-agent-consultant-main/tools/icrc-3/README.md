# ICRC-3 Token Implementation

A complete implementation of the [ICRC-3](https://github.com/dfinity/ICRC-1/blob/main/standards/ICRC-3/README.md) token standard for the Internet Computer Protocol (ICP), with additional mint and burn functionality similar to ERC-20 tokens on Ethereum.

## Features

- **ICRC-1 Compatibility**: Core token functionality including transfers and balance queries
- **ICRC-2 Compatibility**: Allowance and approval functionality for delegated transfers
- **ICRC-3 Compatibility**: Block log standard for transaction history
- **Mint/Burn Capability**: Administrative minting and user-controlled burning
- **Stable Storage**: Token data is stored in stable memory to persist across canister upgrades

## Architecture

The implementation consists of a backend canister written in Rust that implements the ICRC-3 token logic.

### Core Components

- **Token Data**: Metadata like name, symbol, decimals, and fee
- **Balances**: User account balances stored in stable memory
- **Allowances**: Approved spending amounts for delegated transfers
- **Transactions**: Record of all token operations with ICRC-3 block format

## Getting Started

### Prerequisites

- [DFX SDK](https://internetcomputer.org/docs/current/developer-docs/setup/install/) (version 0.14.0 or later)
- Rust (version 1.60.0 or later)

### Installation

1. Clone the repository and navigate to the project directory

2. Start the local Internet Computer replica:

```bash
dfx start --background
```

3. Deploy the canister:

```bash
dfx deploy
```

### Usage

After deployment, you can interact with the token in several ways:

1. **Candid Interface**: Use the Candid UI to interact with the canister
2. **Command Line**: Use `dfx canister call` commands
3. **Programmatically**: Import the generated declarations in your application

## API Reference

### ICRC-1 Standard Methods

- `icrc1_name(): text` - Returns the token name
- `icrc1_symbol(): text` - Returns the token symbol
- `icrc1_decimals(): nat8` - Returns the number of decimals
- `icrc1_fee(): nat` - Returns the standard fee for transactions
- `icrc1_metadata(): vec record { text; Value }` - Returns token metadata
- `icrc1_total_supply(): nat` - Returns the total token supply
- `icrc1_minting_account(): opt Account` - Returns the minting account if available
- `icrc1_balance_of(Account): nat` - Returns the balance of an account
- `icrc1_transfer(TransferArgs): TransferResult` - Transfers tokens between accounts

### ICRC-2 Standard Methods

- `icrc2_approve(ApproveArgs): ApproveResult` - Approves a spender to transfer tokens
- `icrc2_allowance(AllowanceArgs): Allowance` - Returns the approved allowance
- `icrc2_transfer_from(TransferFromArgs): TransferFromResult` - Transfers tokens on behalf of another account

### ICRC-3 Standard Methods

- `icrc3_get_blocks(GetBlocksArgs): GetBlocksResult` - Returns transaction blocks

### Custom Methods

- `mint(Account, nat): TransferResult` - Mints new tokens (admin only)
- `burn(Account, nat): TransferResult` - Burns existing tokens

## Data Types

### Account

```candid
type Account = record {
  owner : principal;
  subaccount : opt vec nat8;
};
```

### Transaction Types

```candid
type Transaction = record {
  kind : text;
  mint : opt Mint;
  burn : opt Burn;
  transfer : opt Transfer;
  approve : opt Approve;
  timestamp : nat64;
};
```

## Security Considerations

1. **Minting Restrictions**: Only the designated minting account can create new tokens
2. **Burning Authorization**: Only account owners can burn their own tokens
3. **Allowance Expiration**: Approvals can be time-limited with expiration timestamps
4. **Transaction Window**: Transactions have a 24-hour validity window

## Development

### Project Structure

```
icrc-3/
├── Cargo.toml              # Rust workspace configuration
├── dfx.json                # Internet Computer deployment configuration
├── backend/                # Backend canister
│   ├── Cargo.toml          # Backend dependencies
│   ├── src/
│   │   ├── lib.rs          # Main implementation
│   │   └── types.rs        # Type definitions
│   ├── tests/
│   │   └── pocket_ic_tests.rs  # Unit tests
│   └── icrc3_token_backend.did  # Candid interface
```

### Testing

Run the tests with:

```bash
cargo test
```

Expected output:

```
running 10 tests
test test_icrc1_decimals ... ok
test test_icrc1_fee ... ok
test test_icrc1_metadata ... ok
test test_icrc1_balance_of ... ok
test test_icrc1_minting_account ... ok
test test_icrc1_name ... ok
test test_icrc1_symbol ... ok
test test_icrc1_total_supply ... ok
test test_icrc2_allowance ... ok
test test_icrc3_get_blocks ... ok

test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

This project is licensed under the MIT License - see the LICENSE file for details.

## Acknowledgments

- Based on the [ICRC-3](https://github.com/dfinity/ICRC-1/blob/main/standards/ICRC-3/README.md) standard
- Inspired by ERC-20 token functionality from Ethereum