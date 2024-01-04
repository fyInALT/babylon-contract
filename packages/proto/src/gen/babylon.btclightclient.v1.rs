// @generated
/// BTCHeaderInfo is a structure that contains all relevant information about a
/// BTC header
///   - Full header bytes
///   - Header hash for easy retrieval
///   - Height of the header in the BTC chain
///   - Total work spent on the header. This is the sum of the work corresponding
///   to the header Bits field
///     and the total work of the header.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct BtcHeaderInfo {
    #[prost(bytes="bytes", tag="1")]
    pub header: ::prost::bytes::Bytes,
    #[prost(bytes="bytes", tag="2")]
    pub hash: ::prost::bytes::Bytes,
    #[prost(uint64, tag="3")]
    pub height: u64,
    #[prost(bytes="bytes", tag="4")]
    pub work: ::prost::bytes::Bytes,
}
/// QueryMainChainResponse is response type for the Query/MainChain RPC method.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct QueryMainChainResponse {
    #[prost(message, repeated, tag="1")]
    pub headers: ::prost::alloc::vec::Vec<BtcHeaderInfo>,
    #[prost(message, optional, tag="2")]
    pub pagination: ::core::option::Option<cosmos_sdk_proto::cosmos::base::query::v1beta1::PageResponse>,
}
// @@protoc_insertion_point(module)