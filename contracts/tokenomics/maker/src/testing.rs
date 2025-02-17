use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
use cosmwasm_std::{from_binary, Addr, Decimal, Uint64};

use crate::contract::{execute, instantiate, query};
use crate::state::{Config, CONFIG};
use astroport::maker::{ConfigResponse, ExecuteMsg, InstantiateMsg, QueryMsg};
use std::str::FromStr;

#[test]
fn proper_initialization() {
    let mut deps = mock_dependencies(&[]);
    let info = mock_info("addr0000", &[]);

    let env = mock_env();
    let owner = Addr::unchecked("owner");
    let factory = Addr::unchecked("factory");
    let staking = Addr::unchecked("staking");
    let governance_contract = Addr::unchecked("governance");
    let governance_percent = Uint64::new(50);
    let astro_token_contract = Addr::unchecked("astro-token");

    let instantiate_msg = InstantiateMsg {
        owner: owner.to_string(),
        factory_contract: factory.to_string(),
        staking_contract: staking.to_string(),
        governance_contract: Option::from(governance_contract.to_string()),
        governance_percent: Option::from(governance_percent),
        astro_token_contract: astro_token_contract.to_string(),
        max_spread: None,
    };
    let res = instantiate(deps.as_mut(), env, info, instantiate_msg).unwrap();
    assert_eq!(0, res.messages.len());

    let state = CONFIG.load(deps.as_mut().storage).unwrap();
    assert_eq!(
        state,
        Config {
            owner: Addr::unchecked("owner"),
            factory_contract: Addr::unchecked("factory"),
            staking_contract: Addr::unchecked("staking"),
            governance_contract: Option::from(governance_contract),
            governance_percent,
            astro_token_contract: Addr::unchecked("astro-token"),
            max_spread: Decimal::from_str("0.05").unwrap(),
        }
    )
}

#[test]
fn update_owner() {
    let mut deps = mock_dependencies(&[]);
    let info = mock_info("addr0000", &[]);

    let owner = Addr::unchecked("owner");
    let factory = Addr::unchecked("factory");
    let staking = Addr::unchecked("staking");
    let governance_contract = Addr::unchecked("governance");
    let governance_percent = Uint64::new(50);
    let astro_token_contract = Addr::unchecked("astro-token");

    let msg = InstantiateMsg {
        owner: owner.to_string(),
        factory_contract: factory.to_string(),
        staking_contract: staking.to_string(),
        governance_contract: Option::from(governance_contract.to_string()),
        governance_percent: Option::from(governance_percent),
        astro_token_contract: astro_token_contract.to_string(),
        max_spread: None,
    };

    let env = mock_env();

    // we can just call .unwrap() to assert this was a success
    instantiate(deps.as_mut(), env, info, msg).unwrap();

    let new_owner = String::from("new_owner");

    // new owner
    let env = mock_env();
    let msg = ExecuteMsg::ProposeNewOwner {
        owner: new_owner.clone(),
        expires_in: 100, // seconds
    };

    let info = mock_info(new_owner.as_str(), &[]);

    // unauthorized check
    let err = execute(deps.as_mut(), env.clone(), info, msg.clone()).unwrap_err();
    assert_eq!(err.to_string(), "Generic error: Unauthorized");

    // claim before proposal
    let info = mock_info(new_owner.as_str(), &[]);
    execute(
        deps.as_mut(),
        env.clone(),
        info,
        ExecuteMsg::ClaimOwnership {},
    )
    .unwrap_err();

    // propose new owner
    let info = mock_info(owner.as_str(), &[]);
    let res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();
    assert_eq!(0, res.messages.len());

    // unauthorized ownership claim
    let info = mock_info("invalid_addr", &[]);
    let err = execute(
        deps.as_mut(),
        env.clone(),
        info,
        ExecuteMsg::ClaimOwnership {},
    )
    .unwrap_err();
    assert_eq!(err.to_string(), "Generic error: Unauthorized");

    // Claim ownership
    let info = mock_info(new_owner.as_str(), &[]);
    let res = execute(
        deps.as_mut(),
        env.clone(),
        info,
        ExecuteMsg::ClaimOwnership {},
    )
    .unwrap();
    assert_eq!(0, res.messages.len());

    // let's query the state
    let config: ConfigResponse =
        from_binary(&query(deps.as_ref(), env.clone(), QueryMsg::Config {}).unwrap()).unwrap();
    assert_eq!(new_owner, config.owner);
}
