# Increasing the block height

You can use `produce_blocks` to help achieve an arbitrary block height; this is useful when you want to do any testing regarding transaction maturity.

> **Note**: For the `produce_blocks` API to work, it is imperative to have `manual_blocks_enabled = true` in the config for the running node. See example below.

````rust,ignore
{{#include ../../../packages/fuels/tests/providers/mod.rs:use_produce_blocks_to_increase_block_height}}
````
