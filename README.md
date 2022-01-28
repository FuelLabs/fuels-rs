# fuels-rs

[![build](https://github.com/FuelLabs/fuels-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/FuelLabs/fuels-rs/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/fuels-rs?label=latest)](https://crates.io/crates/fuels-rs)
[![docs](https://docs.rs/fuels-rs/badge.svg)](https://docs.rs/fuels-rs)
[![discord](https://img.shields.io/badge/chat%20on-discord-orange?&logo=discord&logoColor=ffffff&color=7389D8&labelColor=6A7EC2)](https://discord.gg/xfpK4Pe)

Rust SDK for Fuel. It can be used for a variety of things, including but not limited to:

- Compiling, deploying, and testing [Sway](https://github.com/FuelLabs/sway) contracts;
- Launching a local Fuel network;
- Crafting and signing transactions with hand-crafted scripts or contract calls;
- Generating type-safe Rust bindings of contract methods;
- And more, `fuels-rs` is still in active development.

## Features

- [x] Programmatically compile Sway code
- [x] Launch Fuel nodes
- [x] Deploy contracts
- [x] Interact with deployed contracts
- [x] Type-safe Sway contracts bindings code generation
- [x] Run Sway scripts
- [x] CLI for common operations
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

### Compiling Sway code

In order to instantiate Fuel contracts in Rust, you'll also need to compile a Sway contract. Which is done with the SDK's `Contract`:

```Rust
let salt: [u8; 32] = rng.gen();
let salt = Salt::from(salt);

let compiled =
    Contract::compile_sway_contract("path/to/your/fuel/project", salt).unwrap();
```

If you're running a `script` instead of a `contract`, use the SDK's `Script`:

```Rust
let compiled =
    Script::compile_sway_script("path/to/your/fuel/project")
        .unwrap();
```

### Deploying a Sway contract

Once you have a Fuel node running and the compiled contract in hands, it's time to deploy the contract:

```Rust
// Setup a local node
let server = FuelService::new_node(Config::local_node()).await.unwrap();
let client = FuelClient::from(srv.bound_address);

// Compile the contract
let salt: [u8; 32] = rng.gen();
let salt = Salt::from(salt);

let compiled =
    Contract::compile_sway_contract("path/to/your/fuel/project", salt).unwrap();

// Deploy the contract
let contract_id = Contract::deploy(compiled_contract, fuel_client).await.unwrap();
```

Alternatively, if you want to launch a local node for every deployment, which is usually useful for smaller tests where you don't want to keep state between each test, you can use `Contract::launch_and_deploy()`:

```Rust
// Build the contract
let salt: [u8; 32] = rng.gen();
let salt = Salt::from(salt);

let compiled =
    Contract::compile_sway_contract("path/to/your/fuel/project", salt).unwrap();

// Now you get both the Fuel client _and_ the contract_id back.
let (client, contract_id) = Contract::launch_and_deploy(&compiled).await.unwrap();
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
    compiled: CompiledContract,
    fuel_client: FuelClient,
}

impl MyContract {
    pub fn new(compiled: CompiledContract, fuel_client: FuelClient) -> Self {
        Self {
            compiled,
            fuel_client,
        }
    }
    #[doc = "Calls the contract\'s `initialize_counter` (0x0000000002dadd54) function"]
    pub fn initialize_counter(&self, arg: u64) -> ContractCall<u64> {
        Contract::method_hash(
            &self.fuel_client,
            &self.compiled,
            [0, 0, 0, 0, 2, 218, 221, 84],
            &[arg.into_token()],
        )
        .expect("method not found (this should never happen)")
    }
    #[doc = "Calls the contract\'s `increment_counter` (0x00000000e7f89992) function"]
    pub fn increment_counter(&self, arg: u64) -> ContractCall<u64> {
        Contract::method_hash(
            &self.fuel_client,
            &self.compiled,
            [0, 0, 0, 0, 231, 248, 153, 146],
            &[arg.into_token()],
        )
        .expect("method not found (this should never happen)")
    }
}
```

And, then, you're able to use to call the actual methods on the deployed contract:

```Rust
//...
let contract_instance = MyContract::new(compiled, client);

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

### Calling a Sway script

In case you want to hand-craft a Fuel transaction with a Sway script, you can use the SDK's `Script`:

```Rust
 let compiled =
    Script::compile_sway_script("path/to/fuel/project")
        .unwrap();

let tx = Transaction::Script {
    gas_price: 0,
    gas_limit: 1_000_000,
    maturity: 0,
    receipts_root: Default::default(),
    script: compiled.raw, // Here we pass the compiled script into the transaction
    script_data: vec![],
    inputs: vec![],
    outputs: vec![],
    witnesses: vec![vec![].into()],
    metadata: None,
};

let script = Script::new(tx);

let result = script.call(&fuel_client).await.unwrap();
```

## More examples

You can find runnable examples under `fuels-abigen-macro/tests/harness.rs` and  `fuels-contract/tests/calls.rs`.
