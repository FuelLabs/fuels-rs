# Basic Usage of the SDK

At a high level, the Fuel Rust SDK can be used to build Rust-based applications that can run computations on the Fuel Virtual Machine through interactions with contracts written in Sway.

For this interaction to work, the SDK must be able to communicate to a `fuel-core` node; you have two options at your disposal:

1. Use the SDK's native `launch_provider_and_get_single_wallet()` that runs a short-lived test Fuel node;
2. Run a Fuel node outside your SDK code (using `fuel-core`) and point your SDK to that node's IP and port.

The first option is ideal for contract testing, as you can quickly spin up and tear down nodes between specific test cases.

For application building, you probably want to go with the second option.

## Instantiating a Fuel client

You can instantiate a Fuel client, pointing to a local Fuel node by
using [Fuel Core](https://github.com/FuelLabs/fuel-core):

```rust,ignore
use fuel_core::service::{Config, FuelService};
use fuel_gql_client::client::FuelClient;

let server = FuelService::new_node(Config::local_node()).await.unwrap();

let client = FuelClient::from(srv.bound_address);
```

Alternatively, if you have a Fuel node running separately, you can pass in the `SocketAddr` to `FuelClient::from()`.

## Deploying a Sway contract

There are two intended ways to deploy a contract

- `deploy`
- `deploy_with_salt`

If you are only interested in a single instance of your contract then use `deploy`

```rust,ignore
use fuels::prelude::*;
use fuels_abigen_macro::abigen;

// This will generate your contract's methods onto `MyContract`.
// This means an instance of `MyContract` will have access to all
// your contract's methods that are running on-chain!
abigen!(
    MyContract,
    "your_project/out/debug/contract_test-abi.json",
);

// This helper will launch a local node and provide a test wallet linked to it
let wallet = launch_provider_and_get_single_wallet().await;

// Optional: Configure deployment parameters or use `TxParameters::default()`
let gas_price = 0;
let gas_limit = 1_000_000;
let byte_price = 0;

// This will deploy your contract binary onto the chain so that its ID can
// be used to initialize the instance
let contract_id = Contract::deploy(
    "your_project/out/debug/contract_test.bin",
    &wallet,
    TxParameters::default(gas_price, gas_limit, byte_price),
)
.await
.unwrap();

// Here is an instance of your contract which you can use to make calls to
// your functions
let contract = MyContract::new(contract_id.to_string(), wallet.clone());
```

Alternatively, if you want multiple instances of the same contract then use `deploy_with_salt`

```rust,ignore
use fuels::tx::Salt;
use fuels::prelude::*;
use fuels_abigen_macro::abigen;

// This will generate your contract's methods onto `MyContract`.
// This means an instance of `MyContract` will have access to all
// your contract's methods that are running on-chain!
abigen!(
    MyContract,
    "your_project/out/debug/contract_test-abi.json",
);

// This helper will launch a local node and provide a test wallet linked to it
let wallet = launch_provider_and_get_single_wallet().await;

// Optional: Configure deployment parameters or use `TxParameters::default()`
let gas_price = 0;
let gas_limit = 1_000_000;
let byte_price = 0;

// This will deploy your contract binary onto the chain so that its ID can
// be used to initialize the instance
let contract_id_one = Contract::deploy_with_salt(
    "your_project/out/debug/contract_test.bin",
    &wallet,
    TxParameters::default(gas_price, gas_limit, byte_price),
    Salt::from([1u8; 32]),
)
.await
.unwrap();

// Here is the same contract deployment but under a new ID
let contract_id_two = Contract::deploy_with_salt(
    "your_project/out/debug/contract_test.bin",
    &wallet,
    TxParameters::default(gas_price, gas_limit, byte_price),
    Salt::from([2u8; 32]),
)
.await
.unwrap();

// Here is an instance of your contract which you can use to make calls to
// your functions
let contract_one = MyContract::new(contract_id_one.to_string(), wallet.clone());

// Here is the second instance
let contract_two = MyContract::new(contract_id_two.to_string(), wallet.clone());
```

## Setting up multiple test wallets

If you need multiple test wallets, they can be setup as follows:

```rust,ignore
// This helper will launch a local node and provide 10 test wallets linked to it.
// The initial balance defaults to 1 coin per wallet with an amount of 1_000_000_000
let wallets = launch_provider_and_get_wallets(WalletsConfig::default()).await;
```

The returned test wallets can be customized via `WalletsConfig`

```rust,ignore
let num_wallets = 5;
let coins_per_wallet = 3;
let amount_per_coin = 100;

let config = WalletsConfig::new(
    Some(num_wallets),
    Some(coins_per_wallet),
    Some(amount_per_coin)
);

// Launches a local node and provides test wallets as specified by the config
let wallets = launch_provider_and_get_wallets(WalletsConfig::default()).await;
```

## Calling and configuring contract calls

Once you've deployed your contract, as seen in the previous section, you'll likely want to call contract methods and configure some parameters such as gas price, byte price, gas limit, and forward coins in your contract call.

Start by creating an instance of your contract once you have a wallet set up:

```rust,ignore
let contract_instance = MyContract::new(contract_id.to_string(), wallet);
```

Then we move to configuring contract calls.

### `TxParameters`

Transaction parameters are:

1. Gas price;
2. Gas limit;
3. Byte price;
4. Maturity.

These parameters can be configured by creating an instance of [`TxParameters`](https://github.com/FuelLabs/fuels-rs/blob/adf81bd451d7637ce0976363bd7784408430031a/packages/fuels-contract/src/parameters.rs#L7) and passing it to a chain method called `tx_params`:

```rust,ignore
// In order: gas_price, gas_limit, byte_price, and maturity
let my_tx_params = TxParameters::new(None, Some(1_000_000), None, None);

let result = contract_instance
        .initialize_counter(42)  // Our contract method.
        .tx_params(my_tx_params) // Chain the tx params setting method.
        .call()                  // Perform the contract call.
        .await                   // This is an async call, `.await` for it.
        .unwrap();               // It returns a `Result<CallResponse<D>, Error>,
                                 // unwrap or handle it in any other preferable way.
```

You can also use `TxParameters::default()` to use the [default values](https://github.com/FuelLabs/fuels-rs/blob/adf81bd451d7637ce0976363bd7784408430031a/packages/fuels-core/src/constants.rs#L4-L7).

### `CallParameters`

Call parameters are:

1. Amount;
2. Asset ID;

This is commonly used to forward coins to a contract. These parameters can be configured by creating an instance of [`CallParameters`](https://github.com/FuelLabs/fuels-rs/blob/adf81bd451d7637ce0976363bd7784408430031a/packages/fuels-contract/src/parameters.rs#L15) and passing it to a chain method called `call_params`.

For instance, suppose the following contract that makes use of Sway's `msg_amount()` to return the amount sent in that message to the contract:

```rust,ignore
abi FuelTest {
    fn get_msg_amount() -> u64;
}

impl FuelTest for Contract {
    fn get_msg_amount() -> u64 {
        msg_amount()
    }
}
```

Then, in Rust, after setting up and deploying the above contract, you can configure the amount being sent to the `get_msg_amount()` method like this:

```rust,ignore
let tx_params = TxParameters::new(None, Some(1_000_000), None, None);

// Forward 1_000_000 coin amount of native asset_id
// this is a big number for checking that amount can be a u64
let call_params = CallParameters::new(Some(1_000_000), None);

let response = contract_instance
    .get_msg_amount()          // Our contract method.
    .tx_params(tx_params)      // Chain the tx params setting method.
    .call_params(call_params)  // Chain the call params setting method.
    .call()                    // Perform the contract call.
    .await
    .unwrap();
```

You can also use `CallParameters::default()` to use the default values:

```rust,ignore
pub const DEFAULT_COIN_AMOUNT: u64 = 1_000_000;
pub const NATIVE_ASSET_ID: AssetId = AssetId::new([0u8; 32]);
```

### `CallResponse`: Reading returned values

You've probably noticed that you're often chaining `.call().await.unwrap(). That's because:

1. You have to choose between `.call()` and `.simulate()` (more on this in the next section);
2. Contract calls are asynchronous, so you can choose to either `.await` it or perform concurrent tasks, making full use of Rust's async;
3. `.unwrap()` the `Result<CallResponse, Error>` returned by the contract call.

Once you unwrap the `CallResponse`, you have access to this struct:

```rust,ignore
pub struct CallResponse<D> {
    pub value: D,
    pub receipts: Vec<Receipt>,
    pub logs: Option<Vec<String>>,
}
```

Where `value` will hold the value returned by its respective contract method, represented by the exact type returned by the FuelVM. E.g., if your contract returns a FuelVM's `u64`, `value`'s `D` will be a `u64`. If it's a FuelVM's tuple `(u8,bool)`, then `D` will be a `(u8,bool)`. If it's a custom type, for instance, a Sway struct `MyStruct` containing 2 components, a `u64` and a `b256`, `D` will be a struct generated at compile-time, called `MyStruct` with `u64` and a `[u8; 32]` (the equivalent of `b256` in Rust-land).

`receipts` will hold all [receipts](https://github.com/FuelLabs/fuel-specs/blob/master/specs/protocol/abi.md#receipt) generated by that specific contract call.

And lastly, `logs` will hold all logs that happened within that specific contract call.

In order to log out `receipts` values during testing you have to run `test` as follows:

```sh
RUST_LOG=receipts cargo test --test harness $NAME_OF_TEST
```

### Read-only contract calls

Sometimes you want to call a contract method that doesn't change the state of the blockchain. For instance, a method that only reads a value from storage and returns it.

In this case, there's no need to generate an actual blockchain transaction; you only want to quickly read a value.

You can do this with the SDK by, instead of `.call()`ing the method, using `.simulate()` instead:

```rust,ignore
let my_balance = contract_instance.return_my_balance().simulate().await.unwrap();
```

Note that if you use `.simulate()` on a method that _does_ change the state of the blockchain, it won't work properly; it will just `dry-run` it.

At the moment, it's up to you to know whether a contract method changes state or not, and use `.call()` or `.simulate()` accordingly.

### Variable outputs

In some cases, you might want to send funds to the output of a transaction. Sway has a specific method for that: `transfer_to_output(coins, asset_id, recipient)`. So, if you have a contract that does something like this:

```rust,ignore
contract;

use std::{address::Address, context::balance_of, context::msg_amount, contract_id::ContractId, token::*};

abi FuelTest {
    fn transfer_coins_to_output(coins: u64, asset_id: ContractId, recipient: Address);
}

impl FuelTest for Contract {
    fn transfer_coins_to_output(coins: u64, asset_id: ContractId, recipient: Address) {
        transfer_to_output(coins, asset_id, recipient);
    }
}
```

With the SDK, you can call `transfer_coins_to_output`, by chaining `append_variable_outputs(amount)` to your contract call. Like this:

```rust,ignore
let address = wallet.address();

// withdraw some tokens to wallet
contract_instance
    .transfer_coins_to_output(1_000_000, id, address)
    .append_variable_outputs(1)
    .call()
    .await
    .unwrap();
```

`append_variable_outputs` effectively appends a given amount of `Output::Variable`s to the transaction's list of outputs. This output type indicates that the output's amount and owner may vary based on transaction execution.

Note that the Sway `lib-std` function `mint_to_address` calls `transfer_to_output` under the hood, and so you need to call `append_variable_outputs` in the Rust SDK tests just like you would for `transfer_to_output`.

### Contract calls calling other contracts

Sometimes, you might need to call your contract, which calls other contracts. To do so, you must feed the external contract IDs that your contract depends on to the method you're calling. You do it by chaining `.set_contracts(&[external_contract_id, ...])` to the method you want to call. For instance:

```rust,ignore
let response = contract_instance
.my_method(...)
.set_contracts( & [another_contract_id]) // Add this to set the external contract
.call()
.await
.unwrap();
```

For a more concrete example, see the `test_contract_calling_contract` function in
`fuels-abigen-macro/tests/harness.rs`

## Connecting to existing contracts

If you already have a deployed contract and want to call its methods using the SDK,  but without deploying it again, all you need is the contract ID of your deployed contract. You can skip the whole deployment setup and call `::new(contract_id, wallet)` directly. For example:

```rust,ignore
abigen!(
    MyContract,
    "path/to/abi.json"
);

let wallet = launch_provider_and_get_single_wallet().await;

let contract_id = "0x0123..." // Your contract ID as a string.

let connected_contract_instance = MyContract::new(contract_id, wallet);
```

## More examples

You can find runnable examples under [`fuels-abigen-macro/tests/harness.rs`](https://github.com/FuelLabs/fuels-rs/blob/master/packages/fuels-abigen-macro/tests/harness.rs).
