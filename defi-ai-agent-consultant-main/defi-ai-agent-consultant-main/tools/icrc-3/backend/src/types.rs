use candid::{CandidType, Deserialize, Nat, Principal};
use ic_stable_structures::{BoundedStorable, Storable};
use std::borrow::Cow;
use serde::Serialize;
use std::ops::{Add, AddAssign, Sub, SubAssign};
use std::str::FromStr;
use num_traits::cast::ToPrimitive;

// StableBlockIndex wrapper for u64 that implements BoundedStorable
// This is used as a key for the TRANSACTIONS map
#[derive(CandidType, Serialize, Deserialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct StableBlockIndex(pub u64);

impl StableBlockIndex {
    pub fn new(value: u64) -> Self {
        Self(value)
    }
    
    pub fn to_nat(&self) -> Nat {
        Nat::from(self.0)
    }
    
    pub fn from_nat(nat: &Nat) -> Self {
        // Convert Nat to u64, defaulting to 0 if conversion fails
        let value = nat.0.to_u64().unwrap_or(0);
        Self(value)
    }
}

impl Storable for StableBlockIndex {
    fn to_bytes(&self) -> Cow<[u8]> {
        Cow::Owned(self.0.to_le_bytes().to_vec())
    }
    
    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        let mut data = [0u8; 8];
        if bytes.len() >= 8 {
            data.copy_from_slice(&bytes[0..8]);
            Self(u64::from_le_bytes(data))
        } else {
            Self(0)
        }
    }
}

impl BoundedStorable for StableBlockIndex {
    const MAX_SIZE: u32 = 8; // u64 is 8 bytes
    const IS_FIXED_SIZE: bool = true;
}

// StableNat wrapper for Nat that implements BoundedStorable
#[derive(CandidType, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct StableNat(pub Nat);

impl StableNat {
    pub fn new(value: u64) -> Self {
        Self(Nat::from(value))
    }
    
    pub fn from_nat(nat: Nat) -> Self {
        Self(nat)
    }
    
    pub fn into_nat(self) -> Nat {
        self.0
    }
    
    pub fn as_nat(&self) -> &Nat {
        &self.0
    }
}

impl Storable for StableNat {
    fn to_bytes(&self) -> Cow<[u8]> {
        // Convert Nat to bytes using its string representation
        let bytes = self.0.0.to_string().into_bytes();
        Cow::Owned(bytes)
    }
    
    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        // Convert bytes back to Nat
        let s = String::from_utf8(bytes.to_vec()).unwrap_or_default();
        let nat = Nat::from_str(&s).unwrap_or_else(|_| Nat::from(0u64));
        Self(nat)
    }
}

impl BoundedStorable for StableNat {
    const MAX_SIZE: u32 = 100; // Maximum size in bytes for the Nat representation
    const IS_FIXED_SIZE: bool = false;
}

// Implement common operations for StableNat
impl Add for StableNat {
    type Output = Self;
    
    fn add(self, other: Self) -> Self::Output {
        Self(self.0 + other.0)
    }
}

impl Add<&StableNat> for &StableNat {
    type Output = StableNat;
    
    fn add(self, other: &StableNat) -> StableNat {
        StableNat(self.0.clone() + other.0.clone())
    }
}

impl AddAssign for StableNat {
    fn add_assign(&mut self, other: Self) {
        self.0 += other.0;
    }
}

impl Sub for StableNat {
    type Output = Self;
    
    fn sub(self, other: Self) -> Self::Output {
        Self(self.0 - other.0)
    }
}

impl Sub<&StableNat> for &StableNat {
    type Output = StableNat;
    
    fn sub(self, other: &StableNat) -> StableNat {
        StableNat(self.0.clone() - other.0.clone())
    }
}

impl SubAssign for StableNat {
    fn sub_assign(&mut self, other: Self) {
        self.0 -= other.0;
    }
}

impl From<u64> for StableNat {
    fn from(value: u64) -> Self {
        Self(Nat::from(value))
    }
}

impl PartialOrd for StableNat {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.0.partial_cmp(&other.0)
    }
}

impl Ord for StableNat {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.partial_cmp(other).unwrap_or(std::cmp::Ordering::Equal)
    }
}

// Account Types
pub type Subaccount = Vec<u8>;

#[derive(CandidType, Serialize, Deserialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Account {
    pub owner: Principal,
    pub subaccount: Option<Subaccount>,
}

impl ic_stable_structures::Storable for Account {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        let mut bytes = Vec::new();
        let owner_bytes = self.owner.as_slice();
        
        // Store owner length and bytes
        bytes.extend_from_slice(&(owner_bytes.len() as u32).to_be_bytes());
        bytes.extend_from_slice(owner_bytes);
        
        // Store subaccount if present
        if let Some(subaccount) = &self.subaccount {
            bytes.push(1); // Flag indicating subaccount is present
            bytes.extend_from_slice(&(subaccount.len() as u32).to_be_bytes());
            bytes.extend_from_slice(subaccount);
        } else {
            bytes.push(0); // Flag indicating no subaccount
        }
        
        std::borrow::Cow::Owned(bytes)
    }
    
    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        let bytes = bytes.as_ref();
        let mut pos = 0;
        
        // Read owner length
        let owner_len = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as usize;
        pos += 4;
        
        // Read owner
        let owner = Principal::from_slice(&bytes[pos..pos + owner_len]);
        pos += owner_len;
        
        // Read subaccount flag
        let has_subaccount = bytes[pos] == 1;
        pos += 1;
        
        let subaccount = if has_subaccount {
            // Read subaccount length
            let subaccount_len = u32::from_be_bytes([
                bytes[pos], 
                bytes[pos + 1], 
                bytes[pos + 2], 
                bytes[pos + 3]
            ]) as usize;
            pos += 4;
            
            // Read subaccount
            Some(bytes[pos..pos + subaccount_len].to_vec())
        } else {
            None
        };
        
        Self { owner, subaccount }
    }
}

impl ic_stable_structures::BoundedStorable for Account {
    const MAX_SIZE: u32 = 100; // Maximum size in bytes (Principal + optional subaccount)
    const IS_FIXED_SIZE: bool = false;
}

// Allowance Types
#[derive(CandidType, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Allowance {
    pub allowance: Nat,
    pub expires_at: Option<u64>,
}

impl ic_stable_structures::Storable for Allowance {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        let mut bytes = Vec::new();
        
        // Store allowance as string
        let allowance_str = self.allowance.to_string();
        bytes.extend_from_slice(&(allowance_str.len() as u32).to_be_bytes());
        bytes.extend_from_slice(allowance_str.as_bytes());
        
        // Store expires_at if present
        if let Some(expires_at) = self.expires_at {
            bytes.push(1); // Flag indicating expires_at is present
            bytes.extend_from_slice(&expires_at.to_be_bytes());
        } else {
            bytes.push(0); // Flag indicating no expires_at
        }
        
        std::borrow::Cow::Owned(bytes)
    }
    
    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        let bytes = bytes.as_ref();
        let mut pos = 0;
        
        // Read allowance length
        let allowance_len = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as usize;
        pos += 4;
        
        // Read allowance
        let allowance_str = std::str::from_utf8(&bytes[pos..pos + allowance_len]).unwrap();
        let allowance = Nat::from_str(allowance_str).unwrap();
        pos += allowance_len;
        
        // Read expires_at flag
        let has_expires_at = bytes[pos] == 1;
        pos += 1;
        
        let expires_at = if has_expires_at {
            Some(u64::from_be_bytes([
                bytes[pos],
                bytes[pos + 1],
                bytes[pos + 2],
                bytes[pos + 3],
                bytes[pos + 4],
                bytes[pos + 5],
                bytes[pos + 6],
                bytes[pos + 7],
            ]))
        } else {
            None
        };
        
        Self { allowance, expires_at }
    }
}

impl ic_stable_structures::BoundedStorable for Allowance {
    const MAX_SIZE: u32 = 100; // Maximum size in bytes
    const IS_FIXED_SIZE: bool = false;
}

// Wrapper type for (Account, Account) to implement Storable and BoundedStorable
#[derive(CandidType, Serialize, Deserialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct AccountPair(pub Account, pub Account);

impl From<(Account, Account)> for AccountPair {
    fn from(pair: (Account, Account)) -> Self {
        Self(pair.0, pair.1)
    }
}

impl From<AccountPair> for (Account, Account) {
    fn from(pair: AccountPair) -> Self {
        (pair.0, pair.1)
    }
}

impl ic_stable_structures::Storable for AccountPair {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        let mut bytes = Vec::new();
        
        // Store first account
        let account1_bytes = self.0.to_bytes();
        bytes.extend_from_slice(&(account1_bytes.len() as u32).to_be_bytes());
        bytes.extend_from_slice(&account1_bytes);
        
        // Store second account
        let account2_bytes = self.1.to_bytes();
        bytes.extend_from_slice(&(account2_bytes.len() as u32).to_be_bytes());
        bytes.extend_from_slice(&account2_bytes);
        
        std::borrow::Cow::Owned(bytes)
    }
    
    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        let bytes = bytes.as_ref();
        let mut pos = 0;
        
        // Read first account length
        let account1_len = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as usize;
        pos += 4;
        
        // Read first account
        let account1 = Account::from_bytes(std::borrow::Cow::Borrowed(&bytes[pos..pos + account1_len]));
        pos += account1_len;
        
        // Read second account length
        let account2_len = u32::from_be_bytes([bytes[pos], bytes[pos + 1], bytes[pos + 2], bytes[pos + 3]]) as usize;
        pos += 4;
        
        // Read second account
        let account2 = Account::from_bytes(std::borrow::Cow::Borrowed(&bytes[pos..pos + account2_len]));
        
        Self(account1, account2)
    }
}

impl ic_stable_structures::BoundedStorable for AccountPair {
    const MAX_SIZE: u32 = 200; // Maximum size in bytes (2 * Account::MAX_SIZE)
    const IS_FIXED_SIZE: bool = false;
}

// Transaction Types
#[derive(CandidType, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Mint {
    pub amount: Nat,
    pub to: Account,
    pub memo: Option<Vec<u8>>,
    pub created_at_time: Option<u64>,
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Burn {
    pub amount: Nat,
    pub from: Account,
    pub spender: Option<Account>,
    pub memo: Option<Vec<u8>>,
    pub created_at_time: Option<u64>,
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Transfer {
    pub amount: Nat,
    pub from: Account,
    pub to: Account,
    pub spender: Option<Account>,
    pub memo: Option<Vec<u8>>,
    pub fee: Option<Nat>,
    pub created_at_time: Option<u64>,
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Approve {
    pub from: Account,
    pub spender: Account,
    pub amount: Nat,
    pub expected_allowance: Option<Nat>,
    pub expires_at: Option<u64>,
    pub memo: Option<Vec<u8>>,
    pub fee: Option<Nat>,
    pub created_at_time: Option<u64>,
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Transaction {
    pub kind: String,
    pub mint: Option<Mint>,
    pub burn: Option<Burn>,
    pub transfer: Option<Transfer>,
    pub approve: Option<Approve>,
    pub timestamp: u64,
}

impl Transaction {
    pub fn burn(burn: Burn, timestamp: u64) -> Self {
        Self {
            kind: "burn".into(),
            timestamp,
            mint: None,
            burn: Some(burn),
            transfer: None,
            approve: None,
        }
    }

    pub fn mint(mint: Mint, timestamp: u64) -> Self {
        Self {
            kind: "mint".into(),
            timestamp,
            mint: Some(mint),
            burn: None,
            transfer: None,
            approve: None,
        }
    }

    pub fn transfer(transfer: Transfer, timestamp: u64) -> Self {
        Self {
            kind: "transfer".into(),
            timestamp,
            mint: None,
            burn: None,
            transfer: Some(transfer),
            approve: None,
        }
    }

    pub fn approve(approve: Approve, timestamp: u64) -> Self {
        Self {
            kind: "approve".into(),
            timestamp,
            mint: None,
            burn: None,
            transfer: None,
            approve: Some(approve),
        }
    }
}

impl ic_stable_structures::Storable for Transaction {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        // Use candid serialization for simplicity
        let bytes = candid::encode_one(&self).unwrap();
        std::borrow::Cow::Owned(bytes)
    }
    
    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        candid::decode_one(&bytes).unwrap()
    }
}

impl ic_stable_structures::BoundedStorable for Transaction {
    const MAX_SIZE: u32 = 1024; // Maximum size in bytes
    const IS_FIXED_SIZE: bool = false;
}

// Token Data
#[derive(Clone, Debug)]
pub struct TokenData {
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
    pub fee: Nat,
    pub total_supply: Nat,
    pub minting_account: Option<Account>,
    pub next_block_index: Nat,
}

// ICRC-1 Transfer Types
#[derive(CandidType, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct TransferArgs {
    pub from_subaccount: Option<Vec<u8>>,
    pub to: Account,
    pub amount: Nat,
    pub fee: Option<Nat>,
    pub memo: Option<Vec<u8>>,
    pub created_at_time: Option<u64>,
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum TransferError {
    BadFee { expected_fee: Nat },
    BadBurn { min_burn_amount: Nat },
    InsufficientFunds { balance: Nat },
    TooOld,
    CreatedInFuture { ledger_time: u64 },
    Duplicate { duplicate_of: Nat },
    TemporarilyUnavailable,
    GenericError { error_code: Nat, message: String },
}

pub type TransferResult = Result<Nat, TransferError>;

// ICRC-2 Approve Types
#[derive(CandidType, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct ApproveArgs {
    pub from_subaccount: Option<Vec<u8>>,
    pub spender: Account,
    pub amount: Nat,
    pub expected_allowance: Option<Nat>,
    pub expires_at: Option<u64>,
    pub fee: Option<Nat>,
    pub memo: Option<Vec<u8>>,
    pub created_at_time: Option<u64>,
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum ApproveError {
    BadFee { expected_fee: Nat },
    InsufficientFunds { balance: Nat },
    AllowanceChanged { current_allowance: Nat },
    Expired { ledger_time: u64 },
    TooOld,
    CreatedInFuture { ledger_time: u64 },
    Duplicate { duplicate_of: Nat },
    TemporarilyUnavailable,
    GenericError { error_code: Nat, message: String },
}

pub type ApproveResult = Result<Nat, ApproveError>;

#[derive(CandidType, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct AllowanceArgs {
    pub account: Account,
    pub spender: Account,
}

// ICRC-2 TransferFrom Types
#[derive(CandidType, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct TransferFromArgs {
    pub spender_subaccount: Option<Vec<u8>>,
    pub from: Account,
    pub to: Account,
    pub amount: Nat,
    pub fee: Option<Nat>,
    pub memo: Option<Vec<u8>>,
    pub created_at_time: Option<u64>,
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum TransferFromError {
    BadFee { expected_fee: Nat },
    BadBurn { min_burn_amount: Nat },
    InsufficientFunds { balance: Nat },
    InsufficientAllowance { allowance: Nat },
    TooOld,
    CreatedInFuture { ledger_time: u64 },
    Duplicate { duplicate_of: Nat },
    TemporarilyUnavailable,
    GenericError { error_code: Nat, message: String },
}

pub type TransferFromResult = Result<Nat, TransferFromError>;

// ICRC-3 Block Types
pub type BlockIndex = Nat;

#[derive(CandidType, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct GetBlocksArgs {
    pub start: BlockIndex,
    pub length: Nat,
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct BlockWithId {
    pub id: Nat,
    pub block: Value,
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct ArchivedBlocks {
    pub args: Vec<GetBlocksArgs>,
    pub callback: QueryArchiveFn<Vec<GetBlocksArgs>, GetBlocksResult>,
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct GetBlocksResult {
    pub log_length: Nat,
    pub blocks: Vec<BlockWithId>,
    pub archived_blocks: Vec<ArchivedBlocks>,
}

// Value Types for ICRC-3
#[derive(CandidType, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum Value {
    Blob(Vec<u8>),
    Text(String),
    Nat(Nat),
    Nat64(u64),
    Int(candid::Int),
    Array(Vec<Value>),
    Map(Vec<(String, Value)>),
}

// QueryArchiveFn for ICRC-3
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct QueryArchiveFn<Input: CandidType, Output: CandidType> {
    pub canister_id: Principal,
    pub method: String,
    pub _marker: std::marker::PhantomData<(Input, Output)>,
}

impl<Input: CandidType, Output: CandidType> CandidType for QueryArchiveFn<Input, Output> {
    fn _ty() -> candid::types::Type {
        candid::func!((Input) -> (Output) query)
    }

    fn idl_serialize<S>(&self, serializer: S) -> Result<(), S::Error>
    where
        S: candid::types::Serializer,
    {
        candid::types::reference::Func {
            principal: self.canister_id,
            method: self.method.clone(),
        }
        .idl_serialize(serializer)
    }
}

impl<Input: CandidType, Output: CandidType> Clone for QueryArchiveFn<Input, Output> {
    fn clone(&self) -> Self {
        Self {
            canister_id: self.canister_id,
            method: self.method.clone(),
            _marker: std::marker::PhantomData,
        }
    }
}
