# fuels-rs

[![build](https://github.com/FuelLabs/fuels-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/FuelLabs/fuels-rs/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/fuels-contract?label=latest)](https://crates.io/crates/fuels-contract)
[![docs](https://docs.rs/fuels-contract/badge.svg)](https://docs.rs/fuels-contract)
[![discord](https://img.shields.io/badge/chat%20on-discord-orange?&logo=discord&logoColor=ffffff&color=7389D8&labelColor=6A7EC2)](https://discord.gg/xfpK4Pe)

Rust SDK for Fuel. It can be used for a variety of things, including but not limited to:

- Compiling, deploying, and testing [Sway](https://github.com/FuelLabs/sway) contracts;
- Launching a local Fuel network;
- Crafting and signing transactions with hand-crafted scripts or contract calls;
- Generating type-safe Rust bindings of contract methods;
- And more, `fuels-rs` is still in active development.

## Features

- [x] Launch Fuel nodes
- [x] Deploy contracts
- [x] Interact with deployed contracts
- [x] Type-safe Sway contracts bindings code generation
- [x] Run Sway scripts
- [x] CLI for common operations
- [x] Local test wallets
- [ ] Wallet integration
- [ ] Events querying/monitoring

## Using the SDK

This section describes how to use the basic functionalities of the SDK.

### Instantiating a Fuel client

You can instantiate a Fuel client, pointing to a local Fuel node by using [Fuel Core](https://github.com/FuelLabs/fuel-core):

```Rust
use fuel_core::service::{Config, FuelService};
use fuel_gql_client::client::FuelClient;

let server = FuelService::new_node(Config::local_node()).await.unwrap();

let client = FuelClient::from(srv.bound_address);
```

Alternatively, if you have a Fuel node running separately, you can pass in the `SocketAddr` to `FuelClient::from()`.

It's important to setup this client, as it will be needed later when instantiating contracts. More on that on the section below.

### Deploying a Sway contract

Once you have a Fuel node running and the compiled contract in hands, it's time to deploy the contract:

```Rust
let salt: [u8; 32] = rng.gen();
let salt = Salt::from(salt);
// Setup a local node
let server = FuelService::new_node(Config::local_node()).await.unwrap();
let (provider, wallet) = setup_test_provider_and_wallet().await;

// Load the compiled Sway contract (this is the output from `forc build`)
let compiled = Contract::load_sway_contract(
        "your_project/out/debug/contract_test.bin",
        salt,
    )
    .unwrap();

// Deploy the contract
let contract_id = Contract::deploy(&compiled, provider, wallet).await.unwrap();
```

Alternatively, if you want to launch a local node for every deployment, which is usually useful for smaller tests where you don't want to keep state between each test, you can use `Provider::launch(Config::local_node())`:

```Rust
// Build the contract
let salt: [u8; 32] = rng.gen();
let salt = Salt::from(salt);

let compiled = Contract::load_sway_contract(
        "your_project/out/debug/contract_test.bin",
        salt,
    )
    .unwrap();

// Now get the Fuel client provider _and_ contract_id back.
let (pk, coins) = setup_address_and_coins(1, DEFAULT_COIN_AMOUNT);
let client = Provider::launch(Config::local_node()).await.unwrap();
let provider = Provider::new(client);
let wallet = LocalWallet::new_from_private_key(pk, provider.clone()).unwrap();
let contract_id = Contract::deploy(&compiled, &provider,&wallet).await.unwrap();
```

### Generating type-safe Rust bindings

The SDK lets you transform ABI methods of a contract call, specified as JSON objects (which you can get from [Forc](https://github.com/FuelLabs/sway/tree/master/forc)) into Rust structs and methods that are type-checked at compile time.

For instance, a contract with two methods: `initialize_counter(arg: u64) -> u64` and `increment_counter(arg: u64) -> u64`, with the following JSON ABI:

```json
[
    {
        "type": "function",
        "inputs": [
            {
                "name": "arg",
                "type": "u64"
            }
        ],
        "name": "initialize_counter",
        "outputs": [
            {
                "name": "arg",
                "type": "u64"
            }
        ]
    },
    {
        "type": "function",
        "inputs": [
            {
                "name": "arg",
                "type": "u64"
            }
        ],
        "name": "increment_counter",
        "outputs": [
            {
                "name": "arg",
                "type": "u64"
            }
        ]
    }
]
```

Can become this (shortened for brevity's sake):

```Rust
// Note that is all GENERATED code. No need to write any of that. Ever.
pub struct MyContract {
    contract_id: FuelContractId,
    provider: Provider,
    wallet: LocalWallet,
}
impl MyContract {
    pub fn new(contract_id: String, provider: Provider, wallet: LocalWallet) -> Self {
        let contract_id = FuelContractId::from_str(&contract_id).unwrap();
        Self {
            contract_id,
            provider,
            wallet,
        }
    }
    #[doc = "Calls the contract\'s `initialize_counter` (0x00000000ab64e5f2) function"]
    pub fn initialize_counter(&self, value: u64) -> ContractCall<u64> {
        Contract::method_hash(
            &self.provider,
            self.contract_id,
            &self.wallet,
            [0, 0, 0, 0, 171, 100, 229, 242],
            &[ParamType::U64],
            &[value.into_token()],
        )
            .expect("method not found (this should never happen)")
    }
    #[doc = "Calls the contract\'s `increment_counter` (0x00000000faf90dd3) function"]
    pub fn increment_counter(&self, value: u64) -> ContractCall<u64> {
        Contract::method_hash(
            &self.provider,
            self.contract_id,
            &self.wallet,
            [0, 0, 0, 0, 250, 249, 13, 211],
            &[ParamType::U64],
            &[value.into_token()],
        )
            .expect("method not found (this should never happen)")
    }
}
```

And, then, you're able to use to call the actual methods on the deployed contract:

```Rust
//...
let contract_instance = MyContract::new(contract_id.to_string(), provider, wallet);

let result = contract_instance
    .initialize_counter(42) // Build the ABI call
    .call() // Perform the network call
    .await
    .unwrap();

assert_eq!(42, result.unwrap());

let result = contract_instance
    .increment_counter(10)
    .call()
    .await
    .unwrap();

assert_eq!(52, result.unwrap());
```

To generate these bindings, all you have to do is:

```Rust
use fuels_abigen_macro::abigen;

abigen!(
    MyContractName,
    "path/to/json/abi.json"
);
```

And this `abigen!` macro will _expand_ the code with the type-safe Rust bindings. It takes 2 arguments:

1. The name of the struct that will be generated (`MyContractName`);
2. Either a path as string to the JSON ABI file or the JSON ABI as a multiline string directly.

The same as the example above but passing the ABI definition directly:

```Rust
use fuels_abigen_macro::abigen;

abigen!(
    MyContractName,
    r#"
    [
        {
            "type": "function",
            "inputs": [
                {
                    "name": "arg",
                    "type": "u64"
                }
            ],
            "name": "initialize_counter",
            "outputs": [
                {
                    "name": "arg",
                    "type": "u64"
                }
            ]
        },
        {
            "type": "function",
            "inputs": [
                {
                    "name": "arg",
                    "type": "u64"
                }
            ],
            "name": "increment_counter",
            "outputs": [
                {
                    "name": "arg",
                    "type": "u64"
                }
            ]
        }
    ]
    "#
);
```

### Multi-contract calls

Sometimes, you might need to call your contract, which calls other contracts. To do so, you must feed the external contract IDs that your contract depends on to the method you're calling. You do it by chaining `.set_contracts(&[external_contract_id, ...])` to the method you want to call. For instance:

```Rust
let response = my_contract
    .my_method(...)
    .set_contracts(&[another_contract_id]) // Add this to set the external contract
    .call()
    .await
    .unwrap();
```

For a more concrete example, see the `test_contract_calling_contract` function in
`fuels-abigen-macro/tests/harness.rs`

## More examples

You can find runnable examples under `fuels-abigen-macro/tests/harness.rs` and  `fuels-contract/tests/calls.rs`.
