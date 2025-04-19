use crate::events::{Event, EventBus};
use crate::model::SharedState;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::error::Error;
use tokio::sync::broadcast;
use tokio::task::JoinHandle;

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
                    let api_key = {
                        let state_guard = self.state.read().unwrap();
                        state_guard.project.openai_api_key.clone()
                    };

                    // Check if we have an API key
                    if let Some(api_key) = api_key {
                        // Get the prompt information
                        let (project_name, bank_name, snap_name, parameters) = {
                            let state_guard = self.state.read().unwrap();
                            let project = &state_guard.project;
                            let bank = &project.banks[bank_id];
                            let snap = &bank.snaps[snap_id];

                            (
                                project.project_name.clone(),
                                bank.name.clone(),
                                snap.name.clone(),
                                project.parameters.clone(),
                            )
                        };

                        // Generate values for each parameter
                        let mut generated_values = Vec::new();

                        for param in &parameters {
                            // Build the prompt for this parameter
                            let prompt = format!(
                                "Project: {}\nScene: {}\nBank: {}\nParameter: {} - {}\nCC: {}\n\nReturn the appropriate CC value (0-127) for this parameter in this context. Reply with a single number only.",
                                project_name, snap_name, bank_name, param.name, param.description, param.cc
                            );

                            // Call the OpenAI API
                            match self.generate_value(&api_key, &prompt).await {
                                Ok(value) => generated_values.push(value),
                                Err(_) => generated_values.push(64), // Default to middle value on error
                            }
                        }

                        // Update the snap with the generated values
                        {
                            let mut state_guard = self.state.write().unwrap();
                            let bank = &mut state_guard.project.banks[bank_id];
                            let snap = &mut bank.snaps[snap_id];
                            snap.values = generated_values.clone();
                        }

                        // Send the event that generation completed
                        let _ = self.event_bus.publish(Event::AIGenerationCompleted {
                            bank_id,
                            snap_id,
                            values: generated_values,
                        });
                    } else {
                        // No API key, send failure event
                        let _ = self.event_bus.publish(Event::AIGenerationFailed {
                            bank_id,
                            snap_id,
                            error: "No OpenAI API key provided".to_string(),
                        });
                    }
                }
            }
        })
    }

    /// Generate a single parameter value using OpenAI
    async fn generate_value(
        &self,
        api_key: &str,
        prompt: &str,
    ) -> Result<u8, Box<dyn Error + Send + Sync>> {
        // Create the request
        let request = OpenAIRequest {
            model: "gpt-3.5-turbo".to_string(),
            messages: vec![
                OpenAIMessage {
                    role: "system".to_string(),
                    content: "You are a MIDI parameter value generator. You output only a single number between 0 and 127 based on the description.".to_string(),
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

        // Parse the response
        let response_data: OpenAIResponse = response.json().await?;

        // Extract the value
        let content = &response_data.choices[0].message.content;
        let value = content.trim().parse::<u8>()?;

        // Ensure the value is in range
        let clamped_value = value.min(127);

        Ok(clamped_value)
    }
}
