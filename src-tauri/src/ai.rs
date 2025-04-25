use crate::events::{Event, EventBus};
use crate::model::{Parameter, SharedState};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::env;
use std::error::Error;
use tokio::sync::broadcast;
use tokio::task::JoinHandle;
use tracing::{debug, info, warn};

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
    reasoning: Option<String>,
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
                        let prompt = self.build_audio_reasoning_prompt(
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

    /// Build a prompt focused on reasoning about audio parameters and their relationship to desired sound
    fn build_audio_reasoning_prompt(
        &self,
        project_name: &str,
        bank_name: &str,
        snap_name: &str,
        snap_description: &str,
        parameters: &[Parameter],
    ) -> String {
        let mut prompt = String::new();

        // Core audio engineering concepts
        prompt.push_str("# Audio Engineering Framework\n\n");

        prompt.push_str("You're setting MIDI CC values (0-127) for audio parameters. For each parameter, you must analyze:\n\n");

        prompt.push_str("1. FUNCTION: What does this parameter change in the sound?\n");
        prompt.push_str("2. DIRECTION: As values increase, does the effect increase or decrease?\n");
        prompt.push_str("3. GOAL: What sound qualities does the user want based on the snapshot description?\n");
        prompt.push_str("4. ALIGNMENT: How should this parameter be set to support those goals?\n\n");

        // Parameter categories and their general behavior
        prompt.push_str("## Parameter Category Framework\n\n");

        prompt.push_str("### FILTERS - Control which frequencies pass through\n");
        prompt.push_str("- High-pass filter (HPF): Removes bass frequencies. Higher values = less bass.\n");
        prompt.push_str("- Low-pass filter (LPF): Removes treble frequencies. Lower values = less treble.\n");
        prompt.push_str("- Band-pass filter (BPF): Isolates mid frequencies. Middle values typically best.\n\n");

        prompt.push_str("### DYNAMICS - Control volume relationships\n");
        prompt.push_str("- Compressor threshold: Lower values = more compression.\n");
        prompt.push_str("- Compressor ratio: Higher values = more aggressive compression.\n");
        prompt.push_str("- Gate threshold: Higher values = more gating (silence).\n\n");

        prompt.push_str("### EFFECTS - Add sonic characteristics\n");
        prompt.push_str("- Reverb/delay mix: Higher values = more wet sound.\n");
        prompt.push_str("- Distortion/saturation: Higher values = more distorted sound.\n");
        prompt.push_str("- Modulation (chorus/flanger/phaser): Higher values = more pronounced effect.\n\n");

        prompt.push_str("### EQ - Shape frequency response\n");
        prompt.push_str("- High frequency gain: Higher values = brighter sound.\n");
        prompt.push_str("- Mid frequency gain: Higher values = more present sound.\n");
        prompt.push_str("- Low frequency gain: Higher values = bassier sound.\n\n");

        // Few-shot examples showing reasoning process
        prompt.push_str("## Examples of Parameter Setting Reasoning\n\n");

        prompt.push_str("Example 1:\n");
        prompt.push_str("- Parameter: \"High-pass filter cutoff\"\n");
        prompt.push_str("- Description: \"Full bass techno drop\"\n");
        prompt.push_str("- Reasoning: This is a FILTER parameter that REMOVES bass as value increases. Since we want FULL BASS, we need to set this very LOW.\n");
        prompt.push_str("- Value: 0 (No bass removal)\n\n");

        prompt.push_str("Example 2:\n");
        prompt.push_str("- Parameter: \"Reverb amount\"\n");
        prompt.push_str("- Description: \"Dry, punchy drum section\"\n");
        prompt.push_str("- Reasoning: This is an EFFECT parameter that ADDS space/wetness as value increases. Since we want DRY sound, we need to set this LOW.\n");
        prompt.push_str("- Value: 10 (Minimal reverb)\n\n");

        prompt.push_str("Example 3:\n");
        prompt.push_str("- Parameter: \"Compressor threshold\"\n");
        prompt.push_str("- Description: \"Heavily squashed pad sound\"\n");
        prompt.push_str("- Reasoning: This is a DYNAMICS parameter where LOWER values cause MORE compression. Since we want HEAVILY SQUASHED sound, we need a LOW threshold.\n");
        prompt.push_str("- Value: 30 (Heavy compression)\n\n");

        prompt.push_str("Example 4:\n");
        prompt.push_str("- Parameter: \"Low EQ gain\"\n");
        prompt.push_str("- Description: \"Thin atmospheric pad, no sub frequencies\"\n");
        prompt.push_str("- Reasoning: This is an EQ parameter that ADDS bass as value increases. Since we want a THIN sound with NO SUB, we need this LOW.\n");
        prompt.push_str("- Value: 20 (Minimal bass boost)\n\n");

        // Snapshot context
        prompt.push_str("## Snapshot Context\n\n");
        prompt.push_str(&format!("Project: {}\n", project_name));
        prompt.push_str(&format!("Bank: {}\n", bank_name));
        prompt.push_str(&format!("Snapshot Name: {}\n", snap_name));

        if !snap_description.is_empty() {
            prompt.push_str(&format!("Snapshot Description: {}\n\n", snap_description));
        } else {
            prompt.push_str("Snapshot Description: Default balanced sound\n\n");
        }

        // Key sound goals extraction
        prompt.push_str("## Sound Goals Analysis\n\n");
        prompt.push_str("Based on the snapshot description, extract the key sound qualities desired:\n\n");

        // Ensure sound goals are always provided
        if snap_description.is_empty() {
            prompt.push_str("Using default balanced sound profile since no description provided.\n\n");
        }

        // Parameters to be set
        prompt.push_str("## Parameters to Configure\n\n");
        for param in parameters {
            prompt.push_str(&format!(
                "- Name: {}, CC: {}, Description: {}\n",
                param.name, param.cc, param.description
            ));
        }

        // Response format with reasoning
        prompt.push_str("\n## Response Format\n\n");
        prompt.push_str("For each parameter:\n");
        prompt.push_str("1. Identify its category and function\n");
        prompt.push_str("2. Analyze how it relates to the sound goals\n");
        prompt.push_str("3. Determine the appropriate value\n");
        prompt.push_str("4. Include your reasoning\n\n");

        prompt.push_str("Respond with a JSON object in this format:\n");
        prompt.push_str("{\n  \"values\": [\n    { \"cc\": <cc_number>, \"name\": \"<param_name>\", \"value\": <value_0_to_127>, \"reasoning\": \"Your step-by-step reasoning\" },\n    ...\n  ]\n}\n\n");

        prompt.push_str("Focus on the internal logic of the relationship between parameter function, sound goals, and appropriate values.\n");

        debug!("Audio reasoning prompt:\n{}", prompt);
        prompt
    }

    /// Generate values for all parameters using OpenAI
    async fn generate_values(
        &self,
        api_key: &str,
        prompt: &str,
        parameters: &[Parameter],
    ) -> Result<Vec<u8>, Box<dyn Error + Send + Sync>> {
        // Create the request
        let request = OpenAIRequest {
            model: "gpt-3.5-turbo".to_string(),
            messages: vec![
                OpenAIMessage {
                    role: "system".to_string(),
                    content: "You are an expert audio engineer and music producer. You understand how audio parameters affect sound and how to set them to achieve specific sonic goals. Think step-by-step and use your knowledge of audio engineering principles to set appropriate parameter values.".to_string(),
                },
                OpenAIMessage {
                    role: "user".to_string(),
                    content: prompt.to_string(),
                },
            ],
            temperature: 0.4, // Lower temperature for more consistent reasoning
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

                // Log the reasoning for each parameter setting
                for param_value in &ai_response.values {
                    if let Some(reasoning) = &param_value.reasoning {
                        info!("Parameter '{}' (CC {}) set to {}: {}", 
                              param_value.name, param_value.cc, param_value.value, reasoning);
                    }
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