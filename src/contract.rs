#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, Reply, StdError};

use crate::error::{ContractError, ContractResult};
use crate::execute::{REPLY_ID_EXECUTE_PASS, REPLY_ID_EXECUTE_FAIL};
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::{STATE, State};

const CONTRACT_NAME: &str = "crates.io:trapdoor";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
  deps: DepsMut,
  _env: Env,
  info: MessageInfo,
  _msg: InstantiateMsg,
) -> Result<Response, ContractError> {
  cw2::set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
  STATE.save(deps.storage, &State {
    owner: info.sender,
  })?;
  Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
  deps: DepsMut,
  env: Env,
  info: MessageInfo,
  msg: ExecuteMsg,
) -> ContractResult<Response> {
  let ctx = crate::execute::Context {deps, env, info};
  match msg {
    ExecuteMsg::Execute(msgs, fail) => crate::execute::execute(ctx, msgs, fail),
    ExecuteMsg::Refund(coins) => crate::execute::refund(ctx, coins),
    ExecuteMsg::TransferOwnership(new_owner) => crate::execute::transfer_ownership(ctx, new_owner),
  }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(
  _deps: DepsMut,
  _env: Env,
  msg: Reply,
) -> StdResult<Response> {
  match msg.id {
    REPLY_ID_EXECUTE_PASS => crate::execute::execute_reply_pass(),
    REPLY_ID_EXECUTE_FAIL => crate::execute::execute_reply_fail(),
    id => Err(StdError::generic_err(format!("Unknown reply ID: {}", id))),
  }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(_deps: Deps, _env: Env, _msg: QueryMsg) -> StdResult<Binary> {
  unimplemented!()
}

#[cfg(test)]
mod tests {
  use cosmwasm_std::{BankMsg, Coin, CosmosMsg, Uint128, Addr, WasmMsg, to_binary, Event, Attribute};
  use cw_multi_test::{App, ContractWrapper, SudoMsg, BankSudo, Executor};
  
  use crate::msg::{ExecuteMsg, InstantiateMsg, RefundCoin};
  
  fn mock_app() -> App {
    App::new(|_router, _api, _storage| {})
  }
  
  fn mock_contract(app: &mut App) -> String {
    let contract =
      ContractWrapper::new(super::execute, super::instantiate, super::query)
      .with_reply(super::reply);
    let code_id = app.store_code(Box::new(contract));
    let response = app.execute_multi(Addr::unchecked("alice"), vec![
      CosmosMsg::Wasm(
        WasmMsg::Instantiate {
          admin: Some("alice".to_string()),
          code_id,
          msg: to_binary(&InstantiateMsg {}).unwrap(),
          funds: vec![],
          label: "trapdoor-test".to_string(),
        }
      ),
    ]).unwrap();
    
    let event = find_event(&response[0].events, "instantiate".to_string()).unwrap();
    find_attr(&event.attributes, "_contract_addr".to_string()).unwrap()
  }
  
  fn find_event<'a>(events: &'a Vec<Event>, ty: String) -> Option<&'a Event> {
    events.iter().find(|event| event.ty == ty)
  }
  
  fn find_attr(attrs: &Vec<Attribute>, key: String) -> Option<String> {
    attrs.iter().find(|attr| attr.key == key).and_then(|attr| Some(attr.value.clone()))
  }
  
  #[test]
  fn test_execute() {
    let mut app = mock_app();
    let contract_addr = mock_contract(&mut app);
    
    app.sudo(
      SudoMsg::Bank(
        BankSudo::Mint {
          to_address: contract_addr.clone(),
          amount: vec![
            Coin { denom: "foobar".to_string(), amount: Uint128::from(500u128) }
          ],
        }
      )
    ).unwrap();
    
    let msg = CosmosMsg::Wasm(
      WasmMsg::Execute {
        contract_addr: contract_addr.clone(),
        msg: to_binary(&ExecuteMsg::Execute(
          vec![
            CosmosMsg::Bank(
              BankMsg::Send {
                to_address: "alice".to_string(),
                amount: vec![
                  Coin { denom: "foobar".to_string(), amount: Uint128::from(123u128) },
                ]
              }
            ),
            CosmosMsg::Bank(
              BankMsg::Send {
                to_address: "alice".to_string(),
                amount: vec![
                  Coin { denom: "barfoo".to_string(), amount: Uint128::from(123u128) },
                ]
              }
            ),
          ],
          false,
        )).unwrap(),
        funds: vec![],
      }
    );
    // should fail b/c we don't have enough barfoo, thus also not deducting any foobar
    app.execute(Addr::unchecked("alice"), msg).unwrap_err();
    assert_eq!(app.wrap().query_balance(contract_addr.clone(), "foobar").unwrap().amount, Uint128::from(500u128));
    
    let bankmsg = CosmosMsg::Bank(
      BankMsg::Send {
        to_address: "alice".to_string(),
        amount: vec![
          Coin { denom: "foobar".to_string(), amount: Uint128::from(123u128) }
        ],
      }
    );
    
    let msg = CosmosMsg::Wasm(
      WasmMsg::Execute {
        contract_addr: contract_addr.clone(),
        msg: to_binary(&ExecuteMsg::Execute(vec![bankmsg.clone()], true)).unwrap(),
        funds: vec![],
      }
    );
    // fails due to unauthorized
    app.execute(Addr::unchecked("bob"), msg.clone()).unwrap_err();
    // fails due to fail=true
    app.execute(Addr::unchecked("alice"), msg).unwrap_err();
    assert_eq!(app.wrap().query_balance(contract_addr.clone(), "foobar").unwrap().amount, Uint128::from(500u128));
    
    let msg = CosmosMsg::Wasm(
      WasmMsg::Execute {
        contract_addr: contract_addr.clone(),
        msg: to_binary(&ExecuteMsg::Execute(vec![bankmsg], false)).unwrap(),
        funds: vec![],
      }
    );
    // fails due to unauthorized
    app.execute(Addr::unchecked("bob"), msg.clone()).unwrap_err();
    app.execute(Addr::unchecked("alice"), msg).unwrap();
    assert!(app.wrap().query_balance(contract_addr.clone(), "foobar").unwrap().amount == Uint128::from(377u128));
  }
  
  #[test]
  fn test_refund() {
    let mut app = mock_app();
    let contract_addr = mock_contract(&mut app);
    
    app.sudo(
      SudoMsg::Bank(
        BankSudo::Mint {
          to_address: contract_addr.clone(),
          amount: vec![
            Coin { denom: "foobar".to_string(), amount: Uint128::from(500u128) },
          ],
        }
      )
    ).unwrap();
    
    let msg = CosmosMsg::Wasm(
      WasmMsg::Execute {
        contract_addr: contract_addr.clone(),
        msg: to_binary(&ExecuteMsg::Refund(
          vec![RefundCoin::Native { denom: "foobar".to_string(), amount: Some(Uint128::from(123u128)) }]
        )).unwrap(),
        funds: vec![],
      }
    );
    app.execute(Addr::unchecked("bob"), msg.clone()).unwrap_err();
    app.execute(Addr::unchecked("alice"), msg).unwrap();
    assert_eq!(app.wrap().query_balance(contract_addr.clone(), "foobar").unwrap().amount, Uint128::from(377u128));
    assert_eq!(app.wrap().query_balance("alice", "foobar").unwrap().amount, Uint128::from(123u128));
    
    let msg = CosmosMsg::Wasm(
      WasmMsg::Execute {
        contract_addr: contract_addr.clone(),
        msg: to_binary(&ExecuteMsg::Refund(
          vec![RefundCoin::Native { denom: "foobar".to_string(), amount: None }]
        )).unwrap(),
        funds: vec![],
      }
    );
    app.execute(Addr::unchecked("alice"), msg).unwrap();
    assert_eq!(app.wrap().query_balance(contract_addr.clone(), "foobar").unwrap().amount, Uint128::zero());
    assert_eq!(app.wrap().query_balance("alice", "foobar").unwrap().amount, Uint128::from(500u128));
  }
}
