# The setup_contract_test! macro

When deploying contracts with the `abigen!` macro, as shown in the previous sections, the user has the ability to change the default configuration parameters, lunch several providers, create multiple wallets, create specific assets, etc. However, it is often the case that we want to test only the contract methods and we want to simply deploy the contract with the default configuration parameters. The `setup_contract_test!` macro does exactly that. When expanded, the `setup_contract_test!` macro will: run the abigen, launch a local provider, setup one wallet and deploy the selected contract. The setup code that you have seen in previous sections gets reduced to:

```rust,ignore
{{#include ../../../examples/contracts/src/lib.rs:deploy_contract_setup_macro_short}}
```

The input of the macro are the contract instance variable name, wallet variable name and the forc project path. Both the contract instance and wallet variables get brought into context and they can be used further in the code.

The same contract can be deployed several times as the macro deploys the contracts with salt.

You can also deploy different contracts to the same provider using a shared wallet.

This is done by setting the wallet name of the first macro to `shared_wallet` and all the remaining wallet names to `None`. The first macro will create `shared_wallet` and bring it into context and the other macros will use it instead of creating new ones. An example that showcases the deployment with salt and using the `shared_wallet` is given below.

```rust,ignore
{{#include ../../../packages/fuels/tests/harness.rs:contract_setup_macro_multi}}
```
