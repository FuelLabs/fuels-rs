# Converting Types

Below you can find examples for common type conversions:

- [Convert Between Native Types](#convert-between-native-types)
- [Convert to `Bytes32`](#convert-to-bytes32)
- [Convert to `Address`](#convert-to-address)
- [Convert to `ContractId`](#convert-to-contractid)
- [Convert to `Identity`](#convert-to-identity)
- [Convert to `AssetId`](#convert-to-assetid)
- [Convert to `Bech32`](#convert-to-bech32)
- [Convert to `str`](#convert-to-str)
- [Convert to `Bits256`](#convert-to-bits256)
- [Convert to `Bytes`](#convert-to-bytes)
- [Convert to `B512`](#convert-to-b512)
- [Convert to `EvmAddress`](#convert-to-evmaddress)

## Convert Between Native Types

You might want to convert between the native types (`Bytes32`, `Address`, `ContractId`, and `AssetId`). Because these types are wrappers on `[u8; 32]`, converting is a matter of dereferencing one and instantiating the other using the dereferenced value. Here's an example:

```rust,ignore
{{#include ../../../examples/types/src/lib.rs:type_conversion}}
```

## Convert to `Bytes32`

Convert a `[u8; 32]` array to `Bytes32`:

```rust,ignore
{{#include ../../../examples/types/src/lib.rs:array_to_bytes32}}
```

Convert a hex string to `Bytes32`:

```rust,ignore
{{#include ../../../examples/types/src/lib.rs:hex_string_to_bytes32}}
```

## Convert to `Address`

Convert a `[u8; 32]` array to an `Address`:

```rust,ignore
{{#include ../../../examples/types/src/lib.rs:array_to_address}}
```

Convert a `Bech32` address to an `Address`:

```rust,ignore
{{#include ../../../examples/types/src/lib.rs:bech32_to_address}}
```

Convert a wallet to an `Address`:

```rust,ignore
{{#include ../../../examples/wallets/src/lib.rs:wallet_to_address}}
```

Convert a hex string to an `Address`:

```rust,ignore
{{#include ../../../examples/types/src/lib.rs:hex_string_to_address}}
```

## Convert to `ContractId`

Convert a `[u8; 32]` array to `ContractId`:

```rust,ignore
{{#include ../../../examples/types/src/lib.rs:array_to_contract_id}}
```

Convert a hex string to a `ContractId`:

```rust,ignore
{{#include ../../../examples/types/src/lib.rs:string_to_contract_id}}
```

Convert a contract instance to a `ContractId`:

```rust,ignore
{{#include ../../../e2e/tests/logs.rs:instance_to_contract_id}}
```

## Convert to `Identity`

Convert an `Address` to an `Identity`:

```rust,ignore
{{#include ../../../examples/types/src/lib.rs:address_to_identity}}
```

Convert a `ContractId` to an `Identity`:

```rust,ignore
{{#include ../../../examples/types/src/lib.rs:contract_id_to_identity}}
```

## Convert to `AssetId`

Convert a `[u8; 32]` array to an `AssetId`:

```rust,ignore
{{#include ../../../examples/types/src/lib.rs:array_to_asset_id}}
```

Convert a hex string to an `AssetId`:

```rust,ignore
{{#include ../../../examples/types/src/lib.rs:string_to_asset_id}}
```

## Convert to `Bech32`

Convert a `[u8; 32]` array to a `Bech32` address:

```rust,ignore
{{#include ../../../examples/types/src/lib.rs:array_to_bech32}}
```

Convert `Bytes32` to a `Bech32` address:

```rust,ignore
{{#include ../../../examples/types/src/lib.rs:bytes32_to_bech32}}
```

Convert a string to a `Bech32` address:

```rust,ignore
{{#include ../../../examples/types/src/lib.rs:str_to_bech32}}
```

Convert an `Address` to a `Bech32` address:

```rust,ignore
{{#include ../../../examples/types/src/lib.rs:address_to_bech32}}
```

## Convert to `str`

Convert a `ContractId` to a `str`:

```rust,ignore
{{#include ../../../examples/types/src/lib.rs:contract_id_to_str}}
```

Convert an `Address` to a `str`:

```rust,ignore
{{#include ../../../examples/types/src/lib.rs:address_to_str}}
```

Convert an `AssetId` to a `str`:

```rust,ignore
{{#include ../../../examples/types/src/lib.rs:asset_id_to_str}}
```

Convert `Bytes32` to a `str`:

```rust,ignore
{{#include ../../../examples/types/src/lib.rs:bytes32_to_str}}
```

## Convert to `Bits256`

Convert a hex string to `Bits256`:

```rust,ignore
{{#include ../../../packages/fuels-core/src/types/core/bits.rs:hex_str_to_bits256}}
```

Convert a `ContractId` to `Bits256`:

```rust,ignore
{{#include ../../../examples/types/src/lib.rs:contract_id_to_bits256}}
```

Convert an `Address` to `Bits256`:

```rust,ignore
{{#include ../../../examples/types/src/lib.rs:address_to_bits256}}
```

Convert an `AssetId` to `Bits256`:

```rust,ignore
{{#include ../../../examples/types/src/lib.rs:asset_id_to_bits256}}
```

## Convert to `Bytes`

Convert a string to `Bytes`:

```rust,ignore
{{#include ../../../packages/fuels-core/src/types/core/bytes.rs:hex_string_to_bytes32}}
```

## Convert to `B512`

Convert two hex strings to `B512`:

```rust,ignore
{{#include ../../../e2e/tests/types_contracts.rs:b512_example}}
```

## Convert to `EvmAddress`

Convert a `Bits256` address to an `EvmAddress`:

```rust,ignore
{{#include ../../../examples/types/src/lib.rs:b256_to_evm_address}}
```
