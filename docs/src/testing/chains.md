# Increasing the block height

You can use `produce_blocks` to help achieve an arbitrary block height; this is useful when you want to do any testing regarding transaction maturity.

````rust,ignore
{{#include ../../../packages/fuels-abigen-macro/tests/harness.rs:uses_produce_blocks_to_increase_block_height}}
````
