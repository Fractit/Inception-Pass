use std::marker::PhantomData;

use crate::error::ContractError;
use crate::msg::{BalanceOfResponse, ConfigResponse, ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::{Config, BALANCE, CONFIG, MINTSTATUS, TOTALMINT};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_json_binary, Addr, BankMsg, Binary, Coin, Deps, DepsMut, Empty, Env, MessageInfo, Reply,
    ReplyOn, Response, StdResult, SubMsg, Uint128, WasmMsg,
};
use cw2::set_contract_version;
use cw721::helpers::Cw721Contract;
use cw721::msg::{self, Cw721ExecuteMsg, Cw721InstantiateMsg};
use cw721::state::DefaultOptionMetadataExtension;
use cw_utils::must_pay;
use cw_utils::parse_reply_instantiate_data;

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:cw721-fixed-price";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

const INSTANTIATE_TOKEN_REPLY_ID: u64 = 1;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    if msg.unit_price1 == Uint128::new(0) {
        return Err(ContractError::InvalidUnitPrice {});
    }
    if msg.unit_price2 == Uint128::new(0) {
        return Err(ContractError::InvalidUnitPrice {});
    }

    let config = Config {
        cw721_address: None,
        unit_price1: msg.unit_price1,
        unit_price2: msg.unit_price2,
        owner: info.sender,
        denom1: msg.denom1,
        denom2: msg.denom2,
        name: msg.name.clone(),
        symbol: msg.symbol.clone(),
        token_uri: msg.token_uri.clone(),
        extension: msg.extension.clone(),
        unused_token_id: 0,
    };

    CONFIG.save(deps.storage, &config)?;

    let sub_msg: Vec<SubMsg> = vec![SubMsg {
        msg: WasmMsg::Instantiate {
            code_id: msg.token_code_id,
            msg: to_json_binary(&Cw721InstantiateMsg {
                name: msg.name.clone(),
                symbol: msg.symbol,
                minter: None,
                withdraw_address: msg.withdraw_address,
            })?,
            funds: vec![],
            admin: None,
            label: String::from("Instantiate fixed price NFT contract"),
        }
        .into(),
        id: INSTANTIATE_TOKEN_REPLY_ID,
        gas_limit: None,
        reply_on: ReplyOn::Success,
    }];

    Ok(Response::new().add_submessages(sub_msg))
}

// Reply callback triggered from cw721 contract instantiation
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    let mut config: Config = CONFIG.load(deps.storage)?;

    if config.cw721_address.is_some() {
        return Err(ContractError::Cw721AlreadyLinked {});
    }

    if msg.id != INSTANTIATE_TOKEN_REPLY_ID {
        return Err(ContractError::InvalidTokenReplyId {});
    }

    let reply = parse_reply_instantiate_data(msg).unwrap();
    config.cw721_address = Addr::unchecked(reply.contract_address).into();
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetConfig {} => to_json_binary(&query_config(deps)?),
        QueryMsg::BalanceOf { user } => to_json_binary(&quere_balance(deps, user)?),
    }
}

fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config = CONFIG.load(deps.storage)?;
    let total_mint = TOTALMINT.load(deps.storage).unwrap_or_default();
    Ok(ConfigResponse {
        owner: config.owner,
        cw721_address: config.cw721_address,
        unit_price1: config.unit_price1,
        uint_price2: config.unit_price2,
        denom1: config.denom1,
        denom2: config.denom2,
        name: config.name,
        symbol: config.symbol,
        token_uri: config.token_uri,
        total_mint: total_mint,
        extension: config.extension,
        unused_token_id: config.unused_token_id,
    })
}

fn quere_balance(deps: Deps, user: Addr) -> StdResult<BalanceOfResponse> {
    let balance = BALANCE.load(deps.storage, &user).unwrap_or_default();

    let balance_response = BalanceOfResponse { balance: balance };

    Ok(balance_response)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Mint { denom } => execute_mint(deps, info, denom),
        ExecuteMsg::ChangeStatus { mint_pause } => execute_change_status(deps, info, mint_pause),
        ExecuteMsg::ChangePrice {
            new_price1,
            new_price2,
        } => execute_change_price(deps, info, new_price1, new_price2),
    }
}

fn execute_change_price(
    deps: DepsMut,
    info: MessageInfo,
    new_price1: Option<Uint128>,
    new_price2: Option<Uint128>,
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;

    let sender = info.sender;

    if sender != config.owner {
        return Err(ContractError::NotOwner {});
    }

    if new_price1.is_some() {
        config.unit_price1 = new_price1.unwrap();
    }
    if new_price2.is_some() {
        config.unit_price2 = new_price2.unwrap();
    }

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new())
}

fn execute_change_status(
    deps: DepsMut,
    info: MessageInfo,
    mint_pause: bool,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let sender = info.sender;

    if sender != config.owner {
        return Err(ContractError::NotOwner {});
    }

    MINTSTATUS.save(deps.storage, &mint_pause)?;

    Ok(Response::new().add_attribute("ChangeStatus", mint_pause.to_string()))
}

pub fn execute_mint(
    deps: DepsMut,
    info: MessageInfo,
    denom: String,
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;
    let mint_status = MINTSTATUS.load(deps.storage).unwrap_or_default();

    if mint_status {
        return Err(ContractError::MintPaused {});
    }

    let amount;

    if denom == config.denom1 {
        amount = must_pay(&info, &denom).unwrap();
        if amount != config.unit_price1 {
            return Err(ContractError::WrongPaymentAmount {});
        }
    } else if denom == config.denom2 {
        amount = must_pay(&info, &denom).unwrap();
        if amount != config.unit_price2 {
            return Err(ContractError::WrongPaymentAmount {});
        }
    } else {
        return Err(ContractError::WrongDenom {});
    }

    if config.cw721_address.is_none() {
        return Err(ContractError::Uninitialized {});
    }

    let user = info.sender.clone();

    let minted = BALANCE.load(deps.storage, &user).unwrap_or_default();

    BALANCE.save(deps.storage, &user, &(minted + 1))?;

    let total_mint = TOTALMINT.load(deps.storage).unwrap_or_default();

    TOTALMINT.save(deps.storage, &(&total_mint + 1))?;

    let mint_msg = Cw721ExecuteMsg::<DefaultOptionMetadataExtension, Empty>::Mint {
        token_id: config.unused_token_id.to_string(),
        owner: info.sender.to_string(),
        token_uri: config.token_uri.clone().into(),
        extension: config.extension.clone(),
    };

    match config.cw721_address.clone() {
        Some(cw721) => {
            let callback = Cw721Contract::<DefaultOptionMetadataExtension, Empty>(
                cw721.clone(),
                PhantomData,
                PhantomData,
            )
            .call(mint_msg)
            .map_err(|_| ContractError::Cw721CallFailed {})?;
            let send_funds_msg = BankMsg::Send {
                to_address: config.owner.to_string(),
                amount: vec![Coin {
                    denom: denom.clone(),
                    amount,
                }],
            };
            config.unused_token_id += 1;
            CONFIG.save(deps.storage, &config)?;
            Ok(Response::new()
                .add_message(callback)
                .add_message(send_funds_msg)
                .add_attribute("action", "mint_nft")
                .add_attribute("token_id", config.unused_token_id.to_string())
                .add_attribute("amount", amount.to_string())
                .add_attribute("denom", denom))
        }
        None => Err(ContractError::Cw721NotLinked {}),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info, MOCK_CONTRACT_ADDR};
    use cosmwasm_std::{coins, from_json, to_json_binary, CosmosMsg, SubMsgResponse, SubMsgResult};
    use cw721::state::DefaultOptionMetadataExtension;
    use prost::Message;

    const NFT_CONTRACT_ADDR: &str = "nftcontract";
    const TOKEN_DENOM: &str = "uxion";

    #[derive(Clone, PartialEq, Message)]
    struct MsgInstantiateContractResponse {
        #[prost(string, tag = "1")]
        pub contract_address: ::prost::alloc::string::String,
        #[prost(bytes, tag = "2")]
        pub data: ::prost::alloc::vec::Vec<u8>,
    }

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies();
        let msg = InstantiateMsg {
            owner: Addr::unchecked("owner"),
            denom: "uxion".to_string(),
            unit_price: Uint128::new(1),
            name: String::from("FRACTIT"),
            symbol: String::from("FRACTIT"),
            token_code_id: 10u64,
            token_uri: String::from("https://ipfs.io/ipfs/Q"),
            extension: None,
            withdraw_address: None,
        };

        let info = mock_info("owner", &[]);
        let res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg.clone()).unwrap();

        assert_eq!(
            res.messages,
            vec![SubMsg {
                msg: WasmMsg::Instantiate {
                    code_id: msg.token_code_id,
                    msg: to_json_binary(&Cw721InstantiateMsg {
                        name: msg.name.clone(),
                        symbol: msg.symbol.clone(),
                        minter: None,
                        withdraw_address: None,
                    })
                    .unwrap(),
                    funds: vec![],
                    admin: None,
                    label: String::from("Instantiate fixed price NFT contract"),
                }
                .into(),
                id: INSTANTIATE_TOKEN_REPLY_ID,
                gas_limit: None,
                reply_on: ReplyOn::Success,
            }]
        );

        let instantiate_reply = MsgInstantiateContractResponse {
            contract_address: NFT_CONTRACT_ADDR.to_string(),
            data: vec![2u8; 32769],
        };
        let mut encoded_instantiate_reply =
            Vec::<u8>::with_capacity(instantiate_reply.encoded_len());
        instantiate_reply
            .encode(&mut encoded_instantiate_reply)
            .unwrap();

        let reply_msg = Reply {
            id: INSTANTIATE_TOKEN_REPLY_ID,
            result: SubMsgResult::Ok(SubMsgResponse {
                events: vec![],
                data: Some(encoded_instantiate_reply.into()),
            }),
        };
        reply(deps.as_mut(), mock_env(), reply_msg).unwrap();

        let query_msg = QueryMsg::GetConfig {};
        let res = query(deps.as_ref(), mock_env(), query_msg).unwrap();
        let config: ConfigResponse = from_json(&res).unwrap();
        // assert_eq!(
        //     config,
        //     ConfigResponse {
        //         owner: Addr::unchecked("owner"),
        //         cw721_address: Some(Addr::unchecked(NFT_CONTRACT_ADDR)),
        //         unit_price: msg.unit_price,
        //         name: msg.name,
        //         symbol: msg.symbol,
        //         token_uri: msg.token_uri,
        //         total_mint: msg.total_mint,
        //         extension: None,
        //         unused_token_id: 0
        //     }
        // );
    }

    #[test]
    fn mint_nft() {
        let mut deps = mock_dependencies();
        let msg = InstantiateMsg {
            owner: Addr::unchecked("owner"),
            denom: "uxion".to_string(),
            unit_price: Uint128::new(1),
            name: String::from("FRACTIT"),
            symbol: String::from("FRACTIT"),
            token_code_id: 10u64,
            token_uri: String::from("https://ipfs.io/ipfs/Q"),
            extension: None,
            withdraw_address: None,
        };

        let info = mock_info("owner", &[]);
        instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        let instantiate_reply = MsgInstantiateContractResponse {
            contract_address: NFT_CONTRACT_ADDR.to_string(),
            data: vec![2u8; 32769],
        };
        let mut encoded_instantiate_reply =
            Vec::<u8>::with_capacity(instantiate_reply.encoded_len());
        instantiate_reply
            .encode(&mut encoded_instantiate_reply)
            .unwrap();

        let reply_msg = Reply {
            id: INSTANTIATE_TOKEN_REPLY_ID,
            result: SubMsgResult::Ok(SubMsgResponse {
                events: vec![],
                data: Some(encoded_instantiate_reply.into()),
            }),
        };
        reply(deps.as_mut(), mock_env(), reply_msg).unwrap();

        let msg = ExecuteMsg::Mint {};
        let info = mock_info(MOCK_CONTRACT_ADDR, &coins(1, TOKEN_DENOM));
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        let mint_msg = Cw721ExecuteMsg::<DefaultOptionMetadataExtension, Empty>::Mint {
            token_id: String::from("0"),
            owner: MOCK_CONTRACT_ADDR.to_string(),
            token_uri: Some(String::from("https://ipfs.io/ipfs/Q")),
            extension: None,
        };

        assert_eq!(
            res.messages[0],
            SubMsg {
                msg: CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: NFT_CONTRACT_ADDR.to_string(),
                    msg: to_json_binary(&mint_msg).unwrap(),
                    funds: vec![],
                }),
                id: 0,
                gas_limit: None,
                reply_on: ReplyOn::Never,
            }
        );
    }

    #[test]
    fn invalid_unit_price() {
        let mut deps = mock_dependencies();
        let msg = InstantiateMsg {
            owner: Addr::unchecked("owner"),
            denom: "uxion".to_string(),
            unit_price: Uint128::new(0),
            name: String::from("FRACTIT"),
            symbol: String::from("FRACTIT"),
            token_code_id: 10u64,
            token_uri: String::from("https://ipfs.io/ipfs/Q"),
            extension: None,
            withdraw_address: None,
        };

        let info = mock_info("owner", &[]);
        let err = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap_err();

        match err {
            ContractError::InvalidUnitPrice {} => {}
            e => panic!("unexpected error: {e}"),
        }
    }

    #[test]
    fn invalid_max_tokens() {
        let mut deps = mock_dependencies();
        let msg = InstantiateMsg {
            owner: Addr::unchecked("owner"),
            denom: "uxion".to_string(),
            unit_price: Uint128::new(1),
            name: String::from("FRACTIT"),
            symbol: String::from("FRACTIT"),
            token_code_id: 10u64,
            token_uri: String::from("https://ipfs.io/ipfs/Q"),
            extension: None,
            withdraw_address: None,
        };

        let info = mock_info("owner", &[]);
        let err = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap_err();

        match err {
            ContractError::InvalidMaxTokens {} => {}
            e => panic!("unexpected error: {e}"),
        }
    }

    #[test]
    fn uninitialized() {
        let mut deps = mock_dependencies();
        let msg = InstantiateMsg {
            owner: Addr::unchecked("owner"),
            denom: "uxion".to_string(),
            unit_price: Uint128::new(1),
            name: String::from("FRACTIT"),
            symbol: String::from("FRACTIT"),
            token_code_id: 10u64,
            token_uri: String::from("https://ipfs.io/ipfs/Q"),
            extension: None,
            withdraw_address: None,
        };

        let info = mock_info("owner", &[]);
        instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        let msg = ExecuteMsg::Mint {};
        let info = mock_info(MOCK_CONTRACT_ADDR, &coins(1, TOKEN_DENOM));
        let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();

        match err {
            ContractError::Uninitialized {} => {}
            e => panic!("unexpected error: {e}"),
        }
    }

    #[test]
    fn sold_out() {
        let mut deps = mock_dependencies();
        let msg = InstantiateMsg {
            owner: Addr::unchecked("owner"),
            denom: "uxion".to_string(),
            unit_price: Uint128::new(1),
            name: String::from("FRACTIT"),
            symbol: String::from("FRACTIT"),
            token_code_id: 10u64,
            token_uri: String::from("https://ipfs.io/ipfs/Q"),
            extension: None,
            withdraw_address: None,
        };

        let info = mock_info("owner", &[]);
        instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        let instantiate_reply = MsgInstantiateContractResponse {
            contract_address: NFT_CONTRACT_ADDR.to_string(),
            data: vec![2u8; 32769],
        };
        let mut encoded_instantiate_reply =
            Vec::<u8>::with_capacity(instantiate_reply.encoded_len());
        instantiate_reply
            .encode(&mut encoded_instantiate_reply)
            .unwrap();

        let reply_msg = Reply {
            id: INSTANTIATE_TOKEN_REPLY_ID,
            result: SubMsgResult::Ok(SubMsgResponse {
                events: vec![],
                data: Some(encoded_instantiate_reply.into()),
            }),
        };
        reply(deps.as_mut(), mock_env(), reply_msg).unwrap();

        let msg = ExecuteMsg::Mint {};
        let info = mock_info(MOCK_CONTRACT_ADDR, &coins(1, TOKEN_DENOM));

        // Max mint is 1, so second mint request should fail
        execute(deps.as_mut(), mock_env(), info.clone(), msg.clone()).unwrap();
        // let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();

        // match err {
        //     ContractError::SoldOut {} => {}
        //     e => panic!("unexpected error: {e}"),
        // }
    }

    #[test]
    fn wrong_amount() {
        let mut deps = mock_dependencies();
        let msg = InstantiateMsg {
            owner: Addr::unchecked("owner"),
            denom: "uxion".to_string(),
            unit_price: Uint128::new(1),
            name: String::from("FRACTIT"),
            symbol: String::from("FRACTIT"),
            token_code_id: 10u64,
            token_uri: String::from("https://ipfs.io/ipfs/Q"),
            extension: None,
            withdraw_address: None,
        };

        let info = mock_info("owner", &[]);
        instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        let instantiate_reply = MsgInstantiateContractResponse {
            contract_address: NFT_CONTRACT_ADDR.to_string(),
            data: vec![2u8; 32769],
        };
        let mut encoded_instantiate_reply =
            Vec::<u8>::with_capacity(instantiate_reply.encoded_len());
        instantiate_reply
            .encode(&mut encoded_instantiate_reply)
            .unwrap();

        let reply_msg = Reply {
            id: INSTANTIATE_TOKEN_REPLY_ID,
            result: SubMsgResult::Ok(SubMsgResponse {
                events: vec![],
                data: Some(encoded_instantiate_reply.into()),
            }),
        };
        reply(deps.as_mut(), mock_env(), reply_msg).unwrap();

        let msg = ExecuteMsg::Mint {};
        let info = mock_info(MOCK_CONTRACT_ADDR, &coins(2, TOKEN_DENOM));
        let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();

        match err {
            ContractError::WrongPaymentAmount {} => {}
            e => panic!("unexpected error: {e}"),
        }
    }

    #[test]
    fn unauthorized_token() {
        let mut deps = mock_dependencies();
        let msg = InstantiateMsg {
            owner: Addr::unchecked("owner"),
            denom: "uxion".to_string(),
            unit_price: Uint128::new(1),
            name: String::from("FRACTIT"),
            symbol: String::from("FRACTIT"),
            token_code_id: 10u64,
            token_uri: String::from("https://ipfs.io/ipfs/Q"),
            extension: None,
            withdraw_address: None,
        };

        let info = mock_info("owner", &[]);
        instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        let instantiate_reply = MsgInstantiateContractResponse {
            contract_address: NFT_CONTRACT_ADDR.to_string(),
            data: vec![2u8; 32769],
        };
        let mut encoded_instantiate_reply =
            Vec::<u8>::with_capacity(instantiate_reply.encoded_len());
        instantiate_reply
            .encode(&mut encoded_instantiate_reply)
            .unwrap();

        let reply_msg = Reply {
            id: INSTANTIATE_TOKEN_REPLY_ID,
            result: SubMsgResult::Ok(SubMsgResponse {
                events: vec![],
                data: Some(encoded_instantiate_reply.into()),
            }),
        };
        reply(deps.as_mut(), mock_env(), reply_msg).unwrap();

        let msg = ExecuteMsg::Mint {};
        let info = mock_info("unauthorized-token", &coins(1, TOKEN_DENOM));
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        let mint_msg = Cw721ExecuteMsg::<DefaultOptionMetadataExtension, Empty>::Mint {
            token_id: String::from("0"),
            owner: String::from("unauthorized-token"),
            token_uri: Some(String::from("https://ipfs.io/ipfs/Q")),
            extension: None,
        };

        assert_eq!(
            res.messages[0],
            SubMsg {
                msg: CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: NFT_CONTRACT_ADDR.to_string(),
                    msg: to_json_binary(&mint_msg).unwrap(),
                    funds: vec![],
                }),
                id: 0,
                gas_limit: None,
                reply_on: ReplyOn::Never,
            }
        );
    }
}
