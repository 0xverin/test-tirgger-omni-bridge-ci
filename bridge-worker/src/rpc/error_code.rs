// we should use -32000 to -32099 for implementation defined error codes,
// see https://www.jsonrpc.org/specification#error_object

pub const UNAUTHORIZED_REQUEST_CODE: i32 = -32000;
pub const KEYSTORE_WRITE_ERROR_CODE: i32 = -32001;
pub const SHIELDED_VALUE_DECRYPTION_ERROR_CODE: i32 = -32002;
