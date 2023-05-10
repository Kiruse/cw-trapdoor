use std::marker::PhantomData;

use cosmwasm_std::{BankMsg, Coin, CosmosMsg, DepsMut, Env, Empty, MessageInfo, Response, SubMsg, StdResult, StdError};
use cw20::{Cw20Contract, Cw20ExecuteMsg};
use cw721_base::helpers::Cw721Contract;

use crate::error::{ContractError, ContractResult};
use crate::msg::{RefundCoin};
use crate::state::STATE;

pub const REPLY_ID_EXECUTE_PASS: u64 = 0;
pub const REPLY_ID_EXECUTE_FAIL: u64 = 1;

pub struct Context<'a> {
  pub deps: DepsMut<'a>,
  pub env: Env,
  pub info: MessageInfo,
}

pub fn execute(ctx: Context, msgs: Vec<CosmosMsg>, fail: bool) -> ContractResult<Response> {
  let owner = STATE.load(ctx.deps.storage)?.owner;
  if ctx.info.sender != owner {
    return Err(ContractError::Unauthorized {});
  }
  
  if msgs.is_empty() {
    return Err(StdError::generic_err("Empty messages").into());
  }
  
  // simple proxy
  Ok(Response::new()
    .add_messages(msgs.iter().take(msgs.len() - 1).map(|msg| msg.clone()))
    .add_submessage(SubMsg::reply_on_success(msgs.last().unwrap().clone(), if fail {REPLY_ID_EXECUTE_FAIL} else {REPLY_ID_EXECUTE_PASS}))
  )
}

/// Noop, simply allows the execution to finalize & actualize state changes.
pub fn execute_reply_pass() -> StdResult<Response> {
  Ok(Response::default())
}

/// Fails the execution to prevent actualizing state changes, always.
pub fn execute_reply_fail() -> StdResult<Response> {
  Err(StdError::generic_err("Orchestrated contract failure"))
}

pub fn refund(ctx: Context, coins: Vec<RefundCoin>) -> ContractResult<Response> {
  let owner = STATE.load(ctx.deps.storage)?.owner;
  if ctx.info.sender != owner {
    return Err(ContractError::Unauthorized {});
  }
  
  let mut res = Response::new();
  let addr = ctx.env.contract.address;
  
  for coin in coins {
    match coin {
      RefundCoin::Native { denom, amount } => {
        res = res.add_message(CosmosMsg::Bank(BankMsg::Send {
          to_address: owner.to_string(),
          amount: vec![Coin {
            denom: denom.clone(),
            amount: match amount {
              Some(amount) => amount,
              None => ctx.deps.querier.query_balance(addr.clone(), denom)?.amount,
            },
          }],
        }));
      },
      RefundCoin::CW20 { address, amount } => {
        let address = ctx.deps.api.addr_validate(&address)?;
        let contract = Cw20Contract(address);
        res = res.add_message(contract.call(Cw20ExecuteMsg::Transfer {
          recipient: owner.to_string(),
          amount: match amount {
            Some(amount) => amount,
            None => contract.balance(&ctx.deps.querier, addr.clone())?,
          }
        })?);
      },
      RefundCoin::CW721 { address, token_id } => {
        let address = ctx.deps.api.addr_validate(&address)?;
        let contract = Cw721Contract(address, PhantomData::<Empty>, PhantomData::<Empty>);
        res = res.add_message(
          contract.call::<Empty>(cw721_base::ExecuteMsg::TransferNft {
            recipient: owner.to_string(),
            token_id,
          })?
        );
      },
    };
  }
  Ok(res)
}

pub fn transfer_ownership(ctx: Context, new_owner: String) -> ContractResult<Response> {
  STATE.update(ctx.deps.storage, |mut state| {
    if state.owner != ctx.info.sender {
      Err(ContractError::Unauthorized {})
    } else {
      state.owner = ctx.deps.api.addr_validate(&new_owner)?;
      Ok(state)
    }
  })?;
  Ok(Response::default())
}
