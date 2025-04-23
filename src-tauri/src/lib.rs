// Re-export modules for easier imports
pub mod ai;
pub mod events;
pub mod model;
pub mod morph;
pub mod storage;
pub mod link;

// MIDI subsystem
pub mod midi {
    pub mod controller;
    pub mod manager;
    pub mod controllers {
        pub mod generic;
        pub mod launchpad_x;
    }
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
        midi_manager: Option<Arc<MidiManager>>,
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

            // Store the synchronizer
            self.link_sync = Some(link_sync);
            self.join_handles.push(link_handle);

            // Initialize morph engine with the shared state
            let mut morph_engine = MorphEngine::new(self.state.clone(), self.event_bus.clone());

            // Start the morph engine
            let morph_handle = morph_engine.start();
            self.join_handles.push(morph_handle);

            // Initialize AI service with the shared state
            let ai_service = AIService::new(self.state.clone(), self.event_bus.clone());
            let ai_handle = ai_service.start();
            self.join_handles.push(ai_handle);

            Ok(())
        }

        pub fn link_sync(&self) -> Option<LinkSynchronizer> {
            self.link_sync.clone()
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