# Deploying Large Contracts

If your contract exceeds the size limit for a single deployment:

```rust,ignore
{{#include ../../../examples/contracts/src/lib.rs:show_contract_is_too_big}}
```

you can deploy it in parts using a segmented approach:

```rust,ignore
{{#include ../../../examples/contracts/src/lib.rs:deploy_via_loader}}
```

When you convert a regular contract into a loader contract the following happens:

* Your contract code is replaced with the code of the loader contract
* the original contract code is separated into blobs that will be deployed via blob transactions to the chain prior to the deployment of the contract itself.
* The new loader code will, upon invocation, load the blobs into memory and execute your original contract.

After deploying the loader contract, you can interact with it just as you would with a traditionally deployed contract:

```rust,ignore
{{#include ../../../examples/contracts/src/lib.rs:use_loader}}
```

There is also a helper that will deploy your contract normally, if its size is below the limit, and as a loader contract otherwise:

```rust,ignore
{{#include ../../../examples/contracts/src/lib.rs:auto_convert_to_loader}}
```

You can also separate the blob upload from the deployment of the contract for more fine grained control:

```rust,ignore
{{#include ../../../examples/contracts/src/lib.rs:upload_blobs_then_deploy}}
```

Or split your contract code into blobs however you wish and then create a loader and deploy:

```rust,ignore
{{#include ../../../examples/contracts/src/lib.rs:manual_blobs_then_deploy}}
```

Or even upload the blobs yourself and just do the loader deployment:

```rust,ignore
{{#include ../../../examples/contracts/src/lib.rs:manual_blob_upload_then_deploy}}
```

## Blob size

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
