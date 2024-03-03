//! tashi_token

#[macro_use]
extern crate pbc_contract_codegen;
use pbc_contract_common::address::Address;
use pbc_contract_common::context::ContractContext;
use pbc_contract_common::sorted_vec_map::SortedVecMap;
use std::ops::Sub;

/// This is the state of the token which is persisted on chain.
///
/// ### Fields:
///
///   * `total_supply`: [`u128`], total supply of coins.
///   * `name`: [`String`], name of the token.
///   * `symbol`: [`String`], symbol of the token.
///   * `balances`: [`SortedVecMap`]<[`Address`], [`u128`]>, balances of each address.
///   * `allowed`: [`SortedVecMap`]<[`Address`], [`SortedVecMap`]<[`Address`], [`u128`]>, all balances allotted by an address to other addresses.
///   * `decimals`: [`u8`], the number of decimals the token uses.
///   * `owner`: [`Address`], the owner of the contract.
///   * `_padding`: [[`u16`]; `5`], padding bytes to align the struct.
#[state]
#[repr(C)]
struct TokenState {
    total_supply: u128,
    name: String,
    symbol: String,
    balances: SortedVecMap<Address, u128>,
    allowed: SortedVecMap<Address, SortedVecMap<Address, u128>>,
    decimals: u8,
    owner: Address,
    _padding: [u16; 5],
}

/// A map that can store balances.
trait BalanceMap<K, V>
where
    // K is a type that implements the Ord trait
    K: Ord,
{
    fn insert_balance(&mut self, key: K, amount: V);
}

/// the type SortedVecMap<Address, V> should implement the trait BalanceMap<Address, V>, where V is a type that implements the trait Sub<V, Output = V>
impl<V: Sub<V, Output = V> + PartialEq + Copy> BalanceMap<Address, V> for SortedVecMap<Address, V> {
    #[allow(clippy::eq_op)]
    fn insert_balance(&mut self, key: Address, amount: V) {
        let zero = amount - amount; // can handle different zeroes for different types
        if amount == zero {
            self.remove(&key); // remove address with 0 balance
        } else {
            self.insert(key, amount); // update or insert address with new value
        }
    }
}

// implement struct specific functions
impl TokenState {
    /// Gets the balance of the specified address.
    ///
    /// ### Parameters:
    ///
    ///   * `owner`: [`Address`], account to query balance of
    ///
    /// ### Returns:
    ///
    /// A [`u128`] amount owned by the account.
    pub fn balance_of(&self, owner: &Address) -> u128 {
        self.balances.get(owner).copied().unwrap_or(0)
    }

    /// Gets the amount of tokens that an owner allotted to a spender.
    ///
    /// ### Parameters:
    ///
    ///   * `owner`: [`Address`], account which owns the funds.
    ///   * `spender`: [`Address`], account which will spend the funds.
    ///
    /// ### Returns:
    ///
    /// A [`u128`] amount the `spender` is allowed to withdraw from the `owner`.
    pub fn allowance(&self, owner: &Address, spender: &Address) -> u128 {
        self.allowed
            .get(owner)
            .and_then(|owner_allowances| owner_allowances.get(spender))
            .copied()
            .unwrap_or(0)
    }

    /// Updates the balance an owner allots a spender to `amount`.
    ///
    /// ### Parameters:
    ///
    ///   * `owner`: [`Address`], account which owns the funds.
    ///   * `spender`: [`Address`], account which will spend the funds.
    ///   * `amount`: [`u128`], amount to allot to `spender`.
    pub fn update_allowance(&mut self, owner: Address, spender: Address, amount: u128) {
        if !self.allowed.contains_key(&owner) {
            self.allowed.insert(owner, SortedVecMap::new());
        }
        let owner_allowances = self.allowed.get_mut(&owner).unwrap();
        owner_allowances.insert_balance(spender, amount);
    }
}

/// Initial function to bootstrap the contract's state.
///
/// ### Parameters
///
///   * `ctx`: [`ContractContext`] - the contract context containing sender and chain information.
///   * `name`: [`String`], name of the token.
///   * `symbol`: [`String`], symbol of the token.
///   * `total_supply`: [`u128`], total supply of the token.
///
/// ### Returns
///
/// The new [`TokenState`] state.
#[init]
fn initialize(
    ctx: ContractContext,
    total_supply: u128,
    name: String,
    symbol: String,
    decimals: u8,
) -> TokenState {
    let mut balances: SortedVecMap<Address, u128> = SortedVecMap::new();
    balances.insert(ctx.sender, total_supply);
    TokenState {
        total_supply,
        name,
        symbol,
        balances,
        allowed: SortedVecMap::new(),
        decimals,
        owner: ctx.sender,
        _padding: [0; 5],
    }
}

/// Transfer `amount` tokens to address `to` from caller address.
///
/// Panics if there is insufficient balance in caller account.
///
/// ### Parameters
///
///   * `ctx`: [`ContractContext`], current context for the action.
///   * `state`: [`TokenState`], current state of the contract.
///   * `to`: [`Address`], account to transfer to.
///   * `amount`: [`u128`], amount to transfer.
///
/// ### Returns
///
/// The updated [`TokenState`] state.
#[action(shortname = 0x01)]
fn transfer(
    ctx: ContractContext,
    mut state: TokenState,
    receiver: Address,
    amount: u128,
) -> TokenState {
    let sender_balance = state.balance_of(&ctx.sender);
    let new_sender_balance = sender_balance
        .checked_sub(amount) // subtract amount from sender balance
        .unwrap_or_else(|| {
            // panic if balance < amount
            panic!(
                "Insufficient balance: {}, minimum required balance: {}",
                sender_balance, amount
            )
        });
    state
        .balances
        .insert_balance(ctx.sender, new_sender_balance); // update sender balance

    let new_receiver_balance = state
        .balance_of(&receiver)
        .checked_add(amount) // add amount to receiver balance
        .expect("Overflow when adding to balance.");

    state
        .balances
        .insert_balance(receiver, new_receiver_balance); // update receiver balance

    state
}

/// Transfer `value` tokens to address `to` from address `from`.
///
/// Panics if there is insufficient allowance in caller account.
///
/// ### Parameters
///
///   * `ctx`: [`ContractContext`], current context for the action.
///   * `state`: [`TokenState`], current state of the contract.
///   * `from`: [`Address`], account to transfer from.
///   * `to`: [`Address`], account to transfer to.
///   * `amount`: [`u128`] - amount to transfer.
///
/// ### Returns
///
/// The updated [`TokenState`] state.
#[action(shortname = 0x03)]
fn transfer_from(
    ctx: ContractContext,
    mut state: TokenState,
    from: Address,
    receiver: Address,
    amount: u128,
) -> TokenState {
    let caller_allowance = state.allowance(&from, &ctx.sender);
    let caller_new_allowance = caller_allowance
        .checked_sub(amount) // subtract amount from caller allowance
        .unwrap_or_else(|| {
            // panic if allowance < amount
            panic!(
                "Insufficient allowance: {}, minimum required allowance: {}",
                caller_allowance, amount
            )
        });
    state.update_allowance(from, ctx.sender, caller_new_allowance); // update caller allowance

    let new_receiver_balance = state
        .balance_of(&receiver) // get balance of receiver
        .checked_add(amount) // add amount to receiver balance
        .expect("Overflow when adding to balance.");

    state
        .balances
        .insert_balance(receiver, new_receiver_balance); // update receiver balance

    state
}

/// Approve `amount` tokens for address `spender` from caller address. If no prior approval exists
/// then a new entry is created with approval set as `amount`. Else `amount` replaces the current
/// approval amount.
///
/// Panics if there is insufficient balance in caller account.
///
/// ### Parameters
///
///   * `ctx`: [`ContractContext`], current context for the action.
///   * `state`: [`TokenState`], current state of the contract.
///   * `from`: [`Address`], account to transfer from.
///   * `to`: [`Address`], account to transfer to.
///   * `amount`: [`u128`], amount to transfer.
///
/// ### Returns
///
/// The updated [`TokenState`] state.
#[action(shortname = 0x05)]
fn approve(
    ctx: ContractContext,
    mut state: TokenState,
    spender: Address,
    amount: u128,
) -> TokenState {
    let caller_balance = state.balance_of(&ctx.sender);
    let caller_new_balance = caller_balance
        .checked_sub(amount) // subtract amount from caller balance
        .unwrap_or_else(|| {
            // panic if balance < amount
            panic!(
                "Insufficient balance: {}, minimum required balance: {}",
                caller_balance, amount
            )
        });
    state
        .balances
        .insert_balance(ctx.sender, caller_new_balance); // update caller balance

    state.update_allowance(ctx.sender, spender, amount); // update spender allowance

    state
}

/// Update the allowance for address `spender` from caller address by amount `delta`. If no prior
/// approval exists then a new entry is created with approval set as `delta`. In this case `delta`
/// needs to be positive. `delta` can be negative if there is some allowance already. In this case
/// if `delta` is greater than the allowance, the allowance is set to 0 and that many coins are
/// returned to the caller.
///
/// Panics if there is insufficient balance in caller account.
///
/// ### Parameters
///
///   * `ctx`: [`ContractContext`], current context for the action.
///   * `state`: [`TokenState`], current state of the contract.
///   * `spender`: [`Address`], account to update allowance for.
///   * `delta`: [`i128`], amount to update allowance by.
///
/// ### Returns
///
/// The updated [`TokenState`] state.
#[action(shortname = 0x06)]
fn approve_relative(
    ctx: ContractContext,
    mut state: TokenState,
    spender: Address,
    mut delta: i128,
) -> TokenState {
    let caller_balance_result: Result<i128, _> = state.balance_of(&ctx.sender).try_into();
    let caller_balance = match caller_balance_result {
        Ok(balance) => balance,
        Err(error) => panic!("u128 to i128 conversion failed: {}", error),
    };

    let spender_allowance_result: Result<i128, _> =
        state.allowance(&ctx.sender, &spender).try_into();
    let spender_allowance = match spender_allowance_result {
        Ok(allowance) => allowance,
        Err(error) => panic!("u128 to i128 conversion failed: {}", error),
    };

    // return allowance back to caller
    if delta.is_negative() {
        let abs_delta = delta.checked_abs().unwrap_or(i128::MAX);
        // spender has enough allowance to give delta back to caller
        if abs_delta >= spender_allowance {
            // return whatever allowance is left back to caller
            delta = -spender_allowance;
        }
    }

    let spender_new_allowance = spender_allowance
        .checked_add(delta)
        .expect("Overflow when updating spender allowance.")
        .try_into()
        .unwrap_or_else(|error| panic!("i128 to u128 conversion failed: {}", error));
    state.update_allowance(ctx.sender, spender, spender_new_allowance); // update spender allowance

    let caller_new_balance = caller_balance
        .checked_add(delta) // add amount delta to caller balance
        .expect("Overflow when updating caller balance.")
        .try_into()
        .unwrap_or_else(|error| panic!("i128 to u128 conversion failed: {}", error));
    state
        .balances
        .insert_balance(ctx.sender, caller_new_balance); // update caller balance

    state
}
