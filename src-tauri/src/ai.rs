use crate::events::{Event, EventBus};
use crate::model::SharedState;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::env;
use std::error::Error;
use tokio::sync::broadcast;
use tokio::task::JoinHandle;
use tracing::info;

/// AI Service for generating parameter values with OpenAI
pub struct AIService {
    state: SharedState,
    event_bus: EventBus,
    client: Client,
    event_receiver: broadcast::Receiver<Event>,
}

#[derive(Serialize)]
struct OpenAIRequest {
    model: String,
    messages: Vec<OpenAIMessage>,
    temperature: f32,
}

#[derive(Serialize)]
struct OpenAIMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct OpenAIResponse {
    choices: Vec<OpenAIChoice>,
}

#[derive(Deserialize)]
struct OpenAIChoice {
    message: OpenAIResponseMessage,
}

#[derive(Deserialize)]
struct OpenAIResponseMessage {
    content: String,
}

#[derive(Deserialize)]
struct ParameterValue {
    cc: u8,
    value: u8,
    name: String,
}

#[derive(Deserialize)]
struct AIResponse {
    values: Vec<ParameterValue>,
}

impl AIService {
    /// Create a new AI service
    pub fn new(state: SharedState, event_bus: EventBus) -> Self {
        let event_receiver = event_bus.subscribe();
        let client = Client::new();

        Self {
            state,
            event_bus,
            client,
            event_receiver,
        }
    }

    /// Start the AI service
    pub fn start(mut self) -> JoinHandle<()> {
        tokio::spawn(async move {
            while let Ok(event) = self.event_receiver.recv().await {
                if let Event::GenerateAIValues { bank_id, snap_id } = event {
                    // Try to get API key from environment variable first
                    let api_key = match env::var("OPENAI_API_KEY") {
                        Ok(key) => Some(key),
                        Err(_) => {
                            // Fall back to stored key in project if env var not found
                            let state_guard = self.state.read().unwrap();
                            state_guard.project.openai_api_key.clone()
                        }
                    };

                    // Check if we have an API key
                    if let Some(api_key) = api_key {
                        // Get the prompt information
                        let (project_name, bank_name, snap_name, snap_description, parameters) = {
                            let state_guard = self.state.read().unwrap();
                            let project = &state_guard.project;
                            let bank = &project.banks[bank_id];
                            let snap = &bank.snaps[snap_id];

                            (
                                project.project_name.clone(),
                                bank.name.clone(),
                                snap.name.clone(),
                                snap.description.clone(),
                                project.parameters.clone(),
                            )
                        };

                        // Build a comprehensive prompt with all parameters and context
                        let prompt = self.build_comprehensive_prompt(
                            &project_name,
                            &bank_name,
                            &snap_name,
                            &snap_description,
                            &parameters,
                        );

                        // Call the OpenAI API
                        match self.generate_values(&api_key, &prompt, &parameters).await {
                            Ok(values) => {
                                // Update the snap with the generated values
                                {
                                    let mut state_guard = self.state.write().unwrap();
                                    let bank = &mut state_guard.project.banks[bank_id];
                                    let snap = &mut bank.snaps[snap_id];

                                    // Ensure the values array is large enough
                                    if snap.values.len() < values.len() {
                                        snap.values.resize(values.len(), 64);
                                    }

                                    // Overwrite the existing values with the generated ones
                                    for (i, value) in values.iter().enumerate() {
                                        snap.values[i] = *value;
                                    }
                                }

                                // Send the event that generation completed
                                let _ = self.event_bus.publish(Event::AIGenerationCompleted {
                                    bank_id,
                                    snap_id,
                                    values,
                                });
                            }
                            Err(e) => {
                                // Send failure event
                                let _ = self.event_bus.publish(Event::AIGenerationFailed {
                                    bank_id,
                                    snap_id,
                                    error: e.to_string(),
                                });
                            }
                        }
                    } else {
                        // No API key, send failure event
                        let _ = self.event_bus.publish(Event::AIGenerationFailed {
                            bank_id,
                            snap_id,
                            error: "No OpenAI API key provided. Set OPENAI_API_KEY environment variable or in application settings.".to_string(),
                        });
                    }
                }
            }
        })
    }

    /// Build a comprehensive prompt that includes project, bank, snap, and parameter details
    fn build_comprehensive_prompt(
        &self,
        project_name: &str,
        bank_name: &str,
        snap_name: &str,
        snap_description: &str,
        parameters: &[crate::model::Parameter],
    ) -> String {
        let mut prompt = format!(
            "I need MIDI CC values (0-127) for a snapshot in a MIDI CC controller setup. Please analyze the context and generate appropriate values.\n\n"
        );

        // Add project, bank and snap context
        prompt.push_str(&format!("Project: {}\n", project_name));
        prompt.push_str(&format!("Bank: {}\n", bank_name));
        prompt.push_str(&format!("Snapshot Name: {}\n", snap_name));

        if !snap_description.is_empty() {
            prompt.push_str(&format!("Snapshot Description: {}\n\n", snap_description));
        } else {
            prompt.push_str("\n");
        }

        // Add parameter details
        prompt.push_str("Parameters to set values for:\n");
        for param in parameters {
            prompt.push_str(&format!(
                "- Name: {}, CC: {}, Description: {}\n",
                param.name, param.cc, param.description
            ));
        }

        // Instructions for the output format
        prompt.push_str("\nPlease respond with a JSON object containing an array of parameter values in the following format:\n");
        prompt.push_str("{\n  \"values\": [\n    { \"cc\": <cc_number>, \"name\": \"<param_name>\", \"value\": <value_0_to_127> },\n    ...\n  ]\n}\n\n");

        // Additional context about the values
        prompt.push_str("Consider these guidelines for setting values:\n");
        prompt.push_str("- Values range from 0 (minimum) to 127 (maximum)\n");
        prompt.push_str("- Interpret the snapshot name and description to determine appropriate values\n");
        prompt.push_str("- Use parameter descriptions to inform your decisions\n");
        prompt.push_str("- Ensure consistency across related parameters\n");
        info!(prompt);
        prompt
    }

    /// Generate values for all parameters using OpenAI
    async fn generate_values(
        &self,
        api_key: &str,
        prompt: &str,
        parameters: &[crate::model::Parameter],
    ) -> Result<Vec<u8>, Box<dyn Error + Send + Sync>> {
        // Create the request
        let request = OpenAIRequest {
            model: "gpt-3.5-turbo".to_string(),
            messages: vec![
                OpenAIMessage {
                    role: "system".to_string(),
                    content: "You are a MIDI parameter value generator. You will receive context about a music project, snapshot, and parameters. Respond with a JSON structure containing appropriate MIDI CC values (0-127) for each parameter.".to_string(),
                },
                OpenAIMessage {
                    role: "user".to_string(),
                    content: prompt.to_string(),
                },
            ],
            temperature: 0.7,
        };

        // Send the request
        let response = self
            .client
            .post("https://api.openai.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        // Check for non-success status
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await?;
            return Err(format!("OpenAI API error: {} - {}", status, error_text).into());
        }

        // Parse the response
        let response_data: OpenAIResponse = response.json().await?;

        // Extract the content
        let content = &response_data.choices[0].message.content;

        // Try to extract JSON from the response (handle potential markdown code blocks)
        let json_content = extract_json_from_response(content);

        // Parse the JSON response
        match serde_json::from_str::<AIResponse>(&json_content) {
            Ok(ai_response) => {
                // Create a map of CC numbers to values
                let mut cc_value_map = std::collections::HashMap::new();
                for param_value in ai_response.values {
                    cc_value_map.insert(param_value.cc, param_value.value);
                }

                // Generate values in correct order for all parameters
                let values: Vec<u8> = parameters.iter().map(|param| {
                    // Use the AI-generated value if available, otherwise default to 64
                    *cc_value_map.get(&param.cc).unwrap_or(&64)
                }).collect();

                Ok(values)
            },
            Err(e) => {
                // If parsing fails, provide a detailed error
                Err(format!("Failed to parse AI response: {} - Content: {}", e, json_content).into())
            }
        }
    }
}

/// Helper function to extract JSON from a potentially markdown-formatted response
fn extract_json_from_response(content: &str) -> String {
    // Look for ```json ... ``` pattern
    if let Some(start) = content.find("```json") {
        if let Some(end) = content.rfind("```") {
            let json_start = content[start..].find('\n').map(|pos| start + pos + 1).unwrap_or(start + 7);
            return content[json_start..end].trim().to_string();
        }
    }

    // Look for ```... ``` pattern (without explicit json)
    if let Some(start) = content.find("```") {
        if let Some(end) = content.rfind("```") {
            let json_start = content[start..].find('\n').map(|pos| start + pos + 1).unwrap_or(start + 3);
            return content[json_start..end].trim().to_string();
        }
    }

    // Just return the entire content if no code blocks found
    content.trim().to_string()
}