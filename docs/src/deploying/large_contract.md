# Deploying Large Contracts

If your contract exceeds the size limit for a single deployment:

```rust,ignore
{{#include ../../../examples/contracts/src/lib.rs:show_contract_is_too_big}}
```

you can deploy it in parts using a segmented approach:

```rust,ignore
{{#include ../../../examples/contracts/src/lib.rs:deploy_via_loader}}
```

In this process, your contract code is automatically divided into chunks based on the specified policy:

```rust,ignore
{{#include ../../../examples/contracts/src/lib.rs:blob_policy}}
```

Each unique chunk is deployed as a separate blob transaction. Once all the blob transactions have been successfully committed, a loader contract is created. This loader, when invoked, will load the chunks into memory using the [LDC (Load Code from an External Contract)](https://docs.fuel.network/docs/specs/fuel-vm/instruction-set/#ldc-load-code-from-an-external-contract) instruction and execute your original contract.

After deploying the loader contract, you can interact with it just as you would with a traditionally deployed contract:

```rust,ignore
{{#include ../../../examples/contracts/src/lib.rs:use_loader}}
```

## Chunk sizes

The size of a Blob transaction is limited by three things:

1. The maximum size of a single transaction:

```rust,ignore
{{#include ../../../examples/contracts/src/lib.rs:show_max_tx_size}}
```

2. Maximum gas usage for a single transaction:

```rust,ignore
{{#include ../../../examples/contracts/src/lib.rs:show_max_tx_gas}}
```

3. The maximum HTTP body size the Fuel node will accept.

When deploying, you can use an estimating blob size policy:

```rust,ignore
{{#include ../../../examples/contracts/src/lib.rs:estimate_chunk_size}}
```

and the SDK will limit the blob sizes to the given percentage of the maximum.

Note that this estimation has the following caveats:

* It only accounts for the maximum transaction size (max gas usage and HTTP body limit not considered).
* It doesn't account for any size increase that will happen after the transaction is funded.

As such, you should use a percentage less than 100% to account for the caveats above.

## Manually splitting up the contract

If you wish, for any reason (such as resumability, retries, more control over the transactions, etc.), to manually split up and deploy the contract code, you can do so by following the example below:

```rust,ignore
{{#include ../../../examples/contracts/src/lib.rs:manual_contract_chunking}}
```
