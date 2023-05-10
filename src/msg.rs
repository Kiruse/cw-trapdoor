use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{CosmosMsg, Uint128};

#[cw_serde]
pub struct InstantiateMsg {}

#[cw_serde]
pub enum ExecuteMsg {
  /// Execute a list of CosmosMsg, optionally always failing
  Execute(Vec<CosmosMsg>, bool),
  /// Refund the contract's balances to the owner
  Refund(Vec<RefundCoin>),
  /// Transfer ownership of the contract to another address
  TransferOwnership(String),
}

#[cw_serde]
pub enum RefundCoin {
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

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {}
