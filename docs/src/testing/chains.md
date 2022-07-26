# Increasing the block height

You can use `produce_blocks` to help achieve an arbitrary block height; this is useful when you want to do any testing regarding transaction maturity.

> **Note**: For the `produce_blocks` API to work, it is imperative to *not* have the `fuel-core-lib` feature enabled, and to have  the `manual-blocks` enabled in the running node. See example below. 
 
````rust,ignore
{{#include ../../../packages/fuels/tests/harness.rs:uses_produce_blocks_to_increase_block_height}}
````
