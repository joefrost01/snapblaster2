use crate::events::{Event, EventBus};
use crate::midi::controller::{create_controller, MidiGridController, Rgb};
use midir::{Ignore, MidiInput, MidiOutput, MidiOutputConnection};
use std::error::Error;
use std::sync::{Arc, Mutex, RwLock};
use std::time::Duration;
use tracing::{debug, error, info, warn};

// Import virtual port support for different platforms
#[cfg(target_os = "macos")]
use midir::os::unix::VirtualOutput;
#[cfg(target_os = "linux")]
use midir::os::unix::VirtualOutput;

/// Main MIDI manager for Snap-Blaster
pub struct MidiManager {
    event_bus: EventBus,
    controller: Arc<Mutex<Option<Box<dyn MidiGridController>>>>,
    output_connections: Arc<Mutex<Vec<(String, MidiOutputConnection)>>>,
}

impl MidiManager {
    /// Create a new MIDI manager
    pub fn new(event_bus: EventBus) -> Self {
        Self {
            event_bus,
            controller: Arc::new(Mutex::new(None)),
            output_connections: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// List available MIDI input ports
    pub fn list_input_ports() -> Result<Vec<String>, Box<dyn Error>> {
        let midi_in = MidiInput::new("Snap-Blaster")?;
        let ports = midi_in.ports();
        let mut port_names = Vec::new();

        for port in ports {
            if let Ok(name) = midi_in.port_name(&port) {
                port_names.push(name);
            }
        }

        Ok(port_names)
    }

    /// List available MIDI output ports
    pub fn list_output_ports() -> Result<Vec<String>, Box<dyn Error>> {
        let midi_out = MidiOutput::new("Snap-Blaster")?;
        let ports = midi_out.ports();
        let mut port_names = Vec::new();

        for port in ports {
            if let Ok(name) = midi_out.port_name(&port) {
                port_names.push(name);
            }
        }

        Ok(port_names)
    }

    /// Initialize MIDI controller
    pub fn initialize_controller(&self, controller_name: &str) -> Result<(), Box<dyn Error>> {
        info!("Initializing controller: {}", controller_name);

        // Create controller
        match create_controller(controller_name, self.event_bus.clone()) {
            Ok(controller) => {
                info!("Successfully created {} controller", controller_name);
                let mut controller_guard = self.controller.lock().unwrap();
                *controller_guard = Some(controller);
                Ok(())
            },
            Err(e) => {
                error!("Failed to create controller {}: {}", controller_name, e);
                Err(e)
            }
        }
    }

    /// Create a virtual MIDI port
    pub fn create_virtual_port(&self, port_name: &str) -> Result<(), Box<dyn Error>> {
        info!("Creating virtual MIDI port: {}", port_name);

        // Different implementations for different platforms
        #[cfg(target_os = "macos")]
        {
            let midi_out = MidiOutput::new("Snap-Blaster-Virtual")?;
            match midi_out.create_virtual(port_name) {
                Ok(conn) => {
                    info!("Created virtual MIDI port: {}", port_name);
                    let mut connections = self.output_connections.lock().unwrap();
                    connections.push((port_name.to_string(), conn));
                    Ok(())
                },
                Err(e) => {
                    error!("Failed to create virtual MIDI port: {}", e);
                    Err(e.into())
                }
            }
        }

        #[cfg(target_os = "windows")]
        {
            warn!("Virtual MIDI ports not directly supported on Windows");
            warn!("Please use loopMIDI to create a virtual MIDI port named '{}'", port_name);
            Ok(())  // Return OK even though we didn't create it
        }

        #[cfg(target_os = "linux")]
        {
            let midi_out = MidiOutput::new("Snap-Blaster-Virtual")?;
            match midi_out.create_virtual(port_name) {
                Ok(conn) => {
                    info!("Created virtual MIDI port: {}", port_name);
                    let mut connections = self.output_connections.lock().unwrap();
                    connections.push((port_name.to_string(), conn));
                    Ok(())
                },
                Err(e) => {
                    error!("Failed to create virtual MIDI port: {}", e);
                    Err(e.into())
                }
            }
        }

        #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
        {
            Err("Virtual MIDI ports not supported on this platform".into())
        }
    }

    /// Send a MIDI CC message to all available ports
    pub fn send_cc(&self, channel: u8, cc: u8, value: u8) -> Result<(), Box<dyn Error>> {
        // Send via controller if we have one
        {
            let mut controller_guard = self.controller.lock().unwrap();
            if let Some(controller) = controller_guard.as_mut() {
                if let Err(e) = controller.send_cc(channel, cc, value) {
                    warn!("Failed to send CC via controller: {}", e);
                }
            }
        }

        // Send via virtual ports
        {
            let mut connections = self.output_connections.lock().unwrap();
            for (name, conn) in connections.iter_mut() {
                // Create a CC message: [Status byte (0xB0 + channel), CC#, Value]
                let message = [0xB0 | (channel & 0x0F), cc, value];

                if let Err(e) = conn.send(&message) {
                    warn!("Failed to send CC to {}: {}", name, e);
                } else {
                    debug!("Sent CC to {}: ch={}, cc={}, val={}", name, channel, cc, value);
                }
            }
        }

        Ok(())
    }

    /// Send values for a snap to all MIDI outputs
    pub fn send_snap_values(&self, parameters: &[(u8, u8)]) -> Result<(), Box<dyn Error>> {
        // Log what we're sending
        debug!("Sending {} parameter values via MIDI", parameters.len());

        // For each CC/value pair
        for &(cc, value) in parameters {
            // Send each CC to all outputs on MIDI channel 1 (zero-indexed as 0)
            self.send_cc(0, cc, value)?;

            // Small delay to avoid flooding the MIDI bus
            std::thread::sleep(Duration::from_millis(2));
        }

        debug!("Finished sending all parameter values");
        Ok(())
    }
    
    /// Update controller LEDs
    pub fn update_leds(&self,
                       current_bank: usize,
                       current_snap: usize,
                       bank_count: usize,
                       snap_states: &[bool]) -> Result<(), Box<dyn Error>> {
        let mut controller_guard = self.controller.lock().unwrap();

        // Check if we have a controller
        if let Some(controller) = controller_guard.as_mut() {
            // Clear all LEDs first
            controller.clear_leds();

            // Set active snap LED
            if current_snap < 56 {  // Make sure it's within our grid
                let pad = current_snap as u8 + 8;  // Add 8 to account for modifier row
                controller.set_led(pad, Rgb::orange());
            }

            // Set bank indicators in the top row
            for i in 0..8 {
                let color = if i == current_bank {
                    Rgb::blue()  // Current bank
                } else if i < bank_count {
                    Rgb::new(64, 64, 64)  // Other available banks
                } else {
                    Rgb::black()  // Banks that don't exist
                };

                controller.set_led(i as u8, color);
            }

            // Highlight available snaps
            for (i, &has_snap) in snap_states.iter().enumerate() {
                if i == current_snap || i >= 56 {
                    continue;  // Skip current snap (already lit) or out of grid range
                }

                if has_snap {
                    let pad = i as u8 + 8;  // Add 8 to account for modifier row
                    controller.set_led(pad, Rgb::new(30, 30, 30));  // Dim color for defined snaps
                }
            }

            // Apply all LED changes
            controller.refresh_state();
        }

        Ok(())
    }
}