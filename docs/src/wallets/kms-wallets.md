# Using AWS KMS Wallets

AWS Key Management Service (KMS) provides a secure way to manage cryptographic keys for your Fuel wallets. Rather than storing private keys locally, AWS KMS wallets use AWS's secure infrastructure to handle key storage and signing operations.

```rust,ignore
{{#include ../../../e2e/tests/aws.rs:use_kms_wallet}}
```
