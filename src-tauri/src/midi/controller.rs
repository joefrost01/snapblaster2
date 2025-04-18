use crate::events::{Event, EventBus};
use crate::midi::controllers::apc_mini::ApcMini;
use crate::midi::controllers::generic::GenericController;
use crate::midi::controllers::launchpad_mini::LaunchpadMini;
use crate::midi::controllers::launchpad_x::LaunchpadX;
use crate::midi::controllers::push_2::Push2;
use std::error::Error;
use tracing::{info, warn};

/// RGB color representation for controller LEDs
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgb {
    /// Create a new RGB color
    pub fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    /// Create a black/off color
    pub fn black() -> Self {
        Self { r: 0, g: 0, b: 0 }
    }

    /// Create a white color
    pub fn white() -> Self {
        Self {
            r: 255,
            g: 255,
            b: 255,
        }
    }

    /// Create a red color
    pub fn red() -> Self {
        Self { r: 255, g: 0, b: 0 }
    }

    /// Create a green color
    pub fn green() -> Self {
        Self { r: 0, g: 255, b: 0 }
    }

    /// Create a blue color
    pub fn blue() -> Self {
        Self { r: 0, g: 0, b: 255 }
    }

    /// Create an orange color
    pub fn orange() -> Self {
        Self {
            r: 255,
            g: 165,
            b: 0,
        }
    }

    /// Create a yellow color
    pub fn yellow() -> Self {
        Self {
            r: 255,
            g: 255,
            b: 0,
        }
    }

    /// Create a purple color
    pub fn purple() -> Self {
        Self {
            r: 128,
            g: 0,
            b: 128,
        }
    }

    /// Create a cyan color
    pub fn cyan() -> Self {
        Self {
            r: 0,
            g: 255,
            b: 255,
        }
    }

    /// Create a gray color
    pub fn gray() -> Self {
        Self {
            r: 128,
            g: 128,
            b: 128,
        }
    }
}

/// Common trait for all grid-based MIDI controllers
pub trait MidiGridController: Send + 'static {
    /// Process a Note On message from the controller
    fn handle_note_input(&mut self, note: u8, velocity: u8);

    /// Set the color of a specific pad LED
    fn set_led(&mut self, pad: u8, color: Rgb);

    /// Turn off all LEDs on the controller
    fn clear_leds(&mut self);

    /// Update all LEDs to match the current state
    fn refresh_state(&mut self);

    /// Gets the name of this controller
    fn get_name(&self) -> &str;

    /// Send a CC value to the output MIDI port
    fn send_cc(&mut self, channel: u8, cc: u8, value: u8) -> Result<(), Box<dyn Error>>;
}

/// Factory function to create the appropriate controller based on name
pub fn create_controller(
    name: &str,
    event_bus: EventBus,
) -> Result<Box<dyn MidiGridController>, Box<dyn Error>> {
    info!("Creating controller: {}", name);

    match name {
        "Launchpad Mini" => match LaunchpadMini::new(event_bus.clone()) {
            Ok(controller) => Ok(Box::new(controller)),
            Err(e) => {
                warn!("Failed to create Launchpad Mini: {}", e);
                fallback_controller(event_bus)
            }
        },
        "Launchpad X" => match LaunchpadX::new(event_bus.clone()) {
            Ok(controller) => Ok(Box::new(controller)),
            Err(e) => {
                warn!("Failed to create Launchpad X: {}", e);
                fallback_controller(event_bus)
            }
        },
        "Push 2" => match Push2::new(event_bus.clone()) {
            Ok(controller) => Ok(Box::new(controller)),
            Err(e) => {
                warn!("Failed to create Push 2: {}", e);
                fallback_controller(event_bus)
            }
        },
        "APC Mini" => match ApcMini::new(event_bus.clone()) {
            Ok(controller) => Ok(Box::new(controller)),
            Err(e) => {
                warn!("Failed to create APC Mini: {}", e);
                fallback_controller(event_bus)
            }
        },
        "Generic" | _ => fallback_controller(event_bus),
    }
}

/// Creates a fallback controller when the requested one isn't available
fn fallback_controller(event_bus: EventBus) -> Result<Box<dyn MidiGridController>, Box<dyn Error>> {
    info!("Using generic controller as fallback");
    Ok(Box::new(GenericController::new(event_bus)))
}
