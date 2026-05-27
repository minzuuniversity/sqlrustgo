## ADDED Requirements

### Requirement: Ollama API integration
The system SHALL integrate with Ollama via HTTP API for LLM inference.

#### Scenario: Successful inference
- **WHEN** a chat request is sent to Ollama
- **THEN** the system SHALL receive a valid response with generated text and logprob

#### Scenario: Ollama service unavailable
- **WHEN** Ollama service is unreachable
- **THEN** the system SHALL return an OllamaConnectionError with retry guidance

#### Scenario: Model not found
- **WHEN** a requested model is not available in Ollama
- **THEN** the system SHALL return a ModelNotFoundError listing available models

### Requirement: Ollama client abstraction
The system SHALL provide an OllamaClient trait for testability and abstraction.

#### Scenario: Mock Ollama client
- **WHEN** a mock OllamaClient is used in tests
- **THEN** the system SHALL work without actual Ollama connection

### Requirement: Streaming response support
The system SHALL support SSE (Server-Sent Events) streaming from Ollama for progressive results.

#### Scenario: Streaming judgment
- **WHEN** streaming mode is enabled
- **THEN** the system SHALL receive progressive LLM tokens via SSE
