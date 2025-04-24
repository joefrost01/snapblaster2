use crate::events::{Event, EventBus, MorphCurve};
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

    pub fn get_state(&self) -> Option<SharedState> {
        self.state.clone()
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
                            if vel > 0 {
                                // Note On with velocity > 0
                                let _ = eb.publish(Event::PadPressed { pad: note, velocity: vel });
                            } else {
                                // Note On with velocity = 0 is actually a Note Off
                                let _ = eb.publish(Event::PadReleased { pad: note, velocity: vel });
                            }
                        }
                        // Process explicit note-off messages (0x8n where n is the channel)
                        else if msg.len() >= 3 && (msg[0] & 0xF0) == 0x80 {
                            let note = msg[1];
                            let vel = msg[2];
                            debug!("Received note-off from hardware: note={}, vel={}", note, vel);
                            let _ = eb.publish(Event::PadReleased { pad: note, velocity: vel });
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
                        match event {
                            Event::PadPressed { pad, velocity } => {
                                debug!("Received PadPressed event: pad={}, velocity={}", pad, velocity);
                                if let Err(err) = manager.handle_pad_pressed(pad, velocity).await {
                                    error!("Error handling pad press: {}", err);
                                }
                            }
                            Event::PadReleased { pad, velocity } => {
                                debug!("Received PadReleased event: pad={}, velocity={}", pad, velocity);
                                if let Err(err) = manager.handle_pad_released(pad, velocity).await {
                                    error!("Error handling pad release: {}", err);
                                }
                            }
                            _ => {}
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
        // Only send to virtual MIDI port, not hardware controllers
        for (name, conn) in self.output_connections.lock().unwrap().iter_mut() {
            // Only send to our virtual MIDI port
            if name.contains("Snap-Blaster") {
                let msg = [0xB0 | (channel & 0x0F), cc, value];
                if let Err(e) = conn.send(&msg) {
                    warn!("CC send failed to {}: {}", name, e);
                } else {
                    debug!("Sent CC ch={} cc={} val={} to {}", channel, cc, value, name);
                }
            }
        }
        Ok(())
    }

    /// Send a batch of parameter CCs for a snap
    pub fn send_snap_values(&self, params: &[(u8, u8)]) -> Result<(), Box<dyn Error>> {
        info!("Sending {} CC values for snap", params.len());

        // Get all output connections
        let mut outputs = self.output_connections.lock().unwrap();

        for &(cc, val) in params {
            for (name, conn) in outputs.iter_mut() {
                // ONLY send to ports named exactly "Snap-Blaster"
                if name == "Snap-Blaster" {
                    let msg = [0xB0, cc, val]; // Channel 0 CC message
                    if let Err(e) = conn.send(&msg) {
                        warn!("Failed to send CC {} value {} to {}: {}", cc, val, name, e);
                    } else {
                        debug!("Sent CC ch=0 cc={} val={} to {}", cc, val, name);
                    }
                }
            }

            std::thread::sleep(Duration::from_millis(2));
        }

        Ok(())
    }

    /// Redraw all LEDs based on current state
    pub fn update_controller_leds(&self) -> Result<(), Box<dyn Error>> {
        if let Some(ref mut ctrl) = *self.controller.lock().unwrap() {
            if let Some(ref state) = self.state {
                let st = state.read().unwrap();

                // Clear all LEDs to start with a clean state
                ctrl.clear_leds();

                // Top row: 
                // - Pads 0-4: Morph duration modifiers (red normally, green when active)
                // - Pads 5-7: Bank indicators
                for i in 0..8 {
                    let color = if i < 5 {
                        // Morph duration modifiers (0-4)
                        if Some(i) == st.active_modifier {
                            Rgb::green()  // Active modifier: GREEN
                        } else {
                            Rgb::red()    // Normal modifier: RED
                        }
                    } else if i == (st.current_bank as u8) {
                        // Current bank (5-7): RED (matches UI)
                        Rgb::red()
                    } else if i < (st.project.banks.len() as u8) {
                        // Available bank: dimmed RED
                        Rgb::new(128, 0, 0)
                    } else {
                        // Unavailable bank: very dim RED
                        Rgb::new(64, 0, 0)
                    };

                    ctrl.set_led(i as u8, color);
                }

                // Snap pads (8-63)
                if st.current_bank < st.project.banks.len() {
                    let bank = &st.project.banks[st.current_bank];

                    for idx in 0..bank.snaps.len().min(56) {
                        let pad = (idx + 8) as u8;

                        let has_snap = !bank.snaps[idx].name.is_empty();
                        let is_current = idx == st.current_snap;

                        // Check if a morph is in progress and this is the target snap
                        let is_morph_target = if let Some(ref morph) = st.active_morph {
                            idx == morph.to_snap
                        } else {
                            false
                        };

                        let color = if is_current {
                            // Current snap: GREEN (selected)
                            Rgb::green()
                        } else if is_morph_target {
                            // Morph target: PURPLE (or another distinctive color)
                            Rgb::purple()
                        } else if has_snap {
                            // Available snap: YELLOW
                            Rgb::yellow()
                        } else {
                            // Empty slot: very dim
                            Rgb::new(16, 16, 16)
                        };

                        ctrl.set_led(pad, color);
                    }

                    // Show morph progress by lighting up pads with different colors
                    if let Some(ref morph) = st.active_morph {
                        // Calculate how many pads to light based on progress
                        // We'll light a row (0-7) of pads on top to show progress

                        // Determine how many top-row pads to light based on progress
                        let progress_pads = (morph.progress * 8.0).floor() as usize;

                        // Use a dynamic color based on progress - shift from blue to green
                        for i in 0..progress_pads.min(8) {
                            // Pulse the pad by varying intensity with progress
                            let pulse_factor = 0.7 + 0.3 * ((morph.progress * 5.0) % 1.0);

                            // Blend from blue to green as morph progresses
                            let blue = ((1.0 - morph.progress) * 255.0 * pulse_factor) as u8;
                            let green = ((morph.progress) * 255.0 * pulse_factor) as u8;

                            // Set the progress indicator LEDs
                            ctrl.set_led(i as u8, Rgb::new(0, green, blue));
                        }
                    }
                }

                // Ensure all changes are sent to the device
                ctrl.refresh_state();
            }
        }

        Ok(())
    }

    /// Handle a pad press event from the hardware controller
    pub async fn handle_pad_pressed(&self, pad: u8, velocity: u8) -> Result<(), Box<dyn Error>> {
        info!("Handling pad press: pad={}, velocity={}", pad, velocity);

        if let Some(ref state) = self.state {
            // Check if this is a modifier pad press (top row, pads 0-4)
            if pad < 5 && velocity > 0 {
                // Handle modifier press - set active modifier
                let mut state_guard = state.write().unwrap();

                // Set the morph duration based on the pad
                let duration_bars = match pad {
                    0 => 1,  // 1 bar
                    1 => 2,  // 2 bars
                    2 => 4,  // 4 bars
                    3 => 8,  // 8 bars
                    4 => 16, // 16 bars
                    _ => 4,  // Default: 4 bars
                };

                // Store the active modifier and duration
                state_guard.active_modifier = Some(pad);
                state_guard.morph_duration = duration_bars;
                drop(state_guard); // Release the lock before updating LEDs

                // Color the modifier pad green to indicate it's active
                if let Some(ref mut ctrl) = *self.controller.lock().unwrap() {
                    ctrl.set_led(pad, Rgb::green());
                    ctrl.refresh_state();
                }

                return Ok(());
            }

            // Check bank selection (pads 5-7)
            if pad >= 5 && pad < 8 && velocity > 0 {
                let mut state_guard = state.write().unwrap();

                if pad < state_guard.project.banks.len() as u8 {
                    // Change bank
                    state_guard.current_bank = pad as usize;
                    drop(state_guard);

                    // Publish event
                    let _ = self.event_bus.publish(Event::BankSelected {
                        bank_id: pad as usize
                    });

                    // Update controller display
                    self.update_controller_leds()?;
                }
                return Ok(());
            }

            // Non-top-row pads are for snap selection or morph targets
            let snap_id = (pad - 8) as usize;
            let bank_id;

            // Check if this is a regular snap selection or morph target selection
            let active_modifier = {
                let guard = state.read().unwrap();
                guard.active_modifier
            };

            // Check if an active morph is in progress (before handling any new pad press)
            let morph_in_progress = {
                let guard = state.read().unwrap();
                guard.active_morph.is_some()
            };

            // If a morph is in progress and this is a snap selection (not a modifier),
            // cancel the morph by sending a MorphCompleted event
            if morph_in_progress && pad >= 8 && velocity > 0 {
                info!("Canceling active morph because a new snap was selected");

                // Clear the active morph state
                {
                    let mut state_guard = state.write().unwrap();
                    state_guard.active_morph = None;
                }

                // Send event to notify morph engine and other components
                let _ = self.event_bus.publish(Event::MorphCompleted);
            }

            if let Some(modifier_pad) = active_modifier {
                // This is a morph target selection
                info!("Morph target selected: pad={}, snap_id={}", pad, snap_id);

                // First validate that this is a valid snap
                {
                    let guard = state.read().unwrap();
                    bank_id = guard.current_bank;

                    if bank_id >= guard.project.banks.len() {
                        return Ok(());  // Invalid bank
                    }

                    let bank = &guard.project.banks[bank_id];
                    if snap_id >= bank.snaps.len() || bank.snaps[snap_id].name.is_empty() {
                        return Ok(());  // Invalid snap
                    }
                }

                // Get the current snap (source)
                let from_snap: usize;
                let duration_bars: u8;

                {
                    let guard = state.read().unwrap();
                    from_snap = guard.current_snap;
                    duration_bars = guard.morph_duration;

                    // Don't morph to the same snap
                    if from_snap == snap_id {
                        return Ok(());
                    }
                }

                // Start the morph (use Link-quantized morphing)
                let _ = self.event_bus.publish(Event::MorphInitiated {
                    from_snap,
                    to_snap: snap_id,
                    duration_bars,
                    curve_type: MorphCurve::Linear,
                    quantize: true, // Always quantize to next bar
                });

                info!("Started morph from snap {} to {} over {} bars", 
                  from_snap, snap_id, duration_bars);

                return Ok(());
            } else {
                // Regular snap selection (no modifier active)
                let cc_values: Vec<(u8, u8)>;

                // First check if this is a valid snap
                {
                    let guard = state.read().unwrap();
                    bank_id = guard.current_bank;

                    if bank_id >= guard.project.banks.len() {
                        return Ok(());  // Invalid bank
                    }

                    let bank = &guard.project.banks[bank_id];
                    if snap_id >= bank.snaps.len() || bank.snaps[snap_id].name.is_empty() {
                        return Ok(());  // Invalid snap
                    }

                    // Collect the parameter values we'll need to send
                    cc_values = guard.project.parameters.iter().enumerate()
                        .filter_map(|(idx, param)| {
                            if idx < bank.snaps[snap_id].values.len() {
                                let value = bank.snaps[snap_id].values[idx];
                                Some((param.cc, value))
                            } else {
                                None
                            }
                        })
                        .collect();
                }

                // Update the current state (snap selection)
                {
                    let mut state_guard = state.write().unwrap();
                    state_guard.current_snap = snap_id;
                }

                // Add more logging to debug
                info!("Ready to send CC values for snap {}", snap_id);

                // Send the selected snap's values via MIDI CCs - to the VIRTUAL port, not the hardware
                if !cc_values.is_empty() {
                    info!("Sending {} CC values for snap {}", cc_values.len(), snap_id);

                    // Make sure this is correctly sending to your virtual MIDI port
                    self.send_snap_values(&cc_values)?;
                }

                // Publish the event
                let _ = self.event_bus.publish(Event::SnapSelected {
                    bank: bank_id,
                    snap_id
                });

                // Update the controller LEDs
                self.update_controller_leds()?;
            }
        }

        Ok(())
    }

    /// Handle a pad release event from the hardware controller
    pub async fn handle_pad_released(&self, pad: u8, velocity: u8) -> Result<(), Box<dyn Error>> {
        info!("Handling pad release: pad={}, velocity={}", pad, velocity);

        if let Some(ref state) = self.state {
            // Only handle releases for the modifier pads (0-4)
            if pad < 5 {
                let mut state_guard = state.write().unwrap();

                // Check if this was the active modifier
                if state_guard.active_modifier == Some(pad) {
                    // Clear the active modifier
                    state_guard.active_modifier = None;
                    drop(state_guard); // Release the lock

                    // Update LED to normal red state
                    if let Some(ref mut ctrl) = *self.controller.lock().unwrap() {
                        ctrl.set_led(pad, Rgb::red());
                        ctrl.refresh_state();
                    }
                }
            }
        }

        Ok(())
    }
}