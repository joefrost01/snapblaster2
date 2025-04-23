// Re-export modules for easier imports
pub mod ai;
pub mod events;
pub mod model;
pub mod morph;
pub mod storage;

// MIDI subsystem
pub mod midi {
    pub mod controller;
    pub mod manager;
    pub mod controllers {
        pub mod generic;
        pub mod launchpad_x;
    }
}

// Link integration
pub mod link {
    use crate::events::{Event, EventBus};
    use std::sync::Arc;
    use tokio::sync::Mutex;
    use tokio::task::JoinHandle;
    use tokio::time::{self, Duration};

    /// Link integration for tempo synchronization
    #[derive(Clone)]
    pub struct LinkSynchronizer {
        event_bus: EventBus,
        tempo: Arc<Mutex<f64>>,
        running: Arc<Mutex<bool>>,
    }

    impl LinkSynchronizer {
        /// Create a new Link synchronizer
        pub fn new(event_bus: EventBus) -> Self {
            Self {
                event_bus,
                tempo: Arc::new(Mutex::new(120.0)), // Default tempo
                running: Arc::new(Mutex::new(true)),
            }
        }

        /// Start the Link synchronizer
        pub fn start(&self) -> JoinHandle<()> {
            let event_bus = self.event_bus.clone();
            let tempo = self.tempo.clone();
            let running = self.running.clone();

            tokio::spawn(async move {
                let mut beat_count = 0;
                let mut bar_count = 0;

                while *running.lock().await {
                    // Get the current tempo
                    let current_tempo = *tempo.lock().await;
                    let beats_per_second = current_tempo / 60.0;
                    let ms_per_beat = 1000.0 / beats_per_second;

                    // Calculate the current phase (0.0 - 1.0 within a bar)
                    let phase = (beat_count % 4) as f64 / 4.0;

                    // Send beat event
                    let _ = event_bus.publish(Event::BeatOccurred {
                        beat: beat_count,
                        phase,
                    });

                    // Check if we're at a bar boundary
                    if beat_count % 4 == 0 {
                        let _ = event_bus.publish(Event::BarOccurred { bar: bar_count });
                        bar_count += 1;
                    }

                    // Increment beat count
                    beat_count += 1;

                    // Sleep until the next beat
                    time::sleep(Duration::from_millis(ms_per_beat as u64)).await;
                }
            })
        }

        /// Set the tempo in BPM
        pub async fn set_tempo(&self, bpm: f64) {
            let mut tempo = self.tempo.lock().await;
            *tempo = bpm;
        }

        /// Stop the Link synchronizer
        pub async fn stop(&self) {
            let mut running = self.running.lock().await;
            *running = false;
        }
    }

    // This is a placeholder for the actual Ableton Link integration
    // In a real implementation, we'd use the rust-link crate
}

// App state and initialization
pub mod app {
    use crate::ai::AIService;
    use crate::events::EventBus;
    use crate::link::LinkSynchronizer;
    use crate::midi::manager::MidiManager;
    use crate::model::{new_shared_state, SharedState};
    use crate::morph::MorphEngine;
    use crate::storage::ProjectStorage;
    use std::error::Error;
    use std::path::Path;
    use std::sync::Arc;
    use tokio::task::JoinHandle;
    use tracing::{error, info, warn};

    /// Main application state
    pub struct App {
        state: SharedState,
        event_bus: EventBus,
        midi_manager: Option<Arc<MidiManager>>, // Change to Option
        link_sync: Option<LinkSynchronizer>,
        project_storage: ProjectStorage,
        join_handles: Vec<JoinHandle<()>>,
    }

    /// Initialize the application
    impl App {
        /// Create a new application instance  
        pub fn new(state: SharedState, event_bus: EventBus) -> Result<Self, Box<dyn Error>> {
            let project_storage = ProjectStorage::new(state.clone(), event_bus.clone());

            Ok(Self {
                state,
                event_bus,
                midi_manager: None, // Initialize as None
                link_sync: None,
                project_storage,
                join_handles: Vec::new(),
            })
        }

        /// Initialize the application
        pub fn init(&mut self) -> Result<(), Box<dyn Error>> {
            // Initialize MIDI manager with the shared state
            let midi_manager = Arc::new(MidiManager::new(self.event_bus.clone(), Some(self.state.clone())));

            // Initialize controller
            let controller_name = {
                let state_guard = self.state.read().unwrap();
                state_guard.project.controller.clone()
            };

            // Try to create virtual MIDI port
            if let Err(e) = midi_manager.create_virtual_port("Snap-Blaster") {
                error!("Failed to create virtual MIDI port: {}", e);
                // Continue anyway
            } else {
                info!("Created virtual MIDI port: Snap-Blaster");
            }

            // Now try to connect to a controller
            if let Err(e) = midi_manager.initialize_controller(&controller_name) {
                warn!("Failed to initialize controller {}: {}", controller_name, e);
                // Continue anyway
            } else {
                info!("Initialized controller: {}", controller_name);
            }

            // Store the MIDI manager
            self.midi_manager = Some(midi_manager);

            // Initialize Link synchronizer
            let link_sync = LinkSynchronizer::new(self.event_bus.clone());
            let link_handle = link_sync.start();
            self.link_sync = Some(link_sync);
            self.join_handles.push(link_handle);

            // Initialize AI service with the shared state
            let ai_service = AIService::new(self.state.clone(), self.event_bus.clone());
            let ai_handle = ai_service.start();
            self.join_handles.push(ai_handle);

            // Initialize morph engine with the shared state
            let morph_engine = MorphEngine::new(self.state.clone(), self.event_bus.clone());
            let morph_handle = morph_engine.start();
            self.join_handles.push(morph_handle);

            Ok(())
        }

        pub fn midi_manager(&self) -> Option<Arc<MidiManager>> {
            self.midi_manager.clone()
        }

        /// Save the current project
        pub fn save_project(&self, path: &Path) -> Result<(), Box<dyn Error>> {
            self.project_storage.save_project(path)
        }

        /// Load a project
        pub fn load_project(&self, path: &Path) -> Result<(), Box<dyn Error>> {
            self.project_storage.load_project(path)
        }

        /// Create a new project
        pub fn new_project(&self) -> Result<(), Box<dyn Error>> {
            self.project_storage.new_project()
        }

        /// Shutdown the application
        pub fn shutdown(&self) {
            let _ = self.event_bus.publish(crate::events::Event::Shutdown);
        }
    }
}
