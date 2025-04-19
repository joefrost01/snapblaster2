use crate::events::{Event, EventBus};
use crate::model::{Project, SharedState};
use std::error::Error;
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::Path;

/// ProjectStorage handles saving and loading project files
pub struct ProjectStorage {
    state: SharedState,
    event_bus: EventBus,
}

impl ProjectStorage {
    /// Create a new project storage service
    pub fn new(state: SharedState, event_bus: EventBus) -> Self {
        Self { state, event_bus }
    }

    /// Save the current project to a file
    pub fn save_project(&self, path: &Path) -> Result<(), Box<dyn Error>> {
        let state_guard = self.state.read().unwrap();
        let project = state_guard.project.clone();

        // Create the file
        let file = File::create(path)?;
        let writer = BufWriter::new(file);

        // Serialize and write the project
        serde_json::to_writer_pretty(writer, &project)?;

        // Publish event that project was saved
        let _ = self.event_bus.publish(Event::ProjectSaved);

        Ok(())
    }

    /// Load a project from a file
    pub fn load_project(&self, path: &Path) -> Result<(), Box<dyn Error>> {
        // Open the file
        let file = File::open(path)?;
        let reader = BufReader::new(file);

        // Deserialize the project
        let project: Project = serde_json::from_reader(reader)?;

        // Update the state
        {
            let mut state_guard = self.state.write().unwrap();
            state_guard.project = project;
            state_guard.current_bank = 0;
            state_guard.current_snap = 0;
            state_guard.active_morph = None;
        }

        // Publish event that project was loaded
        let _ = self.event_bus.publish(Event::ProjectLoaded);

        Ok(())
    }

    /// Create a new empty project
    pub fn new_project(&self) -> Result<(), Box<dyn Error>> {
        let mut state_guard = self.state.write().unwrap();
        state_guard.project = Project::default();
        state_guard.current_bank = 0;
        state_guard.current_snap = 0;
        state_guard.active_morph = None;

        // Publish event that project was loaded (new projects are treated as "loaded")
        drop(state_guard); // Release the lock before publishing event
        let _ = self.event_bus.publish(Event::ProjectLoaded);

        Ok(())
    }
}
