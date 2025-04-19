use std::fmt;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use tracing::{debug, error, warn};

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
    name: String, // Adding a name for debugging purposes
}

impl EventBus {
    /// Create a new event bus with the specified capacity and name
    pub fn new(capacity: usize, name: &str) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self {
            sender,
            name: name.to_string(),
        }
    }

    /// Get a new subscription to the event bus
    pub fn subscribe(&self) -> broadcast::Receiver<Event> {
        self.sender.subscribe()
    }

    /// Publish an event to all subscribers
    pub fn publish(&self, event: Event) -> Result<usize, broadcast::error::SendError<Event>> {
        debug!(bus = %self.name, event_type = %event.event_type(), "Publishing event");
        self.sender.send(event)
    }

    /// Try to publish an event, logging any errors but not returning them
    pub fn try_publish(&self, event: Event) {
        match self.publish(event.clone()) {
            Ok(receiver_count) => {
                if receiver_count == 0 {
                    warn!(bus = %self.name, "No receivers for event: {}", event.event_type());
                }
            },
            Err(e) => {
                error!(bus = %self.name, "Failed to publish event: {}", e);
            }
        }
    }

    /// Get the number of active receivers
    pub fn receiver_count(&self) -> usize {
        self.sender.receiver_count()
    }

    /// Get the name of this event bus
    pub fn name(&self) -> &str {
        &self.name
    }
}

/// Default implementation with reasonable defaults
impl Default for EventBus {
    fn default() -> Self {
        Self::new(100, "default") // Default capacity of 100 events and name "default"
    }
}

// Add a helper method to Event for easier logging
impl Event {
    /// Get a short string describing the event type (for logging)
    pub fn event_type(&self) -> &'static str {
        match self {
            Event::PadPressed { .. } => "PadPressed",
            Event::CCValueChanged { .. } => "CCValueChanged",
            Event::BeatOccurred { .. } => "BeatOccurred",
            Event::BarOccurred { .. } => "BarOccurred",
            Event::SnapSelected { .. } => "SnapSelected",
            Event::ParameterEdited { .. } => "ParameterEdited",
            Event::BankSelected { .. } => "BankSelected",
            Event::GenerateAIValues { .. } => "GenerateAIValues",
            Event::AIGenerationCompleted { .. } => "AIGenerationCompleted",
            Event::AIGenerationFailed { .. } => "AIGenerationFailed",
            Event::MorphInitiated { .. } => "MorphInitiated",
            Event::MorphProgressed { .. } => "MorphProgressed",
            Event::MorphCompleted => "MorphCompleted",
            Event::ProjectLoaded => "ProjectLoaded",
            Event::ProjectSaved => "ProjectSaved",
            Event::Shutdown => "Shutdown",
        }
    }
}

// Add Display implementation for better logging
impl fmt::Display for Event {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Event::PadPressed { pad, velocity } =>
                write!(f, "PadPressed: pad={}, velocity={}", pad, velocity),
            Event::CCValueChanged { param_id, value } =>
                write!(f, "CCValueChanged: param_id={}, value={}", param_id, value),
            Event::BeatOccurred { beat, phase } =>
                write!(f, "BeatOccurred: beat={}, phase={:.2}", beat, phase),
            Event::BarOccurred { bar } =>
                write!(f, "BarOccurred: bar={}", bar),
            Event::SnapSelected { bank, snap_id } =>
                write!(f, "SnapSelected: bank={}, snap_id={}", bank, snap_id),
            Event::ParameterEdited { param_id, value } =>
                write!(f, "ParameterEdited: param_id={}, value={}", param_id, value),
            Event::BankSelected { bank_id } =>
                write!(f, "BankSelected: bank_id={}", bank_id),
            Event::GenerateAIValues { bank_id, snap_id } =>
                write!(f, "GenerateAIValues: bank_id={}, snap_id={}", bank_id, snap_id),
            Event::AIGenerationCompleted { bank_id, snap_id, .. } =>
                write!(f, "AIGenerationCompleted: bank_id={}, snap_id={}", bank_id, snap_id),
            Event::AIGenerationFailed { bank_id, snap_id, error } =>
                write!(f, "AIGenerationFailed: bank_id={}, snap_id={}, error={}", bank_id, snap_id, error),
            Event::MorphInitiated { from_snap, to_snap, duration_bars, curve_type } =>
                write!(f, "MorphInitiated: from={}, to={}, duration={}bars, curve={:?}", from_snap, to_snap, duration_bars, curve_type),
            Event::MorphProgressed { progress, .. } =>
                write!(f, "MorphProgressed: progress={:.2}", progress),
            Event::MorphCompleted =>
                write!(f, "MorphCompleted"),
            Event::ProjectLoaded =>
                write!(f, "ProjectLoaded"),
            Event::ProjectSaved =>
                write!(f, "ProjectSaved"),
            Event::Shutdown =>
                write!(f, "Shutdown"),
        }
    }
}

/// EventSubscriber makes it easy to handle specific events
pub struct EventSubscriber {
    receiver: broadcast::Receiver<Event>,
    source_name: String,
}

impl EventSubscriber {
    /// Create a new event subscriber
    pub fn new(event_bus: &EventBus, source_name: &str) -> Self {
        Self {
            receiver: event_bus.subscribe(),
            source_name: source_name.to_string(),
        }
    }

    /// Receive the next event
    pub async fn recv(&mut self) -> Result<Event, broadcast::error::RecvError> {
        self.receiver.recv().await
    }

    /// Handle events with the provided callback
    pub async fn handle_events<F>(&mut self, mut callback: F)
    where
        F: FnMut(Event) -> bool, // Return true to continue, false to stop
    {
        loop {
            match self.receiver.recv().await {
                Ok(event) => {
                    debug!(handler = %self.source_name, event = %event, "Received event");
                    if !callback(event) {
                        break;
                    }
                }
                Err(broadcast::error::RecvError::Closed) => {
                    warn!(handler = %self.source_name, "Event bus closed");
                    break;
                }
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    warn!(handler = %self.source_name, "Lagged behind {} events", n);
                    // Continue processing events
                }
            }
        }
    }

    /// Extract just one specific type of event
    pub async fn filter_event<F, T>(&mut self, filter: F) -> Option<T>
    where
        F: Fn(&Event) -> Option<T>,
        T: Clone,
    {
        loop {
            match self.receiver.recv().await {
                Ok(event) => {
                    if let Some(extracted) = filter(&event) {
                        return Some(extracted);
                    }
                }
                Err(_) => return None,
            }
        }
    }

    /// Get the name of this event subscriber
    pub fn name(&self) -> &str {
        &self.source_name
    }

    /// Create a clone with a fresh subscription
    pub fn clone_with_new_subscription(&self, event_bus: &EventBus) -> Self {
        Self {
            receiver: event_bus.subscribe(),
            source_name: self.source_name.clone(),
        }
    }
}