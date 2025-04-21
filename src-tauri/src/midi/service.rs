// src-tauri/src/midi/service.rs
use crate::events::{Event, EventBus, EventSubscriber};
use crate::midi::controller::{create_controller, MidiGridController, Rgb};
use crate::model::SharedState;
use midir::{MidiInput, MidiOutput};
use std::error::Error;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::broadcast;
use tokio::task::JoinHandle;
use tracing::{debug, error, info, warn};

/// MidiService handles all MIDI I/O for the application
pub struct MidiService {
    state: SharedState,
    event_bus: EventBus,
    controller: Arc<Mutex<Option<Box<dyn MidiGridController>>>>,
    running: Arc<Mutex<bool>>,
}

impl MidiService {
    /// Create a new MIDI service
    pub fn new(state: SharedState, event_bus: EventBus) -> Result<Self, Box<dyn Error>> {
        info!("Creating MIDI service");

        Ok(Self {
            state,
            event_bus,
            controller: Arc::new(Mutex::new(None)),
            running: Arc::new(Mutex::new(true)),
        })
    }

    /// Initialize the controller
    pub fn initialize_controller(&self) -> Result<(), Box<dyn Error>> {
        let controller_name = {
            let state_guard = self.state.read().unwrap();
            state_guard.project.controller.clone()
        };

        info!("Initializing controller: {}", controller_name);

        // Create controller
        match create_controller(&controller_name, self.event_bus.clone()) {
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

    /// List available MIDI input ports
    pub fn list_input_ports() -> Result<Vec<String>, Box<dyn Error>> {
        let midi_in = MidiInput::new("snap-blaster")?;
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
        let midi_out = MidiOutput::new("snap-blaster")?;
        let ports = midi_out.ports();
        let mut port_names = Vec::new();

        for port in ports {
            if let Ok(name) = midi_out.port_name(&port) {
                port_names.push(name);
            }
        }

        Ok(port_names)
    }

    /// Send CC values for the current snap
    pub fn send_current_snap(&self) -> Result<(), Box<dyn Error>> {
        let state_guard = self.state.read().unwrap();

        // Ensure we have valid indices
        if state_guard.current_bank >= state_guard.project.banks.len() {
            return Err("Invalid bank index".into());
        }

        let bank = &state_guard.project.banks[state_guard.current_bank];

        if state_guard.current_snap >= bank.snaps.len() {
            return Err("Invalid snap index".into());
        }

        let snap = &bank.snaps[state_guard.current_snap];

        // Check if we have a controller
        let mut controller_guard = self.controller.lock().unwrap();
        let controller = controller_guard.as_mut().ok_or("No controller initialized")?;

        // Send each parameter's CC value
        for (i, value) in snap.values.iter().enumerate() {
            if i >= state_guard.project.parameters.len() {
                break;
            }

            let param = &state_guard.project.parameters[i];
            controller.send_cc(0, param.cc, *value)?;

            // Small delay to avoid flooding the MIDI bus
            std::thread::sleep(Duration::from_millis(1));
        }

        Ok(())
    }

    /// Send a specific CC value
    pub fn send_cc(&self, param_id: usize, value: u8) -> Result<(), Box<dyn Error>> {
        let state_guard = self.state.read().unwrap();

        if param_id >= state_guard.project.parameters.len() {
            return Err("Parameter ID out of range".into());
        }

        let param = &state_guard.project.parameters[param_id];

        // Check if we have a controller
        let mut controller_guard = self.controller.lock().unwrap();
        let controller = controller_guard.as_mut().ok_or("No controller initialized")?;

        controller.send_cc(0, param.cc, value)?;

        Ok(())
    }

    /// Update controller LEDs based on current state
    pub fn update_controller_leds(&self) -> Result<(), Box<dyn Error>> {
        let mut controller_guard = self.controller.lock().unwrap();

        // Check if we have a controller
        let controller = match controller_guard.as_mut() {
            Some(c) => c,
            None => return Ok(()),
        };

        // Get current state
        let state_guard = self.state.read().unwrap();
        let current_bank = state_guard.current_bank;
        let current_snap = state_guard.current_snap;

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
            } else if i < state_guard.project.banks.len() {
                Rgb::new(64, 64, 64)  // Other available banks
            } else {
                Rgb::black()  // Banks that don't exist
            };

            controller.set_led(i as u8, color);
        }

        // Highlight available snaps in current bank
        if current_bank < state_guard.project.banks.len() {
            let bank = &state_guard.project.banks[current_bank];

            for (i, snap) in bank.snaps.iter().enumerate() {
                if i == current_snap || i >= 56 {
                    continue;  // Skip current snap (already lit) or out of grid range
                }

                // If snap has a name, it's defined
                if !snap.name.is_empty() {
                    let pad = i as u8 + 8;  // Add 8 to account for modifier row
                    controller.set_led(pad, Rgb::new(30, 30, 30));  // Dim color for defined snaps
                }
            }
        }

        // Apply all LED changes
        controller.refresh_state();

        Ok(())
    }

    /// Start the MIDI service
    pub fn start(self) -> (JoinHandle<()>, Self) {
        // Clone required values for the async task
        let event_bus = self.event_bus.clone();
        let state = self.state.clone();
        let controller = self.controller.clone();
        let running = self.running.clone();

        // Create a new instance to return
        let returned_self = Self {
            state: self.state.clone(),
            event_bus: self.event_bus.clone(),
            controller: self.controller.clone(),
            running: self.running.clone(),
        };

        // Initialize the controller
        if let Err(e) = returned_self.initialize_controller() {
            warn!("Failed to initialize controller: {}", e);
            // Continue anyway - we'll try again if controller is changed
        }

        // Create a new event subscriber
        let mut event_subscriber = EventSubscriber::new(&event_bus, "MidiService");

        info!("Starting MIDI service task");

        // Start the event handling task
        let handle = tokio::spawn(async move {
            info!("MIDI service task started");

            // Initialize controller LEDS
            if let Ok(()) = Self::update_controller_leds_static(&controller, &state) {
                info!("Initial controller LEDs updated");
            }

            // Handle events until shutdown or error
            event_subscriber
                .handle_events(|event| {
                    match event {
                        Event::SnapSelected { bank, snap_id } => {
                            debug!(
                                "Handling SnapSelected event: bank={}, snap_id={}",
                                bank, snap_id
                            );

                            // Update the current state
                            {
                                let mut state_guard = state.write().unwrap();
                                state_guard.current_bank = bank;
                                state_guard.current_snap = snap_id;
                            }

                            // Send CC values for the selected snap
                            if let Err(e) = Self::send_snap_values(&state, &controller) {
                                error!("Failed to send snap values: {}", e);
                            }

                            // Update controller LEDs
                            if let Err(e) = Self::update_controller_leds_static(&controller, &state) {
                                error!("Failed to update controller LEDs: {}", e);
                            }
                        }
                        Event::ParameterEdited { param_id, value } => {
                            debug!(
                                "Handling ParameterEdited event: param_id={}, value={}",
                                param_id, value
                            );

                            // Update the parameter value in the current snap
                            {
                                let mut state_guard = state.write().unwrap();
                                let current_bank = state_guard.current_bank;
                                let current_snap = state_guard.current_snap;
                                let param_count = state_guard.project.parameters.len();

                                if current_bank < state_guard.project.banks.len()
                                    && current_snap
                                    < state_guard.project.banks[current_bank].snaps.len()
                                {
                                    let bank = &mut state_guard.project.banks[current_bank];
                                    let snap = &mut bank.snaps[current_snap];

                                    // First check if we need to extend the values vector
                                    if param_id >= snap.values.len() && param_id < param_count {
                                        // Extend the values if needed
                                        snap.values.resize(param_id + 1, 64); // Default to middle value
                                    }

                                    // Now we can safely set the value
                                    if param_id < snap.values.len() {
                                        snap.values[param_id] = value;
                                    }
                                }
                            }

                            // Send the CC value
                            if let Err(e) = Self::send_parameter_value(&state, &controller, param_id, value) {
                                error!("Failed to send parameter value: {}", e);
                            }
                        }
                        Event::MorphProgressed {
                            progress: _,
                            current_values,
                        } => {
                            debug!(
                                "Handling MorphProgressed event with {} values",
                                current_values.len()
                            );

                            // Send all CC values for the current morph state
                            if let Err(e) = Self::send_morph_values(&state, &controller, &current_values) {
                                error!("Failed to send morph values: {}", e);
                            }
                        }
                        Event::PadPressed { pad, velocity } => {
                            debug!(
                                "Handling PadPressed event: pad={}, velocity={}",
                                pad, velocity
                            );

                            // Process the pad press - convert to snap selection if it's not a modifier
                            if velocity > 0 {  // Only handle note-on events
                                if pad < 8 {
                                    // Top row (0-7) is for bank selection
                                    let bank_id = pad as usize;
                                    let bank_count = {
                                        let state_guard = state.read().unwrap();
                                        state_guard.project.banks.len()
                                    };

                                    if bank_id < bank_count {
                                        // Select this bank
                                        debug!("Bank selection pad pressed: {}", bank_id);
                                        let _ = event_bus.publish(Event::BankSelected { bank_id });

                                        // Update controller LEDs
                                        if let Err(e) = Self::update_controller_leds_static(&controller, &state) {
                                            error!("Failed to update controller LEDs: {}", e);
                                        }
                                    }
                                } else {
                                    // Other pads (8-63) are for snap selection
                                    let snap_id = (pad - 8) as usize;
                                    let bank_id = {
                                        let state_guard = state.read().unwrap();
                                        state_guard.current_bank
                                    };

                                    // Check if this snap exists
                                    let snap_exists = {
                                        let state_guard = state.read().unwrap();
                                        bank_id < state_guard.project.banks.len() &&
                                            snap_id < state_guard.project.banks[bank_id].snaps.len() &&
                                            !state_guard.project.banks[bank_id].snaps[snap_id].name.is_empty()
                                    };

                                    if snap_exists {
                                        debug!("Snap selection pad pressed: {}", snap_id);
                                        let _ = event_bus.publish(Event::SnapSelected {
                                            bank: bank_id,
                                            snap_id,
                                        });
                                    } else {
                                        // Empty pad - could create a new snap here
                                        debug!("Empty pad pressed: {}", pad);
                                        // Future: Could send an event to create a new snap
                                    }
                                }
                            }
                        }
                        Event::BankSelected { bank_id } => {
                            debug!("Handling BankSelected event: bank_id={}", bank_id);

                            // Update the current bank
                            {
                                let mut state_guard = state.write().unwrap();
                                state_guard.current_bank = bank_id;

                                // Also select the first snap in this bank if available
                                if bank_id < state_guard.project.banks.len() &&
                                    !state_guard.project.banks[bank_id].snaps.is_empty() {
                                    state_guard.current_snap = 0;
                                }
                            }

                            // Send CC values for the first snap in the bank
                            if let Err(e) = Self::send_snap_values(&state, &controller) {
                                error!("Failed to send snap values: {}", e);
                            }

                            // Update controller LEDs
                            if let Err(e) = Self::update_controller_leds_static(&controller, &state) {
                                error!("Failed to update controller LEDs: {}", e);
                            }
                        }
                        Event::ProjectLoaded => {
                            debug!("Handling ProjectLoaded event");

                            // Reset state
                            {
                                let mut state_guard = state.write().unwrap();
                                state_guard.current_bank = 0;
                                state_guard.current_snap = 0;
                            }

                            // Initialize controller with new project settings
                            let controller_name = {
                                let state_guard = state.read().unwrap();
                                state_guard.project.controller.clone()
                            };

                            // Check if we need to create a new controller
                            let need_new_controller = {
                                let controller_guard = controller.lock().unwrap();
                                controller_guard.is_none() ||
                                    controller_guard.as_ref().map(|c| c.get_name() != controller_name).unwrap_or(true)
                            };

                            if need_new_controller {
                                // Create a new controller
                                match create_controller(&controller_name, event_bus.clone()) {
                                    Ok(new_controller) => {
                                        info!("Created new controller: {}", controller_name);
                                        let mut controller_guard = controller.lock().unwrap();
                                        *controller_guard = Some(new_controller);
                                    },
                                    Err(e) => {
                                        error!("Failed to create controller: {}", e);
                                    }
                                }
                            }

                            // Send initial snap values
                            if let Err(e) = Self::send_snap_values(&state, &controller) {
                                error!("Failed to send initial snap values: {}", e);
                            }

                            // Update controller LEDs
                            if let Err(e) = Self::update_controller_leds_static(&controller, &state) {
                                error!("Failed to update controller LEDs: {}", e);
                            }
                        }
                        Event::Shutdown => {
                            info!("Shutting down MIDI service");
                            *running.lock().unwrap() = false;

                            // Clean up controller
                            let mut controller_guard = controller.lock().unwrap();
                            if let Some(c) = controller_guard.as_mut() {
                                c.clear_leds();
                                c.refresh_state();
                            }

                            return false; // Stop handling events
                        }
                        _ => {}
                    }

                    true // Continue handling events
                })
                .await;
        });

        (handle, returned_self)
    }

    // Helper methods 

    /// Static method for updating controller LEDs
    fn update_controller_leds_static(controller: &Arc<Mutex<Option<Box<dyn MidiGridController>>>>, state: &SharedState) -> Result<(), Box<dyn Error>> {
        let mut controller_guard = controller.lock().unwrap();

        // Check if we have a controller
        let controller = match controller_guard.as_mut() {
            Some(c) => c,
            None => return Ok(()),
        };

        // Clear all LEDs first
        controller.clear_leds();

        // Get current state
        let state_guard = state.read().unwrap();
        let current_bank = state_guard.current_bank;
        let current_snap = state_guard.current_snap;

        // Set active snap LED
        if current_snap < 56 {  // Make sure it's within our grid
            let pad = current_snap as u8 + 8;  // Add 8 to account for modifier row
            controller.set_led(pad, Rgb::orange());
        }

        // Set bank indicators in the top row
        for i in 0..8 {
            let color = if i == current_bank {
                Rgb::blue()  // Current bank
            } else if i < state_guard.project.banks.len() {
                Rgb::new(64, 64, 64)  // Other available banks
            } else {
                Rgb::black()  // Banks that don't exist
            };

            controller.set_led(i as u8, color);
        }

        // Highlight available snaps in current bank
        if current_bank < state_guard.project.banks.len() {
            let bank = &state_guard.project.banks[current_bank];

            for (i, snap) in bank.snaps.iter().enumerate() {
                if i == current_snap || i >= 56 {
                    continue;  // Skip current snap (already lit) or out of grid range
                }

                // If snap has a name, it's defined
                if !snap.name.is_empty() {
                    let pad = i as u8 + 8;  // Add 8 to account for modifier row
                    controller.set_led(pad, Rgb::new(30, 30, 30));  // Dim color for defined snaps
                }
            }
        }

        // Apply all LED changes
        controller.refresh_state();

        Ok(())
    }

    /// Send all snap values (static method that doesn't require ownership of Self)
    fn send_snap_values(state: &SharedState, controller: &Arc<Mutex<Option<Box<dyn MidiGridController>>>>) -> Result<(), Box<dyn Error>> {
        let state_guard = state.read().unwrap();

        if state_guard.current_bank >= state_guard.project.banks.len() {
            return Err("Bank index out of range".into());
        }

        let bank = &state_guard.project.banks[state_guard.current_bank];

        // Ensure the snap exists at this index
        if state_guard.current_snap >= bank.snaps.len() {
            return Err("Snap index out of range".into());
        }

        let snap = &bank.snaps[state_guard.current_snap];

        // Lock the controller to send CC values
        let mut controller_guard = controller.lock().unwrap();
        let controller = controller_guard.as_mut().ok_or("No controller initialized")?;

        // Send each parameter's CC value
        for (i, value) in snap.values.iter().enumerate() {
            if i >= state_guard.project.parameters.len() {
                break;
            }

            let param = &state_guard.project.parameters[i];
            if let Err(e) = controller.send_cc(0, param.cc, *value) {
                error!("Error sending CC: {}", e);
            }

            // Small delay to avoid flooding the MIDI bus
            std::thread::sleep(Duration::from_millis(1));
        }

        Ok(())
    }

    /// Send a single parameter value
    fn send_parameter_value(
        state: &SharedState,
        controller: &Arc<Mutex<Option<Box<dyn MidiGridController>>>>,
        param_id: usize,
        value: u8,
    ) -> Result<(), Box<dyn Error>> {
        let state_guard = state.read().unwrap();

        if param_id >= state_guard.project.parameters.len() {
            return Err("Parameter ID out of range".into());
        }

        let param = &state_guard.project.parameters[param_id];

        // Lock the controller to send CC value
        let mut controller_guard = controller.lock().unwrap();
        let controller = controller_guard.as_mut().ok_or("No controller initialized")?;

        controller.send_cc(0, param.cc, value)?;

        Ok(())
    }

    /// Send morph values
    fn send_morph_values(
        state: &SharedState,
        controller: &Arc<Mutex<Option<Box<dyn MidiGridController>>>>,
        current_values: &[u8],
    ) -> Result<(), Box<dyn Error>> {
        let state_guard = state.read().unwrap();
        let mut controller_guard = controller.lock().unwrap();

        let controller = controller_guard.as_mut().ok_or("No controller initialized")?;

        for (i, value) in current_values.iter().enumerate() {
            if i >= state_guard.project.parameters.len() {
                break;
            }

            let param = &state_guard.project.parameters[i];
            if let Err(e) = controller.send_cc(0, param.cc, *value) {
                error!("Error sending morph CC: {}", e);
            }

            // Small delay to avoid flooding the MIDI bus
            std::thread::sleep(Duration::from_millis(1));
        }

        Ok(())
    }
}
