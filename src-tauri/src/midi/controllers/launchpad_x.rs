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
            // Set Launchpad X to Programmer Mode (0x01 at the end is key)
            info!("Setting Launchpad X to Programmer Mode");

            // This SysEx command sets Programmer mode
            let sysex = [0xF0, 0x00, 0x20, 0x29, 0x02, 0x0C, 0x0E, 0x01, 0xF7];
            //F0h 00h 20h 29h 02h 0Ch 0Eh <mode> F7h
            conn.send(&sysex)?;

            // Add a longer delay to ensure mode changes completely
            thread::sleep(Duration::from_millis(200));

            // Clear all LEDs first
            self.clear_leds_internal(conn)?;

            // Set up the grid with initial state
            // ... rest of your initialization code

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
        // 67 68 69 70 71 72 73 74 (top row)
        // 59 60 61 62 63 64 65 66
        // 41 52 53 54 55 56 57 58
        // 43 44 45 46 47 48 49 50
        // 35 36 37 38 39 40 41 42
        // 27 28 29 30 31 32 33 34
        // 19 20 21 22 23 24 25 26
        // 11 12 13 14 15 16 17 18  (bottom row)
        // ... and so on
        // First digit is row+1, second digit is col+1
        // Invert the row mapping (0->8, 1->7, 2->6, etc.) to match physical layout
        (10 * (8 - row) + (col + 1)) as u8
    }

    /// Convert pad index (0-63) to Launchpad X note
    fn pad_to_note(&self, pad: u8) -> u8 {
        // Get row and column
        let row = pad / 8;
        let col = pad % 8;

        // Convert to Launchpad X programmer mode format
        // Invert the row mapping (0->8, 1->7, 2->6, etc.) to match physical layout
        (10 * (8 - row) + (col + 1)) as u8
    }

    /// Internal method to clear all LEDs
    fn clear_leds_internal(&self, conn: &mut MidiOutputConnection) -> Result<(), Box<dyn Error>> {
        debug!("Clearing all Launchpad X LEDs");

        // We'll clear each pad individually rather than using SysEx
        // This is more reliable on some controllers
        for row in 0..8 {
            for col in 0..8 {
                let pad = self.rc_to_pad(row, col);
                let msg = [0x90, pad, 0]; // Turn off with velocity 0
                conn.send(&msg)?;
            }
        }

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

        // OFF/Very dim (empty slots)
        if color.r <= 20 && color.g <= 20 && color.b <= 20 {
            return 0; // Dim white
        }

        // GREEN (selected snap) - prioritize this
        if color.g > 200 && color.r < 100 && color.b < 100 {
            return 21; // Bright green
        }

        // RED (modifiers/top row)
        if color.r > 200 && color.g < 100 && color.b < 100 {
            return 5; // Bright red
        }

        // YELLOW (available snaps)
        if color.r > 200 && color.g > 200 && color.b < 100 {
            return 13; // Bright yellow
        }

        // Full white as fallback
        if color.r > 200 && color.g > 200 && color.b > 200 {
            return 0;
        }

        // If we get here, do some simple logic based on the dominant color
        let r_weight = (color.r as f32 / 255.0) * 3.0;
        let g_weight = (color.g as f32 / 255.0) * 3.0;
        let b_weight = (color.b as f32 / 255.0) * 3.0;

        // Red range: 5-7
        if r_weight > g_weight && r_weight > b_weight {
            return 5 + r_weight as u8; // Pure red
        }

        // Green range: 17-19
        if g_weight > r_weight && g_weight > b_weight {
            return 17 + g_weight as u8; // Pure green
        }

        // Blue range: 45-47
        if b_weight > r_weight && b_weight > g_weight {
            return 45 + b_weight as u8; // Pure blue
        }

        // Fallback - dim white
        return 0;
    }
}

impl MidiGridController for LaunchpadX {
    fn handle_note_input(&mut self, note: u8, velocity: u8) {
        // Only process note-on events (velocity > 0)
        if velocity > 0 {
            if let Some(pad) = note_to_pad_index(note) {
                debug!("Received note: {}, mapped to pad: {}", note, pad);

                // Publish event through the event bus
                self.event_bus.try_publish(Event::PadPressed {
                    pad,
                    velocity
                });
            } else {
                debug!("Note {} does not map to a valid pad", note);
            }
        }
    }

    fn set_led(&mut self, pad: u8, color: Rgb) {
        // Calculate row/column
        let row = (pad / 8) as usize;
        let col = (pad % 8) as usize;

        // Only update if in range and color is actually different
        if row < 8 && col < 8 {
            // Get the current color from the buffer
            let current_color = self.led_buffer[row][col];

            // Only update if color has changed to avoid unnecessary MIDI traffic
            if current_color != color {
                // Update buffer
                self.led_buffer[row][col] = color;

                // Send MIDI message
                if let Some(conn) = &mut *self.output_connection.lock().unwrap() {
                    let launchpad_pad = self.pad_to_note(pad);
                    let color_index = self.rgb_to_velocity(color);

                    // Use Note On message for faster and more reliable updates
                    let msg = [0x90, launchpad_pad, color_index];

                    if let Err(e) = conn.send(&msg) {
                        error!("Failed to set LED color: {}", e);
                    }
                }
            }
        }
    }

    fn clear_leds(&mut self) {
        debug!("Clearing all Launchpad X LEDs");

        // Reset our buffer first
        for row in 0..8 {
            for col in 0..8 {
                self.led_buffer[row][col] = Rgb::black();
            }
        }

        // Then send to hardware
        if let Some(conn) = &mut *self.output_connection.lock().unwrap() {
            if let Err(e) = self.clear_leds_internal(conn) {
                error!("Failed to clear LEDs: {}", e);
            }
        }
    }

    fn refresh_state(&mut self) {
        // Update all LEDs from our buffer
        if let Some(conn) = &mut *self.output_connection.lock().unwrap() {
            for row in 0..8 {
                for col in 0..8 {
                    let launchpad_pad = self.rc_to_pad(row, col);
                    let color = self.led_buffer[row][col];
                    let color_index = self.rgb_to_velocity(color);

                    // Avoid unnecessary 0 values (they might cause mode issues)
                    if color_index > 0 {
                        let msg = [0x90, launchpad_pad, color_index];

                        if let Err(e) = conn.send(&msg) {
                            error!("Failed to refresh LED state: {}", e);
                        }
                    }
                }
            }
        }
    }

    fn get_name(&self) -> &str {
        "Launchpad X"
    }

    fn set_progress_led(&mut self, pad: u8, progress: f64) -> Result<(), Box<dyn Error>> {
        if let Some(conn) = &mut *self.output_connection.lock().unwrap() {
            // Calculate a pulsing color based on the progress
            // Blue (low progress) to Green (high progress) transition
            let blue = ((1.0 - progress) * 255.0) as u8;
            let green = (progress * 255.0) as u8;

            // Add a pulse effect based on progress
            let pulse = (((progress * 10.0) % 1.0) * 0.5 + 0.5) as f64; // Oscillates between 0.5 and 1.0

            // Apply pulse to brightness
            let b = (blue as f64 * pulse) as u8;
            let g = (green as f64 * pulse) as u8;

            let color = Rgb::new(0, g, b);

            // Use the RGB SysEx method for more accurate color control
            self.send_rgb_color(conn, pad, color)?;
        }

        Ok(())
    }

    fn set_morph_target_led(&mut self, pad: u8) -> Result<(), Box<dyn Error>> {
        if let Some(conn) = &mut *self.output_connection.lock().unwrap() {
            // Use a distinctive color for the morph target (purple)
            let color = Rgb::new(180, 0, 180);
            self.send_rgb_color(conn, pad, color)?;
        }

        Ok(())
    }

    fn set_active_modifier_led(&mut self, pad: u8) -> Result<(), Box<dyn Error>> {
        if let Some(conn) = &mut *self.output_connection.lock().unwrap() {
            // Bright green for active modifiers
            let color = Rgb::new(0, 255, 0);
            self.send_rgb_color(conn, pad, color)?;
        }

        Ok(())
    }

    fn send_cc(&mut self, channel: u8, cc: u8, value: u8) -> Result<(), Box<dyn Error>> {
        // Send CC message to output port
        // if let Some(conn) = &mut *self.output_connection.lock().unwrap() {
        //     // Format: [Status byte (0xB0 + channel), CC number, Value]
        //     let msg = [0xB0 | (channel & 0x0F), cc, value];
        //     conn.send(&msg)?;
        //     debug!("Sent CC: ch={}, cc={}, value={}", channel, cc, value);
        // } else {
        //     return Err("No output connection available".into());
        // }
        debug!("LaunchPad X was asked to send CC: ch={}, cc={}, value={}", channel, cc, value);

        Ok(())
    }
}

/// Helper function to find a Launchpad port by name
fn find_launchpad_port<T>(ports: &[T], conn: &impl PortInfos<T>, name: &str) -> Option<T>
where
    T: Clone,
{
    // First try to find a port with both the controller name and "MIDI" in it
    // This ensures we get the MIDI port and not the DAW port
    let midi_port = ports
        .iter()
        .find(|port| {
            if let Ok(port_name) = conn.port_name(port) {
                port_name.contains(name) && port_name.contains("MIDI")
            } else {
                false
            }
        });

    if midi_port.is_some() {
        return midi_port.cloned();
    }

    // Fallback to the original behavior if no MIDI-specific port is found
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
    // For Programmer Mode, the pads are mapped as follows:
    // Top row: 91, 92, 93, 94, 95, 96, 97, 98
    // Second row: 81, 82, 83, 84, 85, 86, 87, 88
    // and so on...

    // Check if it's in programmer mode format
    if note >= 11 && note <= 99 {
        let row = (note / 10) - 1;
        let col = (note % 10) - 1;

        // Validate row and column
        if col < 8 && row < 8 {
            // This conversion ensures we map properly to 0-63 pad indices
            return Some((7 - row) * 8 + col);
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
