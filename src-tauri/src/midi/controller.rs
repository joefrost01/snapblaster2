use crate::events::{Event, EventBus};
use std::error::Error;

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
        "Launchpad X" => Ok(Box::new(LaunchpadX::new(event_bus)?)),
        "Launchpad Mini" => Ok(Box::new(LaunchpadMini::new(event_bus)?)),
        "Push 2" => Ok(Box::new(Push2::new(event_bus)?)),
        "APC Mini" => Ok(Box::new(ApcMini::new(event_bus)?)),
        _ => Err(format!("Unsupported controller: {}", name).into()),
    }
}

// Implement the LaunchpadX controller
pub struct LaunchpadX {
    event_bus: EventBus,
    // MIDI connections would be stored here
}

impl LaunchpadX {
    pub fn new(event_bus: EventBus) -> Result<Self, Box<dyn Error>> {
        Ok(Self { event_bus })
    }
}

impl MidiGridController for LaunchpadX {
    fn handle_note_input(&mut self, note: u8, velocity: u8) {
        // Convert the note to a pad index
        let pad = note;

        // Publish a pad pressed event
        let _ = self.event_bus.publish(Event::PadPressed { pad, velocity });
    }

    fn set_led(&mut self, pad: u8, color: Rgb) {
        // Implementation for Launchpad X LED control
        // Would use SysEx or note messages with velocity for color
    }

    fn clear_leds(&mut self) {
        // Implementation to clear all LEDs
    }

    fn refresh_state(&mut self) {
        // Implementation to update all LEDs based on current app state
    }

    fn get_name(&self) -> &str {
        "Launchpad X"
    }

    fn send_cc(&mut self, channel: u8, cc: u8, value: u8) -> Result<(), Box<dyn Error>> {
        // Send CC message to MIDI output
        Ok(())
    }
}

// Placeholder implementations for other controllers
// In a real implementation, each would have complete logic

pub struct LaunchpadMini {
    event_bus: EventBus,
}

impl LaunchpadMini {
    pub fn new(event_bus: EventBus) -> Result<Self, Box<dyn Error>> {
        Ok(Self { event_bus })
    }
}

impl MidiGridController for LaunchpadMini {
    fn handle_note_input(&mut self, note: u8, velocity: u8) {
        let pad = note;
        let _ = self.event_bus.publish(Event::PadPressed { pad, velocity });
    }

    fn set_led(&mut self, _pad: u8, _color: Rgb) {}
    fn clear_leds(&mut self) {}
    fn refresh_state(&mut self) {}
    fn get_name(&self) -> &str {
        "Launchpad Mini"
    }

    fn send_cc(&mut self, _channel: u8, _cc: u8, _value: u8) -> Result<(), Box<dyn Error>> {
        Ok(())
    }
}

pub struct Push2 {
    event_bus: EventBus,
}

impl Push2 {
    pub fn new(event_bus: EventBus) -> Result<Self, Box<dyn Error>> {
        Ok(Self { event_bus })
    }
}

impl MidiGridController for Push2 {
    fn handle_note_input(&mut self, note: u8, velocity: u8) {
        let pad = note;
        let _ = self.event_bus.publish(Event::PadPressed { pad, velocity });
    }

    fn set_led(&mut self, _pad: u8, _color: Rgb) {}
    fn clear_leds(&mut self) {}
    fn refresh_state(&mut self) {}
    fn get_name(&self) -> &str {
        "Push 2"
    }

    fn send_cc(&mut self, _channel: u8, _cc: u8, _value: u8) -> Result<(), Box<dyn Error>> {
        Ok(())
    }
}

pub struct ApcMini {
    event_bus: EventBus,
}

impl ApcMini {
    pub fn new(event_bus: EventBus) -> Result<Self, Box<dyn Error>> {
        Ok(Self { event_bus })
    }
}

impl MidiGridController for ApcMini {
    fn handle_note_input(&mut self, note: u8, velocity: u8) {
        let pad = note;
        let _ = self.event_bus.publish(Event::PadPressed { pad, velocity });
    }

    fn set_led(&mut self, _pad: u8, _color: Rgb) {}
    fn clear_leds(&mut self) {}
    fn refresh_state(&mut self) {}
    fn get_name(&self) -> &str {
        "APC Mini"
    }

    fn send_cc(&mut self, _channel: u8, _cc: u8, _value: u8) -> Result<(), Box<dyn Error>> {
        Ok(())
    }
}
