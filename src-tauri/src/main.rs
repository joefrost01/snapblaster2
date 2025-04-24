#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use snapblaster::app::App;
use snapblaster::events::{Event, EventBus, MorphCurve};
use snapblaster::midi::manager::MidiManager;
use snapblaster::model::new_shared_state;
use snapblaster::model::{Parameter, SharedState, Snap};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tauri::{Manager, State, Window};
use tracing::{debug, error, info};

// Import the correct LinkSynchronizer directly
use snapblaster::link::LinkSynchronizer;

// Application state accessible from Tauri commands
struct AppState {
    app: Mutex<App>,
    event_bus: EventBus,
    shared_state: SharedState,
    midi_manager: Option<Arc<MidiManager>>,
    link_sync: Option<LinkSynchronizer>,
}

// Tauri commands that bridge between the UI and Rust backend

/// Debug function to check the state
#[tauri::command]
async fn debug_state(state: State<'_, AppState>) -> Result<String, String> {
    let state_guard = state.shared_state.read().unwrap();
    let debug_info = format!(
        "Parameters: {}\nBanks: {}\nSnaps in first bank: {}\nController: {}",
        state_guard.project.parameters.len(),
        state_guard.project.banks.len(),
        state_guard.project.banks[0].snaps.len(),
        state_guard.project.controller
    );

    debug!("Debug state: {}", debug_info);

    Ok(debug_info)
}

/// List available MIDI input ports
#[tauri::command]
async fn list_midi_inputs() -> Result<String, String> {
    let ports = MidiManager::list_input_ports().map_err(|e| e.to_string())?;

    serde_json::to_string(&ports).map_err(|e| e.to_string())
}

/// List available MIDI output ports
#[tauri::command]
async fn list_midi_outputs() -> Result<String, String> {
    let ports = MidiManager::list_output_ports().map_err(|e| e.to_string())?;

    serde_json::to_string(&ports).map_err(|e| e.to_string())
}

/// Set the current MIDI controller
#[tauri::command]
async fn set_controller(name: String, state: State<'_, AppState>) -> Result<(), String> {
    let mut state_guard = state.shared_state.write().unwrap();
    state_guard.project.controller = name;
    Ok(())
}

/// Get the current project state
#[tauri::command]
async fn get_project(state: State<'_, AppState>) -> Result<String, String> {
    let state_guard = state.shared_state.read().unwrap();
    debug!(
        "Getting project state: {} parameters",
        state_guard.project.parameters.len()
    );

    // Print debug info about the current project
    for (i, param) in state_guard.project.parameters.iter().enumerate() {
        debug!("Parameter {}: {} (CC: {})", i, param.name, param.cc);
    }

    serde_json::to_string(&state_guard.project).map_err(|e| e.to_string())
}

/// Save the current project
#[tauri::command]
async fn save_project(path: String, state: State<'_, AppState>) -> Result<(), String> {
    // Get the shared state directly to ensure we're saving the current state
    let state_guard = state.shared_state.read().unwrap();
    debug!(
        "Before save - Project has {} parameters",
        state_guard.project.parameters.len()
    );

    let app = state.app.lock().unwrap();
    let result = app
        .save_project(&PathBuf::from(path))
        .map_err(|e| e.to_string());

    // Double check the parameters are being saved
    if result.is_ok() {
        debug!(
            "Project saved. Parameters in state: {}",
            state_guard.project.parameters.len()
        );
    }

    result
}

/// Load a project
#[tauri::command]
async fn load_project(path: String, state: State<'_, AppState>) -> Result<(), String> {
    let app = state.app.lock().unwrap();
    app.load_project(&PathBuf::from(path))
        .map_err(|e| e.to_string())
}

/// Create a new project
#[tauri::command]
async fn new_project(state: State<'_, AppState>) -> Result<(), String> {
    let app = state.app.lock().unwrap();
    app.new_project().map_err(|e| e.to_string())
}

/// Select a snap
#[tauri::command]
async fn select_snap(
    bank_id: usize,
    snap_id: usize,
    state: State<'_, AppState>,
) -> Result<(), String> {
    // Collection of parameter values to send via MIDI
    let params_to_send: Vec<(u8, u8)>;

    // First validate and update state
    {
        let mut state_guard = state.shared_state.write().unwrap();

        // Validate indices
        if bank_id >= state_guard.project.banks.len() {
            return Err("Bank ID out of range".to_string());
        }

        let bank = &state_guard.project.banks[bank_id];
        if snap_id >= bank.snaps.len() {
            return Err("Snap ID out of range".to_string());
        }

        // Update state
        state_guard.current_bank = bank_id;
        state_guard.current_snap = snap_id;

        // Ensure the snap has values for all parameters
        let param_count = state_guard.project.parameters.len();

        {
            let bank = &mut state_guard.project.banks[bank_id];
            let snap = &mut bank.snaps[snap_id];

            // Resize the values array if needed
            if snap.values.len() < param_count {
                snap.values.resize(param_count, 64);
            }
        }
    }

    // Now, in a separate step, collect the parameter values with just a read lock
    {
        let state_guard = state.shared_state.read().unwrap();

        params_to_send = state_guard
            .project
            .parameters
            .iter()
            .enumerate()
            .filter_map(|(idx, param)| {
                if idx
                    < state_guard.project.banks[bank_id].snaps[snap_id]
                    .values
                    .len()
                {
                    let value = state_guard.project.banks[bank_id].snaps[snap_id].values[idx];
                    Some((param.cc, value))
                } else {
                    None
                }
            })
            .collect::<Vec<(u8, u8)>>(); // Explicitly collect into Vec<(u8, u8)>
    }

    // Get the MIDI manager
    if let Some(midi_manager) = &state.midi_manager {
        // Send all parameter values via MIDI
        if let Err(e) = midi_manager.send_snap_values(&params_to_send) {
            // Log error but continue - MIDI failure shouldn't stop the snap selection
            error!("Failed to send snap values via MIDI: {}", e);
        }
    }

    // Send the event
    state
        .event_bus
        .publish(Event::SnapSelected {
            bank: bank_id,
            snap_id,
        })
        .map(|_| ())
        .map_err(|e| e.to_string())
}

/// Edit a parameter value
#[tauri::command]
async fn edit_parameter(
    param_id: usize,
    value: u8,
    state: State<'_, AppState>,
) -> Result<(), String> {
    // The CC number for this parameter
    let cc: u8;

    // First update the state directly
    {
        let mut state_guard = state.shared_state.write().unwrap();

        // Store these values locally first
        let current_bank = state_guard.current_bank;
        let current_snap = state_guard.current_snap;

        // Make sure the parameter exists
        if param_id >= state_guard.project.parameters.len() {
            return Err("Parameter ID out of range".to_string());
        }

        // Get the CC number for this parameter
        cc = state_guard.project.parameters[param_id].cc;

        // Now access the snap with the stored indices
        let snap = &mut state_guard.project.banks[current_bank].snaps[current_snap];

        // Ensure the values array is big enough
        while snap.values.len() <= param_id {
            snap.values.push(64); // Default value
        }

        // Update the value
        snap.values[param_id] = value;
    }

    // Send the MIDI CC value
    if let Some(midi_manager) = &state.midi_manager {
        if let Err(e) = midi_manager.send_cc(0, cc, value) {
            // Log error but continue - MIDI failure shouldn't stop the parameter edit
            error!("Failed to send parameter CC via MIDI: {}", e);
        }
    }

    // Then publish the event
    state
        .event_bus
        .publish(Event::ParameterEdited { param_id, value })
        .map(|_| ())
        .map_err(|e| e.to_string())
}

/// Generate AI values for a snap
#[tauri::command]
async fn generate_ai_values(
    bank_id: usize,
    snap_id: usize,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state
        .event_bus
        .publish(Event::GenerateAIValues { bank_id, snap_id })
        .map(|_| ())
        .map_err(|e| e.to_string())
}

/// Send wiggle values for MIDI learn
#[tauri::command]
async fn send_wiggle(cc: u8, values: Vec<u8>, state: State<'_, AppState>) -> Result<(), String> {
    // Get the MIDI manager
    if let Some(midi_manager) = &state.midi_manager {
        // Send each value with a small delay between
        for value in values {
            if let Err(e) = midi_manager.send_cc(0, cc, value) {
                error!("Error sending wiggle value: {}", e);
                // Continue anyway
            }

            // Wait a bit between values
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }

        return Ok(());
    }

    Err("MIDI manager not initialized".to_string())
}

/// Start a morph between two snaps
#[tauri::command]
async fn start_morph(
    from_snap: usize,
    to_snap: usize,
    duration_bars: u8,
    curve_type: String,
    quantize: bool, // Add quantize parameter
    state: State<'_, AppState>,
) -> Result<(), String> {
    // Map the curve type string to the enum
    let curve = match curve_type.as_str() {
        "linear" => MorphCurve::Linear,
        #[cfg(feature = "pro")]
        "exponential" => MorphCurve::Exponential,
        #[cfg(feature = "pro")]
        "logarithmic" => MorphCurve::Logarithmic,
        #[cfg(feature = "pro")]
        "scurve" => MorphCurve::SCurve,
        _ => MorphCurve::Linear,
    };

    state
        .event_bus
        .publish(Event::MorphInitiated {
            from_snap,
            to_snap,
            duration_bars,
            curve_type: curve,
            quantize, // Pass through the quantize flag
        })
        .map(|_| ())
        .map_err(|e| e.to_string())
}

/// Set the OpenAI API key
#[tauri::command]
async fn set_openai_api_key(api_key: String, state: State<'_, AppState>) -> Result<(), String> {
    let mut state_guard = state.shared_state.write().unwrap();
    state_guard.project.openai_api_key = Some(api_key);
    Ok(())
}

/// Add a new parameter
#[tauri::command]
async fn add_parameter(
    name: String,
    description: String,
    cc: u8,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let mut state_guard = state.shared_state.write().unwrap();

    // Add the parameter to the project
    state_guard.project.parameters.push(Parameter {
        name: name.clone(),
        description: description.clone(),
        cc,
    });

    // Add a default value to each snap
    for bank in &mut state_guard.project.banks {
        for snap in &mut bank.snaps {
            snap.values.push(64); // Default to middle value
        }
    }

    debug!(
        "Parameter added: {} (CC: {}), Total parameters: {}",
        name,
        cc,
        state_guard.project.parameters.len()
    );

    Ok(())
}

/// Update a parameter
#[tauri::command]
async fn update_parameter(
    param_id: usize,
    name: String,
    description: String,
    cc: u8,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let mut state_guard = state.shared_state.write().unwrap();

    if param_id >= state_guard.project.parameters.len() {
        return Err("Parameter ID out of range".to_string());
    }

    let param = &mut state_guard.project.parameters[param_id];
    param.name = name.clone();
    param.description = description.clone();
    param.cc = cc;

    debug!(
        "Parameter updated: ID {}, name '{}', CC {}",
        param_id, name, cc
    );

    Ok(())
}

/// Add a new snap
#[tauri::command]
async fn add_snap(
    bank_id: usize,
    pad_index: usize,
    name: String,
    description: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let mut state_guard = state.shared_state.write().unwrap();

    if bank_id >= state_guard.project.banks.len() {
        return Err("Bank ID out of range".to_string());
    }

    // Store the parameter count before the mutable borrow
    let param_count = state_guard.project.parameters.len();

    // Get the bank
    let bank = &mut state_guard.project.banks[bank_id];

    // Ensure we have space for this snap position
    if pad_index >= bank.snaps.len() {
        bank.snaps.resize(
            pad_index + 1,
            Snap {
                name: String::new(),
                description: String::new(),
                values: vec![],
            },
        );
    }

    // Set the snap at the specified pad position
    bank.snaps[pad_index] = Snap {
        name,
        description,
        values: vec![64; param_count], // Default all values to middle
    };

    Ok(())
}

/// Update a snap's description
#[tauri::command]
async fn update_snap_description(
    bank_id: usize,
    snap_id: usize,
    description: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let mut state_guard = state.shared_state.write().unwrap();

    if bank_id >= state_guard.project.banks.len() {
        return Err("Bank ID out of range".to_string());
    }

    let bank = &mut state_guard.project.banks[bank_id];

    if snap_id >= bank.snaps.len() {
        return Err("Snap ID out of range".to_string());
    }

    let snap = &mut bank.snaps[snap_id];
    snap.description = description;

    Ok(())
}

/// Get Link status and peer count
#[tauri::command]
async fn get_link_status(state: State<'_, AppState>) -> Result<String, String> {
    if let Some(link_sync) = &state.link_sync {
        let peers = link_sync.num_peers().await;
        let playing = link_sync.is_playing().await;

        // For the tempo, we need to subscribe to the tempo event
        let mut receiver = state.event_bus.subscribe();
        let _ = state.event_bus.publish(Event::RequestLinkTempo);

        // Wait for the tempo response with a timeout
        let timeout = tokio::time::Duration::from_millis(200);
        let tempo = match tokio::time::timeout(timeout, receiver.recv()).await {
            Ok(Ok(Event::LinkTempoChanged { tempo })) => tempo,
            _ => 120.0, // Default tempo if we can't get it
        };

        let status = serde_json::json!({
            "connected": peers > 0,
            "peers": peers,
            "playing": playing,
            "tempo": tempo
        });

        Ok(status.to_string())
    } else {
        Err("Link synchronizer not initialized".to_string())
    }
}

/// Set Link tempo
#[tauri::command]
async fn set_link_tempo(tempo: f64, state: State<'_, AppState>) -> Result<(), String> {
    if let Some(link_sync) = &state.link_sync {
        link_sync.set_tempo(tempo).await;

        // Publish event to notify all components
        let _ = state.event_bus.publish(Event::LinkTempoChanged { tempo });

        Ok(())
    } else {
        Err("Link synchronizer not initialized".to_string())
    }
}

/// Enable or disable Link
#[tauri::command]
async fn set_link_enabled(enabled: bool, state: State<'_, AppState>) -> Result<(), String> {
    if let Some(link_sync) = &state.link_sync {
        link_sync.enable(enabled).await;

        // Get peer count after state change
        let peers = link_sync.num_peers().await;

        // Publish event with updated status
        let _ = state.event_bus.publish(Event::LinkStatusChanged {
            connected: peers > 0,
            peers
        });

        Ok(())
    } else {
        Err("Link synchronizer not initialized".to_string())
    }
}

/// Start Link transport
#[tauri::command]
async fn start_link_transport(state: State<'_, AppState>) -> Result<(), String> {
    if let Some(link_sync) = &state.link_sync {
        link_sync.start_transport().await;

        // Publish event to notify all components
        let _ = state.event_bus.publish(Event::LinkTransportChanged { playing: true });

        Ok(())
    } else {
        Err("Link synchronizer not initialized".to_string())
    }
}

/// Stop Link transport
#[tauri::command]
async fn stop_link_transport(state: State<'_, AppState>) -> Result<(), String> {
    if let Some(link_sync) = &state.link_sync {
        link_sync.stop_transport().await;

        // Publish event to notify all components
        let _ = state.event_bus.publish(Event::LinkTransportChanged { playing: false });

        Ok(())
    } else {
        Err("Link synchronizer not initialized".to_string())
    }
}

/// Set quantum (beats per bar) for Link
#[tauri::command]
async fn set_link_quantum(beats: f64, state: State<'_, AppState>) -> Result<(), String> {
    if let Some(link_sync) = &state.link_sync {
        link_sync.set_quantum(beats).await;
        Ok(())
    } else {
        Err("Link synchronizer not initialized".to_string())
    }
}

// Set up event listeners and forward events to the frontend
fn setup_event_listener(window: Window, event_bus: EventBus) {
    let mut rx = event_bus.subscribe();

    tokio::spawn(async move {
        while let Ok(event) = rx.recv().await {
            // Convert event to JSON
            if let Ok(json) = serde_json::to_string(&event) {
                // Emit event to frontend
                let _ = window.emit("snap-event", json);
            }
        }
    });
}

#[tokio::main]
async fn main() {
    // Set up tracing for logging
    tracing_subscriber::fmt::init();

    // Initialize application - first create shared state
    let shared_state = new_shared_state();
    let event_bus = EventBus::default();

    // Pass the shared state to the App constructor
    let mut app =
        App::new(shared_state.clone(), event_bus.clone()).expect("Failed to create application");
    app.init().expect("Failed to initialize application");

    // Create a clone of event_bus for the setup closure
    let setup_event_bus = event_bus.clone();
    let midi_manager = app.midi_manager();

    // Get the LinkSynchronizer directly from the app
    let link_sync = app.link_sync();

    // We don't need to start it here - it was already started in app.init()

    // Set up event handler for controller input
    if let Some(midi_manager) = &midi_manager {
        let midi_manager_clone = midi_manager.clone();
        let pad_event_bus = event_bus.clone();

        // Existing pad press handler
        tokio::spawn(async move {
            let mut rx = pad_event_bus.subscribe();

            while let Ok(event) = rx.recv().await {
                if let Event::PadPressed { pad, velocity } = event {
                    debug!(
                        "Received PadPressed event in main handler: pad={}, velocity={}",
                        pad, velocity
                    );
                    if let Err(e) = midi_manager_clone.handle_pad_pressed(pad, velocity).await {
                        error!("Error handling pad press: {}", e);
                    } else {
                        debug!("Successfully handled pad press");
                    }
                }
            }
        });

        // Add a new handler for snap-related events to update LEDs
        let midi_manager_for_events = midi_manager.clone();
        let state_events_bus = event_bus.clone();

        tokio::spawn(async move {
            let mut rx = state_events_bus.subscribe();

            while let Ok(event) = rx.recv().await {
                // Update controller LEDs when state changes
                match event {
                    Event::ProjectLoaded
                    | Event::SnapSelected { .. }
                    | Event::BankSelected { .. } => {
                        // No need for Option pattern - it's an Arc directly
                        if let Err(e) = midi_manager_for_events.update_controller_leds() {
                            error!("Failed to update controller LEDs after state change: {}", e);
                        }
                    }
                    _ => {}
                }
            }
        });
    }

    let midi_manager_for_updates = midi_manager.clone();
    let midi_update_bus = event_bus.clone();

    tokio::spawn(async move {
        let mut rx = midi_update_bus.subscribe();

        while let Ok(event) = rx.recv().await {
            if let Event::RequestMIDIUpdate = event {
                if let Some(ref midi_manager) = midi_manager_for_updates {
                    if let Err(e) = midi_manager.update_controller_leds() {
                        error!("Failed to update controller LEDs: {}", e);
                    } else {
                        debug!("Updated controller LEDs on request");
                    }
                }
            }
        }
    });

    // Add a handler for CC value changes during morphing
    let midi_manager_for_cc = midi_manager.clone();
    let cc_events_bus = event_bus.clone();

    tokio::spawn(async move {
        let mut rx = cc_events_bus.subscribe();

        while let Ok(event) = rx.recv().await {
            if let Event::CCValueChanged { param_id, value } = event {
                if let Some(ref midi_manager) = midi_manager_for_cc {
                    // Get the CC number for this parameter
                    let cc_number = {
                        let state = midi_manager.get_state();
                        if let Some(state) = state {
                            let guard = state.read().unwrap();
                            if param_id < guard.project.parameters.len() {
                                guard.project.parameters[param_id].cc
                            } else {
                                continue; // Skip if parameter doesn't exist
                            }
                        } else {
                            continue; // Skip if no state
                        }
                    };

                    // Send the CC value to the MIDI output
                    if let Err(e) = midi_manager.send_cc(0, cc_number, value) {
                        error!("Failed to send CC during morph: {}", e);
                    } else {
                        debug!("Sent morph CC: ch=0 cc={} val={}", cc_number, value);
                    }
                }
            }
        }
    });

    // Clone event_bus for AppState before we move it into the setup closure
    let app_state_event_bus = event_bus.clone();

    // Create Tauri application
    tauri::Builder::default()
        .setup(move |app_handle| {
            // Get the main window
            let window = app_handle.get_window("main").unwrap();

            // Set up event listeners - use the cloned event_bus
            setup_event_listener(window.clone(), setup_event_bus);

            // Set up Link event handler
            let window_for_link = window.clone();
            let event_bus_for_link = event_bus.clone();

            tokio::spawn(async move {
                let mut rx = event_bus_for_link.subscribe();

                while let Ok(event) = rx.recv().await {
                    match event {
                        Event::LinkStatusChanged { connected, peers } => {
                            // Send to frontend
                            let status = serde_json::json!({
                                "type": "link_status",
                                "connected": connected,
                                "peers": peers
                            });

                            if let Err(e) = window_for_link.emit("link-event", status.to_string()) {
                                error!("Failed to emit link status: {}", e);
                            }
                        },
                        Event::LinkTempoChanged { tempo } => {
                            // Send to frontend
                            let status = serde_json::json!({
                                "type": "link_tempo",
                                "tempo": tempo
                            });

                            if let Err(e) = window_for_link.emit("link-event", status.to_string()) {
                                error!("Failed to emit link tempo: {}", e);
                            }
                        },
                        Event::LinkTransportChanged { playing } => {
                            // Send to frontend
                            let status = serde_json::json!({
                                "type": "link_transport",
                                "playing": playing
                            });

                            if let Err(e) = window_for_link.emit("link-event", status.to_string()) {
                                error!("Failed to emit link transport: {}", e);
                            }
                        },
                        _ => {} // Ignore other events
                    }
                }
            });

            Ok(())
        })
        .manage(AppState {
            app: Mutex::new(app),
            event_bus: app_state_event_bus,  // Use the cloned event_bus here
            shared_state,
            midi_manager,
            link_sync,
        })
        .invoke_handler(tauri::generate_handler![
            list_midi_inputs,
            list_midi_outputs,
            get_project,
            save_project,
            load_project,
            new_project,
            select_snap,
            edit_parameter,
            generate_ai_values,
            start_morph,
            set_openai_api_key,
            add_parameter,
            update_parameter,
            add_snap,
            update_snap_description,
            set_controller,
            send_wiggle,
            debug_state,
            get_link_status,
            set_link_tempo,
            set_link_enabled,
            start_link_transport,
            stop_link_transport,
            set_link_quantum,
        ])
        .run(tauri::generate_context!())
        .expect("Error while running Tauri application");
}