# Trapdoor Test Contract
Trapdoor is a simple CosmWasm proxy smart contract, intended for testing. It simply executes messages its owner provides. It takes a flag to indicate whether the transaction should succeed or always fail. The latter is helpful to test the validity of a supplied message, without actually committing.

Trapdoor is designed as a simplified substitute of a DAO treasury with message execution capabilities - e.g. an [Enterprise](https://enterprise.money) DAO. This can be helpful when executing other smart contracts, because these smart contracts might distinguish between EOAs (Externally Owned Accounts) and smart contracts, thus ensuring your executions and tests are run under similar conditions to a DAO treasury. It is also simpler and faster than testing directly in the governance system of a live DAO.

Trapdoor also sports a `refund` method which can return any native coin or CW20 or CW721 tokens. For fungle coins, the amount is optional, and when omitted it simply refunds the entire balance. For NFTs, the `token_id` is required.

## Executions
```rust
enum ExecuteMsg {
  /// Execute given messages as trapdoor, optionally always failing
  Execute(Vec<CosmosMsg>, bool),
  /// Refund trapdoor balance to trapdoor owner
  Refund(RefundCoin),
  /// Transfer ownership over the trapdoor to the given new_owner
  TransferOwnership(String),
}

enum RefundCoin {
  Native {
    denom: String,
    amount: Option<Uint128>,
  },
  CW20 {
    address: String,
    amount: Option<Uint128>,
  },
  CW721 {
    address: String,
    token_id: String,
  },
}
```

### Execute execution
`execute` executes all given `CosmosMsg`s with the contract as the sender. When the second argument `fail` is set to true, no matter what, the transaction will always fail. This prevents actually changing the state of the blockchain and thus actually using funds, but can be useful to test if the message would've worked from the perspective of this (or similar) smart contracts.

Can only be called by the owner, because the contract might contain assets.

### Refund execution
Simply refunds any coins from the smart contract to the owner. Can only be called by the owner to prevent third parties from meddling with on-going testing.

### TransferOwnership execution
As the name implies, the owner can choose to transfer the ownership over the trapdoor to another account. Useful e.g. when migrating wallets.

## Queries
None yet.
