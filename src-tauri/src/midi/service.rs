use crate::events::{Event, EventBus};
use crate::model::SharedState;
use crate::midi::controller::{MidiGridController, create_controller};
use midir::{MidiInput, MidiOutput, Ignore};
use std::error::Error;
use std::sync::{Arc, Mutex};
use tokio::sync::broadcast;
use tokio::task::JoinHandle;

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

        let controller = create_controller(&controller_name, event_bus.clone())?;
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
        let mut event_receiver = self.event_receiver;
        let controller = self.controller.clone();
        let state = self.state.clone();
        let event_bus = self.event_bus.clone();

        let handle = tokio::spawn(async move {
            while *running.lock().unwrap() {
                match event_receiver.recv().await {
                    Ok(event) => {
                        match event {
                            Event::SnapSelected { bank, snap_id } => {
                                // Update the current state
                                {
                                    let mut state_guard = state.write().unwrap();
                                    state_guard.current_bank = bank;
                                    state_guard.current_snap = snap_id;
                                }

                                // Get a reference to the MidiService to send CC values
                                // This requires a separate function that doesn't take self ownership
                                Self::send_snap_values(&state, &controller);

                                // Update the controller LEDs
                                let mut controller = controller.lock().unwrap();
                                controller.refresh_state();
                            },
                            Event::ParameterEdited { param_id, value } => {
                                // Update the parameter value in the current snap
                                {
                                    let mut state_guard = state.write().unwrap();

                                    // Store indices in local variables first
                                    let current_bank = state_guard.current_bank;
                                    let current_snap = state_guard.current_snap;

                                    // Now use the local variables
                                    let bank = &mut state_guard.project.banks[current_bank];
                                    let snap = &mut bank.snaps[current_snap];

                                    if param_id < snap.values.len() {
                                        snap.values[param_id] = value;
                                    }
                                }

                                // Send the CC value
                                if let Ok(param) = Self::get_parameter(&state, param_id) {
                                    let mut controller_lock = controller.lock().unwrap();
                                    let _ = controller_lock.send_cc(0, param.cc, value);
                                }
                            },
                            Event::MorphProgressed { progress: _, current_values } => {
                                // Send all CC values for the current morph state
                                Self::send_morph_values(&state, &controller, &current_values);
                            },
                            Event::Shutdown => {
                                *running.lock().unwrap() = false;
                                break;
                            },
                            _ => {}
                        }
                    },
                    Err(_) => {
                        // Channel closed or error
                        break;
                    }
                }
            }
        });

        (handle, returned_self)
    }

    // Helper methods to avoid ownership issues

    /// Send all snap values (static method that doesn't require ownership of Self)
    fn send_snap_values(state: &SharedState, controller: &Arc<Mutex<Box<dyn MidiGridController>>>) {
        let state_guard = state.read().unwrap();
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
    fn get_parameter(state: &SharedState, param_id: usize) -> Result<crate::model::Parameter, Box<dyn Error>> {
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
        current_values: &[u8]
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