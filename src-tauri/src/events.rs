// src-tauri/src/events.rs
use serde::{Deserialize, Serialize};
use std::fmt;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;
use tokio::sync::broadcast;
use tokio::time::timeout;
use tracing::{debug, error, info, warn};

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
    PadPressed {
        pad: u8,
        velocity: u8,
    },
    CCValueChanged {
        param_id: usize,
        value: u8,
    },

    // Link events
    BeatOccurred {
        beat: u32,
        phase: f64,
    },
    BarOccurred {
        bar: u32,
    },

    // UI events
    SnapSelected {
        bank: usize,
        snap_id: usize,
    },
    ParameterEdited {
        param_id: usize,
        value: u8,
    },
    BankSelected {
        bank_id: usize,
    },

    // AI events
    GenerateAIValues {
        bank_id: usize,
        snap_id: usize,
    },
    AIGenerationCompleted {
        bank_id: usize,
        snap_id: usize,
        values: Vec<u8>,
    },
    AIGenerationFailed {
        bank_id: usize,
        snap_id: usize,
        error: String,
    },

    // Morphing events
    MorphInitiated {
        from_snap: usize,
        to_snap: usize,
        duration_bars: u8,
        curve_type: MorphCurve,
        quantize: bool,
    },
    MorphProgressed {
        progress: f64,
        current_values: Vec<u8>,
    },
    MorphCompleted,

    // Project events
    ProjectLoaded,
    ProjectSaved,

    // System events
    Shutdown,

    // Link events
    LinkStatusChanged {
        connected: bool,
        peers: usize,
    },
    LinkTempoChanged {
        tempo: f64,
    },
    LinkTransportChanged {
        playing: bool,
    },
    RequestLinkStatus,
    RequestLinkTempo,
    RequestNextBarTime,
    NextBarTime {
        wait_time_ms: u64,
    },
}

/// Event statistics for monitoring
#[derive(Default, Debug)]
pub struct EventStats {
    pub messages_sent: AtomicUsize,
    pub messages_received: AtomicUsize,
    pub subscribers_peak: AtomicUsize,
}

/// EventBus is the central message bus for the application
#[derive(Clone)]
pub struct EventBus {
    sender: broadcast::Sender<Event>,
    name: String,        // Name for debugging purposes
    stats: std::sync::Arc<EventStats>,
}

impl fmt::Debug for EventBus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EventBus")
            .field("name", &self.name)
            .field("current_receivers", &self.sender.receiver_count())
            .field("stats", &self.stats)
            .finish()
    }
}

impl EventBus {
    /// Create a new event bus with the specified capacity and name
    pub fn new(capacity: usize, name: &str) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self {
            sender,
            name: name.to_string(),
            stats: std::sync::Arc::new(EventStats::default()),
        }
    }

    /// Get a new subscription to the event bus
    pub fn subscribe(&self) -> broadcast::Receiver<Event> {
        let rx = self.sender.subscribe();

        // Update statistics
        let current = self.sender.receiver_count();
        let peak = self.stats.subscribers_peak.load(Ordering::Relaxed);
        if current > peak {
            self.stats.subscribers_peak.store(current, Ordering::Relaxed);
        }

        rx
    }

    /// Publish an event to all subscribers
    pub fn publish(&self, event: Event) -> Result<usize, broadcast::error::SendError<Event>> {
        debug!(bus = %self.name, event_type = %event.event_type(), "Publishing event");

        // Update statistics
        self.stats.messages_sent.fetch_add(1, Ordering::Relaxed);

        let result = self.sender.send(event);

        // Log if no receivers
        if let Ok(receiver_count) = &result {
            if *receiver_count == 0 {
                warn!(bus = %self.name, "No receivers for event");
            }
        }

        result
    }

    /// Try to publish an event, logging any errors but not returning them
    pub fn try_publish(&self, event: Event) {
        match self.publish(event.clone()) {
            Ok(receiver_count) => {
                if receiver_count == 0 {
                    warn!(bus = %self.name, "No receivers for event: {}", event.event_type());
                }
            }
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

    /// Get a clone of the statistics
    pub fn stats(&self) -> std::sync::Arc<EventStats> {
        self.stats.clone()
    }

    /// Reset the event bus statistics
    pub fn reset_stats(&self) {
        self.stats.messages_sent.store(0, Ordering::Relaxed);
        self.stats.messages_received.store(0, Ordering::Relaxed);
        // Keep peak subscribers as-is
    }
}

/// Default implementation with reasonable defaults
impl Default for EventBus {
    fn default() -> Self {
        Self::new(1000, "default") // Larger default capacity for better buffering
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
            Event::LinkStatusChanged { .. } => "LinkStatusChanged",
            Event::LinkTempoChanged { .. } => "LinkTempoChanged",
            Event::LinkTransportChanged { .. } => "LinkTransportChanged",
            Event::RequestLinkStatus => "RequestLinkStatus",
            Event::RequestLinkTempo => "RequestLinkTempo",
            Event::RequestNextBarTime => "RequestNextBarTime",
            Event::NextBarTime { .. } => "NextBarTime",
        }
    }

    /// Returns true if the event is high-priority
    pub fn is_high_priority(&self) -> bool {
        matches!(
            self,
            Event::PadPressed { .. } | 
            Event::ParameterEdited { .. } | 
            Event::CCValueChanged { .. } |
            Event::Shutdown
        )
    }
}

// Add Display implementation for better logging
impl fmt::Display for Event {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Event::PadPressed { pad, velocity } => {
                write!(f, "PadPressed: pad={}, velocity={}", pad, velocity)
            }
            Event::CCValueChanged { param_id, value } => {
                write!(f, "CCValueChanged: param_id={}, value={}", param_id, value)
            }
            Event::BeatOccurred { beat, phase } => {
                write!(f, "BeatOccurred: beat={}, phase={:.2}", beat, phase)
            }
            Event::BarOccurred { bar } => write!(f, "BarOccurred: bar={}", bar),
            Event::SnapSelected { bank, snap_id } => {
                write!(f, "SnapSelected: bank={}, snap_id={}", bank, snap_id)
            }
            Event::ParameterEdited { param_id, value } => {
                info!("Backend received parameter edit: param={}, value={}", param_id, value);
                write!(f, "ParameterEdited: param_id={}, value={}", param_id, value)
            }
            Event::BankSelected { bank_id } => write!(f, "BankSelected: bank_id={}", bank_id),
            Event::GenerateAIValues { bank_id, snap_id } => write!(
                f,
                "GenerateAIValues: bank_id={}, snap_id={}",
                bank_id, snap_id
            ),
            Event::AIGenerationCompleted {
                bank_id, snap_id, ..
            } => write!(
                f,
                "AIGenerationCompleted: bank_id={}, snap_id={}",
                bank_id, snap_id
            ),
            Event::AIGenerationFailed {
                bank_id,
                snap_id,
                error,
            } => write!(
                f,
                "AIGenerationFailed: bank_id={}, snap_id={}, error={}",
                bank_id, snap_id, error
            ),
            Event::MorphInitiated {
                from_snap,
                to_snap,
                duration_bars,
                curve_type,
                quantize: bool,
            } => write!(
                f,
                "MorphInitiated: from={}, to={}, duration={}bars, curve={:?}",
                from_snap, to_snap, duration_bars, curve_type
            ),
            Event::MorphProgressed { progress, .. } => {
                write!(f, "MorphProgressed: progress={:.2}", progress)
            }
            Event::MorphCompleted => write!(f, "MorphCompleted"),
            Event::ProjectLoaded => write!(f, "ProjectLoaded"),
            Event::ProjectSaved => write!(f, "ProjectSaved"),
            Event::Shutdown => write!(f, "Shutdown"),
            Event::LinkStatusChanged { connected, peers } => {
                write!(f, "LinkStatusChanged: connected={}, peers={}", connected, peers)
            },
            Event::LinkTempoChanged { tempo } => {
                write!(f, "LinkTempoChanged: tempo={:.1}", tempo)
            },
            Event::LinkTransportChanged { playing } => {
                write!(f, "LinkTransportChanged: playing={}", playing)
            },
            Event::RequestLinkStatus => write!(f, "RequestLinkStatus"),
            Event::RequestLinkTempo => write!(f, "RequestLinkTempo"),
            Event::RequestNextBarTime => write!(f, "RequestNextBarTime"),
            Event::NextBarTime { wait_time_ms } => {
                write!(f, "NextBarTime: wait_time_ms={}", wait_time_ms)
            },
        }
    }
}

/// EventSubscriber makes it easy to handle specific events
pub struct EventSubscriber {
    receiver: broadcast::Receiver<Event>,
    source_name: String,
    stats: std::sync::Arc<EventStats>,
}

impl EventSubscriber {
    /// Create a new event subscriber
    pub fn new(event_bus: &EventBus, source_name: &str) -> Self {
        Self {
            receiver: event_bus.subscribe(),
            source_name: source_name.to_string(),
            stats: event_bus.stats(),
        }
    }

    /// Receive the next event with timeout
    pub async fn recv_timeout(&mut self, duration: Duration) -> Result<Event, RecvTimeoutError> {
        match timeout(duration, self.receiver.recv()).await {
            Ok(Ok(event)) => {
                self.stats.messages_received.fetch_add(1, Ordering::Relaxed);
                debug!(handler = %self.source_name, event = %event, "Received event");
                Ok(event)
            },
            Ok(Err(broadcast::error::RecvError::Closed)) => {
                warn!(handler = %self.source_name, "Event bus closed");
                Err(RecvTimeoutError::Closed)
            },
            Ok(Err(broadcast::error::RecvError::Lagged(n))) => {
                warn!(handler = %self.source_name, "Lagged behind {} events", n);
                Err(RecvTimeoutError::Lagged(n))
            },
            Err(_) => Err(RecvTimeoutError::Timeout),
        }
    }

    /// Receive the next event
    pub async fn recv(&mut self) -> Result<Event, broadcast::error::RecvError> {
        match self.receiver.recv().await {
            Ok(event) => {
                self.stats.messages_received.fetch_add(1, Ordering::Relaxed);
                debug!(handler = %self.source_name, event = %event, "Received event");
                Ok(event)
            },
            Err(e) => {
                if let broadcast::error::RecvError::Lagged(n) = &e {
                    warn!(handler = %self.source_name, "Lagged behind {} events", n);
                }
                Err(e)
            }
        }
    }

    /// Handle events with the provided callback
    pub async fn handle_events<F>(&mut self, mut callback: F)
    where
        F: FnMut(Event) -> bool, // Return true to continue, false to stop
    {
        loop {
            match self.receiver.recv().await {
                Ok(event) => {
                    self.stats.messages_received.fetch_add(1, Ordering::Relaxed);
                    debug!(handler = %self.source_name, event = %event, "Handling event");
                    if !callback(event) {
                        debug!(handler = %self.source_name, "Stopping event handling loop (callback returned false)");
                        break;
                    }
                }
                Err(broadcast::error::RecvError::Closed) => {
                    warn!(handler = %self.source_name, "Event bus closed");
                    break;
                }
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    warn!(handler = %self.source_name, "Lagged behind {} events", n);
                    // Continue processing
                }
            }
        }
    }

    /// Process only high-priority events, skipping others
    pub async fn handle_priority_events<F>(&mut self, mut callback: F)
    where
        F: FnMut(Event) -> bool, // Return true to continue, false to stop
    {
        loop {
            match self.receiver.recv().await {
                Ok(event) => {
                    self.stats.messages_received.fetch_add(1, Ordering::Relaxed);

                    // Only process high-priority events
                    if event.is_high_priority() {
                        debug!(handler = %self.source_name, event = %event, "Handling priority event");
                        if !callback(event) {
                            debug!(handler = %self.source_name, "Stopping priority event handling loop");
                            break;
                        }
                    }
                }
                Err(broadcast::error::RecvError::Closed) => {
                    warn!(handler = %self.source_name, "Event bus closed");
                    break;
                }
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    warn!(handler = %self.source_name, "Lagged behind {} events", n);
                    // Continue processing
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
                    self.stats.messages_received.fetch_add(1, Ordering::Relaxed);
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
            stats: event_bus.stats(),
        }
    }
}

/// Error type for recv_timeout
#[derive(Debug)]
pub enum RecvTimeoutError {
    Timeout,
    Closed,
    Lagged(u64),
}

impl fmt::Display for RecvTimeoutError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RecvTimeoutError::Timeout => write!(f, "Timeout waiting for event"),
            RecvTimeoutError::Closed => write!(f, "Event bus closed"),
            RecvTimeoutError::Lagged(n) => write!(f, "Lagged behind {} events", n),
        }
    }
}

impl std::error::Error for RecvTimeoutError {}