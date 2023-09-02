use crate::error::ContractError;
use classic_bindings::{TerraMsg, TerraQuery};
use classic_cyberswap::asset::AssetInfo;
use classic_cyberswap::router::{
    QueryMsg as RouterQueryMsg, SimulateSwapOperationsResponse, SwapOperation,
};
use cosmwasm_std::{
    to_binary, Addr, BalanceResponse as NativeBalanceResponse, BankMsg, BankQuery, Coin, CosmosMsg,
    QuerierWrapper, QueryRequest, Uint128, WasmMsg, WasmQuery,
};
use cw20::{BalanceResponse as CW20BalanceResponse, Cw20ExecuteMsg, Cw20QueryMsg, Denom};

pub fn get_token_amount(
    querier: QuerierWrapper<TerraQuery>,
    denom: Denom,
    contract_addr: Addr,
) -> Result<Uint128, ContractError> {
    match denom.clone() {
        Denom::Native(native_str) => {
            let native_response: NativeBalanceResponse =
                querier.query(&QueryRequest::Bank(BankQuery::Balance {
                    address: contract_addr.clone().into(),
                    denom: native_str,
                }))?;
            return Ok(native_response.amount.amount);
        }
        Denom::Cw20(cw20_address) => {
            let balance_response: CW20BalanceResponse =
                querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
                    contract_addr: cw20_address.clone().into(),
                    msg: to_binary(&Cw20QueryMsg::Balance {
                        address: contract_addr.clone().into(),
                    })?,
                }))?;
            return Ok(balance_response.balance);
        }
    }
}

pub fn transfer_token_message(
    denom: Denom,
    amount: Uint128,
    receiver: Addr,
) -> Result<CosmosMsg<TerraMsg>, ContractError> {
    match denom.clone() {
        Denom::Native(native_str) => {
            return Ok(CosmosMsg::Bank(
                BankMsg::Send {
                    to_address: receiver.clone().into(),
                    amount: vec![Coin {
                        denom: native_str,
                        amount,
                    }],
                }
                .into(),
            ));
        }
        Denom::Cw20(cw20_address) => {
            return Ok(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: cw20_address.clone().into(),
                funds: vec![],
                msg: to_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: receiver.clone().into(),
                    amount,
                })?,
            }));
        }
    }
}

pub fn get_simulate_swap_operations(
    querier: QuerierWrapper<TerraQuery>,
    router_contract_addr: Addr,
    offer_amount: Uint128,
    offer_asset_info: AssetInfo,
    ask_asset_info: AssetInfo,
) -> Result<Uint128, ContractError> {
    if offer_amount.is_zero() {
        return Ok(Uint128::zero());
    }
    if offer_asset_info.equal(&ask_asset_info) {
        return Ok(offer_amount);
    }
    let amount_response: SimulateSwapOperationsResponse =
        querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: router_contract_addr.clone().into(),
            msg: to_binary(&RouterQueryMsg::SimulateSwapOperations {
                offer_amount,
                operations: vec![SwapOperation::CyberSwap {
                    offer_asset_info,
                    ask_asset_info,
                }],
            })?,
        }))?;
    Ok(amount_response.amount)
}
