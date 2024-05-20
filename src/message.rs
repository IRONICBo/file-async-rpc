use crate::error::RpcError;

/// The size of the request header.
pub const REQ_HEADER_SIZE: u64 = 17;
/// The size of the response header.
pub const RESP_HEADER_SIZE: u64 = 17;

/// The Encode trait is used to encode a data structure into a byte buffer.
pub trait Encode {
    fn encode(&self) -> Vec<u8>;
}

/// The Decode trait is used to decode a byte buffer into a data structure.
pub trait Decode {
    fn decode(buf: &[u8]) -> Result<Self, RpcError<String>>
    where
        Self: Sized;
}

/// The message module contains the data structures shared between the client and server.
#[derive(Debug)]
pub struct ReqHeader {
    /// The sequence number of the request.
    pub seq: u64,
    /// The operation type of the request.
    pub req_type: ReqType,
    /// The length of the request.
    pub len: u64,
}

/// The request type of the request.
#[derive(Debug)]
pub enum ReqType {
    FileBlockRequest,
    KeepAliveRequest,
}

impl ReqType {
    pub fn from_u8(op: u8) -> Result<Self, RpcError<String>> {
        match op {
            0 => Ok(Self::FileBlockRequest),
            1 => Ok(Self::KeepAliveRequest),
            _ => Err(RpcError::InternalError(format!(
                "Invalid operation type: {}",
                op
            ))),
        }
    }

    pub fn to_u8(&self) -> u8 {
        match self {
            Self::FileBlockRequest => 0,
            Self::KeepAliveRequest => 1,
        }
    }
}

impl Encode for ReqHeader {
    fn encode(&self) -> Vec<u8> {
        encode_req_header(self)
    }
}

impl Decode for ReqHeader {
    fn decode(buf: &[u8]) -> Result<Self, RpcError<String>> {
        decode_req_header(buf)
    }
}

/// Decode the request header from the buffer.
pub fn decode_req_header(buf: &[u8]) -> Result<ReqHeader, RpcError<String>> {
    if buf.len() < 17 {
        return Err(RpcError::InternalError("Insufficient bytes".to_string()));
    }
    let seq = u64::from_be_bytes(buf[0..8].try_into().unwrap());
    let op = buf[8];
    let len = u64::from_be_bytes(buf[9..17].try_into().unwrap());

    let req_type = ReqType::from_u8(op)?;

    Ok(ReqHeader { seq, req_type, len })
}

/// Encode the request header into a buffer.
pub fn encode_req_header(header: &ReqHeader) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend(header.seq.to_be_bytes());
    bytes.push(header.req_type.to_u8());
    bytes.extend(header.len.to_be_bytes());
    bytes
}

/// The response header.
#[derive(Debug)]
pub struct RespHeader {
    /// The sequence number of the response.
    pub seq: u64,
    /// The operation type of the request.
    pub resp_type: RespType,
    /// The length of the response.
    pub len: u64,
}

/// The operation type of the response.
#[derive(Debug)]
pub enum RespType {
    FileBlockResponse,
    KeepAliveResponse,
}

impl RespType {
    pub fn from_u8(op: u8) -> Result<Self, RpcError<String>> {
        match op {
            0 => Ok(Self::FileBlockResponse),
            1 => Ok(Self::KeepAliveResponse),
            _ => Err(RpcError::InternalError(format!(
                "Invalid operation type: {}",
                op
            ))),
        }
    }

    pub fn to_u8(&self) -> u8 {
        match self {
            Self::FileBlockResponse => 0,
            Self::KeepAliveResponse => 1,
        }
    }
}

impl Encode for RespHeader {
    fn encode(&self) -> Vec<u8> {
        encode_resp_header(self)
    }
}

impl Decode for RespHeader {
    fn decode(buf: &[u8]) -> Result<Self, RpcError<String>> {
        decode_resp_header(buf)
    }
}

/// Decode the response header from the buffer.
pub fn decode_resp_header(buf: &[u8]) -> Result<RespHeader, RpcError<String>> {
    if buf.len() < 17 {
        return Err(RpcError::InternalError("Insufficient bytes".to_string()));
    }
    let seq = u64::from_be_bytes(buf[0..8].try_into().unwrap());
    let op = buf[8];
    let len = u64::from_be_bytes(buf[9..17].try_into().unwrap());

    let resp_type = RespType::from_u8(op)?;

    Ok(RespHeader {
        seq,
        resp_type,
        len,
    })
}

/// Encode the response header into a buffer.
pub fn encode_resp_header(header: &RespHeader) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend(header.seq.to_be_bytes());
    bytes.push(header.resp_type.to_u8());
    bytes.extend(header.len.to_be_bytes());
    bytes
}

/// Common data structures shared between the client and server.
#[derive(Debug, Default)]
pub struct FileBlockRequest {
    /// The sequence number of the request.
    pub seq: u64,
    /// The file ID.
    pub file_id: u64,
    /// The block ID.
    pub block_id: u64,
    /// The block size.
    pub block_size: u64,
}

impl Encode for FileBlockRequest {
    fn encode(&self) -> Vec<u8> {
        encode_file_block_request(self)
    }
}

impl Decode for FileBlockRequest {
    fn decode(buf: &[u8]) -> Result<Self, RpcError<String>> {
        decode_file_block_request(buf)
    }
}

/// Decode the file block request from the buffer.
pub fn decode_file_block_request(buf: &[u8]) -> Result<FileBlockRequest, RpcError<String>> {
    if buf.len() < 32 {
        return Err(RpcError::InternalError("Insufficient bytes".to_string()));
    }
    let seq = u64::from_be_bytes(buf[0..8].try_into().unwrap());
    let file_id = u64::from_be_bytes(buf[8..16].try_into().unwrap());
    let block_id = u64::from_be_bytes(buf[16..24].try_into().unwrap());
    let block_size = u64::from_be_bytes(buf[24..32].try_into().unwrap());

    Ok(FileBlockRequest {
        seq,
        file_id,
        block_id,
        block_size,
    })
}

/// Encode the file block request into a buffer.
pub fn encode_file_block_request(req: &FileBlockRequest) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend(req.seq.to_be_bytes());
    bytes.extend(req.file_id.to_be_bytes());
    bytes.extend(req.block_id.to_be_bytes());
    bytes.extend(req.block_size.to_be_bytes());
    bytes
}

/// The response to a file block request.
#[derive(Debug, Default)]
pub struct FileBlockResponse {
    /// The sequence number of the response.
    pub seq: u64,
    /// The file ID.
    pub file_id: u64,
    /// The block ID.
    pub block_id: u64,
    /// The block size.
    pub block_size: u64,
    /// The status of the response.
    pub status: StatusCode,
    /// The data of the block.
    pub data: Vec<u8>,
}

impl Encode for FileBlockResponse {
    fn encode(&self) -> Vec<u8> {
        encode_file_block_response(self)
    }
}

impl Decode for FileBlockResponse {
    fn decode(buf: &[u8]) -> Result<Self, RpcError<String>> {
        decode_file_block_response(buf)
    }
}

/// Decode the file block response from the buffer.
pub fn decode_file_block_response(buf: &[u8]) -> Result<FileBlockResponse, RpcError<String>> {
    if buf.len() < 32 {
        return Err(RpcError::InternalError("Insufficient bytes".to_string()));
    }
    let seq = u64::from_be_bytes(
        buf[0..8]
            .try_into()
            .map_err(|_| RpcError::InternalError("Failed to convert bytes".to_string()))?,
    );
    let file_id = u64::from_be_bytes(
        buf[8..16]
            .try_into()
            .map_err(|_| RpcError::InternalError("Failed to convert bytes".to_string()))?,
    );
    let block_id = u64::from_be_bytes(
        buf[16..24]
            .try_into()
            .map_err(|_| RpcError::InternalError("Failed to convert bytes".to_string()))?,
    );
    let status = match buf[24] {
        0 => StatusCode::Success,
        1 => StatusCode::NotFound,
        2 => StatusCode::InternalError,
        3 => StatusCode::VersionMismatch,
        _ => return Err(RpcError::InternalError("Invalid status code".to_string())),
    };
    let block_size = u64::from_be_bytes(
        buf[25..33]
            .try_into()
            .map_err(|_| RpcError::InternalError("Failed to convert bytes".to_string()))?,
    );
    let data = buf[33..].to_vec();

    Ok(FileBlockResponse {
        seq,
        file_id,
        block_id,
        block_size,
        status,
        data,
    })
}

/// Encode the file block response into a buffer.
pub fn encode_file_block_response(resp: &FileBlockResponse) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend(resp.seq.to_be_bytes());
    bytes.extend(resp.file_id.to_be_bytes());
    bytes.extend(resp.block_id.to_be_bytes());
    match resp.status {
        StatusCode::Success => bytes.push(0),
        StatusCode::NotFound => bytes.push(1),
        StatusCode::InternalError => bytes.push(2),
        StatusCode::VersionMismatch => bytes.push(3),
    }
    bytes.extend(resp.block_size.to_be_bytes());
    bytes.extend(&resp.data);
    bytes
}

/// The request to keep the connection alive.
#[derive(Debug, Default)]
pub struct KeepAliveRequest {
    /// The sequence number of the request.
    seq: u64,
}

impl Encode for KeepAliveRequest {
    fn encode(&self) -> Vec<u8> {
        encode_keep_alive_request(self)
    }
}

impl Decode for KeepAliveRequest {
    fn decode(buf: &[u8]) -> Result<Self, RpcError<String>> {
        decode_keep_alive_request(buf)
    }
}

/// Decode the keep-alive request from the buffer.
pub fn decode_keep_alive_request(buf: &[u8]) -> Result<KeepAliveRequest, RpcError<String>> {
    if buf.len() < 8 {
        return Err(RpcError::InternalError("Insufficient bytes".to_string()));
    }
    let seq = u64::from_be_bytes(buf[0..8].try_into().unwrap());

    Ok(KeepAliveRequest { seq })
}

/// Encode the keep-alive request into a buffer.
pub fn encode_keep_alive_request(req: &KeepAliveRequest) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend(req.seq.to_be_bytes());
    bytes
}

/// The response to a keep-alive request.
#[derive(Debug, Default)]
pub struct KeepAliveResponse {
    /// The sequence number of the response.
    seq: u64,
}

impl Encode for KeepAliveResponse {
    fn encode(&self) -> Vec<u8> {
        encode_keep_alive_response(self)
    }
}

impl Decode for KeepAliveResponse {
    fn decode(buf: &[u8]) -> Result<Self, RpcError<String>> {
        decode_keep_alive_response(buf)
    }
}

/// Decode the keep-alive response from the buffer.
pub fn decode_keep_alive_response(buf: &[u8]) -> Result<KeepAliveResponse, RpcError<String>> {
    if buf.len() < 8 {
        return Err(RpcError::InternalError("Insufficient bytes".to_string()));
    }
    let seq = u64::from_be_bytes(buf[0..8].try_into().unwrap());

    Ok(KeepAliveResponse { seq })
}

/// Encode the keep-alive response into a buffer.
pub fn encode_keep_alive_response(resp: &KeepAliveResponse) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend(resp.seq.to_be_bytes());
    bytes
}

/// The status code of the response.
#[derive(Debug)]
pub enum StatusCode {
    /// The request is successful.
    Success,
    /// The request is not found.
    NotFound,
    /// The request is invalid.
    InternalError,
    /// The request is out dated.
    VersionMismatch,
}

impl Default for StatusCode {
    fn default() -> Self {
        Self::Success
    }
}
