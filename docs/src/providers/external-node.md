# Connecting to the Testnet or an external Fuel node

You can connect to the `Testnet` by using the following code snippet.

```rust,ignore
    use fuels::prelude::*;
    use std::net::{SocketAddr, ToSocketAddrs};

    // This is the testnet's address
    let server_address: SocketAddr = "https://node-beta-1.fuel.network/graphql:443"
        .to_socket_addrs()
        .expect("Unable to parse or lookup address")
        .next() // Take the first resolved address
        .unwrap();

    // Create the provider using the client.
    let provider = Provider::connect(server_address).await.unwrap();

    // Create the wallet.
    let _wallet = WalletUnlocked::new_random(Some(provider));
```

> **Note:** Assets for the Tesnet node can be obtained from the faucet at
>
>[faucet-beta-1.fuel.network](https://faucet-beta-1.fuel.network)
>
> There is also a block explerer for the Tesnet at
>
> [block-explorer](https://fuellabs.github.io/block-explorer-v2)

If you want to connect to another node just change the `server_address`. For example, to connect to a local node that was created with `fuel-core` you can use:

```rust,ignore
{{#include ../../../examples/providers/src/lib.rs:local_node_address}}
```

