# The JSON ABI file

<!-- This section should talk about the importance of the ABI -->
<!-- abi:example:start -->
Whether you want to deploy or connect to a pre-existing smart contract, the JSON ABI file is extremely important: it's what tells the SDK about the [ABI methods](https://docs.fuel.network/guides/quickstart/building-a-smart-contract/#abi) in your smart contracts.
<!-- abi:example:end -->

For the same example Sway code as above:

```Rust
contract;

abi MyContract {
    fn test_function() -> bool;
}

impl MyContract for Contract {
    fn test_function() -> bool {
        true
    }
}
```

The JSON ABI file looks like this:

```json
$ cat out/release/my-test-abi.json
[
  {
    "type": "function",
    "inputs": [],
    "name": "test_function",
    "outputs": [
      {
        "name": "",
        "type": "bool",
        "components": null
      }
    ]
  }
]
```

The Fuel Rust SDK will take this file as input and generate equivalent methods (and custom types if applicable) that you can call from your Rust code.
