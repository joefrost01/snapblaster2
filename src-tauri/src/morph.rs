use crate::events::{Event, EventBus, MorphCurve};
use crate::model::{SharedState, ActiveMorph};
use tokio::sync::broadcast;
use tokio::task::JoinHandle;
use tokio::time::{self, Duration};
use std::f64::consts::PI;

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
        tokio::spawn(async move {
            let mut morph_active = false;

            while let Ok(event) = self.event_receiver.recv().await {
                match event {
                    Event::MorphInitiated { from_snap, to_snap, duration_bars, curve_type } => {
                        // Start a new morph
                        self.start_morph(from_snap, to_snap, duration_bars, curve_type).await;
                        morph_active = true;
                    },
                    Event::BeatOccurred { beat: _, phase } => {
                        // Update morph progress based on beat phase if a morph is active
                        if morph_active {
                            morph_active = self.update_morph(phase).await;
                        }
                    },
                    Event::Shutdown => {
                        break;
                    },
                    _ => {}
                }
            }
        })
    }

    /// Start a new morph between two snaps
    async fn start_morph(&self, from_snap: usize, to_snap: usize, duration_bars: u8, curve_type: MorphCurve) {
        // Get the values for both snaps
        let (from_values, to_values) = {
            let state_guard = self.state.read().unwrap();
            let bank = &state_guard.project.banks[state_guard.current_bank];
            let from = &bank.snaps[from_snap];
            let to = &bank.snaps[to_snap];

            (from.values.clone(), to.values.clone())
        };

        // Create a new active morph
        let active_morph = ActiveMorph {
            from_snap,
            to_snap,
            duration_bars,
            progress: 0.0,
            from_values,
            to_values,
            current_values: Vec::new(), // Will be set in update_morph
        };

        // Update the state
        {
            let mut state_guard = self.state.write().unwrap();
            state_guard.active_morph = Some(active_morph);
        }

        // If we're not using Link, start a timer-based morph
        // For simplicity, we'll use a 60 BPM default tempo if not synced
        if !self.is_link_connected() {
            let state = self.state.clone();
            let event_bus = self.event_bus.clone();

            tokio::spawn(async move {
                // Calculate total duration based on 60 BPM
                let beats_per_second = 60.0 / 60.0; // 60 BPM = 1 beat per second
                let bars = duration_bars as f64;
                let beats_per_bar = 4.0; // Assuming 4/4 time
                let total_duration_ms = (bars * beats_per_bar / beats_per_second) * 1000.0;
                let steps = 30; // Update 30 times per second
                let step_duration_ms = total_duration_ms / steps as f64;

                for i in 0..=steps {
                    let progress = i as f64 / steps as f64;

                    // Update the morph
                    {
                        let mut state_guard = state.write().unwrap();
                        if let Some(morph) = &mut state_guard.active_morph {
                            morph.progress = progress;

                            // Calculate current values based on curve
                            morph.current_values = Self::interpolate_values(
                                &morph.from_values,
                                &morph.to_values,
                                progress,
                                curve_type.clone(),
                            );
                        }
                    }

                    // Publish progress event
                    let current_values = {
                        let state_guard = state.read().unwrap();
                        state_guard.active_morph.as_ref().map(|m| m.current_values.clone()).unwrap_or_default()
                    };

                    let _ = event_bus.publish(Event::MorphProgressed {
                        progress,
                        current_values,
                    });

                    // If we're at the end, complete the morph
                    if i == steps {
                        // Clear the active morph
                        {
                            let mut state_guard = state.write().unwrap();
                            state_guard.active_morph = None;
                        }

                        // Publish completion event
                        let _ = event_bus.publish(Event::MorphCompleted);
                    }

                    // Sleep until next step
                    time::sleep(Duration::from_millis(step_duration_ms as u64)).await;
                }
            });
        }
    }

    /// Update morph progress based on Ableton Link beat phase
    async fn update_morph(&self, phase: f64) -> bool {
        let (progress, current_values, curve_type) = {
            let state_guard = self.state.read().unwrap();

            if let Some(morph) = &state_guard.active_morph {
                // Calculate progress based on phase and duration
                let beats_per_bar = 4.0; // Assuming 4/4 time
                let duration_beats = morph.duration_bars as f64 * beats_per_bar;
                let progress = (phase % duration_beats) / duration_beats;

                // Check if morph is complete
                if phase >= duration_beats {
                    return false;
                }

                // Calculate current values
                let current_values = Self::interpolate_values(
                    &morph.from_values,
                    &morph.to_values,
                    progress,
                    MorphCurve::Linear, // Replace with the actual curve from the morph
                );

                // Update the morph state
                let curve_type = match state_guard.active_morph.as_ref() {
                    Some(morph) => MorphCurve::Linear, // Replace with actual curve type
                    None => MorphCurve::Linear,
                };

                (progress, current_values, curve_type)
            } else {
                return false;
            }
        };

        // Update the morph progress
        {
            let mut state_guard = self.state.write().unwrap();
            if let Some(morph) = &mut state_guard.active_morph {
                morph.progress = progress;
                morph.current_values = current_values.clone();
            }
        }

        // Publish progress event
        let _ = self.event_bus.publish(Event::MorphProgressed {
            progress,
            current_values,
        });

        true
    }

    /// Check if Ableton Link is connected
    fn is_link_connected(&self) -> bool {
        // Placeholder - would check Link state
        false
    }

    /// Interpolate between two sets of values based on a curve
    fn interpolate_values(from: &[u8], to: &[u8], progress: f64, curve_type: MorphCurve) -> Vec<u8> {
        let mut result = Vec::with_capacity(from.len());

        for i in 0..from.len() {
            let from_val = *from.get(i).unwrap_or(&0) as f64;
            let to_val = *to.get(i).unwrap_or(&0) as f64;

            // Apply the curve to the progress value
            let curved_progress = match curve_type {
                MorphCurve::Linear => progress,
                #[cfg(feature = "pro")]
                MorphCurve::Exponential => progress * progress,
                #[cfg(feature = "pro")]
                MorphCurve::Logarithmic => progress.sqrt(),
                #[cfg(feature = "pro")]
                MorphCurve::SCurve => 0.5 * (1.0 - (PI * progress).cos()),
            };

            // Interpolate
            let value = from_val + (to_val - from_val) * curved_progress;

            // Clamp and convert back to u8
            let clamped = value.max(0.0).min(127.0) as u8;
            result.push(clamped);
        }

        result
    }
}