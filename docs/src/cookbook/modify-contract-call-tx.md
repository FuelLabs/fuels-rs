# Modifying the contract call transaction

The Rust SDK lets you modify the underlying transaction of a contract call. An example use case for this would be to create a transaction that atomically executes a contract call and transfers assets between addresses. 

First we need to include some traits to access the inputs/outputs of the transaction:

```rust,ignore
{{#include ../../../examples/cookbook/src/lib.rs:modify_call_inputs_include}}
```

Now let's setup 2 wallets that hold a base asset coin, and a coin of some other asset.

```rust,ignore
{{#include ../../../examples/cookbook/src/lib.rs:modify_call_inputs_setup}}
```

In the next step, we deploy our test contract, create an instance of the contract, get the call handler and
finally, the underlying call by using `get_executable_call()`. 

```rust,ignore
{{#include ../../../examples/cookbook/src/lib.rs:modify_call_inputs_instance}}
```

Since the executable call acts as a wrapper fot the actual transaction, we can use it to modify fields like the inputs, outputs, maturity etc. Here, we setup inputs coming from `wallet_1` to be transferred to `wallet_2`.

```rust,ignore
{{#include ../../../examples/cookbook/src/lib.rs:modify_call_inputs_execute}}
```

Lastly, we verify the result of the contract call and the transfer.

```rust,ignore
{{#include ../../../examples/cookbook/src/lib.rs:modify_call_inputs_verify}}
```