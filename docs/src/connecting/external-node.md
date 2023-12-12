# Connecting to the Testnet or an external node

We can interact with the `Testnet` node by using the following example.

```rust,ignore
{{#include ../../../examples/providers/src/lib.rs:connect_to_testnet}}
```
>
> For detailed information about various testnet networks and their optimal toolchain configurations for your project, please visit the following link:
>
> [networks](https://fuelbook.fuel.network/master/networks/networks.html)

In the code example, we connected a new provider to the Testnet node and created a new wallet from a private key.

> **Note:** New wallets on the Testnet will not have any assets! They can be obtained by providing the wallet address to the faucet at
>
>[faucet-beta-4.fuel.network](https://faucet-beta-4.fuel.network)
>
> Once the assets have been transferred to the wallet, you can reuse it in other tests by providing the private key!
>
> In addition to the faucet, there is a block explorer for the Testnet at
>
> [block-explorer](https://fuellabs.github.io/block-explorer-v2)

If you want to connect to another node just change the URL or IP and port. For example, to connect to a local node that was created with `fuel-core` you can use:

```rust,ignore
{{#include ../../../examples/providers/src/lib.rs:local_node_address}}
```
