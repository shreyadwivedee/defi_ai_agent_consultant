use candid::{Nat, Principal};
use ic_cdk::api::time;
use ic_cdk_macros::*;
use ic_stable_structures::memory_manager::{MemoryId, MemoryManager, VirtualMemory};
use ic_stable_structures::{DefaultMemoryImpl, StableBTreeMap};
use std::cell::RefCell;
use num_traits::cast::ToPrimitive;

mod types;
use types::*;

// Define the type of memory
type Memory = VirtualMemory<DefaultMemoryImpl>;

// Thread-local storage for the memory manager and stable storage
thread_local! {
    static MEMORY_MANAGER: RefCell<MemoryManager<DefaultMemoryImpl>> = RefCell::new(
        MemoryManager::init(DefaultMemoryImpl::default())
    );

    static BALANCES: RefCell<StableBTreeMap<Account, StableNat, Memory>> = RefCell::new(
        StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(0)))
        )
    );

    static ALLOWANCES: RefCell<StableBTreeMap<AccountPair, Allowance, Memory>> = RefCell::new(
        StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(1)))
        )
    );

    static TRANSACTIONS: RefCell<StableBTreeMap<StableBlockIndex, Transaction, Memory>> = RefCell::new(
        StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(2)))
        )
    );

    static TOKEN_DATA: RefCell<TokenData> = RefCell::new(TokenData {
        name: "ICRC3 Token".to_string(),
        symbol: "ICR3".to_string(),
        decimals: 8,
        fee: Nat::from(10_000), // 0.0001 token
        total_supply: Nat::from(0u64),
        minting_account: Some(Account {
            owner: Principal::anonymous(),
            subaccount: None,
        }),
        next_block_index: Nat::from(0u64),
    });
}

// Token Constants
const DEFAULT_SUBACCOUNT: Option<Subaccount> = None;
const TX_WINDOW: u64 = 24 * 60 * 60 * 1_000_000_000; // 24 hours in nanoseconds

// Helper functions
fn get_caller_account() -> Account {
    Account {
        owner: ic_cdk::caller(),
        subaccount: DEFAULT_SUBACCOUNT,
    }
}

// Helper function to get account balance
fn get_account_balance(account: &Account) -> Nat {
    BALANCES.with(|balances| {
        balances
            .borrow()
            .get(account)
            .map(|stable_nat| stable_nat.as_nat().clone())
            .unwrap_or_else(|| Nat::from(0u64))
    })
}

fn record_transaction(tx: Transaction) -> BlockIndex {
    let block_index = TOKEN_DATA.with(|data| {
        let mut data = data.borrow_mut();
        let current_index = data.next_block_index.clone();
        data.next_block_index += 1u64;
        current_index
    });

    let stable_block_index = StableBlockIndex::from_nat(&block_index);
    
    TRANSACTIONS.with(|txs| {
        txs.borrow_mut().insert(stable_block_index, tx);
    });

    block_index
}

// ICRC-1 Standard Query Methods
#[query]
fn icrc1_name() -> String {
    TOKEN_DATA.with(|data| data.borrow().name.clone())
}

#[query]
fn icrc1_symbol() -> String {
    TOKEN_DATA.with(|data| data.borrow().symbol.clone())
}

#[query]
fn icrc1_decimals() -> u8 {
    TOKEN_DATA.with(|data| data.borrow().decimals)
}

#[query]
fn icrc1_fee() -> Nat {
    TOKEN_DATA.with(|data| data.borrow().fee.clone())
}

#[query]
fn icrc1_metadata() -> Vec<(String, Value)> {
    vec![
        ("icrc1:name".to_string(), Value::Text(icrc1_name())),
        ("icrc1:symbol".to_string(), Value::Text(icrc1_symbol())),
        ("icrc1:decimals".to_string(), Value::Nat(Nat::from(icrc1_decimals() as u64))),
        ("icrc1:fee".to_string(), Value::Nat(icrc1_fee())),
    ]
}

#[query]
fn icrc1_total_supply() -> Nat {
    TOKEN_DATA.with(|data| data.borrow().total_supply.clone())
}

#[query]
fn icrc1_minting_account() -> Option<Account> {
    TOKEN_DATA.with(|data| data.borrow().minting_account.clone())
}

#[query]
fn icrc1_balance_of(account: Account) -> Nat {
    get_account_balance(&account)
}

// ICRC-1 Transfer
#[update]
fn icrc1_transfer(args: TransferArgs) -> TransferResult {
    let caller = ic_cdk::caller();
    let from = Account {
        owner: caller,
        subaccount: args.from_subaccount,
    };
    let to = args.to;
    let amount = args.amount.clone();
    let fee = args.fee.unwrap_or_else(|| TOKEN_DATA.with(|data| data.borrow().fee.clone()));
    let memo = args.memo;
    let created_at_time = args.created_at_time;
    
    // Validate the transaction
    if let Some(created_at) = created_at_time {
        let now = time();
        if created_at > now {
            return TransferResult::Err(TransferError::CreatedInFuture { ledger_time: now });
        }
        if now > created_at + TX_WINDOW {
            return TransferResult::Err(TransferError::TooOld);
        }
    }
    
    // Check if the fee is correct
    let expected_fee = TOKEN_DATA.with(|data| data.borrow().fee.clone());
    if fee != expected_fee {
        return TransferResult::Err(TransferError::BadFee { expected_fee });
    }
    
    // Check if the sender has enough funds
    let from_balance = get_account_balance(&from);
    let total_deduction = amount.clone() + fee.clone();
    if from_balance < total_deduction {
        return TransferResult::Err(TransferError::InsufficientFunds { balance: from_balance });
    }
    
    // Update balances
    BALANCES.with(|balances| {
        let mut balances = balances.borrow_mut();
        
        // Convert Nat to StableNat for storage
        let stable_amount = StableNat::from_nat(amount.clone());
        let total_deduction_clone = total_deduction.clone();
        let from_balance_clone = from_balance.clone();
        
        // Deduct from sender
        if from_balance_clone == total_deduction_clone {
            // If exact amount, remove the entry
            balances.remove(&from);
        } else {
            // Otherwise, update with new balance
            let new_stable_balance = StableNat::from_nat(from_balance_clone - total_deduction_clone);
            balances.insert(from.clone(), new_stable_balance);
        }
        
        // Add to recipient
        let stable_to_balance = balances.get(&to)
            .unwrap_or_else(|| StableNat::from(0u64));
        balances.insert(to.clone(), stable_to_balance + stable_amount);
    });
    
    // Record the transaction
    let transfer = Transfer {
        amount: amount.clone(),
        from: from.clone(),
        to: to.clone(),
        spender: None,
        memo: memo.clone(),
        fee: Some(fee.clone()),
        created_at_time,
    };
    
    let tx = Transaction::transfer(transfer, time());
    let block_index = record_transaction(tx);
    
    TransferResult::Ok(block_index)
}

// ICRC-2 Approve
#[update]
fn icrc2_approve(args: ApproveArgs) -> ApproveResult {
    let caller = ic_cdk::caller();
    let from = Account {
        owner: caller,
        subaccount: args.from_subaccount,
    };
    let spender = args.spender;
    let amount = args.amount.clone();
    let expected_allowance = args.expected_allowance.clone();
    let expires_at = args.expires_at;
    let fee = args.fee.unwrap_or_else(|| TOKEN_DATA.with(|data| data.borrow().fee.clone()));
    let memo = args.memo;
    let created_at_time = args.created_at_time;
    
    // Validate the transaction
    if let Some(created_at) = created_at_time {
        let now = time();
        if created_at > now {
            return ApproveResult::Err(ApproveError::CreatedInFuture { ledger_time: now });
        }
        if now > created_at + TX_WINDOW {
            return ApproveResult::Err(ApproveError::TooOld);
        }
    }
    
    // Check if the fee is correct
    let expected_fee = TOKEN_DATA.with(|data| data.borrow().fee.clone());
    if fee != expected_fee {
        return ApproveResult::Err(ApproveError::BadFee { expected_fee });
    }
    
    // Check if the sender has enough funds for the fee
    let from_balance = get_account_balance(&from);
    if from_balance < fee {
        return ApproveResult::Err(ApproveError::InsufficientFunds { balance: from_balance });
    }
    
    // Check if the current allowance matches the expected allowance
    if let Some(expected) = &expected_allowance {
        let current = ALLOWANCES.with(|allowances| {
            allowances
                .borrow()
                .get(&AccountPair(from.clone(), spender.clone()))
                .map(|a| a.allowance.clone())
                .unwrap_or_else(|| Nat::from(0u64))
        });
        
        if &current != expected {
            return ApproveResult::Err(ApproveError::AllowanceChanged { current_allowance: current });
        }
    }
    
    // Check if the approval has expired
    if let Some(expires) = expires_at {
        let now = time();
        if expires < now {
            return ApproveResult::Err(ApproveError::Expired { ledger_time: now });
        }
    }
    
    // Update balances for the fee
    BALANCES.with(|balances| {
        let mut balances = balances.borrow_mut();
        
        // Convert fee to StableNat
        let fee_clone = fee.clone();
        let from_balance_clone = from_balance.clone();
        
        if from_balance_clone == fee_clone {
            // If exact fee amount, remove the entry
            balances.remove(&from);
        } else {
            // Otherwise, update with new balance
            let new_stable_balance = StableNat::from_nat(from_balance_clone - fee_clone);
            balances.insert(from.clone(), new_stable_balance);
        }
    });
    
    // Update allowance
    let allowance = Allowance {
        allowance: amount.clone(),
        expires_at,
    };
    
    ALLOWANCES.with(|allowances| {
        allowances.borrow_mut().insert(AccountPair(from.clone(), spender.clone()), allowance);
    });
    
    // Record the transaction
    let approve = Approve {
        from: from.clone(),
        spender: spender.clone(),
        amount,
        expected_allowance,
        expires_at,
        memo,
        fee: Some(fee.clone()),
        created_at_time,
    };
    
    let tx = Transaction::approve(approve, time());
    let block_index = record_transaction(tx);
    
    ApproveResult::Ok(block_index)
}

// ICRC-2 Allowance
#[query]
fn icrc2_allowance(args: AllowanceArgs) -> Allowance {
    let account = args.account;
    let spender = args.spender;
    
    ALLOWANCES.with(|allowances| {
        allowances
            .borrow()
            .get(&AccountPair(account, spender))
            .unwrap_or_else(|| Allowance {
                allowance: Nat::from(0u64),
                expires_at: None,
            })
    })
}

// ICRC-2 Transfer From
#[update]
fn icrc2_transfer_from(args: TransferFromArgs) -> TransferFromResult {
    let caller = ic_cdk::caller();
    let spender = Account {
        owner: caller,
        subaccount: args.spender_subaccount,
    };
    let from = args.from;
    let to = args.to;
    let amount = args.amount.clone();
    let fee = args.fee.unwrap_or_else(|| TOKEN_DATA.with(|data| data.borrow().fee.clone()));
    let memo = args.memo;
    let created_at_time = args.created_at_time;
    
    // Validate the transaction
    if let Some(created_at) = created_at_time {
        let now = time();
        if created_at > now {
            return TransferFromResult::Err(TransferFromError::CreatedInFuture { ledger_time: now });
        }
        if now > created_at + TX_WINDOW {
            return TransferFromResult::Err(TransferFromError::TooOld);
        }
    }
    
    // Check if the fee is correct
    let expected_fee = TOKEN_DATA.with(|data| data.borrow().fee.clone());
    if fee != expected_fee {
        return TransferFromResult::Err(TransferFromError::BadFee { expected_fee });
    }
    
    // Check if the sender has enough funds
    let from_balance = get_account_balance(&from);
    let total_deduction = amount.clone() + fee.clone();
    if from_balance < total_deduction {
        return TransferFromResult::Err(TransferFromError::InsufficientFunds { balance: from_balance });
    }
    
    // Check allowance
    let allowance = ALLOWANCES.with(|allowances| {
        allowances
            .borrow()
            .get(&AccountPair(from.clone(), spender.clone()))
            .unwrap_or_else(|| Allowance {
                allowance: Nat::from(0u64),
                expires_at: None,
            })
    });
    
    // Check if the allowance has expired
    if let Some(expires_at) = allowance.expires_at {
        if expires_at < time() {
            return TransferFromResult::Err(TransferFromError::InsufficientAllowance {
                allowance: Nat::from(0u64),
            });
        }
    }
    
    // Check if the allowance is sufficient
    if allowance.allowance < amount {
        return TransferFromResult::Err(TransferFromError::InsufficientAllowance {
            allowance: allowance.allowance,
        });
    }
    
    // Update balances
    BALANCES.with(|balances| {
        let mut balances = balances.borrow_mut();
        
        // Convert Nat to StableNat for storage
        let stable_amount = StableNat::from_nat(amount.clone());
        let total_deduction_clone = total_deduction.clone();
        let from_balance_clone = from_balance.clone();
        
        // Deduct from sender
        if from_balance_clone == total_deduction_clone {
            // If exact amount, remove the entry
            balances.remove(&from);
        } else {
            // Otherwise, update with new balance
            let new_stable_balance = StableNat::from_nat(from_balance_clone - total_deduction_clone);
            balances.insert(from.clone(), new_stable_balance);
        }
        
        // Add to recipient
        let stable_to_balance = balances.get(&to)
            .unwrap_or_else(|| StableNat::from(0u64));
        balances.insert(to.clone(), stable_to_balance + stable_amount);
    });
    
    // Update allowance
    let allowance_clone = allowance.allowance.clone();
    let amount_clone = amount.clone();
    let new_allowance = allowance_clone - amount_clone;
    
    ALLOWANCES.with(|allowances| {
        let mut allowances = allowances.borrow_mut();
        if new_allowance == Nat::from(0u64) {
            allowances.remove(&AccountPair(from.clone(), spender.clone()));
        } else {
            allowances.insert(
                AccountPair(from.clone(), spender.clone()),
                Allowance {
                    allowance: new_allowance,
                    expires_at: allowance.expires_at,
                },
            );
        }
    });
    
    // Record the transaction
    let transfer = Transfer {
        amount: amount.clone(),
        from: from.clone(),
        to: to.clone(),
        spender: Some(spender.clone()),
        memo,
        fee: Some(fee.clone()),
        created_at_time,
    };
    
    let tx = Transaction::transfer(transfer, time());
    let block_index = record_transaction(tx);
    
    TransferFromResult::Ok(block_index)
}

// ICRC-3 Get Blocks
#[query]
fn icrc3_get_blocks(args: GetBlocksArgs) -> GetBlocksResult {
    let start = args.start.clone();
    let length = args.length.clone();
    
    let mut blocks = Vec::new();
    
    TRANSACTIONS.with(|txs| {
        let txs = txs.borrow();
        let log_length = txs.len();
        
        // Convert transactions to blocks
        for i in start.0.to_u64().unwrap_or(0)..std::cmp::min(
            start.0.to_u64().unwrap_or(0) + length.0.to_u64().unwrap_or(0),
            log_length as u64,
        ) {
            let stable_index = StableBlockIndex::new(i);
            if let Some(tx) = txs.get(&stable_index) {
                let block_value = transaction_to_value(&tx);
                blocks.push(BlockWithId {
                    id: Nat::from(i),
                    block: block_value,
                });
            }
        }
    });
    
    let log_length = TRANSACTIONS.with(|txs| Nat::from(txs.borrow().len() as u64));
    
    GetBlocksResult {
        log_length,
        blocks,
        archived_blocks: Vec::new(), // No archived blocks in this implementation
    }
}

// Custom mint function (only callable by the minting account)
#[update]
fn mint(to: Account, amount: Nat) -> TransferResult {
    let caller = ic_cdk::caller();
    let minting_account = TOKEN_DATA.with(|data| data.borrow().minting_account.clone());
    
    // Check if the caller is the minting account
    if minting_account.is_none() || minting_account.as_ref().unwrap().owner != caller {
        return TransferResult::Err(TransferError::GenericError {
            error_code: Nat::from(1u64),
            message: "Only the minting account can mint tokens".to_string(),
        });
    }
    
    // Convert Nat to StableNat for storage
    let stable_amount = StableNat::from_nat(amount.clone());
    
    // Update the recipient's balance
    BALANCES.with(|balances| {
        let mut balances = balances.borrow_mut();
        let stable_balance = balances.get(&to)
            .unwrap_or_else(|| StableNat::from(0u64));
        balances.insert(to.clone(), stable_balance + stable_amount);
    });
    
    // Update total supply
    let amount_clone = amount.clone();
    TOKEN_DATA.with(|data| {
        let mut data = data.borrow_mut();
        data.total_supply += amount_clone;
    });
    
    // Record the transaction
    let mint = Mint {
        amount: amount.clone(),
        to: to.clone(),
        memo: None,
        created_at_time: Some(time()),
    };
    
    let tx = Transaction::mint(mint, time());
    let block_index = record_transaction(tx);
    
    TransferResult::Ok(block_index)
}

// Function to update the minting account (callable by the current minting account or canister controller)
#[update]
fn update_minting_account(new_minting_account: Account) -> Result<(), String> {
    let _caller = ic_cdk::caller();
    
    // Allow the controller to update the minting account regardless of current setting
    // This is needed for initial setup when minting account is anonymous
    
    // Update the minting account
    TOKEN_DATA.with(|data| {
        let mut data = data.borrow_mut();
        data.minting_account = Some(new_minting_account);
    });
    
    Ok(())
}

// Custom burn function
#[update]
fn burn(from: Account, amount: Nat) -> TransferResult {
    let caller = ic_cdk::caller();
    
    // Check if the caller is authorized to burn tokens
    if from.owner != caller {
        return TransferResult::Err(TransferError::GenericError {
            error_code: Nat::from(1u64),
            message: "Only the account owner can burn their tokens".to_string(),
        });
    }
    
    // Check if the account has enough tokens to burn
    let from_balance = get_account_balance(&from);
    if from_balance < amount {
        return TransferResult::Err(TransferError::InsufficientFunds { balance: from_balance });
    }
    
    // Convert Nat to StableNat for storage
    let _stable_amount = StableNat::from_nat(amount.clone());
    
    // Update the balance
    BALANCES.with(|balances| {
        let mut balances = balances.borrow_mut();
        let stable_balance = balances.get(&from)
            .unwrap_or_else(|| StableNat::from(0u64));
            
        // Calculate new balance
        if stable_balance.as_nat().clone() == amount.clone() {
            // If burning the exact amount, remove the entry
            balances.remove(&from);
        } else {
            // Otherwise, update with new balance
            let from_balance_clone = from_balance.clone();
            let amount_clone = amount.clone();
            let new_stable_balance = StableNat::from_nat(from_balance_clone - amount_clone);
            balances.insert(from.clone(), new_stable_balance);
        }
    });
    
    // Update total supply
    let amount_clone = amount.clone();
    TOKEN_DATA.with(|data| {
        let mut data = data.borrow_mut();
        data.total_supply -= amount_clone;
    });
    
    // Record the transaction
    let burn = Burn {
        amount: amount.clone(),
        from: from.clone(),
        spender: None,
        memo: None,
        created_at_time: Some(time()),
    };
    
    let tx = Transaction::burn(burn, time());
    let block_index = record_transaction(tx);
    
    TransferResult::Ok(block_index)
}

// Helper function to convert Transaction to Value for ICRC-3 blocks
fn transaction_to_value(tx: &Transaction) -> Value {
    let mut map = Vec::new();
    
    // Common fields
    map.push(("ts".to_string(), Value::Nat64(tx.timestamp)));
    
    // Transaction-specific fields
    match tx.kind.as_str() {
        "mint" => {
            if let Some(mint) = &tx.mint {
                map.push(("op".to_string(), Value::Text("mint".to_string())));
                map.push(("to".to_string(), account_to_value(&mint.to)));
                map.push(("amt".to_string(), Value::Nat(mint.amount.clone())));
                
                if let Some(memo) = &mint.memo {
                    map.push(("memo".to_string(), Value::Blob(memo.clone())));
                }
            }
        },
        "burn" => {
            if let Some(burn) = &tx.burn {
                map.push(("op".to_string(), Value::Text("burn".to_string())));
                map.push(("from".to_string(), account_to_value(&burn.from)));
                map.push(("amt".to_string(), Value::Nat(burn.amount.clone())));
                
                if let Some(spender) = &burn.spender {
                    map.push(("spender".to_string(), account_to_value(spender)));
                }
                
                if let Some(memo) = &burn.memo {
                    map.push(("memo".to_string(), Value::Blob(memo.clone())));
                }
            }
        },
        "transfer" => {
            if let Some(transfer) = &tx.transfer {
                map.push(("op".to_string(), Value::Text("xfer".to_string())));
                map.push(("from".to_string(), account_to_value(&transfer.from)));
                map.push(("to".to_string(), account_to_value(&transfer.to)));
                map.push(("amt".to_string(), Value::Nat(transfer.amount.clone())));
                
                if let Some(spender) = &transfer.spender {
                    map.push(("spender".to_string(), account_to_value(spender)));
                }
                
                if let Some(fee) = &transfer.fee {
                    map.push(("fee".to_string(), Value::Nat(fee.clone())));
                }
                
                if let Some(memo) = &transfer.memo {
                    map.push(("memo".to_string(), Value::Blob(memo.clone())));
                }
            }
        },
        "approve" => {
            if let Some(approve) = &tx.approve {
                map.push(("op".to_string(), Value::Text("approve".to_string())));
                map.push(("from".to_string(), account_to_value(&approve.from)));
                map.push(("spender".to_string(), account_to_value(&approve.spender)));
                map.push(("amt".to_string(), Value::Nat(approve.amount.clone())));
                
                if let Some(expected_allowance) = &approve.expected_allowance {
                    map.push(("expected_allowance".to_string(), Value::Nat(expected_allowance.clone())));
                }
                
                if let Some(expires_at) = approve.expires_at {
                    map.push(("expires_at".to_string(), Value::Nat64(expires_at)));
                }
                
                if let Some(fee) = &approve.fee {
                    map.push(("fee".to_string(), Value::Nat(fee.clone())));
                }
                
                if let Some(memo) = &approve.memo {
                    map.push(("memo".to_string(), Value::Blob(memo.clone())));
                }
            }
        },
        _ => {}
    }
    
    Value::Map(map)
}

// Helper function to convert Account to Value
fn account_to_value(account: &Account) -> Value {
    let mut arr = Vec::new();
    arr.push(Value::Blob(account.owner.as_slice().to_vec()));
    
    if let Some(subaccount) = &account.subaccount {
        arr.push(Value::Blob(subaccount.clone()));
    }
    
    Value::Array(arr)
}
