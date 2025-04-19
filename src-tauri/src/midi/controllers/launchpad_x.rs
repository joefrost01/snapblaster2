use crate::events::{Event, EventBus};
use crate::midi::controller::{MidiGridController, Rgb};
use midir::{MidiInput, MidiInputConnection, MidiOutput, MidiOutputConnection};
use std::error::Error;
use std::sync::{Arc, Mutex};
use wmidi::{MidiMessage, Note, U7};

/// Launchpad X Controller Implementation
pub struct LaunchpadX {
    event_bus: EventBus,
    input_connection: Option<MidiInputConnection<()>>,
    output_connection: Arc<Mutex<Option<MidiOutputConnection>>>,
    led_buffer: [[Rgb; 8]; 8], // Store current LED colors
}

impl LaunchpadX {
    /// Create a new Launchpad X controller instance
    pub fn new(event_bus: EventBus) -> Result<Self, Box<dyn Error>> {
        // Create a new instance with no connections yet
        let instance = Self {
            event_bus,
            input_connection: None,
            output_connection: Arc::new(Mutex::new(None)),
            led_buffer: [[Rgb { r: 0, g: 0, b: 0 }; 8]; 8],
        };

        // Connect to MIDI ports
        let instance = instance.connect_midi()?;

        // Initialize the controller
        instance.initialize()?;

        Ok(instance)
    }

    /// Connect to MIDI input and output ports
    fn connect_midi(mut self) -> Result<Self, Box<dyn Error>> {
        // Create MIDI input
        let midi_in = MidiInput::new("snapblaster-launchpad-x-in")?;

        // Find Launchpad X input port
        let in_port = find_launchpad_port(&midi_in.ports(), &midi_in, "Launchpad X")
            .ok_or("Launchpad X input port not found")?;

        // Clone values for closure
        let event_bus = self.event_bus.clone();

        // Connect to input port
        let input_connection = midi_in.connect(
            &in_port,
            "launchpad-x-input",
            move |_stamp, message, _| {
                // Process incoming MIDI message
                if let Ok(midi_msg) = wmidi::MidiMessage::try_from(message) {
                    match midi_msg {
                        MidiMessage::NoteOn(channel, note, velocity) => {
                            // Convert to pad index
                            let pad = note_to_pad_index(note);
                            let vel = u7_to_u8(velocity);

                            // Publish event
                            let _ = event_bus.publish(Event::PadPressed { pad, velocity: vel });
                        }
                        _ => {} // Ignore other message types
                    }
                }
            },
            (),
        )?;

        // Create MIDI output
        let midi_out = MidiOutput::new("snapblaster-launchpad-x-out")?;

        // Find Launchpad X output port
        let out_port = find_launchpad_port(&midi_out.ports(), &midi_out, "Launchpad X")
            .ok_or("Launchpad X output port not found")?;

        // Connect to output port
        let output_connection = midi_out.connect(&out_port, "launchpad-x-output")?;

        // Store connections
        self.input_connection = Some(input_connection);
        *self.output_connection.lock().unwrap() = Some(output_connection);

        Ok(self)
    }

    /// Initialize the controller (enter Programmer Mode)
    fn initialize(&self) -> Result<(), Box<dyn Error>> {
        // Enter Programmer Mode (Device Mode 1)
        if let Some(conn) = &mut *self.output_connection.lock().unwrap() {
            // Launchpad X uses SysEx for mode switching
            let sysex = [0xF0, 0x00, 0x20, 0x29, 0x02, 0x0C, 0x0E, 0x01, 0xF7];
            conn.send(&sysex)?;

            // Clear all pads
            self.clear_leds_internal(conn)?;
        }

        Ok(())
    }

    /// Converts a row and column to pad index
    fn rc_to_pad(&self, row: usize, col: usize) -> u8 {
        // In Programmer Mode, pad numbering starts at top-left (except for CC buttons)
        // Each row is 10 apart, with each column being 1 apart
        // Example: Top-left grid pad is 11, next is 12, etc.
        let result = 10 * (row + 1) + (col + 1);
        result as u8
    }

    /// Internal method to clear all LEDs
    fn clear_leds_internal(&self, conn: &mut MidiOutputConnection) -> Result<(), Box<dyn Error>> {
        // Reset all pads to off
        let sysex = [0xF0, 0x00, 0x20, 0x29, 0x02, 0x0C, 0x0E, 0x00, 0x00, 0xF7];
        conn.send(&sysex)?;

        Ok(())
    }

    /// Convert RGB color to Launchpad X color index
    fn rgb_to_launchpad_color(&self, color: Rgb) -> u8 {
        // For simplicity, we'll map RGB to the closest color in the Launchpad palette
        // This is a simplified conversion; a more accurate one would use the full color map

        if color.r == 0 && color.g == 0 && color.b == 0 {
            return 0; // Off
        }

        // Very simple mapping for demonstration
        if color.r > color.g && color.r > color.b {
            return 5; // Red
        } else if color.g > color.r && color.g > color.b {
            return 21; // Green
        } else if color.b > color.r && color.b > color.g {
            return 45; // Blue
        } else if color.r > 0 && color.g > 0 && color.b == 0 {
            return 13; // Yellow
        } else if color.r > 0 && color.b > 0 && color.g == 0 {
            return 53; // Magenta
        } else if color.g > 0 && color.b > 0 && color.r == 0 {
            return 37; // Cyan
        } else if color.r > 0 && color.g > 0 && color.b > 0 {
            return 3; // White
        }

        3 // Default to white
    }
}

impl MidiGridController for LaunchpadX {
    fn handle_note_input(&mut self, note: u8, velocity: u8) {
        // This is handled in the MIDI input callback
        // But we could process it here as well for testing
        let _ = self.event_bus.publish(Event::PadPressed {
            pad: note,
            velocity,
        });
    }

    fn set_led(&mut self, pad: u8, color: Rgb) {
        // Convert pad to row/column
        let (row, col) = pad_to_row_col(pad);

        // Store in buffer
        if row < 8 && col < 8 {
            self.led_buffer[row][col] = color;
        }

        // Send MIDI message to update LED
        if let Some(conn) = &mut *self.output_connection.lock().unwrap() {
            let launchpad_pad = self.rc_to_pad(row, col);
            let color_index = self.rgb_to_launchpad_color(color);

            // Note On message with velocity as color
            let msg = [0x90, launchpad_pad, color_index];
            let _ = conn.send(&msg);
        }
    }

    fn clear_leds(&mut self) {
        // Reset the LED buffer
        self.led_buffer = [[Rgb { r: 0, g: 0, b: 0 }; 8]; 8];

        // Send MIDI message to clear all LEDs
        if let Some(conn) = &mut *self.output_connection.lock().unwrap() {
            let _ = self.clear_leds_internal(conn);
        }
    }

    fn refresh_state(&mut self) {
        // Update all LEDs based on current buffer
        if let Some(conn) = &mut *self.output_connection.lock().unwrap() {
            for row in 0..8 {
                for col in 0..8 {
                    let launchpad_pad = self.rc_to_pad(row, col);
                    let color_index = self.rgb_to_launchpad_color(self.led_buffer[row][col]);

                    // Note On message with velocity as color
                    let msg = [0x90, launchpad_pad, color_index];
                    let _ = conn.send(&msg);
                }
            }
        }
    }

    fn get_name(&self) -> &str {
        "Launchpad X"
    }

    fn send_cc(&mut self, channel: u8, cc: u8, value: u8) -> Result<(), Box<dyn Error>> {
        // Send CC message to output port
        if let Some(conn) = &mut *self.output_connection.lock().unwrap() {
            let msg = [0xB0 | (channel & 0x0F), cc, value];
            conn.send(&msg)?;
        }

        Ok(())
    }
}

/// Helper function to find a Launchpad port by name
fn find_launchpad_port<T>(ports: &[T], conn: &impl PortInfos<T>, name: &str) -> Option<T>
where
    T: Clone,
{
    ports
        .iter()
        .find(|port| {
            if let Ok(port_name) = conn.port_name(port) {
                port_name.contains(name)
            } else {
                false
            }
        })
        .cloned()
}

/// Simple trait to abstract port name retrieval
trait PortInfos<T> {
    fn port_name(&self, port: &T) -> Result<String, Box<dyn Error>>;
}

impl PortInfos<midir::MidiInputPort> for MidiInput {
    fn port_name(&self, port: &midir::MidiInputPort) -> Result<String, Box<dyn Error>> {
        self.port_name(port).map_err(|e| e.into())
    }
}

impl PortInfos<midir::MidiOutputPort> for MidiOutput {
    fn port_name(&self, port: &midir::MidiOutputPort) -> Result<String, Box<dyn Error>> {
        self.port_name(port).map_err(|e| e.into())
    }
}

/// Convert a Note to a pad index (0-63)
fn note_to_pad_index(note: Note) -> u8 {
    // Get the raw u8 value from the note
    let note_num = u8::from(note);

    // Convert from Launchpad's format to our 0-63 grid
    let row = (note_num / 10) - 1;
    let col = (note_num % 10) - 1;

    // Ensure it's in our grid range (0-7 for both row and col)
    if row >= 0 && row < 8 && col >= 0 && col < 8 {
        return (row * 8 + col) as u8;
    }

    0 // Default for buttons outside the grid
}

/// Convert a pad index (0-63) to row and column
fn pad_to_row_col(pad: u8) -> (usize, usize) {
    let row = (pad / 8) as usize;
    let col = (pad % 8) as usize;

    (row, col)
}

/// Convert a wmidi::U7 to u8
fn u7_to_u8(value: U7) -> u8 {
    u8::from(value)
}

/// Convert a Note to Launchpad X pad number
fn note_to_launchpad_pad(note: u8) -> u8 {
    let row = note / 8;
    let col = note % 8;

    // Launchpad X pad mapping in programmer mode
    10 * (row + 1) + (col + 1)
}

impl Drop for LaunchpadX {
    fn drop(&mut self) {
        // Clean up on drop - reset the controller
        if let Some(conn) = &mut *self.output_connection.lock().unwrap() {
            // Exit Programmer Mode (back to default)
            let sysex = [0xF0, 0x00, 0x20, 0x29, 0x02, 0x0C, 0x0E, 0x00, 0xF7];
            let _ = conn.send(&sysex);
        }

        // Connections will be closed automatically when dropped
    }
}
