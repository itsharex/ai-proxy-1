use ai_proxy_lib::converter::generators::anthropic::AnthropicGenerator;
use ai_proxy_lib::converter::generators::completions::CompletionsGenerator;
use ai_proxy_lib::converter::generators::gemini::GeminiGenerator;
use ai_proxy_lib::converter::generators::responses::ResponsesGenerator;
use ai_proxy_lib::converter::ir::*;
use ai_proxy_lib::converter::parsers::anthropic::AnthropicParser;
use ai_proxy_lib::converter::parsers::completions::CompletionsParser;
use ai_proxy_lib::converter::parsers::gemini::GeminiParser;
use ai_proxy_lib::converter::parsers::responses::ResponsesParser;
use ai_proxy_lib::converter::{FormatGenerator, FormatParser};
use serde_json::json;

fn sample_ir_request() -> IrRequest {
    IrRequest {
        model: "gpt-4o".into(),
        messages: vec![
            IrMessage {
                role: IrRole::System,
                content: vec![IrContentPart::Text {
                    text: "You are helpful.".into(),
                }],
                name: None,
                tool_call_id: None,
                tool_calls: None,
            },
            IrMessage {
                role: IrRole::User,
                content: vec![IrContentPart::Text {
                    text: "Hello!".into(),
                }],
                name: None,
                tool_call_id: None,
                tool_calls: None,
            },
        ],
        tools: None,
        tool_choice: None,
        temperature: Some(0.7),
        top_p: None,
        max_tokens: Some(1024),
        stream: false,
        stop_sequences: None,
        response_format: None,
        metadata: std::collections::HashMap::new(),
    }
}

fn sample_ir_response() -> IrResponse {
    IrResponse {
        id: Some("resp-123".into()),
        model: Some("gpt-4o".into()),
        message: IrMessage {
            role: IrRole::Assistant,
            content: vec![IrContentPart::Text {
                text: "Hi there!".into(),
            }],
            name: None,
            tool_call_id: None,
            tool_calls: None,
        },
        finish_reason: Some("stop".into()),
        usage: IrUsage {
            prompt_tokens: 10,
            completion_tokens: 5,
            total_tokens: 15,
        },
    }
}

fn assert_request_roundtrip(parsed: &IrRequest, original: &IrRequest) {
    assert_eq!(parsed.model, original.model);
    assert_eq!(parsed.messages.len(), original.messages.len());
    for (p, o) in parsed.messages.iter().zip(original.messages.iter()) {
        assert_eq!(std::mem::discriminant(&p.role), std::mem::discriminant(&o.role));
        assert_eq!(p.content.len(), o.content.len());
        for (pc, oc) in p.content.iter().zip(o.content.iter()) {
            match (pc, oc) {
                (IrContentPart::Text { text: t1 }, IrContentPart::Text { text: t2 }) => {
                    assert_eq!(t1, t2);
                }
                _ => panic!("content part type mismatch"),
            }
        }
    }
    assert_eq!(parsed.temperature, original.temperature);
    assert_eq!(parsed.max_tokens, original.max_tokens);
    assert_eq!(parsed.stream, original.stream);
}

fn assert_response_roundtrip(parsed: &IrResponse, original: &IrResponse) {
    assert_eq!(parsed.message.content.len(), original.message.content.len());
    for (p, o) in parsed
        .message
        .content
        .iter()
        .zip(original.message.content.iter())
    {
        match (p, o) {
            (IrContentPart::Text { text: t1 }, IrContentPart::Text { text: t2 }) => {
                assert_eq!(t1, t2);
            }
            _ => panic!("content type mismatch"),
        }
    }
    assert_eq!(parsed.usage.prompt_tokens, original.usage.prompt_tokens);
    assert_eq!(
        parsed.usage.completion_tokens,
        original.usage.completion_tokens
    );
}

fn run_roundtrip<P: FormatParser, G: FormatGenerator>(parser: P, generator: G) {
    let ir_req = sample_ir_request();
    let ir_resp = sample_ir_response();

    let generated_req = generator.generate_request(&ir_req).unwrap();
    let parsed_req = parser.parse_request(&generated_req).unwrap();
    assert_request_roundtrip(&parsed_req, &ir_req);

    let generated_resp = generator.generate_response(&ir_resp).unwrap();
    let parsed_resp = parser.parse_response(&generated_resp).unwrap();
    assert_response_roundtrip(&parsed_resp, &ir_resp);
}

#[test]
fn completions_roundtrip() {
    run_roundtrip(CompletionsParser, CompletionsGenerator);
}

#[test]
fn responses_roundtrip() {
    run_roundtrip(ResponsesParser, ResponsesGenerator);
}

#[test]
fn anthropic_roundtrip() {
    run_roundtrip(AnthropicParser, AnthropicGenerator);
}

#[test]
fn gemini_roundtrip() {
    let ir_req = sample_ir_request();
    let ir_resp = sample_ir_response();

    let generator = GeminiGenerator;
    let parser = GeminiParser;

    let generated_req = generator.generate_request(&ir_req).unwrap();
    let parsed_req = parser.parse_request(&generated_req).unwrap();
    assert_eq!(parsed_req.model, "");
    assert_eq!(parsed_req.messages.len(), ir_req.messages.len());

    let generated_resp = generator.generate_response(&ir_resp).unwrap();
    let parsed_resp = parser.parse_response(&generated_resp).unwrap();
    assert_response_roundtrip(&parsed_resp, &ir_resp);
}

#[test]
fn completions_stream_chunk_done() {
    let parser = CompletionsParser;
    let chunk = parser.parse_stream_chunk("data: [DONE]").unwrap();
    assert!(chunk.is_some());
    let c = chunk.unwrap();
    assert_eq!(c.finish_reason.as_deref(), Some("stop"));
}

#[test]
fn completions_stream_chunk_data() {
    let parser = CompletionsParser;
    let input = r#"data: {"id":"chatcmpl-1","model":"gpt-4o","choices":[{"index":0,"delta":{"content":"Hi"},"finish_reason":null}]}"#;
    let chunk = parser.parse_stream_chunk(input).unwrap().unwrap();
    assert_eq!(chunk.id.as_deref(), Some("chatcmpl-1"));
    assert_eq!(chunk.delta_content.as_deref(), Some("Hi"));
}

#[test]
fn completions_tool_calls_request() {
    let generator = CompletionsGenerator;
    let ir = IrRequest {
        model: "gpt-4o".into(),
        messages: vec![IrMessage {
            role: IrRole::User,
            content: vec![IrContentPart::Text {
                text: "What's the weather?".into(),
            }],
            name: None,
            tool_call_id: None,
            tool_calls: None,
        }],
        tools: Some(vec![IrTool {
            name: "get_weather".into(),
            description: Some("Get current weather".into()),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "location": {"type": "string"}
                },
                "required": ["location"]
            }),
        }]),
        tool_choice: None,
        temperature: None,
        top_p: None,
        max_tokens: None,
        stream: false,
        stop_sequences: None,
        response_format: None,
        metadata: std::collections::HashMap::new(),
    };

    let body = generator.generate_request(&ir).unwrap();
    let tools = body.get("tools").unwrap().as_array().unwrap();
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0]["function"]["name"].as_str().unwrap(), "get_weather");

    let parser = CompletionsParser;
    let parsed = parser.parse_request(&body).unwrap();
    assert!(parsed.tools.is_some());
    assert_eq!(parsed.tools.unwrap().len(), 1);
}

#[test]
fn cross_format_completions_to_anthropic() {
    let ir = sample_ir_request();

    let gen = CompletionsGenerator;
    let comp_body = gen.generate_request(&ir).unwrap();
    assert!(comp_body.get("messages").is_some());

    let gen = AnthropicGenerator;
    let ant_body = gen.generate_request(&ir).unwrap();
    assert!(ant_body.get("messages").is_some());
    assert_eq!(ant_body["model"].as_str().unwrap(), "gpt-4o");
}

#[test]
fn cross_format_completions_to_gemini() {
    let ir = sample_ir_request();

    let gen = GeminiGenerator;
    let gem_body = gen.generate_request(&ir).unwrap();
    assert!(gem_body.get("contents").is_some());
    assert!(gem_body.get("systemInstruction").is_some());
}

#[test]
fn anthropic_response_with_tool_use() {
    let body = json!({
        "id": "msg-1",
        "type": "message",
        "role": "assistant",
        "content": [
            { "type": "text", "text": "Let me check." },
            {
                "type": "tool_use",
                "id": "tool-1",
                "name": "get_weather",
                "input": { "location": "Tokyo" }
            }
        ],
        "model": "claude-3",
        "stop_reason": "tool_use",
        "usage": {
            "input_tokens": 20,
            "output_tokens": 30
        }
    });

    let parser = AnthropicParser;
    let ir = parser.parse_response(&body).unwrap();

    assert_eq!(ir.message.content.len(), 1);
    assert!(matches!(ir.message.content[0], IrContentPart::Text { .. }));

    let tool_calls = ir.message.tool_calls.as_ref().expect("expected tool_calls");
    assert_eq!(tool_calls.len(), 1);
    assert_eq!(tool_calls[0].name, "get_weather");
    assert_eq!(tool_calls[0].id, "tool-1");
}

#[test]
fn model_pattern_matching() {
    assert!(model_matches("gpt-4o", "gpt-4o"));
    assert!(model_matches("gpt-4o", "*"));
    assert!(model_matches("gpt-4o-mini", "gpt-4o*"));
    assert!(!model_matches("claude-3", "gpt-4o*"));
    assert!(model_matches("claude-3-opus", "*opus"));
}

fn model_matches(model: &str, pattern: &str) -> bool {
    if pattern == "*" || pattern == model {
        return true;
    }
    if let Some(prefix) = pattern.strip_suffix('*') {
        return model.starts_with(prefix);
    }
    if let Some(suffix) = pattern.strip_prefix('*') {
        return model.ends_with(suffix);
    }
    false
}
