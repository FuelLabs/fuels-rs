# Wallets

Wallets serve as a centralized interface for all account-related behaviors. They allow you to:

- **Inspect UTXOs:** Check available coins for spending.
- **Prepare and send transactions:** Build, sign, and submit transfers.
- **Manage network fees:** Pay for transaction execution and contract deployment.

Every wallet requires a **provider** to communicate with the network.

---

## Types of Wallets

There are two primary types of wallets available in the SDK:

### [Locked Wallets](./access.md)

- **Purpose:** Used for read-only operations.
- **Interface:** Implements the [`ViewOnlyAccount`](../accounts.md) trait.
- **Use Cases:** Checking balances, viewing UTXOs, and monitoring transactions without the ability to sign or submit transactions.

### [Unlocked Wallets](./access.md)

- **Purpose:** Supports full account functionality.
- **Interface:** Implements the [`ViewOnlyAccount`](../accounts.md) and [`Account`](../accounts.md) traits.
- **Additional Requirement:** In addition to a provider, an unlocked wallet must include a **signer**.
- **Use Cases:** Transferring funds, signing messages, submitting transactions, and performing state-changing operations.

---

## Signer Options

The SDK offers multiple signing methods to suit different scenarios:

- [**Private Key Signer:**](./private_key_signer.md)  
  Use when you have direct access to your account’s private key.
- [**AWS KMS Signer:**](./kms.md)
  Delegate signing operations to AWS Key Management Service, enhancing key security by offloading cryptographic operations.

- [**Google KMS Signer:**](./kms.md)  
  Similar to AWS KMS, this option delegates signing to Google’s Key Management Service.

- [**Fake Signer:**](./fake_signer.md)  
  Generates dummy signatures, which is useful for impersonation while testing. Only possible when using a network that does not enforce signature validation.

---
