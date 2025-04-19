use crate::events::{Event, EventBus};
use std::error::Error;
use crate::midi::controllers::apc_mini::ApcMini;
use crate::midi::controllers::launchpad_mini::LaunchpadMini;
use crate::midi::controllers::launchpad_x::LaunchpadX;
use crate::midi::controllers::push_2::Push2;

/// RGB color representation for controller LEDs
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
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
    match name {
        "Launchpad Mini" => Ok(Box::new(LaunchpadMini::new(event_bus)?)),
        "Launchpad X" => Ok(Box::new(LaunchpadX::new(event_bus)?)),
        "Push 2" => Ok(Box::new(Push2::new(event_bus)?)),
        "APC Mini" => Ok(Box::new(ApcMini::new(event_bus)?)),
        _ => Err(format!("Unsupported controller: {}", name).into()),
    }
}
