## RocksDb

RocksDb enables the preservation of the blockchain's state locally, facilitating its future utilization.

To create or use a local database, follow these instructions:

```rust,ignore
{{#include ../../../examples/cookbook/src/lib.rs:create_or_use_rocksdb}}
```

> Note 1: If the specified database does not exist, a new database will be created at that path.

> Note 2: To utilize the code snippets above, either:
> - the `fuel-core` binary must be present
> or
> - both the `fuel-core-lib` and `rocksdb` features need to be enabled.