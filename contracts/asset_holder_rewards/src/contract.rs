use astroport::asset::{addr_validate_to_lower, Asset, AssetInfo};
use schemars::JsonSchema;
use std::collections::HashSet;
use std::convert::TryInto;
use std::fmt;

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Addr, Api, Binary, CosmosMsg, Decimal256, Deps, DepsMut, Empty, Env, MessageInfo,
    Response, StdError, StdResult, SubMsg, Uint128, Uint256, WasmMsg,
};

use cw1::CanExecuteResponse;
use cw2::set_contract_version;

use crate::error::ContractError;
use crate::state::{AdminList, ADMIN_LIST, GLOBAL_INDEXES, INFO, USER_INDEXES};
use astroport::asset_holder_rewards::{
    AdminListResponse, AssetsBalancesAndClaimRewardsMessages, ExecuteMsg, Handler, InstantiateMsg,
    QueryMsg,
};

// version info for migration info
const CONTRACT_NAME: &str = "astroport-asset-holder-rewards";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    for rewarder in &msg.info.rewarders {
        addr_validate_to_lower(deps.api, &rewarder.address)?;
    }

    let mut unique_asset_infos: HashSet<&AssetInfo> = HashSet::new();

    for asset_info in &msg.info.asset_infos {
        asset_info.check(deps.api)?;

        if !unique_asset_infos.insert(asset_info) {
            return Err(StdError::generic_err(
                "Asset infos for rewards should be unique!",
            ));
        }
    }

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    let cfg = AdminList {
        admins: map_validate(deps.api, &msg.admins)?,
        mutable: msg.mutable,
    };

    INFO.save(deps.storage, &msg.info)?;
    ADMIN_LIST.save(deps.storage, &cfg)?;

    Ok(Response::default())
}

pub fn map_validate(api: &dyn Api, admins: &[String]) -> StdResult<Vec<Addr>> {
    admins.iter().map(|addr| api.addr_validate(&addr)).collect()
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    // Note: implement this function with different type to add support for custom messages
    // and then import the rest of this contract code.
    msg: ExecuteMsg<Empty>,
) -> Result<Response<Empty>, ContractError> {
    match msg {
        ExecuteMsg::Execute { msgs } => execute_execute(deps, env, info, msgs),
        ExecuteMsg::Freeze {} => execute_freeze(deps, env, info),
        ExecuteMsg::UpdateAdmins { admins } => execute_update_admins(deps, env, info, admins),
        ExecuteMsg::HandleRewards {
            previous_assets_balances,
            old_user_share,
            old_total_share,
            user,
            receiver,
        } => handle_rewards(
            deps,
            &env,
            previous_assets_balances,
            old_user_share,
            old_total_share,
            user,
            receiver,
        ),
    }
}

pub fn execute_execute<T>(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msgs: Vec<CosmosMsg<T>>,
) -> Result<Response<T>, ContractError>
where
    T: Clone + fmt::Debug + PartialEq + JsonSchema,
{
    if !can_execute(deps.as_ref(), info.sender.as_ref())? {
        Err(ContractError::Unauthorized {})
    } else {
        let res = Response::new()
            .add_messages(msgs)
            .add_attribute("action", "execute");
        Ok(res)
    }
}

pub fn execute_freeze(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let mut cfg = ADMIN_LIST.load(deps.storage)?;
    if !cfg.can_modify(info.sender.as_ref()) {
        Err(ContractError::Unauthorized {})
    } else {
        cfg.mutable = false;
        ADMIN_LIST.save(deps.storage, &cfg)?;

        let res = Response::new().add_attribute("action", "freeze");
        Ok(res)
    }
}

pub fn execute_update_admins(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    admins: Vec<String>,
) -> Result<Response, ContractError> {
    let mut cfg = ADMIN_LIST.load(deps.storage)?;
    if !cfg.can_modify(info.sender.as_ref()) {
        Err(ContractError::Unauthorized {})
    } else {
        cfg.admins = map_validate(deps.api, &admins)?;
        ADMIN_LIST.save(deps.storage, &cfg)?;

        let res = Response::new().add_attribute("action", "update_admins");
        Ok(res)
    }
}

fn can_execute(deps: Deps, sender: &str) -> StdResult<bool> {
    let cfg = ADMIN_LIST.load(deps.storage)?;
    let can = cfg.is_admin(sender.as_ref());
    Ok(can)
}

pub fn handle_rewards(
    deps: DepsMut,
    env: &Env,
    previous_assets_balances: Vec<Asset>,
    old_user_share: Uint128,
    old_total_share: Uint128,
    user: Addr,
    receiver: Option<String>,
) -> Result<Response, ContractError> {
    let mut response = Response::new();

    // TODO: add attributes

    if let Some(info) = INFO.may_load(deps.storage)? {
        let latest_rewards: Vec<Asset> = previous_assets_balances
            .iter()
            .zip(query_assets_balances(
                deps.as_ref(),
                env,
                &info.asset_infos,
            )?)
            .map(|(previous, current)| Asset {
                info: previous.info.clone(),
                amount: current.amount.saturating_sub(previous.amount),
            })
            .collect();

        let receiver = receiver
            .as_ref()
            .map(|v| deps.api.addr_validate(v))
            .transpose()?
            .unwrap_or_else(|| user.clone());

        for reward in latest_rewards {
            let mut global_index = GLOBAL_INDEXES.load(deps.storage, &reward.info)?;

            global_index = global_index + Decimal256::from_ratio(reward.amount, old_total_share);
            GLOBAL_INDEXES.save(deps.storage, &reward.info, &global_index)?;

            let user_reward: Uint128 = ((global_index
                - USER_INDEXES.load(deps.storage, (&user, &reward.info))?)
                * Uint256::from(old_user_share))
            .try_into()
            .map_err(|e| ContractError::Std(StdError::from(e)))?;

            response.messages.push(SubMsg::new(
                Asset {
                    info: reward.info.clone(),
                    amount: user_reward,
                }
                .into_msg(&deps.querier, receiver.clone())?,
            ));

            USER_INDEXES.save(deps.storage, (&user, &reward.info), &global_index)?;
        }
    }
    Ok(response)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::AdminList {} => to_binary(&query_admin_list(deps)?),
        QueryMsg::CanExecute { sender, msg } => to_binary(&query_can_execute(deps, sender, msg)?),
        QueryMsg::AssetsBalancesAndClaimRewardsMessages {} => to_binary(
            &query_asset_balances_and_claim_rewards_messages(deps, &env)?,
        ),
    }
}

pub fn query_admin_list(deps: Deps) -> StdResult<AdminListResponse> {
    let cfg = ADMIN_LIST.load(deps.storage)?;
    Ok(AdminListResponse {
        admins: cfg.admins.into_iter().map(|a| a.into()).collect(),
        mutable: cfg.mutable,
    })
}

pub fn query_can_execute(
    deps: Deps,
    sender: String,
    _msg: CosmosMsg,
) -> StdResult<CanExecuteResponse> {
    Ok(CanExecuteResponse {
        can_execute: can_execute(deps, &sender)?,
    })
}

pub fn query_asset_balances_and_claim_rewards_messages(
    deps: Deps,
    env: &Env,
) -> StdResult<AssetsBalancesAndClaimRewardsMessages> {
    Ok(if let Some(info) = INFO.may_load(deps.storage)? {
        AssetsBalancesAndClaimRewardsMessages {
            balances: query_assets_balances(deps, env, &info.asset_infos)?,
            messages: build_claim_rewards_messages(deps)?,
        }
    } else {
        AssetsBalancesAndClaimRewardsMessages {
            balances: vec![],
            messages: vec![],
        }
    })
}

fn query_assets_balances(
    deps: Deps,
    env: &Env,
    asset_infos: &[AssetInfo],
) -> StdResult<Vec<Asset>> {
    let mut result: Vec<Asset> = vec![];
    for asset_info in asset_infos {
        let amount = asset_info.query_pool(&deps.querier, env.contract.address.clone())?;
        result.push(Asset {
            info: asset_info.clone(),
            amount,
        })
    }

    Ok(result)
}

fn build_claim_rewards_messages(deps: Deps) -> StdResult<Vec<CosmosMsg>> {
    let info = INFO.load(deps.storage)?;

    let mut result: Vec<CosmosMsg> = vec![];

    for rewarder in info.rewarders {
        result.append(&mut match rewarder.handler {
            Handler::AnchorBluna => vec![CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: rewarder.address,
                msg: to_binary(&anchor_basset::reward::ExecuteMsg::ClaimRewards {
                    recipient: None,
                })?,
                funds: vec![],
            })],
        });
    }

    Ok(result)
}
