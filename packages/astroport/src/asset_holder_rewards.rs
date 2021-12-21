use crate::asset::{Asset, AssetInfo};
use cosmwasm_std::{Addr, CosmosMsg, Empty, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use std::fmt;

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct InstantiateMsg {
    pub admins: Vec<String>,
    pub mutable: bool,
    pub info: Info,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg<T = Empty>
where
    T: Clone + fmt::Debug + PartialEq + JsonSchema,
{
    /// Execute requests the contract to re-dispatch all these messages with the
    /// contract's address as sender. Every implementation has it's own logic to
    /// determine in
    Execute { msgs: Vec<CosmosMsg<T>> },
    /// Freeze will make a mutable contract immutable, must be called by an admin
    Freeze {},
    /// UpdateAdmins will change the admin set of the contract, must be called by an existing admin,
    /// and only works if the contract is mutable
    UpdateAdmins { admins: Vec<String> },
    HandleRewards {
        previous_assets_balances: Vec<Asset>,
        old_user_share: Uint128,
        old_total_share: Uint128,
        user: Addr,
        receiver: Option<String>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg<T = Empty>
where
    T: Clone + fmt::Debug + PartialEq + JsonSchema,
{
    /// Shows all admins and whether or not it is mutable
    AdminList {},
    /// Checks permissions of the caller on this proxy.
    /// If CanExecute returns true then a call to `Execute` with the same message,
    /// before any further state changes, should also succeed.
    CanExecute {
        sender: String,
        msg: CosmosMsg<T>,
    },
    AssetsBalancesAndClaimRewardsMessages {},
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct AdminListResponse {
    pub admins: Vec<String>,
    pub mutable: bool,
}

#[cfg(any(test, feature = "test-utils"))]
impl AdminListResponse {
    /// Utility function forconverting message to its canonical form, so two messages with
    /// different representation but same semantical meaning can be easly compared.
    ///
    /// It could be encapsulated in custom `PartialEq` implementation, but `PartialEq` is expected
    /// to be quickly, so it seems to be reasonable to keep it as representation-equality, and
    /// canonicalize message only when it is needed
    ///
    /// Example:
    ///
    /// ```
    /// # use cw1_whitelist::msg::AdminListResponse;
    ///
    /// let resp1 = AdminListResponse {
    ///   admins: vec!["admin1".to_owned(), "admin2".to_owned()],
    ///   mutable: true,
    /// };
    ///
    /// let resp2 = AdminListResponse {
    ///   admins: vec!["admin2".to_owned(), "admin1".to_owned(), "admin2".to_owned()],
    ///   mutable: true,
    /// };
    ///
    /// assert_eq!(resp1.canonical(), resp2.canonical());
    /// ```
    pub fn canonical(mut self) -> Self {
        self.admins.sort();
        self.admins.dedup();
        self
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum Handler {
    AnchorBluna,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Rewarder {
    pub address: String,
    pub handler: Handler,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Info {
    pub rewarders: Vec<Rewarder>,
    pub asset_infos: Vec<AssetInfo>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct AssetsBalancesAndClaimRewardsMessages {
    pub balances: Vec<Asset>,
    pub messages: Vec<CosmosMsg>,
}
