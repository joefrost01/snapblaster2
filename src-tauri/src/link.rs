// src-tauri/src/link.rs
use crate::events::{Event, EventBus};
use rusty_link::{AblLink, SessionState};
use std::convert::TryInto;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tokio::time;
use tracing::{debug, info, warn};

/// Link integration for tempo synchronization with Ableton Link
#[derive(Clone)]
pub struct LinkSynchronizer {
    link: Arc<Mutex<AblLink>>,
    event_bus: EventBus,
    running: Arc<Mutex<bool>>,
    quantum: Arc<Mutex<f64>>,
    last_beat: Arc<Mutex<u32>>,
    last_bar: Arc<Mutex<u32>>,
}

impl LinkSynchronizer {
    /// Create a new Link synchronizer
    pub fn new(event_bus: EventBus) -> Self {
        // Create Link instance with default tempo of 120 BPM
        let link = AblLink::new(120.0);

        // Enable Link by default but don't start transport
        link.enable(true);

        info!(
            "Ableton Link initialized: enabled={}, default tempo=120.0",
            link.is_enabled()
        );

        Self {
            link: Arc::new(Mutex::new(link)),
            event_bus,
            running: Arc::new(Mutex::new(true)),
            quantum: Arc::new(Mutex::new(4.0)),  // Default to 4 beats per bar
            last_beat: Arc::new(Mutex::new(0)),
            last_bar: Arc::new(Mutex::new(0)),
        }
    }

    /// Start the Link synchronizer
    pub fn start(&self) -> JoinHandle<()> {
        let link = self.link.clone();
        let event_bus = self.event_bus.clone();
        let running = self.running.clone();
        let quantum = self.quantum.clone();
        let last_beat = self.last_beat.clone();
        let last_bar = self.last_bar.clone();

        // Clone for event handler
        let event_link = link.clone();
        let event_quantum = quantum.clone();
        let event_bus_clone = event_bus.clone();

        // Listen for specific events related to Link
        let mut event_receiver = event_bus.subscribe();
        tokio::spawn(async move {
            while let Ok(event) = event_receiver.recv().await {
                match event {
                    Event::RequestLinkStatus => {
                        // Get current Link status
                        let peers = {
                            let link_guard = event_link.lock().await;
                            let num_peers = link_guard.num_peers();
                            (link_guard.is_enabled(), num_peers.try_into().unwrap_or(0))
                        };

                        // Send status back
                        let _ = event_bus_clone.publish(Event::LinkStatusChanged {
                            connected: peers.0 && peers.1 > 0,
                            peers: peers.1
                        });
                    },
                    Event::RequestLinkTempo => {
                        // Get current tempo
                        let tempo = {
                            let link_guard = event_link.lock().await;
                            let mut session_state = SessionState::new();
                            link_guard.capture_app_session_state(&mut session_state);
                            session_state.tempo()
                        };

                        // Send tempo back
                        let _ = event_bus_clone.publish(Event::LinkTempoChanged {
                            tempo
                        });
                    },
                    Event::RequestNextBarTime => {
                        // Calculate time to next bar boundary
                        let wait_time = {
                            let link_guard = event_link.lock().await;
                            let mut session_state = SessionState::new();
                            link_guard.capture_audio_session_state(&mut session_state);

                            let quantum = *event_quantum.lock().await;
                            let micros = link_guard.clock_micros();
                            let beat_position = session_state.beat_at_time(micros, quantum);

                            // Calculate beats until next bar
                            let beat_in_bar = beat_position % quantum;
                            let beats_to_next_bar = if beat_in_bar < 0.01 { 0.0 } else { quantum - beat_in_bar };

                            // Calculate time at next bar in microseconds
                            let next_bar_beat = beat_position + beats_to_next_bar;
                            let next_bar_micros = session_state.time_at_beat(next_bar_beat, quantum);

                            // Calculate wait time in milliseconds
                            if next_bar_micros > micros {
                                (next_bar_micros - micros) / 1000
                            } else {
                                0
                            }
                        };

                        // Convert to u64 and send time back
                        let wait_time_ms: u64 = wait_time.try_into().unwrap_or(0);
                        let _ = event_bus_clone.publish(Event::NextBarTime {
                            wait_time_ms
                        });
                    },
                    _ => {}
                }
            }
        });

        // Start the main Link loop
        tokio::spawn(async move {
            info!("Starting Link synchronizer");

            // Announce initial num peers
            let peers = {
                let link_guard = link.lock().await;
                let num_peers = link_guard.num_peers();
                // Convert to usize
                num_peers.try_into().unwrap_or(0)
            };

            // Send initial connection status
            let _ = event_bus.publish(Event::LinkStatusChanged {
                connected: peers > 0,
                peers
            });

            // Track the last tempo to detect changes
            let mut last_known_tempo = 120.0;

            // Main polling loop for Link
            let mut interval = time::interval(Duration::from_millis(16)); // ~60fps

            // Track the last time we checked for peers
            let mut last_peer_check = Instant::now();
            let peer_check_interval = Duration::from_secs(1);

            while *running.lock().await {
                interval.tick().await;

                // Get Link state
                let mut session_state = SessionState::new();
                let (time_micros, beats_per_bar) = {
                    let mut link_guard = link.lock().await;
                    link_guard.capture_audio_session_state(&mut session_state);

                    // Get host time in microseconds and quantum
                    let micros = link_guard.clock_micros();
                    let quantum = *quantum.lock().await;

                    (micros, quantum)
                };

                // Check if tempo has changed
                let current_tempo = session_state.tempo();
                if (current_tempo - last_known_tempo).abs() > 0.01 {
                    // We detected a tempo change
                    info!("Link tempo changed: {:.1} BPM (was {:.1})", current_tempo, last_known_tempo);
                    last_known_tempo = current_tempo;

                    // Notify the rest of the application
                    let _ = event_bus.publish(Event::LinkTempoChanged {
                        tempo: current_tempo
                    });
                }

                // Get current beat/phase position
                let beat_position = session_state.beat_at_time(time_micros, beats_per_bar);
                let phase = beat_position % beats_per_bar;
                let beat = beat_position.floor() as u32;

                // Calculate bar position
                let bar = (beat_position / beats_per_bar).floor() as u32;

                // Check if we're on a new beat
                let mut last_beat_val = last_beat.lock().await;
                if beat != *last_beat_val {
                    *last_beat_val = beat;

                    // Send beat event
                    debug!("Beat: {}, phase: {:.2}, tempo: {:.1}", beat, phase, session_state.tempo());
                    let _ = event_bus.publish(Event::BeatOccurred {
                        beat,
                        phase,
                    });

                    // Check if we're also on a new bar
                    let mut last_bar_val = last_bar.lock().await;
                    if bar != *last_bar_val {
                        *last_bar_val = bar;

                        // Send bar event
                        debug!("Bar: {}", bar);
                        let _ = event_bus.publish(Event::BarOccurred { bar });
                    }
                }

                // Check for peer count changes periodically
                if last_peer_check.elapsed() >= peer_check_interval {
                    let peers = {
                        let link_guard = link.lock().await;
                        let num_peers = link_guard.num_peers();
                        let converted_peers = num_peers.try_into().unwrap_or(0);

                        // Debug logging for peer detection
                        info!("Link peers: {} (raw: {})", converted_peers, num_peers);

                        // Return the converted value
                        converted_peers
                    };

                    // Send connection status update
                    let _ = event_bus.publish(Event::LinkStatusChanged {
                        connected: peers > 0,
                        peers
                    });

                    last_peer_check = Instant::now();
                }
            }

            info!("Link synchronizer stopped");
        })
    }

    /// Set the tempo in BPM
    pub async fn set_tempo(&self, bpm: f64) {
        let mut link_guard = self.link.lock().await;

        // Create a session state and apply the tempo change
        let mut session_state = SessionState::new();
        link_guard.capture_app_session_state(&mut session_state);

        // The method expects an i64, so we use the micros directly
        let micros = link_guard.clock_micros();
        session_state.set_tempo(bpm, micros);
        link_guard.commit_app_session_state(&session_state);

        info!("Tempo set to {:.1} BPM", bpm);
    }

    /// Set the quantum (beats per bar)
    pub async fn set_quantum(&self, beats: f64) {
        let mut quantum = self.quantum.lock().await;
        *quantum = beats;
        info!("Quantum set to {:.1} beats per bar", beats);
    }

    /// Enable or disable Link
    pub async fn enable(&self, enabled: bool) {
        let mut link_guard = self.link.lock().await;
        link_guard.enable(enabled);
        let num_peers = link_guard.num_peers();
        let peers_usize: usize = num_peers.try_into().unwrap_or(0);
        info!("Link {} (peers: {})", if enabled { "enabled" } else { "disabled" }, peers_usize);
    }

    /// Force a transport start on the Link network
    pub async fn start_transport(&self) {
        let mut link_guard = self.link.lock().await;
        let mut session_state = SessionState::new();
        link_guard.capture_app_session_state(&mut session_state);

        // Get the current time
        let micros = link_guard.clock_micros();

        // Convert i64 to u64 correctly for set_is_playing, handling negative values
        let time_u64 = if micros >= 0 {
            micros as u64
        } else {
            info!("Warning: negative clock micros ({}), using 0 instead", micros);
            0
        };

        session_state.set_is_playing(true, time_u64);
        link_guard.commit_app_session_state(&session_state);
        info!("Link transport started");
    }

    /// Force a transport stop on the Link network
    pub async fn stop_transport(&self) {
        let mut link_guard = self.link.lock().await;
        let mut session_state = SessionState::new();
        link_guard.capture_app_session_state(&mut session_state);

        // Get the current time
        let micros = link_guard.clock_micros();

        // Convert i64 to u64 correctly for set_is_playing, handling negative values
        let time_u64 = if micros >= 0 {
            micros as u64
        } else {
            info!("Warning: negative clock micros ({}), using 0 instead", micros);
            0
        };

        session_state.set_is_playing(false, time_u64);
        link_guard.commit_app_session_state(&session_state);
        info!("Link transport stopped");
    }

    /// Check if transport is playing
    pub async fn is_playing(&self) -> bool {
        let mut session_state = SessionState::new();

        {
            let mut link_guard = self.link.lock().await;
            link_guard.capture_app_session_state(&mut session_state);
        }

        session_state.is_playing()
    }

    /// Get the current number of connected peers
    pub async fn num_peers(&self) -> usize {
        let link_guard = self.link.lock().await;
        let raw_num_peers = link_guard.num_peers();
        let converted_peers = raw_num_peers.try_into().unwrap_or(0);

        debug!("Link peer detection: Raw count={}, Converted={}", raw_num_peers, converted_peers);

        converted_peers
    }

    /// Stop the Link synchronizer
    pub async fn stop(&self) {
        let mut running = self.running.lock().await;
        *running = false;

        // Disable Link
        let mut link_guard = self.link.lock().await;
        link_guard.enable(false);

        info!("Link synchronizer stopping");
    }
}