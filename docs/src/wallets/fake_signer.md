# Fake signer (impersonating another account)

To facilitate account impersonation, the Rust SDK provides the `FakeSigner`. We can use it to simulate ownership of assets held by an account with a given address. This also implies that we can impersonate contract calls from that address. A wallet with a `FakeSigner` will only succeed in unlocking assets if the network is set up with `utxo_validation = false`.

```rust,ignore
{{#include ../../../examples/contracts/src/lib.rs:utxo_validation_off}}
```

```rust,ignore
{{#include ../../../examples/contracts/src/lib.rs:utxo_validation_off_node_start}}
```

```rust,ignore
{{#include ../../../examples/contracts/src/lib.rs:contract_call_impersonation}}
```
