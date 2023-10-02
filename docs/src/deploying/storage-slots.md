# Overriding storage slots

If you use storage in your contract, the default storage values will be generated in a JSON file (e.g. `my_contract-storage_slots.json`) by the Sway compiler. These are loaded automatically for you when you load a contract binary. If you wish to override some of the defaults, you need to provide the corresponding storage slots manually:

```rust,ignore
{{#include ../../../examples/contracts/src/lib.rs:storage_slots_override}}
```

If you don't have the slot storage file (`my_contract-storage_slots.json` example from above) for some reason, or you don't wish to load any of the default values, you can disable the auto-loading of storage slots:

```rust,ignore
{{#include ../../../examples/contracts/src/lib.rs:storage_slots_disable_autoload}}
```
