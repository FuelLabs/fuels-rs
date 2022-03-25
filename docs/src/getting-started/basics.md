## Basic usage of the SDK

### Instantiating a Fuel client

You can instantiate a Fuel client, pointing to a local Fuel node by
using [Fuel Core](https://github.com/FuelLabs/fuel-core):

```Rust
use fuel_core::service::{Config, FuelService};
use fuel_gql_client::client::FuelClient;

let server = FuelService::new_node(Config::local_node()).await.unwrap();

let client = FuelClient::from(srv.bound_address);
```

Alternatively, if you have a Fuel node running separately, you can pass in the `SocketAddr`
to `FuelClient::from()`.

It's important to setup this client, as it will be needed later when instantiating contracts. More
on that on the section below.

### Deploying a Sway contract

Once you have a Fuel node running and the compiled contract in hands, it's time to deploy the
contract:

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

// Configure deployment parameters.
// Alternatively you can use the defaults through `TxParameters::default()`.
let gas_price = 0;
let gas_limit = 1_000_000;
let byte_price = 0;

// Deploy the contract
let contract_id = Contract::deploy( & compiled, provider, wallet,
TxParameters::new(gas_price, gas_limit, byte_price)).await.unwrap();
```

Alternatively, if you want to launch a local node for every deployment, which is usually useful
for smaller tests where you don't want to keep state between each test, you can
use `Provider::launch(Config::local_node())`:

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
let contract_id = Contract::deploy( & compiled, & provider, & wallet, TxParameters::default ()).await.unwrap();
```

### Multi-contract calls

Sometimes, you might need to call your contract, which calls other contracts. To do so, you must
feed the external contract IDs that your contract depends on to the method you're calling. You do
it by chaining `.set_contracts(&[external_contract_id, ...])` to the method you want to call. For
instance:

```Rust
let response = my_contract
.my_method(...)
.set_contracts( & [another_contract_id]) // Add this to set the external contract
.call()
.await
.unwrap();
```

For a more concrete example, see the `test_contract_calling_contract` function in
`fuels-abigen-macro/tests/harness.rs`

## More examples

You can find runnable examples under `fuels-abigen-macro/tests/harness.rs`
and  `fuels-contract/tests/calls.rs`.
