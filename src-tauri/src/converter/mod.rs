pub mod ir;
pub mod parsers;
pub mod generators;

use crate::error::ProxyError;
use ir::{IrRequest, IrResponse, IrStreamChunk};

pub trait FormatParser {
    fn parse_request(&self, body: &serde_json::Value) -> Result<IrRequest, ProxyError>;
    fn parse_stream_chunk(&self, line: &str) -> Result<Option<IrStreamChunk>, ProxyError>;
    fn parse_response(&self, body: &serde_json::Value) -> Result<IrResponse, ProxyError>;
}

pub trait FormatGenerator {
    fn generate_request(&self, ir: &IrRequest) -> Result<serde_json::Value, ProxyError>;
    fn generate_stream_chunk(&self, chunk: &IrStreamChunk) -> String;
    fn generate_response(&self, ir: &IrResponse) -> Result<serde_json::Value, ProxyError>;
}
