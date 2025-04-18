use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

/// Different types of curve functions for morphing between snaps
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum MorphCurve {
    Linear,
    #[cfg(feature = "pro")]
    Exponential,
    #[cfg(feature = "pro")]
    Logarithmic,
    #[cfg(feature = "pro")]
    SCurve,
}

/// Core events that flow through the system
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Event {
    // MIDI events
    PadPressed { pad: u8, velocity: u8 },
    CCValueChanged { param_id: usize, value: u8 },

    // Link events
    BeatOccurred { beat: u32, phase: f64 },
    BarOccurred { bar: u32 },

    // UI events
    SnapSelected { bank: usize, snap_id: usize },
    ParameterEdited { param_id: usize, value: u8 },
    BankSelected { bank_id: usize },

    // AI events
    GenerateAIValues { bank_id: usize, snap_id: usize },
    AIGenerationCompleted { bank_id: usize, snap_id: usize, values: Vec<u8> },
    AIGenerationFailed { bank_id: usize, snap_id: usize, error: String },

    // Morphing events
    MorphInitiated {
        from_snap: usize,
        to_snap: usize,
        duration_bars: u8,
        curve_type: MorphCurve,
    },
    MorphProgressed { progress: f64, current_values: Vec<u8> },
    MorphCompleted,

    // Project events
    ProjectLoaded,
    ProjectSaved,

    // System events
    Shutdown,
}

/// EventBus is the central message bus for the application
#[derive(Clone, Debug)]
pub struct EventBus {
    sender: broadcast::Sender<Event>,
}

impl EventBus {
    /// Create a new event bus with the specified capacity
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self { sender }
    }

    /// Get a new subscription to the event bus
    pub fn subscribe(&self) -> broadcast::Receiver<Event> {
        self.sender.subscribe()
    }

    /// Publish an event to all subscribers
    pub fn publish(&self, event: Event) -> Result<usize, broadcast::error::SendError<Event>> {
        self.sender.send(event)
    }
}

/// Default implementation with reasonable defaults
impl Default for EventBus {
    fn default() -> Self {
        Self::new(100) // Default capacity of 100 events
    }
}