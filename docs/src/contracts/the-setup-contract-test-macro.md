# The setup_contract_test! macro

When deploying contracts with the `abigen!` macro, as shown in the previous sections, the user can:
- change the default configuration parameters
- launch several providers
- create multiple wallets
- create specific assets, etc.

However, it is often the case that we want to test only the contract methods and we want to simply deploy the contract with the default configuration parameters. The `setup_contract_test!` macro does exactly that. When expanded, the `setup_contract_test!` macro will:
1. run the `abigen`
2. launch a local provider
3. setup one wallet
4. deploy the selected contract

The setup code that you have seen in previous sections gets reduced to:

```rust,ignore
{{#include ../../../examples/contracts/src/lib.rs:deploy_contract_setup_macro_short}}
```

The input of the macro are the contract instance variable name, wallet variable name and the forc project path. Both the contract instance and wallet variables get brought into context and they can be used further in the code.

>**Note** The same contract can be deployed several times as the macro deploys the contracts with salt. You can also deploy different contracts to the same provider using a shared wallet.

If you want to deploy contract to the same provider you have to set the wallet name of the first macro to `shared_wallet` and all the remaining wallet names to `None`. The first macro will create `shared_wallet` and bring it into context and the other macros will use it instead of creating new ones. Let's see it in an example.

```rust,ignore
{{#include ../../../packages/fuels/tests/harness.rs:contract_setup_macro_multi}}
```
In this example, three contracts are deploy on the same provider using the `shared_wallet`. The second and third macro use the same contract but they have different IDs because of the deployment with salt. Both of them can call the first contract by using its ID.
