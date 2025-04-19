use serde::{Deserialize, Serialize};
use std::sync::{Arc, RwLock};

/// A Parameter represents a single MIDI CC control
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Parameter {
    /// User-friendly name for the parameter
    pub name: String,

    /// Description of the parameter's function
    pub description: String,

    /// MIDI CC number (0-127)
    pub cc: u8,
}

/// A Snap represents a complete state of all parameters
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Snap {
    /// User-friendly name for the snap
    pub name: String,

    /// Description of the snap's purpose
    pub description: String,

    /// CC values for each parameter (index corresponds to parameter index)
    pub values: Vec<u8>,
}

/// A Bank contains multiple snaps
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Bank {
    /// User-friendly name for the bank
    pub name: String,

    /// Collection of snaps in this bank
    pub snaps: Vec<Snap>,
}

/// Project is the main data container for all snap-blaster settings
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Project {
    /// Name of the project
    pub project_name: String,

    /// Optional OpenAI API key for AI features
    pub openai_api_key: Option<String>,

    /// Selected MIDI controller
    pub controller: String,

    /// Banks of snaps
    pub banks: Vec<Bank>,

    /// All parameters configured for this project
    pub parameters: Vec<Parameter>,
}

/// Default implementation creates an empty project
impl Default for Project {
    fn default() -> Self {
        Self {
            project_name: "New Project".to_string(),
            openai_api_key: None,
            controller: "Launchpad X".to_string(),
            banks: vec![Bank {
                name: "Default Bank".to_string(),
                snaps: vec![Snap {
                    name: "Initial Snap".to_string(),
                    description: "A starting point".to_string(),
                    values: vec![64; 64], // Default all values to 64 (middle)
                }],
            }],
            parameters: Vec::new(),
        }
    }
}

/// ProjectState holds the current state of the project and runtime information
pub struct ProjectState {
    /// The project data
    pub project: Project,

    /// Currently selected bank
    pub current_bank: usize,

    /// Currently selected snap
    pub current_snap: usize,

    /// Currently active morphing operation, if any
    pub active_morph: Option<ActiveMorph>,
}

/// Information about an active morphing operation
#[derive(Clone, Debug)]
pub struct ActiveMorph {
    /// Source snap index
    pub from_snap: usize,

    /// Target snap index
    pub to_snap: usize,

    /// Total duration in bars
    pub duration_bars: u8,

    /// Current progress (0.0 - 1.0)
    pub progress: f64,

    /// Starting values (snapshot of source snap when morph began)
    pub from_values: Vec<u8>,

    /// Target values (snapshot of target snap when morph began)
    pub to_values: Vec<u8>,

    /// Current interpolated values
    pub current_values: Vec<u8>,
}

impl Default for ProjectState {
    fn default() -> Self {
        Self {
            project: Project::default(),
            current_bank: 0,
            current_snap: 0,
            active_morph: None,
        }
    }
}

/// Shared application state that can be accessed from multiple components
pub type SharedState = Arc<RwLock<ProjectState>>;

/// Create a new shared state
pub fn new_shared_state() -> SharedState {
    Arc::new(RwLock::new(ProjectState::default()))
}
