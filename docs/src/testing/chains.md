# Increasing the block height

You can use `produce_blocks` to help achieve an arbitrary block height; this is useful when you want to do any testing regarding transaction maturity.

> **Note**: For the `produce_blocks` API to work, it is imperative to have `manual_blocks_enabled = true` in the config for the running node. See example below.

````rust,ignore
{{#include ../../../e2e/tests/providers.rs:use_produce_blocks_to_increase_block_height}}
````

You can also set a custom block time as the second, optional argument. Here is an example:

````rust,ignore
{{#include ../../../e2e/tests/providers.rs:use_produce_blocks_custom_time}}
````
