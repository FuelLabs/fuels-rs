# Assemble Transactions

Assemble transactions makes it possible to create a minimal `TransactionBuilder` and let the fuel node fill in the missing details. The node will add missing inputs, outputs, set transactions limits etc. Below is an example how the assemble strategy can be used to create a transfer.

Let's first launch a local node with a funded wallet and create a random wallet that will receive some base asset.

```rust,ignore
{{#include ../../../e2e/tests/providers.rs:assemble_wallets}}
```

Next, we create an base asset output to the receiver wallet.

```rust,ignore
{{#include ../../../e2e/tests/providers.rs:assemble_output}}
```

Now we tell the node what kind of inputs do we require. Note that we do not specify any inputs just the amount, asset id and which require balance should be used to pay for the fees.

```rust,ignore
{{#include ../../../e2e/tests/providers.rs:assemble_req_balance}}
```

We can now build the transaction using the assemble strategy.

```rust,ignore
{{#include ../../../e2e/tests/providers.rs:assemble_tb}}
```

> **Note** The assemble strategy will make sure that we have enough base asset coins in the inputs to cover the transfer and the fee. Also a change output is added to the tx.

At the end, we send the transaction and make sure that the receiver the receiver balance matches the sent amount.

```rust,ignore
{{#include ../../../e2e/tests/providers.rs:assemble_response}}
```
