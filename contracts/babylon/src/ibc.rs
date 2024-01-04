use crate::error::ContractError;
use crate::msg::bindings::BabylonMsg;
use crate::msg::ibc::{new_ack_err, new_ack_res};
use babylon_proto::babylon::zoneconcierge::v1::{
    zoneconcierge_packet_data, BtcTimestamp, ZoneconciergePacketData,
};
use cosmwasm_std::{
    DepsMut, Env, Event, Ibc3ChannelOpenResponse, IbcBasicResponse, IbcChannelCloseMsg,
    IbcChannelConnectMsg, IbcChannelOpenMsg, IbcChannelOpenResponse, IbcOrder, IbcPacketAckMsg,
    IbcPacketReceiveMsg, IbcPacketTimeoutMsg, IbcReceiveResponse, Never, StdError, StdResult,
};
use prost::Message;

pub const IBC_VERSION: &str = "zoneconcierge-1";
pub const IBC_ORDERING: IbcOrder = IbcOrder::Ordered;

/// This is executed during the ChannelOpenInit and ChannelOpenTry
/// of the IBC 4-step channel protocol
/// (see https://github.com/cosmos/ibc/tree/main/spec/core/ics-004-channel-and-packet-semantics#channel-lifecycle-management)
/// In the case of ChannelOpenTry there's a counterparty_version attribute in the message.
/// Here we ensure the ordering and version constraints.
pub fn ibc_channel_open(
    _deps: DepsMut,
    _env: Env,
    msg: IbcChannelOpenMsg,
) -> Result<IbcChannelOpenResponse, ContractError> {
    // the IBC channel has to be ordered
    let channel = msg.channel();
    if channel.order != IBC_ORDERING {
        return Err(ContractError::IbcUnorderedChannel {});
    }

    // In ibcv3 we don't check the version string passed in the message
    // and only check the counterparty version.
    if let Some(counter_version) = msg.counterparty_version() {
        if counter_version != IBC_VERSION {
            return Err(ContractError::IbcInvalidCounterPartyVersion {
                version: IBC_VERSION.to_string(),
            });
        }
    }

    // We return the version we need (which could be different than the counterparty version)
    Ok(Some(Ibc3ChannelOpenResponse {
        version: IBC_VERSION.to_string(),
    }))
}

/// Second part of the 4-step handshake, i.e.,
/// ChannelOpenAck and ChannelOpenConfirm.
pub fn ibc_channel_connect(
    _deps: DepsMut,
    _env: Env,
    msg: IbcChannelConnectMsg,
) -> StdResult<IbcBasicResponse> {
    let channel = msg.channel();
    let chan_id = &channel.endpoint.channel_id;

    Ok(IbcBasicResponse::new()
        .add_attribute("action", "ibc_connect")
        .add_attribute("channel_id", chan_id)
        .add_event(Event::new("ibc").add_attribute("channel", "connect")))
}

/// This is invoked on the IBC Channel Close message
/// We perform any cleanup related to the channel
pub fn ibc_channel_close(
    _deps: DepsMut,
    _env: Env,
    msg: IbcChannelCloseMsg,
) -> StdResult<IbcBasicResponse> {
    let channel = msg.channel();
    // get contract address and remove lookup
    let channel_id = channel.endpoint.channel_id.as_str();

    Ok(IbcBasicResponse::new()
        .add_attribute("action", "ibc_close")
        .add_attribute("channel_id", channel_id))

    // TODO: erase all contract state upon closing the channel
}

/// Invoked when an IBC packet is received
/// We decode the contents of the packet and if it matches one of the packets we support
/// execute the relevant function, otherwise return an error.
/// NOTE: In its current form, this method does not modify state.
/// If we want to modify state here, we have to follow the techniques outlined here:
/// https://github.com/CosmWasm/cosmwasm/blob/main/IBC.md#acknowledging-errors
/// That's because we want to send an ACK for the packet regardless if there's an error or not,
/// but in the case of an error, we do not want the state to be committed.
pub fn ibc_packet_receive(
    deps: DepsMut,
    _env: Env,
    msg: IbcPacketReceiveMsg,
) -> Result<IbcReceiveResponse<BabylonMsg>, Never> {
    // put this in a closure so we can convert all error responses into acknowledgements
    (|| {
        let packet = msg.packet;
        // which local channel did this packet come on
        let caller = packet.dest.channel_id;
        let zc_packet_data =
            ZoneconciergePacketData::decode(packet.data.as_slice()).map_err(|e| {
                StdError::generic_err(format!("failed to decode ZoneconciergePacketData: {e}"))
            })?;
        let zc_packet = zc_packet_data
            .packet
            .ok_or(StdError::generic_err("empty IBC packet"))?;
        match zc_packet {
            zoneconcierge_packet_data::Packet::BtcTimestamp(btc_ts) => {
                ibc_packet::handle_btc_timestamp(deps, caller, &btc_ts)
            }
        }
    })()
    .or_else(|e| {
        // we try to capture all app-level errors and convert them into
        // acknowledgement packets that contain an error code.
        let acknowledgement = new_ack_err(format!("invalid packet: {e}")); // TODO: design error ack format
        Ok(IbcReceiveResponse::new()
            .set_ack(acknowledgement.encode_to_vec())
            .add_event(Event::new("ibc").add_attribute("packet", "receive")))
    })
}

// Methods to handle PacketMsg variants
mod ibc_packet {
    use super::*;
    use crate::state::config::CONFIG;

    // processes PacketMsg::WhoAmI variant
    pub fn handle_btc_timestamp(
        deps: DepsMut,
        _caller: String,
        btc_ts: &BtcTimestamp,
    ) -> StdResult<IbcReceiveResponse<BabylonMsg>> {
        let storage = deps.storage;
        let cfg = CONFIG.load(storage)?;

        // handle the BTC timestamp, i.e., verify the BTC timestamp and update the contract state
        let msg_option = crate::state::handle_btc_timestamp(storage, btc_ts)?;

        // construct response
        let mut resp: IbcReceiveResponse<BabylonMsg> = IbcReceiveResponse::new();
        // add attribute to response
        resp = resp.add_attribute("action", "receive_btc_timestamp");
        // add ack message to response
        let acknowledgement = new_ack_res(); // TODO: design response format
        resp = resp.set_ack(acknowledgement.encode_to_vec());

        // if the BTC timestamp carries a Babylon message for the Cosmos zone, and
        // the contract enables sending messages to the Cosmos zone, then
        // add this message to response
        if let Some(msg) = msg_option {
            if cfg.notify_cosmos_zone {
                resp = resp.add_message(msg);
            }
        }

        Ok(resp)
    }
}

/// never should be called as we do not send packets
pub fn ibc_packet_ack(
    _deps: DepsMut,
    _env: Env,
    _msg: IbcPacketAckMsg,
) -> Result<IbcBasicResponse, ContractError> {
    Err(ContractError::IbcUnsupportedMethod {})
}

/// never should be called as we do not send packets
pub fn ibc_packet_timeout(
    _deps: DepsMut,
    _env: Env,
    _msg: IbcPacketTimeoutMsg,
) -> Result<IbcBasicResponse, ContractError> {
    Err(ContractError::IbcUnsupportedMethod {})
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contract::instantiate;
    use crate::msg::contract::InstantiateMsg;
    use cosmwasm_std::testing::{
        mock_dependencies, mock_env, mock_ibc_channel_open_try, mock_info, MockApi, MockQuerier,
        MockStorage,
    };
    use cosmwasm_std::OwnedDeps;

    const CREATOR: &str = "creator";

    fn setup() -> OwnedDeps<MockStorage, MockApi, MockQuerier> {
        let mut deps = mock_dependencies();
        let msg = InstantiateMsg {
            network: babylon_bitcoin::chain_params::Network::Regtest,
            babylon_tag: "01020304".to_string(),
            btc_confirmation_depth: 10,
            checkpoint_finalization_timeout: 100,
            notify_cosmos_zone: false,
        };
        let info = mock_info(CREATOR, &[]);
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());
        deps
    }

    #[test]
    fn enforce_version_in_handshake() {
        let mut deps = setup();

        let wrong_order = mock_ibc_channel_open_try("channel-12", IbcOrder::Unordered, IBC_VERSION);
        ibc_channel_open(deps.as_mut(), mock_env(), wrong_order).unwrap_err();

        let wrong_version = mock_ibc_channel_open_try("channel-12", IbcOrder::Ordered, "reflect");
        ibc_channel_open(deps.as_mut(), mock_env(), wrong_version).unwrap_err();

        let valid_handshake = mock_ibc_channel_open_try("channel-12", IBC_ORDERING, IBC_VERSION);
        ibc_channel_open(deps.as_mut(), mock_env(), valid_handshake).unwrap();
    }
}