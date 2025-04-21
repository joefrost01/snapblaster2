use crate::events::{Event, EventBus};
use crate::midi::controller::{create_controller, MidiGridController, Rgb};
use crate::model::SharedState;
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
    state: Option<SharedState>,
}

impl Clone for MidiManager {
    fn clone(&self) -> Self {
        Self {
            event_bus: self.event_bus.clone(),
            controller: self.controller.clone(),
            output_connections: self.output_connections.clone(),
            state: self.state.clone(),
        }
    }
}

impl MidiManager {
    /// Create a new MIDI manager
    pub fn new(event_bus: EventBus, state: Option<SharedState>) -> Self {
        Self {
            event_bus,
            controller: Arc::new(Mutex::new(None)),
            output_connections: Arc::new(Mutex::new(Vec::new())),
            state,
        }
    }

    /// Set the shared state (call after initialization)
    pub fn set_state(&mut self, state: SharedState) {
        self.state = Some(state);
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

                // Update LEDs right away
                drop(controller_guard);
                self.update_controller_leds()?;

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

    /// Update controller LEDs to match current state
    pub fn update_controller_leds(&self) -> Result<(), Box<dyn Error>> {
        let mut controller_guard = self.controller.lock().unwrap();

        if let Some(controller) = controller_guard.as_mut() {  // Change as_ref() to as_mut()
            // Check if we have a state
            if let Some(state) = &self.state {
                let state_guard = state.read().unwrap();

                // First, clear all LEDs
                controller.clear_leds();

                let current_bank = state_guard.current_bank;
                let current_snap = state_guard.current_snap;

                // Light top row (modifier row) blue, with active bank brighter
                for i in 0..8 {
                    let color = if i == current_bank {
                        // Current bank gets bright blue
                        Rgb::new(0, 64, 255)
                    } else if i < state_guard.project.banks.len() {
                        // Available banks get dim blue
                        Rgb::new(0, 32, 128)
                    } else {
                        // Non-existent banks are off
                        Rgb::black()
                    };

                    controller.set_led(i as u8, color);  // Convert i to u8
                }

                // Light pads with snaps
                if current_bank < state_guard.project.banks.len() {
                    let bank = &state_guard.project.banks[current_bank];

                    // Go through each snap position (pad 8-63 map to snap 0-55)
                    for i in 0..56 {
                        let snap_idx = i;
                        let pad_idx = i + 8; // Add 8 to account for modifier row

                        if snap_idx < bank.snaps.len() && !bank.snaps[snap_idx].name.is_empty() {
                            let color = if snap_idx == current_snap {
                                // Current snap is orange
                                Rgb::new(255, 128, 0)
                            } else {
                                // Other snaps are yellow
                                Rgb::new(255, 255, 0)
                            };

                            controller.set_led(pad_idx as u8, color);  // Convert pad_idx to u8
                        }
                    }
                }

                // Refresh the controller to apply all LED changes
                controller.refresh_state();
            } else {
                // No state, just clear all LEDs
                controller.clear_leds();
                controller.refresh_state();
            }
        }

        Ok(())
    }

    /// Handle a pad press event from the hardware controller
    pub async fn handle_pad_pressed(&self, pad: u8, velocity: u8) -> Result<(), Box<dyn Error>> {
        // Only handle note-on events (velocity > 0)
        if velocity == 0 {
            return Ok(());
        }

        // Check if we have a state
        if let Some(state) = &self.state {
            let state_guard = state.read().unwrap();

            // Check if this is a modifier (top row, pads 0-7)
            if pad < 8 {
                // This is a modifier/bank selection pad
                // For now we don't handle bank switching - could implement later
                return Ok(());
            }

            // Regular snap pad (8-63 map to snaps 0-55)
            let snap_id = (pad - 8) as usize;
            let bank_id = state_guard.current_bank;

            // Check if this is a valid snap position
            if bank_id < state_guard.project.banks.len() {
                let bank = &state_guard.project.banks[bank_id];

                if snap_id < bank.snaps.len() && !bank.snaps[snap_id].name.is_empty() {
                    // Valid snap, trigger select
                    drop(state_guard); // Release lock before publishing event

                    // Publish select snap event - this will be handled by the backend
                    self.event_bus.publish(Event::SnapSelected {
                        bank: bank_id,
                        snap_id,
                    })?;

                    // Update LEDs to reflect the change
                    self.update_controller_leds()?;
                }
            }
        }

        Ok(())
    }
}