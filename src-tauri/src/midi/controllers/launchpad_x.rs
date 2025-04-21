use crate::events::{Event, EventBus};
use crate::midi::controller::{MidiGridController, Rgb};
use midir::{MidiInput, MidiInputConnection, MidiOutput, MidiOutputConnection};
use std::error::Error;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::thread;
use tracing::{debug, error, info, warn};
use wmidi::{MidiMessage, Note, U7};

/// LaunchpadX Controller Implementation
pub struct LaunchpadX {
    event_bus: EventBus,
    input_connection: Option<MidiInputConnection<()>>,
    output_connection: Arc<Mutex<Option<MidiOutputConnection>>>,
    led_buffer: [[Rgb; 8]; 8], // Store current LED colors
}

impl LaunchpadX {
    /// Create a new Launchpad X controller instance
    pub fn new(event_bus: EventBus) -> Result<Self, Box<dyn Error>> {
        info!("Initializing Launchpad X controller");

        // Create a new instance with no connections yet
        let instance = Self {
            event_bus,
            input_connection: None,
            output_connection: Arc::new(Mutex::new(None)),
            led_buffer: [[Rgb::black(); 8]; 8],
        };

        // Connect to MIDI ports - this may fail if hardware isn't connected
        match instance.connect_midi() {
            Ok(connected_instance) => {
                // Initialize the controller
                match connected_instance.initialize() {
                    Ok(_) => {
                        info!("Launchpad X controller initialized successfully");
                        Ok(connected_instance)
                    },
                    Err(e) => {
                        error!("Failed to initialize Launchpad X: {}", e);
                        Err(e)
                    }
                }
            },
            Err(e) => {
                error!("Failed to connect to Launchpad X: {}", e);
                Err(e)
            }
        }
    }

    /// Connect to MIDI input and output ports
    fn connect_midi(mut self) -> Result<Self, Box<dyn Error>> {
        // Create MIDI input with descriptive name
        let midi_in = MidiInput::new("snapblaster-launchpad-x-in")?;

        // Find Launchpad X input port
        let in_port = find_launchpad_port(&midi_in.ports(), &midi_in, "Launchpad X")
            .ok_or_else(|| {
                let err = "Launchpad X input port not found - check USB connection".to_string();
                error!("{}", err);
                err
            })?;

        info!("Found Launchpad X input port: {}", midi_in.port_name(&in_port)?);

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
                        MidiMessage::NoteOn(_, note, velocity) => {
                            // Convert to pad index
                            let note_num = u8::from(note);

                            // Use the custom note-to-pad mapping
                            if let Some(pad) = note_to_pad_index(note_num) {
                                let vel = u7_to_u8(velocity);
                                debug!("Received NoteOn: note={:?}, pad={}, velocity={}", note, pad, vel);

                                // Only process if velocity > 0 (ignore note off events)
                                if vel > 0 {
                                    // Publish event
                                    let _ = event_bus.publish(Event::PadPressed { pad, velocity: vel });
                                }
                            } else {
                                debug!("Note {:?} mapped to no valid pad", note);
                            }
                        },
                        MidiMessage::ControlChange(_, cc, value) => {
                            // Handle control change messages if needed
                            debug!("Received CC: cc={:?}, value={:?}", cc, value);
                        },
                        _ => {} // Ignore other message types like aftertouch
                    }
                }
            },
            (),
        )?;

        // Create MIDI output
        let midi_out = MidiOutput::new("snapblaster-launchpad-x-out")?;

        // Find Launchpad X output port
        let out_port = find_launchpad_port(&midi_out.ports(), &midi_out, "Launchpad X")
            .ok_or_else(|| {
                let err = "Launchpad X output port not found".to_string();
                error!("{}", err);
                err
            })?;

        info!("Found Launchpad X output port: {}", midi_out.port_name(&out_port)?);

        // Connect to output port
        let output_connection = midi_out.connect(&out_port, "launchpad-x-output")?;

        // Store connections
        self.input_connection = Some(input_connection);
        *self.output_connection.lock().unwrap() = Some(output_connection);

        info!("Connected to Launchpad X MIDI ports");

        Ok(self)
    }

    /// Initialize the controller (set to Programmer Mode)
    fn initialize(&self) -> Result<(), Box<dyn Error>> {
        if let Some(conn) = &mut *self.output_connection.lock().unwrap() {
            // Launchpad X uses SysEx for mode switching - enter Programmer Mode (Device Mode 1)
            info!("Setting Launchpad X to Programmer Mode");
            let sysex = [0xF0, 0x00, 0x20, 0x29, 0x02, 0x0C, 0x0E, 0x01, 0xF7];
            conn.send(&sysex)?;

            // Wait a bit for the mode change to take effect
            thread::sleep(Duration::from_millis(100));

            // Clear all pads
            self.clear_leds_internal(conn)?;

            // Set all pads to very dim to show it's ready
            for row in 0..8 {
                for col in 0..8 {
                    let note = self.rc_to_pad(row, col);
                    let msg = [0x90, note, 1]; // Very dim color
                    conn.send(&msg)?;
                }
            }

            info!("Launchpad X initialized in Programmer Mode");
        } else {
            return Err("No output connection available".into());
        }

        Ok(())
    }

    /// Converts a row and column to pad index
    fn rc_to_pad(&self, row: usize, col: usize) -> u8 {
        if row > 7 || col > 7 {
            warn!("Invalid row/col: {}/{}", row, col);
            return 0;
        }

        // In Programmer Mode, pad numbering for the main grid:
        // 11 12 13 14 15 16 17 18  (top row)
        // 21 22 23 24 25 26 27 28  (second row)
        // ... and so on
        // First digit is row+1, second digit is col+1
        (10 * (row + 1) + (col + 1)) as u8
    }

    /// Converts Launchpad note to our internal pad index (0-63)
    fn note_to_pad_index(&self, note: u8) -> Option<u8> {
        // LaunchPad X in custom mode 5 sends notes from B-2 to E-5
        // This maps to MIDI note numbers 35-104

        // Check if it's a valid 2-digit format note (for programmer mode)
        if note >= 11 && note <= 88 && note % 10 != 0 && note % 10 <= 8 && note / 10 <= 8 {
            // Extract row and column
            let row = (note / 10) - 1;
            let col = (note % 10) - 1;

            // Convert to our 0-63 pad index
            return Some(row * 8 + col);
        }

        // Try the standard MIDI note mapping (B-2 to E-5 range)
        if note >= 35 && note <= 104 {
            let row = (note - 35) / 8;
            let col = (note - 35) % 8;

            if row < 8 && col < 8 {
                return Some(row * 8 + col);
            }
        }

        None
    }

    /// Convert pad index (0-63) to Launchpad X note
    fn pad_to_note(&self, pad: u8) -> u8 {
        // Get row and column
        let row = pad / 8;
        let col = pad % 8;

        // Convert to Launchpad X programmer mode format
        (10 * (row + 1) + (col + 1)) as u8
    }

    /// Internal method to clear all LEDs
    fn clear_leds_internal(&self, conn: &mut MidiOutputConnection) -> Result<(), Box<dyn Error>> {
        debug!("Clearing all Launchpad X LEDs");

        // Standard way to clear all LEDs - use SysEx to reset all pad LEDs at once
        let sysex = [0xF0, 0x00, 0x20, 0x29, 0x02, 0x0C, 0x0E, 0x00, 0x00, 0xF7];
        conn.send(&sysex)?;

        Ok(())
    }

    /// Convert RGB color to Launchpad X color index
    /// Launchpad X in Programmer mode can accept RGB directly via SysEx
    fn send_rgb_color(&self, conn: &mut MidiOutputConnection, pad: u8, color: Rgb) -> Result<(), Box<dyn Error>> {
        // Use SysEx to send RGB directly (more accurate but slower)
        let sysex = [
            0xF0, 0x00, 0x20, 0x29, 0x02, 0x0C, 0x03, // Header
            pad,                   // Pad number
            color.r,               // Red
            color.g,               // Green  
            color.b,               // Blue
            0xF7                   // End of SysEx
        ];
        conn.send(&sysex)?;

        Ok(())
    }

    /// Convert RGB to velocity color for simple note-based updates
    /// This is faster but less accurate than SysEx RGB
    fn rgb_to_velocity(&self, color: Rgb) -> u8 {
        // Simplified mapping of RGB to Launchpad velocity colors

        // Black/Off
        if color.r == 0 && color.g == 0 && color.b == 0 {
            return 0;
        }

        // Full white
        if color.r > 200 && color.g > 200 && color.b > 200 {
            return 3;
        }

        // Primary and mixed colors - weighted simplification
        let r_weight = (color.r as f32 / 255.0) * 3.0;
        let g_weight = (color.g as f32 / 255.0) * 3.0;
        let b_weight = (color.b as f32 / 255.0) * 3.0;

        // Red range: 5-7
        if r_weight > g_weight && r_weight > b_weight {
            if g_weight > 1.0 { // Red + Green = Yellow-ish
                return 13;
            }
            if b_weight > 1.0 { // Red + Blue = Purple-ish
                return 45;
            }
            return 5 + r_weight as u8; // Pure red
        }

        // Green range: 17-19
        if g_weight > r_weight && g_weight > b_weight {
            if r_weight > 1.0 { // Green + Red = Yellow-ish
                return 13;
            }
            if b_weight > 1.0 { // Green + Blue = Cyan-ish
                return 37;
            }
            return 17 + g_weight as u8; // Pure green
        }

        // Blue range: 45-47
        if b_weight > r_weight && b_weight > g_weight {
            if r_weight > 1.0 { // Blue + Red = Purple-ish
                return 45;
            }
            if g_weight > 1.0 { // Blue + Green = Cyan-ish
                return 37;
            }
            return 45 + b_weight as u8; // Pure blue
        }

        // Fallback - dim white
        return 1;
    }
}

impl MidiGridController for LaunchpadX {
    fn handle_note_input(&mut self, note: u8, velocity: u8) {
        // Filter out aftertouch (velocity > 0 for note-on only)
        if velocity > 0 {
            if let Some(pad) = self.note_to_pad_index(note) {
                debug!("Simulated note input: note={}, mapped to pad={}", note, pad);
                let _ = self.event_bus.publish(Event::PadPressed { pad, velocity });
            }
        }
        // Ignore note-off events (velocity = 0)
    }

    fn set_led(&mut self, pad: u8, color: Rgb) {
        // Convert pad to row/column (our internal 0-63 index)
        let (row, col) = pad_to_row_col(pad);

        // Store in buffer
        if row < 8 && col < 8 {
            self.led_buffer[row][col] = color;
        }

        // Send MIDI message to update LED
        if let Some(conn) = &mut *self.output_connection.lock().unwrap() {
            let launchpad_pad = self.pad_to_note(pad);

            // For frequent updates, use Note On with velocity color (faster)
            let color_index = self.rgb_to_velocity(color);
            let msg = [0x90, launchpad_pad, color_index];

            if let Err(e) = conn.send(&msg) {
                error!("Failed to set LED color: {}", e);
            }
        }
    }

    fn clear_leds(&mut self) {
        // Reset the LED buffer
        self.led_buffer = [[Rgb::black(); 8]; 8];

        // Send MIDI message to clear all LEDs
        if let Some(conn) = &mut *self.output_connection.lock().unwrap() {
            if let Err(e) = self.clear_leds_internal(conn) {
                error!("Failed to clear LEDs: {}", e);
            }
        }
    }

    fn refresh_state(&mut self) {
        // Update all LEDs based on current buffer
        if let Some(conn) = &mut *self.output_connection.lock().unwrap() {
            for row in 0..8 {
                for col in 0..8 {
                    let launchpad_pad = self.rc_to_pad(row, col);
                    let color = self.led_buffer[row][col];

                    // Color to velocity for faster updates
                    let color_index = self.rgb_to_velocity(color);
                    let msg = [0x90, launchpad_pad, color_index];

                    if let Err(e) = conn.send(&msg) {
                        error!("Failed to refresh LED state: {}", e);
                    }
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
            // Format: [Status byte (0xB0 + channel), CC number, Value]
            let msg = [0xB0 | (channel & 0x0F), cc, value];
            conn.send(&msg)?;
            debug!("Sent CC: ch={}, cc={}, value={}", channel, cc, value);
        } else {
            return Err("No output connection available".into());
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

/// Convert a pad index (0-63) to row and column
fn pad_to_row_col(pad: u8) -> (usize, usize) {
    let row = (pad / 8) as usize;
    let col = (pad % 8) as usize;

    (row, col)
}

/// Convert a MIDI note number to a pad index
fn note_to_pad_index(note: u8) -> Option<u8> {
    // Check if it's in programmer mode format (11-88)
    if note >= 11 && note <= 88 && note % 10 != 0 && note % 10 <= 8 && note / 10 <= 8 {
        let row = (note / 10) - 1;
        let col = (note % 10) - 1;
        return Some(row * 8 + col);
    }

    // Try standard MIDI note mapping for B-2 to E-5 range (35-104)
    if note >= 35 && note <= 104 {
        let row = (note - 35) / 8;
        let col = (note - 35) % 8;

        if row < 8 && col < 8 {
            return Some(row * 8 + col);
        }
    }

    None
}

/// Convert a wmidi::U7 to u8
fn u7_to_u8(value: U7) -> u8 {
    u8::from(value)
}

impl Drop for LaunchpadX {
    fn drop(&mut self) {
        // Clean up on drop
        debug!("Cleaning up Launchpad X controller");

        if let Some(conn) = &mut *self.output_connection.lock().unwrap() {
            // Clear all LEDs first
            let _ = self.clear_leds_internal(conn);

            // Exit Programmer Mode (back to default mode)
            let sysex = [0xF0, 0x00, 0x20, 0x29, 0x02, 0x0C, 0x0E, 0x00, 0xF7];
            if let Err(e) = conn.send(&sysex) {
                error!("Failed to exit Programmer Mode: {}", e);
            }
        }

        // Connections will be dropped automatically
        info!("Launchpad X controller cleaned up");
    }
}