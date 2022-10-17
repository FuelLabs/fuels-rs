// ANCHOR: test_modules
mod bindings; // generating Rust bindings
mod contracts; // contracts, contract calls and call behaviors
mod from_token; // generating instances from decoded tokens
mod logs; // parsing and decoding contract logs
mod predicates; // calling predicates and encoding predicate data
mod providers; // launching and querying providers
mod scripts; // building and calling scripts
mod storage; // storage initialization and forwarding
mod types; // encoding/decoding of native and custom types
mod wallets; // wallet creation and balance checks
             // ANCHOR_END: test_modules
