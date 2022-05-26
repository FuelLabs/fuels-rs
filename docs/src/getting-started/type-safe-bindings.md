# Generating Type-safe Rust Bindings

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

```rust
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

```rust
//...
let contract_instance = MyContract::new(contract_id.to_string(), provider, wallet);

let result = contract_instance
.initialize_counter(42) // Build the ABI call
// Perform the network call, this will use the default values for
// gas price (0), gas limit (1_000_000), and byte price (0).
.call()
.await
.unwrap();

assert_eq!(42, result.unwrap());

let result = contract_instance
.increment_counter(10)
.call()
// You can configure the parameters for a specific contract call:
.tx_params(TxParameters::new(Some(100), Some(1_000_000), Some(0)))
.await
.unwrap();

assert_eq!(52, result.unwrap());
```

To generate these bindings, all you have to do is:

```rust
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

```rust
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
