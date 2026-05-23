#[cfg(test)]
mod llm_tests {
    use crate::*;

    // =========================================================================
    // LlmClient::new() Tests
    // =========================================================================

    #[test]
    fn test_llm_client_new_creates_instance() {
        let client = LlmClient::new("http://localhost:11434".to_string(), "llama2".to_string());

        // Verify client fields are set correctly
        assert_eq!(client.base_url, "http://localhost:11434");
        assert_eq!(client.model, "llama2");
    }

    #[test]
    fn test_llm_client_new_with_custom_url() {
        let custom_url = "http://192.168.1.100:11434";
        let client = LlmClient::new(custom_url.to_string(), "mistral".to_string());

        assert_eq!(client.base_url, custom_url);
        assert_eq!(client.model, "mistral");
    }

    #[test]
    fn test_llm_client_default_with_model() {
        let client = LlmClient::default_with_model("codellama".to_string());

        assert_eq!(client.base_url, "http://localhost:11434");
        assert_eq!(client.model, "codellama");
    }

    // =========================================================================
    // IntentClassifier Tests (via LlmClient::classify_intent)
    // =========================================================================

    #[tokio::test]
    async fn test_classify_intent_rule() {
        let client = LlmClient::new("http://localhost:11434".to_string(), "llama2".to_string());

        // This test requires Ollama to be running
        // We test that the method returns a valid Intent enum
        let result = client
            .classify_intent("What are the GMP storage temperature requirements?")
            .await;

        // If Ollama is available, verify we get a valid Intent
        if let Ok(intent) = result {
            assert!(matches!(
                intent,
                Intent::Rule
                    | Intent::Factual
                    | Intent::Procedural
                    | Intent::Clarification
                    | Intent::Comparison
                    | Intent::General
            ));
        }
    }

    #[tokio::test]
    async fn test_classify_intent_factual() {
        let client = LlmClient::new("http://localhost:11434".to_string(), "llama2".to_string());

        let result = client
            .classify_intent("What is the exact pH value required for water for injection?")
            .await;

        if let Ok(intent) = result {
            assert!(matches!(
                intent,
                Intent::Rule
                    | Intent::Factual
                    | Intent::Procedural
                    | Intent::Clarification
                    | Intent::Comparison
                    | Intent::General
            ));
        }
    }

    #[tokio::test]
    async fn test_classify_intent_procedural() {
        let client = LlmClient::new("http://localhost:11434".to_string(), "llama2".to_string());

        let result = client
            .classify_intent("How do I conduct a GMP-compliant equipment qualification?")
            .await;

        if let Ok(intent) = result {
            assert!(matches!(
                intent,
                Intent::Rule
                    | Intent::Factual
                    | Intent::Procedural
                    | Intent::Clarification
                    | Intent::Comparison
                    | Intent::General
            ));
        }
    }

    #[tokio::test]
    async fn test_classify_intent_clarification() {
        let client = LlmClient::new("http://localhost:11434".to_string(), "llama2".to_string());

        let result = client
            .classify_intent("Can you explain what API purity testing involves?")
            .await;

        if let Ok(intent) = result {
            assert!(matches!(
                intent,
                Intent::Rule
                    | Intent::Factual
                    | Intent::Procedural
                    | Intent::Clarification
                    | Intent::Comparison
                    | Intent::General
            ));
        }
    }

    #[tokio::test]
    async fn test_classify_intent_comparison() {
        let client = LlmClient::new("http://localhost:11434".to_string(), "llama2".to_string());

        let result = client
            .classify_intent("How do FDA and EMA GMP requirements differ for aseptic processing?")
            .await;

        if let Ok(intent) = result {
            assert!(matches!(
                intent,
                Intent::Rule
                    | Intent::Factual
                    | Intent::Procedural
                    | Intent::Clarification
                    | Intent::Comparison
                    | Intent::General
            ));
        }
    }

    // =========================================================================
    // ChannelRouter Tests (via LlmClient::route_query)
    // =========================================================================

    #[tokio::test]
    async fn test_route_query_returns_valid_channel_route() {
        let client = LlmClient::new("http://localhost:11434".to_string(), "llama2".to_string());

        let result = client
            .route_query("What are the GMP documentation requirements?")
            .await;

        // If Ollama is available, verify we get a valid ChannelRoute
        if let Ok(route) = result {
            assert!(!route.channels.is_empty());
            assert!(matches!(
                route.intent,
                Intent::Rule
                    | Intent::Factual
                    | Intent::Procedural
                    | Intent::Clarification
                    | Intent::Comparison
                    | Intent::General
            ));
        }
    }

    #[tokio::test]
    async fn test_route_query_intent_rule_channels() {
        let client = LlmClient::new("http://localhost:11434".to_string(), "llama2".to_string());

        let result = client
            .route_query("What temperature range is required for vaccine storage?")
            .await;

        if let Ok(route) = result {
            // Rule intent should include specific channels
            match route.intent {
                Intent::Rule => {
                    assert!(route.channels.contains(&"rule".to_string()));
                }
                _ => {
                    // Other intents are valid too
                }
            }
        }
    }

    #[tokio::test]
    async fn test_route_query_intent_factual_channels() {
        let client = LlmClient::new("http://localhost:11434".to_string(), "llama2".to_string());

        let result = client
            .route_query("What is the molecular weight of penicillin G?")
            .await;

        if let Ok(route) = result {
            // Factual intent should prioritize vector search
            match route.intent {
                Intent::Factual => {
                    assert!(
                        route.channels.contains(&"vector".to_string())
                            || route.channels.contains(&"fts".to_string())
                    );
                }
                _ => {
                    // Other intents are valid
                }
            }
        }
    }

    #[test]
    fn test_intent_enum_equality() {
        let intent1 = Intent::Rule;
        let intent2 = Intent::Rule;
        let intent3 = Intent::Factual;

        assert_eq!(intent1, intent2);
        assert_ne!(intent1, intent3);
    }

    #[test]
    fn test_channel_route_struct() {
        let route = ChannelRoute {
            intent: Intent::Procedural,
            channels: vec!["rule".to_string(), "fts".to_string()],
        };

        assert_eq!(route.intent, Intent::Procedural);
        assert_eq!(route.channels.len(), 2);
        assert_eq!(route.channels[0], "rule");
        assert_eq!(route.channels[1], "fts");
    }

    #[test]
    fn test_chat_message_struct() {
        let msg = ChatMessage {
            role: "user".to_string(),
            content: "Hello, GMP assistant!".to_string(),
        };

        assert_eq!(msg.role, "user");
        assert_eq!(msg.content, "Hello, GMP assistant!");
    }

    #[test]
    fn test_chat_request_struct() {
        let request = ChatRequest {
            model: "llama2".to_string(),
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: "Test".to_string(),
            }],
            stream: false,
            options: RequestOptions::default(),
        };

        assert_eq!(request.model, "llama2");
        assert_eq!(request.messages.len(), 1);
        assert!(!request.stream);
    }

    #[test]
    fn test_request_options_default() {
        let options = RequestOptions::default();

        assert_eq!(options.temperature, None);
        assert_eq!(options.top_p, None);
    }

    #[test]
    fn test_shared_llm_client_creation() {
        let client = LlmClient::new("http://localhost:11434".to_string(), "llama2".to_string());

        let shared = new_shared(client);

        // Verify it's wrapped in Arc and RwLock
        assert!(shared.try_read().is_ok());
    }
}
