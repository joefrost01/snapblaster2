// src-tauri/src/morph.rs
use crate::events::{Event, EventBus, MorphCurve};
use crate::model::{ActiveMorph, Parameter, SharedState};
use std::f64::consts::PI;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, Mutex as TokioMutex};
use tokio::task::JoinHandle;
use tokio::time::{self, Instant};
use tracing::{debug, error, info, warn};

/// MorphEngine handles interpolation between snaps
pub struct MorphEngine {
    state: SharedState,
    event_bus: EventBus,
    event_receiver: broadcast::Receiver<Event>,
}

impl MorphEngine {
    /// Create a new morph engine
    pub fn new(state: SharedState, event_bus: EventBus) -> Self {
        let event_receiver = event_bus.subscribe();

        Self {
            state,
            event_bus,
            event_receiver,
        }
    }

    /// Start the morph engine
    pub fn start(mut self) -> JoinHandle<()> {
        info!("Starting morph engine");

        tokio::spawn(async move {
            // Track if we are currently morphing
            let mut morph_task: Option<JoinHandle<()>> = None;

            // Event loop
            while let Ok(event) = self.event_receiver.recv().await {
                match event {
                    Event::PadPressed { pad, velocity } => {
                        // Handle note-off events for modifiers (pads 0-4)
                        if pad < 5 && velocity == 0 {
                            // Note-off for a modifier pad
                            let mut state_guard = self.state.write().unwrap();
                            if state_guard.active_modifier == Some(pad) {
                                // Clear the active modifier
                                state_guard.active_modifier = None;

                                // Let the MIDI manager know to update LEDs
                                drop(state_guard);
                                // Use try_publish to avoid error handling
                                let _ = self.event_bus.try_publish(Event::RequestUpdateLEDs);
                            }
                        }
                    },
                    Event::MorphInitiated {
                        from_snap,
                        to_snap,
                        duration_bars,
                        curve_type,
                        quantize,
                    } => {
                        info!("Starting morph: {} -> {}, duration: {}bars, quantize: {}", 
                              from_snap, to_snap, duration_bars, quantize);

                        // Cancel any existing morph task
                        if let Some(task) = morph_task.take() {
                            task.abort();
                            info!("Cancelled previous morph task");
                        }

                        // Check if we have valid snap indices
                        let state_guard = self.state.read().unwrap();
                        let current_bank = &state_guard.project.banks[state_guard.current_bank];
                        if from_snap >= current_bank.snaps.len() || to_snap >= current_bank.snaps.len() {
                            error!("Invalid snap indices for morph: {} -> {}", from_snap, to_snap);
                            continue;
                        }

                        // Clone necessary data for the morph task
                        let state = self.state.clone();
                        let event_bus = self.event_bus.clone();
                        let bank_id = state_guard.current_bank;

                        // Start a new morph task - pass the quantize flag directly
                        morph_task = Some(tokio::spawn(async move {
                            Self::run_morph(
                                state, event_bus, bank_id,
                                from_snap, to_snap, duration_bars, curve_type, quantize
                            ).await;
                        }));
                    },

                    // Handle RequestUpdateLEDs event
                    Event::RequestUpdateLEDs => {
                        // Forward to the MIDI manager to update LEDs
                        self.event_bus.try_publish(Event::RequestMIDIUpdate);
                        debug!("Requested MIDI LED update");
                    },

                    Event::Shutdown => {
                        info!("Shutting down morph engine");

                        // Cancel any active morph
                        if let Some(task) = morph_task.take() {
                            task.abort();
                        }

                        break;
                    },
                    _ => {
                        // Ignore other events
                    }
                }
            }

            info!("Morph engine shutdown complete");
        })
    }

    /// Run a morph from one snap to another
    async fn run_morph(
        state: SharedState,
        event_bus: EventBus,
        bank_id: usize,
        from_snap: usize,
        to_snap: usize,
        duration_bars: u8,
        curve_type: MorphCurve,
        quantize: bool,
    ) {
        // Get the values for both snaps and parameters
        let (from_values, to_values, parameters, param_count) = {
            let state_guard = state.read().unwrap();
            let bank = &state_guard.project.banks[bank_id];

            // Safely get the snap values
            let from = if from_snap < bank.snaps.len() {
                &bank.snaps[from_snap]
            } else {
                error!("Invalid from_snap index: {}", from_snap);
                return;
            };

            let to = if to_snap < bank.snaps.len() {
                &bank.snaps[to_snap]
            } else {
                error!("Invalid to_snap index: {}", to_snap);
                return;
            };

            let param_count = state_guard.project.parameters.len();
            let parameters = state_guard.project.parameters.clone();

            // Clone the values to avoid holding the lock for too long
            (from.values.clone(), to.values.clone(), parameters, param_count)
        };

        // Create a new active morph
        let active_morph = ActiveMorph {
            from_snap,
            to_snap,
            duration_bars,
            progress: 0.0,
            from_values: from_values.clone(),
            to_values: to_values.clone(),
            current_values: from_values.clone(), // Start with source values
        };

        // Update the state
        {
            let mut state_guard = state.write().unwrap();
            state_guard.active_morph = Some(active_morph);
        }

        // If quantization is requested, get the link state and potentially wait
        if quantize {
            // First check if Link is connected to any peers by requesting status
            let mut status_receiver = event_bus.subscribe();
            let _ = event_bus.publish(Event::RequestLinkStatus);

            let mut should_quantize = false;
            let mut wait_ms = 0u64;

            // Wait up to 200ms for a response
            let timeout = time::Duration::from_millis(200);
            if let Ok(Ok(event)) = time::timeout(timeout, status_receiver.recv()).await {
                if let Event::LinkStatusChanged { connected, .. } = event {
                    should_quantize = connected;

                    // If connected, request the next bar time
                    if connected {
                        let _ = event_bus.publish(Event::RequestNextBarTime);

                        // Wait for response with timeout
                        if let Ok(Ok(time_event)) = time::timeout(timeout, status_receiver.recv()).await {
                            if let Event::NextBarTime { wait_time_ms } = time_event {
                                wait_ms = wait_time_ms;
                            }
                        }
                    }
                }
            }

            // If we should quantize and have a wait time, sleep until the next bar
            if should_quantize && wait_ms > 0 {
                info!("Quantizing morph to next bar boundary in {} ms", wait_ms);
                tokio::time::sleep(Duration::from_millis(wait_ms)).await;
            }
        }

        // Get the current tempo (default or from Link)
        let mut tempo_receiver = event_bus.subscribe();
        let _ = event_bus.publish(Event::RequestLinkTempo);

        // Default tempo
        let mut bpm = 120.0;

        // Wait up to 200ms for a tempo response
        let timeout = time::Duration::from_millis(200);
        if let Ok(Ok(event)) = time::timeout(timeout, tempo_receiver.recv()).await {
            if let Event::LinkTempoChanged { tempo } = event {
                bpm = tempo;
            }
        }

        let beats_per_second = bpm / 60.0;
        let bars = duration_bars as f64;
        let beats_per_bar = 4.0; // Assuming 4/4 time
        let total_duration_secs = (bars * beats_per_bar) / beats_per_second;

        // Use more updates for longer morphs
        let updates_per_second = 30.0; // 30 fps is smooth enough
        let total_updates = (total_duration_secs * updates_per_second) as u32;
        let update_interval = Duration::from_millis((1000.0 / updates_per_second) as u64);

        info!("Starting morph with duration: {}s, total updates: {}", total_duration_secs, total_updates);

        // Track morph start time
        let start_time = Instant::now();
        let total_duration = Duration::from_secs_f64(total_duration_secs);

        // Create an interval for regular updates
        let mut interval = time::interval(update_interval);
        interval.set_missed_tick_behavior(time::MissedTickBehavior::Skip);

        // Last sent values for each parameter - avoid sending duplicates
        let mut last_sent_values: Vec<Option<u8>> = vec![None; param_count];

        loop {
            interval.tick().await;

            // Calculate progress
            let elapsed = start_time.elapsed();
            if elapsed >= total_duration {
                // Morph complete - send final values
                send_morph_cc_values(&event_bus, &parameters, &to_values, &mut last_sent_values).await;

                // Morph complete
                Self::complete_morph(&state, &event_bus, &to_values).await;
                break;
            }

            let progress = elapsed.as_secs_f64() / total_duration_secs;

            // Apply curve to the progress
            let curved_progress = Self::apply_curve(progress, &curve_type);

            // Calculate and update current values
            let current_values = Self::interpolate_values(
                &from_values,
                &to_values,
                curved_progress,
                param_count
            );

            // Update the morph state
            {
                let mut state_guard = state.write().unwrap();
                if let Some(morph) = &mut state_guard.active_morph {
                    morph.progress = progress;
                    morph.current_values = current_values.clone();
                }
            }

            // Send current CC values to MIDI output
            send_morph_cc_values(&event_bus, &parameters, &current_values, &mut last_sent_values).await;

            // Publish progress event
            let _ = event_bus.publish(Event::MorphProgressed {
                progress,
                current_values,
            });
        }
    }

    /// Apply a curve function to the progress value
    fn apply_curve(progress: f64, curve_type: &MorphCurve) -> f64 {
        match curve_type {
            MorphCurve::Linear => progress,
            #[cfg(feature = "pro")]
            MorphCurve::Exponential => progress * progress,
            #[cfg(feature = "pro")]
            MorphCurve::Logarithmic => progress.sqrt(),
            #[cfg(feature = "pro")]
            MorphCurve::SCurve => 0.5 * (1.0 - (PI * progress).cos()),
        }
    }

    /// Complete a morph and finalize to the target values
    async fn complete_morph(state: &SharedState, event_bus: &EventBus, final_values: &[u8]) {
        // First, extract what we need from the active morph
        let (to_snap, current_bank) = {
            let state_guard = state.read().unwrap();
            match &state_guard.active_morph {
                Some(morph) => (morph.to_snap, state_guard.current_bank),
                None => {
                    warn!("No active morph to complete");
                    return;
                }
            }
        };

        // Now update everything with the extracted values
        {
            let mut state_guard = state.write().unwrap();
            // Update the current snap
            state_guard.current_snap = to_snap;

            // Update the snap's values to ensure they match exactly
            if let Some(bank) = state_guard.project.banks.get_mut(current_bank) {
                if let Some(snap) = bank.snaps.get_mut(to_snap) {
                    snap.values = final_values.to_vec();
                }
            }

            // Clear the active morph
            state_guard.active_morph = None;
        }

        // Send the final values
        let _ = event_bus.publish(Event::MorphProgressed {
            progress: 1.0,
            current_values: final_values.to_vec(),
        });

        // Send completion event
        let _ = event_bus.publish(Event::MorphCompleted);

        info!("Morph completed");
    }

    /// Interpolate between two sets of values based on a progress value
    fn interpolate_values(
        from: &[u8],
        to: &[u8],
        progress: f64,
        param_count: usize,
    ) -> Vec<u8> {
        let mut result = Vec::with_capacity(param_count);

        for i in 0..param_count {
            let from_val = *from.get(i).unwrap_or(&0) as f64;
            let to_val = *to.get(i).unwrap_or(&0) as f64;

            // Interpolate
            let value = from_val + (to_val - from_val) * progress;

            // Clamp and convert back to u8
            let clamped = value.round().max(0.0).min(127.0) as u8;
            result.push(clamped);
        }

        result
    }
}


/// Helper function to send CC values during morphing
async fn send_morph_cc_values(
    event_bus: &EventBus,
    parameters: &[Parameter],
    values: &[u8],
    last_sent: &mut [Option<u8>]
) {
    // For each parameter that has a value
    for (idx, param) in parameters.iter().enumerate() {
        if idx < values.len() {
            let value = values[idx];

            // Only send if value has changed since last update
            if last_sent[idx] != Some(value) {
                // Save this value to avoid redundant sends
                last_sent[idx] = Some(value);

                // Send CC value changed event
                let _ = event_bus.publish(Event::CCValueChanged {
                    param_id: idx,
                    value,
                });
            }
        }
    }
}