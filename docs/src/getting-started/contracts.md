# Deploying contracts

There are two main ways of working with contracts in the SDK: deploying a contract with SDK or using the SDK to interact with existing contracts.

## Deploying a contract binary

Once you've written a contract in Sway and compiled it with `forc build` (read [here](https://fuellabs.github.io/sway/master/introduction/sway_quickstart.html) for more on how to work with Sway), you'll have in your hands two important artifacts: the compiled binary file and the JSON ABI file.

Below is how you can deploy your contracts using the SDK. For more details about each component in this process, read [The abigen macro](../abigen/the-abigen-macro.md), [The FuelVM binary file](../contracts/the-fuelvm-binary-file.md), and [The JSON ABI file](../abigen/the-json-abi-file.md).

### The deploy functions

The `Contract::deploy` function is used to deploy a contract binary to the blockhain. To configure the deployment you can use the `DeployConfiguration` struct.

If you are only interested in a single instance of your contract, use the default configuration: `DeployConfiguration::default()`

```rust,ignore
{{#include ../../../examples/contracts/src/lib.rs:deploy_contract}}
```

You can then use the contract methods very simply:

```rust,ignore
{{#include ../../../examples/contracts/src/lib.rs:use_deployed_contract}}
```

Alternatively, you can use `DeployConfiguration` to configure the deployment. You can, for example, deploy multiple instances of the same contract, change the storage or `tx_parameters` or update the binary using `configurables`.

```rust,ignore
{{#include ../../../examples/contracts/src/lib.rs:deploy_with_parameters}}
```

> Note: The next section will give more information on how `configurables` can be used.
