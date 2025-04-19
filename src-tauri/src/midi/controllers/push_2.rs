use std::error::Error;
use crate::events::{Event, EventBus};
use crate::midi::controller::{MidiGridController, Rgb};

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