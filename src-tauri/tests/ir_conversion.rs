#[cfg(test)]
mod tests {
    use ai_proxy_lib::converter::generators::completions::CompletionsGenerator;
    use ai_proxy_lib::converter::ir::*;
    use ai_proxy_lib::converter::parsers::anthropic::AnthropicParser;
    use ai_proxy_lib::converter::parsers::completions::CompletionsParser;
    use ai_proxy_lib::converter::parsers::responses::ResponsesParser;
    use ai_proxy_lib::converter::{FormatGenerator, FormatParser};
    use serde_json::json;

    #[test]
    fn test_completions_to_ir_roundtrip() {
        let input = json!({
            "model": "gpt-4o",
            "messages": [
                {"role": "system", "content": "You are helpful."},
                {"role": "user", "content": "Hello"}
            ],
            "temperature": 0.7,
            "stream": false
        });

        let parser = CompletionsParser;
        let ir = parser.parse_request(&input).unwrap();
        assert_eq!(ir.model, "gpt-4o");
        assert_eq!(ir.messages.len(), 2);
        assert!((ir.temperature.unwrap() - 0.7f32).abs() < 0.01);

        let gen = CompletionsGenerator;
        let output = gen.generate_request(&ir).unwrap();

        assert_eq!(output["model"], "gpt-4o");
        assert_eq!(output["messages"][0]["content"], "You are helpful.");
        assert_eq!(output["messages"][1]["content"], "Hello");
        assert_eq!(output["stream"], false);
    }

    #[test]
    fn test_responses_to_completions_conversion() {
        let input = json!({
            "model": "gpt-4o",
            "input": "What is Rust?",
            "instructions": "Be concise"
        });

        let parser = ResponsesParser;
        let ir = parser.parse_request(&input).unwrap();
        assert_eq!(ir.model, "gpt-4o");
        assert_eq!(ir.messages.len(), 2);

        let gen = CompletionsGenerator;
        let output = gen.generate_request(&ir).unwrap();

        assert_eq!(output["model"], "gpt-4o");
        assert_eq!(output["messages"][0]["role"], "system");
        assert_eq!(output["messages"][0]["content"], "Be concise");
        assert_eq!(output["messages"][1]["role"], "user");
        assert_eq!(output["messages"][1]["content"], "What is Rust?");
    }

    #[test]
    fn test_anthropic_to_completions_conversion() {
        let input = json!({
            "model": "claude-sonnet-4-5",
            "system": "You are helpful.",
            "messages": [
                {"role": "user", "content": "Hello"}
            ],
            "max_tokens": 4096,
            "stream": false
        });

        let parser = AnthropicParser;
        let ir = parser.parse_request(&input).unwrap();
        assert_eq!(ir.model, "claude-sonnet-4-5");
        assert_eq!(ir.max_tokens, Some(4096));

        let gen = CompletionsGenerator;
        let output = gen.generate_request(&ir).unwrap();

        assert_eq!(output["model"], "claude-sonnet-4-5");
        assert_eq!(output["messages"][0]["role"], "system");
        assert_eq!(output["messages"][0]["content"], "You are helpful.");
        assert_eq!(output["messages"][1]["role"], "user");
        assert_eq!(output["messages"][1]["content"], "Hello");
        assert_eq!(output["max_tokens"], 4096);
    }

    #[test]
    fn test_stream_chunk_parsing() {
        let line = "data: {\"id\":\"chatcmpl-123\",\"object\":\"chat.completion.chunk\",\"choices\":[{\"delta\":{\"content\":\"Hello\"},\"index\":0}]}";
        let parser = CompletionsParser;
        let chunk = parser.parse_stream_chunk(line).unwrap();
        assert!(chunk.is_some());
        let chunk = chunk.unwrap();
        assert_eq!(chunk.delta_content, Some("Hello".into()));
        assert_eq!(chunk.id, Some("chatcmpl-123".into()));
    }

    #[test]
    fn test_stream_done_detection() {
        let line = "data: [DONE]";
        let parser = CompletionsParser;
        let chunk = parser.parse_stream_chunk(line).unwrap();
        assert!(chunk.is_some());
        assert_eq!(chunk.unwrap().finish_reason, Some("stop".into()));
    }

    #[test]
    fn test_client_format_detection() {
        assert_eq!(
            ClientFormat::from_path("/v1/chat/completions"),
            Some(ClientFormat::Completions)
        );
        assert_eq!(
            ClientFormat::from_path("/v1/responses"),
            Some(ClientFormat::Responses)
        );
        assert_eq!(
            ClientFormat::from_path("/v1/messages"),
            Some(ClientFormat::Anthropic)
        );
        assert_eq!(
            ClientFormat::from_path("/v1beta/models/gemini-pro:generateContent"),
            Some(ClientFormat::Gemini)
        );
        assert_eq!(ClientFormat::from_path("/unknown"), None);
    }

    #[test]
    fn test_stream_chunk_with_tool_calls() {
        let line = "data: {\"id\":\"chatcmpl-456\",\"choices\":[{\"delta\":{\"tool_calls\":[{\"index\":0,\"id\":\"call_abc\",\"function\":{\"name\":\"get_weather\",\"arguments\":\"{\\\"city\\\":\\\"SF\\\"}\"}}]},\"index\":0}]}";
        let parser = CompletionsParser;
        let chunk = parser.parse_stream_chunk(line).unwrap().unwrap();
        assert!(chunk.delta_tool_calls.is_some());
        let tc = &chunk.delta_tool_calls.unwrap()[0];
        assert_eq!(tc.index, 0);
        assert_eq!(tc.id, Some("call_abc".into()));
        assert_eq!(tc.name, Some("get_weather".into()));
    }

    #[test]
    fn test_completions_generator_stream_chunk() {
        let chunk = IrStreamChunk {
            id: Some("chatcmpl-789".into()),
            model: Some("gpt-4o".into()),
            delta_content: Some("world".into()),
            delta_tool_calls: None,
            finish_reason: None,
            usage: None,
        };

        let gen = CompletionsGenerator;
        let output = gen.generate_stream_chunk(&chunk);

        assert!(output.starts_with("data: "));
        assert!(output.contains("world"));
        assert!(output.contains("chatcmpl-789"));
    }

    #[test]
    fn test_non_stream_line_returns_none() {
        let parser = CompletionsParser;
        let chunk = parser.parse_stream_chunk("not a data line").unwrap();
        assert!(chunk.is_none());
    }
}
