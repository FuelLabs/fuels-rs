contract;

use std::{address::*, assert::assert, block::*, chain::auth::*, context::{*, call_frames::*}, contract_id::ContractId, hash::*, panic::panic, storage::*, token::*};
use std::result::*;
use swayswap_abi::{Exchange, RemoveLiquidityReturn};

////////////////////////////////////////
// Constants
////////////////////////////////////////

/// Token ID of Ether
const ETH_ID = 0x0000000000000000000000000000000000000000000000000000000000000000;

/// Contract ID of the token on the other side of the pool.
/// Modify at compile time for different pool.
const TOKEN_ID = 0xea03f497f8a85eba06be23cf49bc0e5d82fd0511711573761b448e98d152ba0c;

/// Minimum ETH liquidity to open a pool.
const MINIMUM_LIQUIDITY = 1; //A more realistic value would be 1000000000;

////////////////////////////////////////
// Storage declarations
////////////////////////////////////////

/// Storage delimited
const S_DEPOSITS: b256 = 0x0000000000000000000000000000000000000000000000000000000000000000;
const S_TOTAL_SUPPLY: b256 = 0x0000000000000000000000000000000000000000000000000000000000000001;

////////////////////////////////////////
// Helper functions
////////////////////////////////////////

/// Compute the storage slot for an address's deposits.
fn key_deposits(a: Address, asset_id: b256) -> b256 {
    let inner = hash_pair(a.into(), asset_id, HashMethod::Sha256);
    hash_pair(S_DEPOSITS, inner, HashMethod::Sha256)
}

/// Pricing function for converting between ETH and Tokens.
fn get_input_price(input_amount: u64, input_reserve: u64, output_reserve: u64) -> u64 {
    assert(input_reserve > 0 && output_reserve > 0);
    let input_amount_with_fee: u64 = input_amount * 997;
    let numerator: u64 = input_amount_with_fee * output_reserve;
    let denominator: u64 = (input_reserve * 1000) + input_amount_with_fee;
    numerator / denominator
}

/// Pricing function for converting between ETH and Tokens.
fn get_output_price(output_amount: u64, input_reserve: u64, output_reserve: u64) -> u64 {
    assert(input_reserve > 0 && output_reserve > 0);
    let numerator: u64 = input_reserve * output_reserve * 1000;
    let denominator: u64 = (output_reserve - output_amount) * 997;
    numerator / denominator + 1
}

/// Return the sender as an Address or panic
fn get_msg_sender_address_or_panic() -> Address {
    let result: Result<Sender, AuthError> = msg_sender();
    let mut ret = ~Address::from(0x0000000000000000000000000000000000000000000000000000000000000000);
    if result.is_err() {
        panic(0);
    } else {
        let unwrapped = result.unwrap();
        if let Sender::Address(v) = unwrapped {
            ret = v;
        } else {
            panic(0);
        };
    };

    ret
}

// ////////////////////////////////////////
// // ABI definitions
// ////////////////////////////////////////

impl Exchange for Contract {
    fn deposit() {
        assert((msg_asset_id()).into() == ETH_ID || (msg_asset_id()).into() == TOKEN_ID);

        let sender = get_msg_sender_address_or_panic();

        let key = key_deposits(sender, (msg_asset_id()).into());

        let total_amount = get::<u64>(key) + msg_amount();
        store(key, total_amount);
    }

    fn withdraw(amount: u64, asset_id: ContractId) {
        assert(asset_id.into() == ETH_ID || asset_id.into() == TOKEN_ID);

        let sender = get_msg_sender_address_or_panic();

        let key = key_deposits(sender, asset_id.into());
        let deposited_amount = get::<u64>(key);
        assert(deposited_amount >= amount);

        let new_amount = deposited_amount - amount;
        store(key, new_amount);

        transfer_to_output(amount, asset_id, sender)
    }

    fn add_liquidity(min_liquidity: u64, max_tokens: u64, deadline: u64) -> u64 {
        // No coins should be sent with this call. Coins should instead be `deposit`ed prior.
        assert(msg_amount() == 0);
        assert(deadline > height());
        assert(max_tokens > 0);
        assert((msg_asset_id()).into() == ETH_ID || (msg_asset_id()).into() == TOKEN_ID);

        let sender = get_msg_sender_address_or_panic();

        let total_liquidity = get::<u64>(S_TOTAL_SUPPLY);

        let eth_amount_key = key_deposits(sender, ETH_ID);
        let eth_amount = get::<u64>(eth_amount_key);
        store(eth_amount_key, 0);
        let token_amount_key = key_deposits(sender, TOKEN_ID);
        let current_token_amount = get::<u64>(token_amount_key);

        assert(eth_amount > 0);

        let mut minted: u64 = 0;
        if total_liquidity > 0 {
            assert(min_liquidity > 0);

            let eth_reserve = this_balance(~ContractId::from(ETH_ID));
            let token_reserve = this_balance(~ContractId::from(TOKEN_ID));
            let token_amount = (eth_amount * token_reserve) / eth_reserve;
            let liquidity_minted = (eth_amount * total_liquidity) / eth_reserve;

            assert(max_tokens >= token_amount);
            assert(liquidity_minted >= min_liquidity);
            assert(current_token_amount >= token_amount);

            mint(liquidity_minted);
            store(S_TOTAL_SUPPLY, total_liquidity + liquidity_minted);

            transfer_to_output(liquidity_minted, contract_id(), sender);

            store(token_amount_key, current_token_amount - token_amount);

            minted = liquidity_minted;
        } else {
            assert(eth_amount > MINIMUM_LIQUIDITY);

            let token_amount = max_tokens;
            let initial_liquidity = this_balance(~ContractId::from(ETH_ID));
            assert(current_token_amount >= token_amount);

            mint(initial_liquidity);
            store(S_TOTAL_SUPPLY, initial_liquidity);

            transfer_to_output(initial_liquidity, contract_id(), sender);

            store(token_amount_key, current_token_amount - token_amount);

            minted = initial_liquidity;
        };

        minted
    }

    fn remove_liquidity(min_eth: u64, min_tokens: u64, deadline: u64) -> RemoveLiquidityReturn {
        assert(msg_amount() > 0);
        assert((msg_asset_id()).into() == (contract_id()).into());
        assert(deadline > height());
        assert(min_eth > 0 && min_tokens > 0);

        let sender = get_msg_sender_address_or_panic();

        let total_liquidity = get::<u64>(S_TOTAL_SUPPLY);
        assert(total_liquidity > 0);

        let eth_reserve = this_balance(~ContractId::from(ETH_ID));
        let token_reserve = this_balance(~ContractId::from(TOKEN_ID));

        let eth_amount = (msg_amount() * eth_reserve) / total_liquidity;
        let token_amount = (msg_amount() * token_reserve) / total_liquidity;

        assert((eth_amount >= min_eth) && (token_amount >= min_tokens));

        burn(msg_amount());
        store(S_TOTAL_SUPPLY, total_liquidity - msg_amount());

        transfer_to_output(eth_amount, ~ContractId::from(ETH_ID), sender);
        transfer_to_output(token_amount, ~ContractId::from(TOKEN_ID), sender);

        RemoveLiquidityReturn {
            eth_amount: eth_amount,
            token_amount: token_amount,
        }
    }

    fn swap_with_minimum(min: u64, deadline: u64) -> u64 {
        assert(deadline >= height());
        assert(msg_amount() > 0 && min > 0);
        assert((msg_asset_id()).into() == ETH_ID || (msg_asset_id()).into() == TOKEN_ID);

        let sender = get_msg_sender_address_or_panic();

        let eth_reserve = this_balance(~ContractId::from(ETH_ID));
        let token_reserve = this_balance(~ContractId::from(TOKEN_ID));

        let mut bought = 0;
        if ((msg_asset_id()).into() == ETH_ID) {
            let tokens_bought = get_input_price(msg_amount(), eth_reserve, token_reserve);
            assert(tokens_bought >= min);
            transfer_to_output(tokens_bought, ~ContractId::from(TOKEN_ID), sender);
            bought = tokens_bought;
        } else {
            let eth_bought = get_input_price(msg_amount(), token_reserve, eth_reserve);
            assert(eth_bought >= min);
            transfer_to_output(eth_bought, ~ContractId::from(ETH_ID), sender);
            bought = eth_bought;
        };

        bought
    }

    fn swap_with_maximum(amount: u64, deadline: u64) -> u64 {
        assert(deadline >= height());
        assert(amount > 0 && msg_amount() > 0);
        assert((msg_asset_id()).into() == ETH_ID || (msg_asset_id()).into() == TOKEN_ID);

        let sender = get_msg_sender_address_or_panic();

        let eth_reserve = this_balance(~ContractId::from(ETH_ID));
        let token_reserve = this_balance(~ContractId::from(TOKEN_ID));

        let mut sold = 0;
        if ((msg_asset_id()).into() == ETH_ID) {
            let eth_sold = get_output_price(amount, eth_reserve, token_reserve);
            assert(msg_amount() >= eth_sold);
            let refund = msg_amount() - eth_sold;
            if refund > 0 {
                transfer_to_output(refund, ~ContractId::from(ETH_ID), sender);
            };
            transfer_to_output(amount, ~ContractId::from(TOKEN_ID), sender);
            sold = eth_sold;
        } else {
            let tokens_sold = get_output_price(amount, token_reserve, eth_reserve);
            assert(msg_amount() >= tokens_sold);
            let refund = msg_amount() - tokens_sold;
            if refund > 0 {
                transfer_to_output(refund, ~ContractId::from(TOKEN_ID), sender);
            };
            transfer_to_output(amount, ~ContractId::from(ETH_ID), sender);
            sold = tokens_sold;
        };

        sold
    }
}
