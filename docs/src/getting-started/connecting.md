# Connecting to a Fuel node

<!-- This section should explain at a high level the main ways to connect to a node with the Rust SDK and when they are appropriate to use-->
<!-- rs_node:example:start -->
At a high level, you can use the Fuel Rust SDK to build Rust-based applications that can run computations on the Fuel Virtual Machine through interactions with smart contracts written in Sway.

For this interaction to work, the SDK must be able to communicate with a `fuel-core` node; you have two options at your disposal:

1. Use the testnet or run a Fuel node (using `fuel-core`) and instantiate a provider that points to that node's IP and port.
2. Use the SDK's native `launch_provider_and_get_wallet()` that runs a short-lived test Fuel node;

The second option is ideal for smart contract testing, as you can quickly spin up and tear down nodes between specific test cases.

For application building, you should use the first option.
<!-- rs_node:example:end -->
