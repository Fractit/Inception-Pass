use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Uint128};
// expose to all others using contract, so others dont need to import cw721
pub use cw721::state::*;

use cw_storage_plus::{Item, Map};

#[cw_serde]
pub struct Config {
    pub owner: Addr,
    pub denom1: String,
    pub denom2: String,
    pub cw721_address: Option<Addr>,
    pub unit_price1: Uint128,
    pub unit_price2: Uint128,
    pub name: String,
    pub symbol: String,
    pub token_uri: String,
    pub extension: DefaultOptionMetadataExtension,
    pub unused_token_id: u32,
}

pub const CONFIG: Item<Config> = Item::new("config");
pub const MINTSTATUS: Item<bool> = Item::new("mintstatus");
pub const BALANCE: Map<&Addr, u128> = Map::new("balance");
pub const TOTALMINT: Item<u128> = Item::new("totalmint");
