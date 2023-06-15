## RocksDb

RocksDb enables the establishment of a node and the preservation of the blockchain's state locally, facilitating its future utilization.

To create a local database, follow these instructions:

```rust,ignore
{{#include ../../../examples/contracts/src/lib.rs:create_rocksdb}}
```

To utilize an existing database, follow these instructions:

```rust,ignore
{{#include ../../../examples/cookbook/src/lib.rs:use_created_rocksdb}}
```

> Note 1: If the specified database does not exist a new database will be created at that path.

> Note 2: If the `fuel-core` library is not installed locally you will need to use the `rocksdb` feature to utilize the code snippets above.