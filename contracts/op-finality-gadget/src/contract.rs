use crate::error::ContractError;
use crate::finality::{handle_finality_signature, handle_public_randomness_commit};
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::queries::{query_block_finalized, query_config};
use crate::state::config::{Config, ADMIN, CONFIG};
use cosmwasm_std::{to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};
use cw_utils::maybe_addr;

use babylon_apis::queries::BabylonQueryWrapper;

pub fn instantiate(
    mut deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    let api = deps.api;
    ADMIN.set(deps.branch(), maybe_addr(api, Some(msg.admin))?)?;

    let config = Config {
        consumer_id: msg.consumer_id,
        activated_height: msg.activated_height,
    };
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new().add_attribute("action", "instantiate"))
}

pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::QueryBlockFinalized {
            height,
            hash,
            timestamp,
        } => Ok(to_json_binary(&query_block_finalized(
            deps, height, hash, timestamp,
        )?)?),
        QueryMsg::Config {} => Ok(to_json_binary(&query_config(deps)?)?),
    }
}

pub fn execute(
    deps: DepsMut<BabylonQueryWrapper>,
    env: Env,
    _info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::CommitPublicRandomness {
            fp_pubkey_hex,
            start_height,
            num_pub_rand,
            commitment,
            signature,
        } => handle_public_randomness_commit(
            deps,
            &fp_pubkey_hex,
            start_height,
            num_pub_rand,
            &commitment,
            &signature,
        ),
        ExecuteMsg::SubmitFinalitySignature {
            fp_pubkey_hex,
            height,
            pub_rand,
            proof,
            block_hash,
            signature,
        } => handle_finality_signature(
            deps,
            env,
            &fp_pubkey_hex,
            height,
            &pub_rand,
            &proof,
            &block_hash,
            &signature,
        ),
    }
}