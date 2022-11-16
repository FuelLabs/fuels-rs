# Deploying contracts

There are two main ways of working with contracts in the SDK: deploying a contract with SDK or using the SDK to interact with existing contracts.

## Deploying a contract binary

Once you've written a contract in Sway and compiled it with `forc build` (read [here](https://fuellabs.github.io/sway/master/introduction/overview.html) for more on how to work with Sway), you'll have in your hands two important artifacts: the compiled binary file and the JSON ABI file.

Below is how you can deploy your contracts using the SDK. For more details about each component in this process, read [The abigen macro](../contracts/the-abigen-macro.md), [The FuelVM binary file](../contracts/the-fuelvm-binary-file.md), and [The JSON ABI file](../contracts/the-json-abi-file.md).

### The deploy functions

There are two intended ways to deploy a contract

- `deploy`
- `deploy_with_parameters`

If you are only interested in a single instance of your contract, then use `deploy`

```rust,ignore
{{#include ../../../examples/contracts/src/lib.rs:deploy_contract}}
```

You can then use the contract methods very simply:

```rust,ignore
{{#include ../../../examples/contracts/src/lib.rs:use_deployed_contract}}
```

Alternatively, if you want multiple instances of the same contract then use `deploy_with_parameters` and set the salt parameter.

```rust,ignore
{{#include ../../../examples/contracts/src/lib.rs:deploy_with_parameters}}
```
