use crate::events::{Event, EventBus};
use crate::midi::controller::{MidiGridController, Rgb};
use std::error::Error;
use tracing::{debug, info, warn};

/// A generic controller implementation that can be used when no hardware is available
/// or when the specific controller could not be initialized
pub struct GenericController {
    event_bus: EventBus,
    led_state: [[Rgb; 8]; 8], // Virtual grid of LEDs
}

impl GenericController {
    /// Create a new generic controller
    pub fn new(event_bus: EventBus) -> Self {
        info!("Creating generic controller");
        Self {
            event_bus,
            led_state: [[Rgb::black(); 8]; 8],
        }
    }

    /// Simulate a pad press (useful for testing)
    pub fn simulate_pad_press(&mut self, pad: u8, velocity: u8) {
        info!("Simulating pad press: pad={}, velocity={}", pad, velocity);
        let _ = self.event_bus.publish(Event::PadPressed { pad, velocity });
    }

    /// Get the current state of all LEDs (useful for UI rendering)
    pub fn get_led_state(&self) -> &[[Rgb; 8]; 8] {
        &self.led_state
    }
}

impl MidiGridController for GenericController {
    fn handle_note_input(&mut self, note: u8, velocity: u8) {
        debug!(
            "Generic controller received note input: note={}, velocity={}",
            note, velocity
        );
        let _ = self.event_bus.publish(Event::PadPressed {
            pad: note,
            velocity,
        });
    }

    fn set_led(&mut self, pad: u8, color: Rgb) {
        // Convert pad index to row/column
        let row = (pad / 8) as usize;
        let col = (pad % 8) as usize;

        // Update internal state
        if row < 8 && col < 8 {
            self.led_state[row][col] = color;
            debug!("Generic controller LED set: pad={}, color={:?}", pad, color);
        }
    }

    fn clear_leds(&mut self) {
        // Reset all virtual LEDs
        self.led_state = [[Rgb::black(); 8]; 8];
        debug!("Generic controller LEDs cleared");
    }

    fn refresh_state(&mut self) {
        // In a hardware controller, this would send MIDI messages
        // But here we just log the event
        debug!("Generic controller refreshing state");
    }

    fn get_name(&self) -> &str {
        "Generic Controller"
    }

    fn set_progress_led(&mut self, pad: u8, progress: f64) -> Result<(), Box<dyn Error>> {
        todo!()
    }

    fn set_morph_target_led(&mut self, pad: u8) -> Result<(), Box<dyn Error>> {
        todo!()
    }

    fn set_active_modifier_led(&mut self, pad: u8) -> Result<(), Box<dyn Error>> {
        todo!()
    }

    fn send_cc(&mut self, channel: u8, cc: u8, value: u8) -> Result<(), Box<dyn Error>> {
        // Log the CC message that would have been sent
        debug!(
            "Generic controller sending CC: channel={}, cc={}, value={}",
            channel, cc, value
        );

        // In a real implementation, this would send a MIDI message
        // Here, we just return success
        Ok(())
    }
}