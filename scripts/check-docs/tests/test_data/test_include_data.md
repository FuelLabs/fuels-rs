
```rust,ignore
{{#include ./test_anchor_data.rs:test_anchor_line_comment}}
```

```rust,ignore
{{#include ./test_anchor_data.rs:test_anchor_block_comment}}
```

```rust,ignore
{{#include ./test_anchor_data.rs:test_with_more_forward_slashes}}
```

```rust,ignore
{{#include ./test_anchor_data.rs:no_existing_anchor}}
```

Include file with correct path

```rust,ignore
{{#include ./test_anchor_data.rs}}
```

Include file with wrong path

```rust,ignore
{{#include ./test_anchor_data2.rs}}
```

Another include file with wrong path

```rust,ignore
{{#include ./test_anchor_data3.rs}}
```
