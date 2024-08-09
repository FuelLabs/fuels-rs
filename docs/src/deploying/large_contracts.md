# Deploying Large Contracts

If your contract exceeds the size limit for a single deployment:

```rust,ignore
{{#include ../../../examples/contracts/src/lib.rs:show_contract_is_too_big}}
```

you can deploy it in segments using a partitioned approach:

```rust,ignore
{{#include ../../../examples/contracts/src/lib.rs:deploy_via_loader}}
```

When you convert a standard contract into a loader contract, the following changes occur:

* The original contract code is replaced with the loader contract code.
* The original contract code is split into blobs, which will be deployed via blob transactions before deploying the contract itself.
* The new loader code, when invoked, loads these blobs into memory and executes your original contract.

After deploying the loader contract, you can interact with it just as you would with a standard contract:

```rust,ignore
{{#include ../../../examples/contracts/src/lib.rs:use_loader}}
```

A helper function is available to deploy your contract normally if it is within the size limit, or as a loader contract if it exceeds the limit:

```rust,ignore
{{#include ../../../examples/contracts/src/lib.rs:auto_convert_to_loader}}
```

You also have the option to separate the blob upload from the contract deployment for more granular control:

```rust,ignore
{{#include ../../../examples/contracts/src/lib.rs:upload_blobs_then_deploy}}
```

Alternatively, you can manually split your contract code into blobs and then create and deploy a loader:

```rust,ignore
{{#include ../../../examples/contracts/src/lib.rs:manual_blobs_then_deploy}}
```

Or you can upload the blobs yourself and proceed with just the loader deployment:

```rust,ignore
{{#include ../../../examples/contracts/src/lib.rs:manual_blob_upload_then_deploy}}
```

## Blob Size Considerations

The size of a Blob transaction is constrained by three factors:

<!--Needed to disable lints because the multiline ordered list is messing with the linter. It keeps suggesting that each item is a start of a new list.-->
<!-- markdownlint-disable -->
1. The maximum size of a single transaction:

```rust,ignore
{{#include ../../../examples/contracts/src/lib.rs:show_max_tx_size}}
```

2. The maximum gas usage for a single transaction:

```rust,ignore
{{#include ../../../examples/contracts/src/lib.rs:show_max_tx_gas}}
```

3. The maximum HTTP body size accepted by the Fuel node.

To estimate an appropriate size for your blobs, you can run:

```rust,ignore
{{#include ../../../examples/contracts/src/lib.rs:estimate_max_blob_size}}
```
<!-- markdownlint-restore -->

However, keep in mind the following limitations:

* The estimation only considers the maximum transaction size, not the max gas usage or HTTP body limit.
* It does not account for any size increase that may occur after the transaction is funded.

Therefore, it is advisable to make your blobs a few percent smaller than the estimated maximum size.
