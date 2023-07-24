# Deploying contracts

There are two main ways of working with contracts in the SDK: deploying a contract with SDK or using the SDK to interact with existing contracts.

## Deploying a contract binary

<!-- This section should explain the artifacts produced by `forc build`  -->
<!-- build:example:start -->
Once you've written a contract in Sway and compiled it with `forc build`, you'll have in your hands two important artifacts: the compiled binary file and the JSON ABI file.
<!-- build:example:end -->
> Note: Read [here](https://fuellabs.github.io/sway/master/book/introduction/sway_quickstart.html) for more on how to work with Sway.

Below is how you can deploy your contracts using the SDK. For more details about each component in this process, read [The abigen macro](../abigen/the-abigen-macro.md), [The FuelVM binary file](../contracts/the-fuelvm-binary-file.md), and [The JSON ABI file](../abigen/the-json-abi-file.md).

<!-- This section should explain how to load and deploy a contract  -->
<!-- deploy:example:start -->
First, the `Contract::load_from` function is used to load a contract binary with a `LoadConfiguration`. If you are only interested in a single instance of your contract, use the default configuration: `LoadConfiguration::default()`. After the contract binary is loaded, you can use the `deploy()` method to deploy the contract to the blockchain.
<!-- deploy:example:end -->

```rust,ignore
{{#include ../../../examples/contracts/src/lib.rs:deploy_contract}}
```

Alternatively, you can use `LoadConfiguration` to configure how the contract is loaded. `LoadConfiguration` let's you:
- Load the same contract binary with `Salt` to get a new `contract_id`
- Change the contract's storage slots
- Update the contract's configurables
    > Note: The next section will give more information on how `configurables` can be used.

Additionally, you can set custom `TxParameters` when deploying the loaded contract.

```rust,ignore
{{#include ../../../examples/contracts/src/lib.rs:deploy_with_parameters}}
```

After the contract is deployed, you can use the contract's methods like this:

```rust,ignore
{{#include ../../../examples/contracts/src/lib.rs:use_deployed_contract}}
```

