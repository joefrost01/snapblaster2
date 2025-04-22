use crate::events::{Event, EventBus};
use crate::midi::controller::{create_controller, MidiGridController, Rgb};
use crate::model::SharedState;
use midir::{Ignore, MidiInput, MidiInputConnection, MidiOutput, MidiOutputConnection};
use std::error::Error;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use midir::os::unix::VirtualOutput;
use tracing::{debug, error, info, warn};

/// Main MIDI manager for Snap-Blaster with both virtual and hardware I/O
pub struct MidiManager {
    event_bus: EventBus,
    controller: Arc<Mutex<Option<Box<dyn MidiGridController>>>>,
    input_connection: Arc<Mutex<Option<MidiInputConnection<()>>>>,
    output_connections: Arc<Mutex<Vec<(String, MidiOutputConnection)>>>,
    state: Option<SharedState>,
}

impl Clone for MidiManager {
    fn clone(&self) -> Self {
        Self {
            event_bus: self.event_bus.clone(),
            controller: self.controller.clone(),
            input_connection: self.input_connection.clone(),
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
            input_connection: Arc::new(Mutex::new(None)),
            output_connections: Arc::new(Mutex::new(Vec::new())),
            state,
        }
    }

    /// Create a virtual MIDI port for other apps
    pub fn create_virtual_port(&self, port_name: &str) -> Result<(), Box<dyn Error>> {
        let midi_out = MidiOutput::new("Snap-Blaster Virtual")?;
        let conn = midi_out.create_virtual(port_name)?;
        self.output_connections.lock().unwrap().push((port_name.to_string(), conn));
        info!("Created virtual MIDI port: {}", port_name);
        Ok(())
    }

    /// List available MIDI input ports
    pub fn list_input_ports() -> Result<Vec<String>, Box<dyn Error>> {
        let midi_in = MidiInput::new("Snap-Blaster Input")?;
        let mut port_names = Vec::new();
        for port in midi_in.ports() {
            port_names.push(midi_in.port_name(&port)?);
        }
        Ok(port_names)
    }

    /// List available MIDI output ports
    pub fn list_output_ports() -> Result<Vec<String>, Box<dyn Error>> {
        let midi_out = MidiOutput::new("Snap-Blaster Output")?;
        let mut port_names = Vec::new();
        for port in midi_out.ports() {
            port_names.push(midi_out.port_name(&port)?);
        }
        Ok(port_names)
    }

    /// Open hardware MIDI ports and wire up callbacks that publish PadPressed events
    fn connect_hardware_ports(&self, controller_name: &str) -> Result<(), Box<dyn Error>> {
        // INPUT - Connect to the hardware controller's input port
        // This allows us to receive note messages from the controller
        let mut midi_in = MidiInput::new("Snap-Blaster Input")?;
        midi_in.ignore(Ignore::None);

        let mut found_input = false;
        for port in midi_in.ports() {
            let name = midi_in.port_name(&port)?;
            if name.contains(controller_name) {
                let eb = self.event_bus.clone();
                let conn = midi_in.connect(
                    &port,
                    "snapblaster-in",
                    move |_ts, msg: &[u8], _| {
                        // Process note-on messages (0x9n where n is the channel)
                        if msg.len() >= 3 && (msg[0] & 0xF0) == 0x90 {
                            let note = msg[1];
                            let vel = msg[2];
                            debug!("Received note from hardware: note={}, vel={}", note, vel);
                            let _ = eb.publish(Event::PadPressed { pad: note, velocity: vel });
                        }
                    },
                    (),
                )?;
                *self.input_connection.lock().unwrap() = Some(conn);
                info!("Connected MIDI input port: {}", name);
                found_input = true;
                break;
            }
        }

        if !found_input {
            warn!("Could not find MIDI input port for {}", controller_name);
        }

        // OUTPUT - Connect to the hardware controller's output port
        // This allows us to send LED updates to the controller
        let midi_out = MidiOutput::new("Snap-Blaster Output")?;

        let mut found_output = false;
        for port in midi_out.ports() {
            let name = midi_out.port_name(&port)?;
            if name.contains(controller_name) {
                let conn = midi_out.connect(&port, "snapblaster-out")?;
                self.output_connections.lock().unwrap().push((name.clone(), conn));
                info!("Connected MIDI output port: {}", name);
                found_output = true;
                break;
            }
        }

        if !found_output {
            warn!("Could not find MIDI output port for {}", controller_name);
        }

        Ok(())
    }

    /// Initialize both virtual and hardware controllers and subscribe to pad events
    pub fn initialize_controller(&self, controller_name: &str) -> Result<(), Box<dyn Error>> {
        info!("Initializing controller: {}", controller_name);

        // 1) Create virtual port for DAW output
        // This port allows the DAW to receive MIDI from Snap-Blaster
        if let Err(e) = self.create_virtual_port("Snap-Blaster") {
            warn!("Failed to create virtual port: {}", e);
        } else {
            info!("Created virtual MIDI port 'Snap-Blaster' for DAW communication");
        }

        // 2) Connect to real hardware I/O ports
        // This establishes connections to the physical controller
        if let Err(e) = self.connect_hardware_ports(controller_name) {
            warn!("Failed to connect hardware MIDI ports: {}", e);
        } else {
            info!("Successfully connected to {} hardware ports", controller_name);
        }

        // 3) Create grid controller abstraction
        // This provides a unified interface for LED control and input handling
        match create_controller(controller_name, self.event_bus.clone()) {
            Ok(ctrl) => {
                // Store the controller
                *self.controller.lock().unwrap() = Some(ctrl);

                // Initialize controller LEDs based on current state
                if let Err(e) = self.update_controller_leds() {
                    warn!("Failed to update controller LEDs: {}", e);
                } else {
                    info!("Updated controller LEDs based on current state");
                }

                info!("Grid controller initialized: {}", controller_name);

                // 4) Subscribe to PadPressed events and route to handle_pad_pressed
                // This handles user interaction with the controller
                let mut subscriber = self.event_bus.subscribe();
                let manager = self.clone();
                tokio::spawn(async move {
                    info!("Started event handler for PadPressed events");
                    while let Ok(event) = subscriber.recv().await {
                        if let Event::PadPressed { pad, velocity } = event {
                            debug!("Received PadPressed event: pad={}, velocity={}", pad, velocity);
                            if let Err(err) = manager.handle_pad_pressed(pad, velocity).await {
                                error!("Error handling pad press: {}", err);
                            }
                        }
                    }
                });

                Ok(())
            }
            Err(e) => {
                error!("Grid controller init failed for {}: {}", controller_name, e);
                Err(e)
            }
        }
    }

    /// Send a CC message to grid controller and all outputs
    pub fn send_cc(&self, channel: u8, cc: u8, value: u8) -> Result<(), Box<dyn Error>> {
        if let Some(ref mut ctrl) = *self.controller.lock().unwrap() {
            if let Err(e) = ctrl.send_cc(channel, cc, value) {
                warn!("Controller CC send failed: {}", e);
            }
        }
        for (_, conn) in self.output_connections.lock().unwrap().iter_mut() {
            let msg = [0xB0 | (channel & 0x0F), cc, value];
            if let Err(e) = conn.send(&msg) {
                warn!("CC send failed to {}: {}", cc, e);
            } else {
                debug!("Sent CC ch={} cc={} val={}", channel, cc, value);
            }
        }
        Ok(())
    }

    /// Send a batch of parameter CCs for a snap
    pub fn send_snap_values(&self, params: &[(u8, u8)]) -> Result<(), Box<dyn Error>> {
        for &(cc, val) in params {
            self.send_cc(0, cc, val)?;
            std::thread::sleep(Duration::from_millis(2));
        }
        Ok(())
    }

    /// Redraw all LEDs based on current state
    pub fn update_controller_leds(&self) -> Result<(), Box<dyn Error>> {
        if let Some(ref mut ctrl) = *self.controller.lock().unwrap() {
            ctrl.clear_leds();
            if let Some(ref state) = self.state {
                let st = state.read().unwrap();
                for i in 0..8 {
                    let color = if i == st.current_bank {
                        Rgb::new(0, 64, 255)
                    } else if i < st.project.banks.len() {
                        Rgb::new(0, 32, 128)
                    } else {
                        Rgb::black()
                    };
                    ctrl.set_led(i as u8, color);
                }
                if st.current_bank < st.project.banks.len() {
                    let bank = &st.project.banks[st.current_bank];
                    for idx in 0..bank.snaps.len() {
                        let pad = (idx + 8) as u8;
                        let color = if idx == st.current_snap { Rgb::orange() } else { Rgb::gray() };
                        ctrl.set_led(pad, color);
                    }
                }
                ctrl.refresh_state();
            }
        }
        Ok(())
    }

    /// Handle a pad press event from the hardware controller
    pub async fn handle_pad_pressed(&self, pad: u8, velocity: u8) -> Result<(), Box<dyn Error>> {
        if velocity == 0 { return Ok(()); }
        if let Some(ref state) = self.state {
            let guard = state.read().unwrap();
            if pad < 8 {
                // TODO: publish Event::BankSelected
                return Ok(());
            }
            let snap_id = (pad - 8) as usize;
            let bank_id = guard.current_bank;
            if bank_id < guard.project.banks.len() {
                let bank = &guard.project.banks[bank_id];
                if snap_id < bank.snaps.len() && !bank.snaps[snap_id].name.is_empty() {
                    drop(guard);
                    self.event_bus.publish(Event::SnapSelected { bank: bank_id, snap_id })?;
                    self.update_controller_leds()?;
                }
            }
        }
        Ok(())
    }
}
