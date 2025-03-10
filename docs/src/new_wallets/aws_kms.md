### Using AWS KMS Wallets

AWS Key Management Service (KMS) offers a robust and secure solution for managing cryptographic keys for your Fuel wallets. Instead of keeping private keys on your local system, AWS KMS Wallets leverage AWS's secure infrastructure to handle both key storage and signing operations.

Below is an example of how to initialize a wallet with a AWS KMS signer:

```rust,ignore
{{#include ../../../e2e/tests/aws.rs:use_kms_wallet}}
```
