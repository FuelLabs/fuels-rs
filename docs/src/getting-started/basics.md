# Basic Usage of the SDK

At a high level, the Fuel Rust SDK can be used to build Rust-based applications that can run computations on the Fuel Virtual Machine through interactions with contracts written in Sway.

For this interaction to work, the SDK must be able to communicate to a `fuel-core` node; you have two options at your disposal:

1. Use the SDK's native `launch_provider_and_get_single_wallet()` that runs a short-lived test Fuel node;
2. Run a Fuel node outside your SDK code (using `fuel-core`) and point your SDK to that node's IP and port.

The first option is ideal for contract testing, as you can quickly spin up and tear down nodes between specific test cases.

For application building, you probably want to go with the second option.

## Instantiating a Fuel client

You can instantiate a Fuel client, pointing to a local Fuel node by
using [`FuelClient`](https://docs.rs/fuels/*/fuels/client/struct.FuelClient.html):

```rust,ignore
{{#include ../../../examples/contracts/src/lib.rs:instantiate_client}}
```

Alternatively, if you have a Fuel node running separately, you can pass in the `SocketAddr` to `FuelClient::from()`.

## Deploying a Sway contract

There are two intended ways to deploy a contract

- `deploy`
- `deploy_with_salt`

If you are only interested in a single instance of your contract then use `deploy`

```rust,ignore
{{#include ../../../examples/contracts/src/lib.rs:deploy_contract}}
```

You can then use the contract methods very simply:


```rust,ignore
{{#include ../../../examples/contracts/src/lib.rs:use_deployed_contract}}
```

Alternatively, if you want multiple instances of the same contract then use `deploy_with_salt`

```rust,ignore
{{#include ../../../examples/contracts/src/lib.rs:deploy_with_salt}}
```

## Setting up multiple test wallets

If you need multiple test wallets, they can be setup as follows:

```rust,ignore
{{#include ../../../examples/wallets/src/lib.rs:multiple_wallets_helper}}
```

The returned test wallets can be customized via `WalletsConfig`

```rust,ignore
{{#include ../../../examples/wallets/src/lib.rs:setup_5_wallets}}
```

## Setting up a test wallet with multiple assets

You can create a test wallet which contains multiple different assets (including the base asset to pay for gas).

```rust,ignore
{{#include ../../../examples/wallets/src/lib.rs:multiple_assets_wallet}}
```

- `coins: Vec<(UtxoId, Coin)>` has `num_assets * coins_per_assets` coins (UTXOs)
- `asset_ids: Vec<AssetId>` contains the `num_assets` randomly generated `AssetId`s (always includes the base asset)

## Calling and configuring contract calls

Once you've deployed your contract, as seen in the previous section, you'll likely want to call contract methods and configure some parameters such as gas price, byte price, gas limit, and forward coins in your contract call.

Start by creating an instance of your contract once you have a wallet set up:

```rust,ignore
{{#include ../../../examples/contracts/src/lib.rs:instantiate_contract}}
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
{{#include ../../../examples/contracts/src/lib.rs:tx_parameters}}
```
You can also use `TxParameters::default()` to use the default values:

```rust,ignore
{{#include ../../../packages/fuels-core/src/constants.rs:default_tx_parameters}}
```

this way:

```rust,ignore
{{#include ../../../examples/contracts/src/lib.rs:tx_parameters_default}}
```


### `CallParameters`

Call parameters are:

1. Amount;
2. Asset ID;
3. Gas forwarded.

This is commonly used to forward coins to a contract. These parameters can be configured by creating an instance of [`CallParameters`](https://github.com/FuelLabs/fuels-rs/blob/adf81bd451d7637ce0976363bd7784408430031a/packages/fuels-contract/src/parameters.rs#L15) and passing it to a chain method called `call_params`.

For instance, suppose the following contract that makes use of Sway's `msg_amount()` to return the amount sent in that transaction.

```rust,ignore
{{#include ../../../packages/fuels-abigen-macro/tests/test_projects/contract_test/src/main.sw:msg_amount}}
```

Then, in Rust, after setting up and deploying the above contract, you can configure the amount being sent in the transaction like this:

```rust,ignore
{{#include ../../../examples/contracts/src/lib.rs:call_parameters}}
```

You can also use `CallParameters::default()` to use the default values:

```rust,ignore
{{#include ../../../packages/fuels-core/src/constants.rs:default_call_parameters}}
```

this way:


```rust,ignore
{{#include ../../../examples/contracts/src/lib.rs:call_parameters_default}}
```

The `gas_forwarded` parameter defines the limit for the actual contract call as opposed to the gas limit for the whole transaction. This means that it is constrained by the transaction limit. If it is set to an amount greater than the available gas, all available gas will be forwarded.


```rust,ignore
{{#include ../../../examples/contracts/src/lib.rs:call_params_gas}}
```

### `CallResponse`: Reading returned values

You've probably noticed that you're often chaining `.call().await.unwrap(). That's because:

1. You have to choose between `.call()` and `.simulate()` (more on this in the next section);
2. Contract calls are asynchronous, so you can choose to either `.await` it or perform concurrent tasks, making full use of Rust's async;
3. `.unwrap()` the `Result<CallResponse, Error>` returned by the contract call.

Once you unwrap the `CallResponse`, you have access to this struct:

```rust,ignore
{{#include ../../../packages/fuels-contract/src/contract.rs:call_response}}
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
{{#include ../../../examples/contracts/src/lib.rs:simulate}}
```

Note that if you use `.simulate()` on a method that _does_ change the state of the blockchain, it won't work properly; it will just `dry-run` it.

At the moment, it's up to you to know whether a contract method changes state or not, and use `.call()` or `.simulate()` accordingly.

### Variable outputs

In some cases, you might want to send funds to the output of a transaction. Sway has a specific method for that: `transfer_to_output(coins, asset_id, recipient)`. So, if you have a contract that does something like this:

```rust,ignore
{{#include ../../../packages/fuels-abigen-macro/tests/test_projects/token_ops/src/main.sw:variable_outputs}}
```

With the SDK, you can call `transfer_coins_to_output`, by chaining `append_variable_outputs(amount)` to your contract call. Like this:

```rust,ignore
{{#include ../../../examples/contracts/src/lib.rs:variable_outputs}}
```

`append_variable_outputs` effectively appends a given amount of `Output::Variable`s to the transaction's list of outputs. This output type indicates that the output's amount and owner may vary based on transaction execution.

Note that the Sway `lib-std` function `mint_to_address` calls `transfer_to_output` under the hood, and so you need to call `append_variable_outputs` in the Rust SDK tests just like you would for `transfer_to_output`.

### Contract calls calling other contracts

Sometimes, you might need to call your contract, which calls other contracts. To do so, you must feed the external contract IDs that your contract depends on to the method you're calling. You do it by chaining `.set_contracts(&[external_contract_id, ...])` to the method you want to call. For instance:

```rust,ignore
{{#include ../../../packages/fuels-abigen-macro/tests/harness.rs:external_contract}}
```

For a more concrete example, see the `test_contract_calling_contract` function in
`fuels-abigen-macro/tests/harness.rs`

## Connecting to existing contracts

If you already have a deployed contract and want to call its methods using the SDK,  but without deploying it again, all you need is the contract ID of your deployed contract. You can skip the whole deployment setup and call `::new(contract_id, wallet)` directly. For example:

```rust,ignore
{{#include ../../../examples/contracts/src/lib.rs:deployed_contracts}}
```

## Getting the contract call outputs

- Getting the contract call outputs is done this way:

```rust,ignore
{{#include ../../../examples/contracts/src/lib.rs:contract_receipts}}
```

> **Note:** It is generally considered good practice when you expect the call to succeed, to unwrap the response with `?`, this way:
>
> ```rust, ignore
> {{#include ../../../examples/contracts/src/lib.rs:good_practice}}
> ```

## More examples

You can find runnable examples under [`fuels-abigen-macro/tests/harness.rs`](https://github.com/FuelLabs/fuels-rs/blob/master/packages/fuels-abigen-macro/tests/harness.rs).
