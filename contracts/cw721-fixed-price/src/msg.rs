use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Uint128};
use cw20::Cw20ReceiveMsg;
use cw721::state::DefaultOptionMetadataExtension;

#[cw_serde]
pub struct InstantiateMsg {
    pub owner: Addr,
    pub unit_price: Uint128,
    pub name: String,
    pub symbol: String,
    pub token_code_id: u64,
    pub token_uri: String,
    pub extension: DefaultOptionMetadataExtension,
    pub withdraw_address: Option<String>,
}

#[cw_serde]
pub enum ExecuteMsg {
    Mint {},
    ChangeStatus { mint_pause: bool },
    ChangePrice { new_price: Uint128 },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(ConfigResponse)]
    GetConfig {},
    #[returns(BalanceOfResponse)]
    BalanceOf { user: Addr },
}

#[cw_serde]
pub struct BalanceOfResponse {
    pub balance: u128,
}
#[cw_serde]
pub struct ConfigResponse {
    pub owner: Addr,
    pub cw721_address: Option<Addr>,
    pub unit_price: Uint128,
    pub name: String,
    pub symbol: String,
    pub token_uri: String,
    pub total_mint: u128,
    pub extension: DefaultOptionMetadataExtension,
    pub unused_token_id: u32,
}
