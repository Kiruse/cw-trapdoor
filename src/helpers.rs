use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{to_binary, Addr, CosmosMsg, StdResult, WasmMsg};

use crate::msg::ExecuteMsg;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct TrapdoorContract(pub Addr);

impl TrapdoorContract {
  pub fn addr(&self) -> Addr {
    self.0.clone()
  }
  
  pub fn call<T: Into<ExecuteMsg>>(&self, msg: T) -> StdResult<CosmosMsg> {
    let msg = to_binary(&msg.into())?;
    Ok(WasmMsg::Execute {
      contract_addr: self.addr().into(),
      msg,
      funds: vec![],
    }
    .into())
  }
}
