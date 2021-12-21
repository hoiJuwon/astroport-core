use astroport::factory::MigrateMsg;
use cosmwasm_std::{Addr, DepsMut, Env, Response, StdResult};
use cw2::get_contract_version;
use cw_storage_plus::Item;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::state::{Config, CONFIG};

/// This structure describes the main control config of factory.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ConfigV100 {
    /// The Contract address that used for controls settings for factory, pools and tokenomics contracts
    pub owner: Addr,
    /// CW20 token contract code identifier
    pub token_code_id: u64,
    /// contract address that used for auto_stake from pools
    pub generator_address: Option<Addr>,
    /// contract address to send fees to
    pub fee_address: Option<Addr>,
}

pub fn migrate(deps: DepsMut, _env: Env, msg: MigrateMsg) -> StdResult<Response> {
    let info = get_contract_version(deps.storage)?;
    if info.version == "1.0.0" {
        let old_config_item: Item<ConfigV100> = Item::new("config");

        let old_config = old_config_item.load(deps.storage)?;

        let new_config = Config {
            asset_holder_rewards_code_id: msg.asset_holder_rewards_code_id,
            fee_address: old_config.fee_address,
            generator_address: old_config.generator_address,
            owner: old_config.owner,
            token_code_id: old_config.token_code_id,
        };

        CONFIG.save(deps.storage, &new_config)?;
    }

    Ok(Response::default())
}
