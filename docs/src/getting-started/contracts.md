# Deploying contracts

There are two main ways of working with contracts in the SDK: deploying a contract with SDK or using the SDK to interact with existing contracts.

## Deploying a contract binary

Once you've written a contract in Sway and compiled it with `forc build` (read [here](https://fuellabs.github.io/sway/master/introduction/sway_quickstart.html) for more on how to work with Sway), you'll have in your hands two important artifacts: the compiled binary file and the JSON ABI file.

Below is how you can deploy your contracts using the SDK. For more details about each component in this process, read [The abigen macro](../abigen/the-abigen-macro.md), [The FuelVM binary file](../contracts/the-fuelvm-binary-file.md), and [The JSON ABI file](../abigen/the-json-abi-file.md).

First, the `Contract::load_from` function is used to load a contract binary with a `LoadConfiguration`. If you are only interested in a single instance of your contract, use the default configuration: `LoadConfiguration::default()`. After the contract binary is loaded you can use the `deploy()` method to deploy the contract to the blockchain.

```rust,ignore
{{#include ../../../examples/contracts/src/lib.rs:deploy_contract}}
```

You can then use the contract methods very simply:

```rust,ignore
{{#include ../../../examples/contracts/src/lib.rs:use_deployed_contract}}
```

Alternatively, you can use `LoadConfiguration` to configure how the contract is loaded. You can, for example, load the same contract with different `Salt`, change the contract's storage or update the binary using `configurables`.

```rust,ignore
{{#include ../../../examples/contracts/src/lib.rs:deploy_with_parameters}}
```

> Note: The next section will give more information on how `configurables` can be used.
