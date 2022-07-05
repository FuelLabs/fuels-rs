## Increase the block height 

`produce_blocks` can be used to help achieve a desired block height. Useful for when you want to do any testing regarding transaction maturity.

Example of usage:

````rust,ignore
{{#include ../../../packages/fuels-abigen-macro/tests/harness.rs:uses_produce_blocks_to_increase_block_height}}
````
