use crate::events::{Event, EventBus, EventSubscriber};
use crate::midi::controller::{create_controller, MidiGridController, Rgb};
use crate::model::SharedState;
use midir::{MidiInput, MidiOutput};
use std::error::Error;
use std::sync::{Arc, Mutex};
use tokio::sync::broadcast;
use tokio::task::JoinHandle;
use tracing::{debug, error, info};

/// MidiService handles all MIDI I/O for the application
pub struct MidiService {
    state: SharedState,
    event_bus: EventBus,
    controller: Arc<Mutex<Box<dyn MidiGridController>>>,
    event_receiver: broadcast::Receiver<Event>,
    running: Arc<Mutex<bool>>,
}

impl MidiService {
    /// Create a new MIDI service
    pub fn new(state: SharedState, event_bus: EventBus) -> Result<Self, Box<dyn Error>> {
        let controller_name = {
            let state_guard = state.read().unwrap();
            state_guard.project.controller.clone()
        };

        info!("Creating MIDI service with controller: {}", controller_name);

        // Create controller
        let controller = match create_controller(&controller_name, event_bus.clone()) {
            Ok(c) => c,
            Err(e) => {
                error!("Failed to create controller '{}': {}", controller_name, e);
                // Use a fallback controller - a more generic dummy implementation
                create_controller("Generic", event_bus.clone())?
            }
        };

        let event_receiver = event_bus.subscribe();

        Ok(Self {
            state,
            event_bus: event_bus.clone(),
            controller: Arc::new(Mutex::new(controller)),
            event_receiver,
            running: Arc::new(Mutex::new(true)),
        })
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
        let bank = &state_guard.project.banks[state_guard.current_bank];
        let snap = &bank.snaps[state_guard.current_snap];

        // Lock the controller to send CC values
        let mut controller = self.controller.lock().unwrap();

        // Send each parameter's CC value
        for (i, value) in snap.values.iter().enumerate() {
            if i >= state_guard.project.parameters.len() {
                break;
            }

            let param = &state_guard.project.parameters[i];
            controller.send_cc(0, param.cc, *value)?;
        }

        // Update controller LEDs to reflect current state
        controller.refresh_state();

        Ok(())
    }

    /// Send a specific CC value
    pub fn send_cc(&self, param_id: usize, value: u8) -> Result<(), Box<dyn Error>> {
        let state_guard = self.state.read().unwrap();

        if param_id >= state_guard.project.parameters.len() {
            return Err("Parameter ID out of range".into());
        }

        let param = &state_guard.project.parameters[param_id];
        let mut controller = self.controller.lock().unwrap();
        controller.send_cc(0, param.cc, value)?;

        Ok(())
    }

    /// Start the MIDI service
    pub fn start(self) -> (JoinHandle<()>, Self) {
        // Create a new instance to return
        let returned_self = Self {
            state: self.state.clone(),
            event_bus: self.event_bus.clone(),
            controller: self.controller.clone(),
            event_receiver: self.event_bus.subscribe(), // Get a fresh receiver
            running: self.running.clone(),
        };

        // Use the original self in the async block
        let running = self.running.clone();
        let controller = self.controller.clone();
        let state = self.state.clone();
        let event_bus = self.event_bus.clone();

        // Create a new event subscriber
        let mut event_subscriber = EventSubscriber::new(&self.event_bus, "MidiService");

        info!("Starting MIDI service");

        // Initialize controller state
        {
            let mut controller_lock = controller.lock().unwrap();
            controller_lock.clear_leds();
            controller_lock.refresh_state();
        }

        let handle = tokio::spawn(async move {
            info!("MIDI service task started");

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

                            // Send CC values
                            Self::send_snap_values(&state, &controller);

                            // Update controller LEDs
                            self.update_controller_leds(&controller, bank, snap_id);
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

                                if current_bank < state_guard.project.banks.len()
                                    && current_snap
                                        < state_guard.project.banks[current_bank].snaps.len()
                                {
                                    let bank = &mut state_guard.project.banks[current_bank];
                                    let snap = &mut bank.snaps[current_snap];

                                    if param_id < snap.values.len() {
                                        snap.values[param_id] = value;
                                    }
                                }
                            }

                            // Send the CC value
                            if let Ok(param) = Self::get_parameter(&state, param_id) {
                                let mut controller_lock = controller.lock().unwrap();
                                let _ = controller_lock.send_cc(0, param.cc, value);
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
                            Self::send_morph_values(&state, &controller, &current_values);
                        }
                        Event::PadPressed { pad, velocity } => {
                            debug!(
                                "Handling PadPressed event: pad={}, velocity={}",
                                pad, velocity
                            );

                            // Process the pad press - convert to snap selection if it's not a modifier
                            if pad >= 8 {
                                // First row (0-7) are modifiers
                                let snap_id = pad as usize - 8;
                                let bank_id = {
                                    let state_guard = state.read().unwrap();
                                    state_guard.current_bank
                                };

                                // Only select if the snap exists
                                let snap_exists = {
                                    let state_guard = state.read().unwrap();
                                    bank_id < state_guard.project.banks.len()
                                        && snap_id < state_guard.project.banks[bank_id].snaps.len()
                                };

                                if snap_exists && velocity > 0 {
                                    let _ = event_bus.publish(Event::SnapSelected {
                                        bank: bank_id,
                                        snap_id,
                                    });
                                }
                            }
                        }
                        Event::Shutdown => {
                            info!("Shutting down MIDI service");
                            *running.lock().unwrap() = false;
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

    /// Update the controller LEDs to reflect current state
    fn update_controller_leds(
        &self,
        controller: &Arc<Mutex<Box<dyn MidiGridController>>>,
        bank: usize,
        snap_id: usize,
    ) {
        if let Ok(mut controller) = controller.lock() {
            // Clear all LEDs first
            controller.clear_leds();

            // Set the active snap LED
            let pad = snap_id as u8 + 8; // Add 8 to account for top row being modifiers
            controller.set_led(
                pad,
                Rgb {
                    r: 255,
                    g: 165,
                    b: 0,
                },
            ); // Orange color

            // Set modifier row LEDs
            for i in 0..8 {
                // Different color for the current bank's modifier button
                let color = if i == bank {
                    Rgb {
                        r: 0,
                        g: 128,
                        b: 255,
                    } // Blue color
                } else {
                    Rgb {
                        r: 64,
                        g: 64,
                        b: 64,
                    } // Gray color
                };

                controller.set_led(i as u8, color);
            }

            // Set other snap LEDs
            let state_guard = self.state.read().unwrap();
            if bank < state_guard.project.banks.len() {
                let bank_data = &state_guard.project.banks[bank];

                for i in 0..56 {
                    // 56 snaps per bank (7 rows of 8)
                    if i != snap_id && i < bank_data.snaps.len() {
                        let pad = i as u8 + 8; // Add 8 to account for top row being modifiers
                        controller.set_led(
                            pad,
                            Rgb {
                                r: 50,
                                g: 50,
                                b: 50,
                            },
                        ); // Dim color for other snaps
                    }
                }
            }

            // Apply the changes
            controller.refresh_state();
        }
    }

    /// Send all snap values (static method that doesn't require ownership of Self)
    fn send_snap_values(state: &SharedState, controller: &Arc<Mutex<Box<dyn MidiGridController>>>) {
        let state_guard = state.read().unwrap();

        if state_guard.current_bank >= state_guard.project.banks.len()
            || state_guard.current_snap
                >= state_guard.project.banks[state_guard.current_bank]
                    .snaps
                    .len()
        {
            return;
        }

        let bank = &state_guard.project.banks[state_guard.current_bank];
        let snap = &bank.snaps[state_guard.current_snap];

        // Lock the controller to send CC values
        if let Ok(mut controller) = controller.lock() {
            // Send each parameter's CC value
            for (i, value) in snap.values.iter().enumerate() {
                if i >= state_guard.project.parameters.len() {
                    break;
                }

                let param = &state_guard.project.parameters[i];
                let _ = controller.send_cc(0, param.cc, *value);
            }
        }
    }

    /// Get a parameter by ID
    fn get_parameter(
        state: &SharedState,
        param_id: usize,
    ) -> Result<crate::model::Parameter, Box<dyn Error>> {
        let state_guard = state.read().unwrap();

        if param_id >= state_guard.project.parameters.len() {
            return Err("Parameter ID out of range".into());
        }

        Ok(state_guard.project.parameters[param_id].clone())
    }

    /// Send morph values
    fn send_morph_values(
        state: &SharedState,
        controller: &Arc<Mutex<Box<dyn MidiGridController>>>,
        current_values: &[u8],
    ) {
        let state_guard = state.read().unwrap();

        if let Ok(mut controller) = controller.lock() {
            for (i, value) in current_values.iter().enumerate() {
                if i >= state_guard.project.parameters.len() {
                    break;
                }

                let param = &state_guard.project.parameters[i];
                let _ = controller.send_cc(0, param.cc, *value);
            }
        }
    }
}
