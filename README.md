[![CI Pipeline](https://github.com/joefrost01/snapblaster2/actions/workflows/build.yml/badge.svg)](https://github.com/joefrost01/snapblaster2/actions/workflows/build.yml)
[![Quarto Docs](https://img.shields.io/badge/docs-online-blue.svg)](https://joefrost01.github.io/snapblaster2/)

# Snap-Blaster: Functional and Technical Design Document

## Overview
Snap-Blaster is a MIDI CC snapshot controller designed to integrate tightly with digital audio workstations (DAWs) such as Ableton Live, Bitwig, Reason, Logic, and others. It enables musicians to manage, edit, and perform complex MIDI automation through an intuitive interface and grid controller (e.g. Launchpad, Push 2, APC Mini), removing the need to draw automation by hand.

It is especially designed for live performance and expressive composition workflows.

This document provides a full specification for the implementation of Snap-Blaster as a desktop application using Tauri, Tailwind, HTML/JavaScript, and a Rust backend.

---

## Core Concepts

### Project
- Represents a full setup for a DAW session.
- Contains up to 56 snaps per bank.
- Can be saved and loaded via `.sb` files.
- Stores all parameter CC mappings and values.

### Bank (Pro Feature)
- Each bank holds up to 56 snaps.
- Navigable via modifier+pad presses.
- Free users get 1 bank; Pro users get 56.

### Snap
- A full 64-parameter state triggered by pressing a pad.
- Sends all 64 CCs to the DAW on fire.
- Lives inside a bank.
- Snap slots are mapped to controller pads.

### Parameter
- One of up to 64 CCs used in a project.
- Named and described by the user.
- Mapped to a specific CC number.
- Shown in the Snap Editor in groups of 16 (tabs 1–4).

### Controller Pad
- Top row (8 pads): Reserved for modifiers.
- Remaining 56 pads: Trigger snaps.
- Pad note messages are used to detect input.

---

## Features

### MIDI Input
- Snap trigger detection via Note On messages.
- Modifier key detection (multiple pads pressed).

### MIDI Output
- Snap sends 64 CC values on trigger.
- Sends Note messages to light controller LEDs.

### Ableton Link
- Shows sync status in top bar.
- Governs when snaps are allowed to fire if quantization is active.

### Snap Editor
- 16 parameters per tab (4 tabs max).
- Editable fields: name, description, value.
- Value shown beside slider.
- Wiggle icon triggers CC automation to support DAW learn.

### Parameter Setup
- Config page to define 64 project parameters.
- Editable fields: name, description, CC number.
- Shared across all snaps in the project.

### AI Integration
- Prompts generated from:
  - Project name
  - Snap name
  - Parameter name and description
  - Bank name (Pro only)
- Users provide OpenAI API key in settings.
- AI button only visible when key is provided.
- AI suggests a value (0–127) for each parameter in a snap.

### Morphing
- Free users get linear morph.
- Pro users can select different morph curves (exponential, etc).

---

### Functional Summary
- **Project Management**
    - Load/save projects (.sb file format)
    - Projects contain 56 snaps per bank
    - Free version: 1 bank (56 snaps)
    - Pro version: Up to 56 banks (3,136 snaps total)

- **Snap Definition**
    - A snap is a complete state of up to 64 CC parameter values
    - Each snap is triggered by a single pad press on the grid controller

- **Parameter Configuration**
    - Up to 64 user-defined parameters per project
    - Each parameter has:
        - Name
        - Description
        - CC number

- **Snap Editing**
    - Slider-based UI for adjusting each parameter's value per snap
    - Tabs for paginated editing (16 sliders per page, 4 pages max)
    - Optional AI-assisted value generation based on project/snap/param context

- **AI Integration**
    - Optional: Uses OpenAI API key provided in settings
    - AI generates parameter values contextually per snap

- **Launchpad/Grid Support**
    - Supports note input from grid to select snaps
    - Sends note messages to update pad LED states
    - Modifiers:
        - One or more modifier buttons can be held to access different banks
        - Pro-only: access to more than 1 bank

- **MIDI Routing**
    - On snap trigger:
        - Sends all configured CC values out over selected MIDI port
        - Real-time updates if slider values change while DAW is playing

- **Link Integration**
    - Synchronize with Ableton Link for tempo-aware features
    - Enable timed morphing between snapshots over musical time (1, 2, 4, 8, or 16 bars)

---

## Technical Architecture

### Stack
- **Frontend**
    - HTML/CSS (Tailwind)
    - JavaScript (Vanilla)
    - Layout via show/hide views (no SPA routing)

- **Backend**
    - Rust + Tauri
    - Direct JS/Tauri bindings for MIDI I/O, file I/O, AI integration
    - Event-driven architecture using broadcast channels

- **No Web Server**
    - No Axum or REST required
    - HTML/JS served as static assets

### Tauri-Specific Details
- **IPC Mechanism**: Use `invoke` for JS <-> Rust calls
- **Backend Capabilities**:
    - MIDI send/receive
    - Access filesystem to read/write project files
    - Call OpenAI API if key is provided
    - Manage controller LED state
    - Maintain internal representation of the current project state

### Event-Driven Architecture
- **Core Event Bus**
    - Central message bus for all application events
    - Based on `tokio::sync::broadcast` channels
    - Decouples components for better modularity and testability

- **Event Types**
    - MIDI events (pad pressed, CC values)
    - Link events (beat/bar synchronization)
    - UI events (snap selection, parameter edits)
    - AI events (generation requests/responses)
    - Morph events (initiating/progressing/completing morphs)

- **Component Communication**
    - Each component publishes and subscribes to relevant events
    - Clean separation of concerns while maintaining system coordination

---

## UI Structure

### Layout Overview
- **Top Bar (Header)**
    - App title, MIDI controller selection dropdown
    - Link sync status
    - Project name
    - Load/Save buttons

- **Left Sidebar**
    - Snap selector (1–56 snaps)
    - Add Snap button
    - Pro: bank selector (56 banks)

- **Main Content Area**
    - Context-dependent views:
        - Snap editor (default)
        - Parameter setup page
        - Project settings dialog
        - Load project overlay

### Snap Editor View
- **Scene Info + Controls**
    - Scene name + current snap name
    - Generate Values button
    - Description field (project-level context for AI)

- **Launchpad Grid View**
    - 8x8 grid
    - Top row = modifiers (reserved)
    - Remaining 56 pads trigger individual snaps
    - Pads styled based on current/target/morph state

- **Parameter Slider Panel**
    - Tab buttons (1–4)
    - Each tab displays 16 parameters
    - Each slider cell contains:
        - Param name
        - Slider
        - Value
        - Wiggle button (sends CC to enable MIDI Learn in DAW)

---

## Project Data Model
```json
{
  "projectName": "dark_techno.sb",
  "openaiApiKey": "...",
  "controller": "Launchpad X",
  "banks": [
    {
      "name": "Default Bank",
      "snaps": [
        {
          "name": "Intro",
          "description": "a quiet intro",
          "values": [0, 64, 24, ...] // length = number of configured params
        },
        ...
      ]
    }
  ],
  "parameters": [
    {
      "name": "Rumble Bass Mix",
      "description": "deep rumbling bass",
      "cc": 65
    },
    ...
  ]
}
```

---

## MIDI Integration

### Output
- On snap selection:
    - Send all 64 CCs (or fewer if not defined)
    - Sent over selected virtual/outbound MIDI port

- On slider change:
    - Immediately send updated CC
    - Reflect change in pad LED if needed

### Input
- On note received from grid controller:
    - Translate pad to snap number
    - Trigger corresponding snap (send CCs)
    - Update grid LEDs

- On note + modifier:
    - Switch bank (pro-only if bank > 1)

### Controller Abstraction
```rust
trait MidiGridController {
    fn handle_note_input(&mut self, note: u8, velocity: u8);
    fn set_led(&mut self, pad: u8, color: Rgb);
    fn clear_leds(&mut self);
    fn refresh_state(&self);
}
```

---

## Event System Implementation

### Event Bus
```rust
use tokio::sync::broadcast;
use serde::{Serialize, Deserialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum SnapBlasterEvent {
    // MIDI events
    PadPressed { pad: u8, velocity: u8 },
    CCValueChanged { param_id: usize, value: u8 },
    
    // Link events
    BeatOccurred { beat: u32, phase: f64 },
    BarOccurred { bar: u32 },
    
    // UI events
    SnapSelected { bank: usize, snap_id: usize },
    ParameterEdited { param_id: usize, value: u8 },
    
    // AI events
    GenerateAIValues { bank_id: usize, snap_id: usize },
    AIGenerationCompleted { bank_id: usize, snap_id: usize },
    AIGenerationFailed { bank_id: usize, snap_id: usize, error: String },
    
    // Morphing events
    MorphInitiated { 
        from_snap: usize, 
        to_snap: usize, 
        duration_bars: u8,
        curve_type: MorphCurve,
    },
    MorphProgressed { progress: f64 },
    MorphCompleted,
}

pub struct EventBus {
    sender: broadcast::Sender<SnapBlasterEvent>,
}

impl EventBus {
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self { sender }
    }
    
    pub fn subscribe(&self) -> broadcast::Receiver<SnapBlasterEvent> {
        self.sender.subscribe()
    }
    
    pub fn publish(&self, event: SnapBlasterEvent) -> Result<usize, broadcast::error::SendError<SnapBlasterEvent>> {
        self.sender.send(event)
    }
}
```

### Morph Engine

```rust
pub struct MorphEngine {
    event_receiver: broadcast::Receiver<SnapBlasterEvent>,
    event_sender: broadcast::Sender<SnapBlasterEvent>,
    active_morph: Option<ActiveMorph>,
    project_state: Arc<RwLock<ProjectState>>,
}

impl MorphEngine {
    pub async fn run(&mut self) {
        while let Ok(event) = self.event_receiver.recv().await {
            match event {
                // Handle morph-related events and Link sync
                // ...
            }
        }
    }
    
    async fn update_parameters_for_morph(&self, progress: f64) {
        // Calculate intermediate parameter values
        // Send CC messages
    }
}
```

### AI Integration using Rig

```rust
use rig::providers::openai::{self, Client, completion::CompletionModel};

pub struct SnapAIService {
    model: CompletionModel,
    event_receiver: broadcast::Receiver<SnapBlasterEvent>,
    event_sender: broadcast::Sender<SnapBlasterEvent>,
}

impl SnapAIService {
    pub fn new(
        api_key: String,
        model_name: &str,
        event_receiver: broadcast::Receiver<SnapBlasterEvent>,
        event_sender: broadcast::Sender<SnapBlasterEvent>,
    ) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let client = Client::new(&api_key);
        let model = CompletionModel::new(client, model_name);
        
        Ok(Self {
            model,
            event_receiver,
            event_sender,
        })
    }
    
    pub async fn run(&mut self) {
        // Listen for AI generation requests
        // Format prompts and call OpenAI API
        // Parse responses and publish results
    }
}
```

## System Architecture Components

| Component | Primary Responsibility | Key Dependencies |
|-----------|------------------------|------------------|
| EventBus | Central message bus | tokio::sync::broadcast |
| ProjectManager | State management | serde, serde_json |
| MIDIController | Hardware I/O | midir, wmidi |
| LinkSynchronizer | Tempo sync | rust-link |
| MorphEngine | Parameter interpolation | interpolation |
| SnapAIService | AI generation | rig |
| TauriBackend | JS/Rust bridge | tauri |

---

## Licensing / Monetization

### Free Version
- 1 bank (56 snaps)
- Linear morphing only
- No morph curve editor
- AI generation (if OpenAI key provided)

### Pro Version
- 64 parameters (vs 32 in free)
- 56 banks of 56 snaps (vs 1)
- Morph curve selection and editing
- Cost: ~£25 one-time purchase

---

## Development Scope

### MVP
- Tauri app shell with working UI
- Load/save project
- Snap editing
- Grid controller support (Launchpad Mini / X, Push 2, APC Mini)
- CC sending via MIDI
- Snap selection via pad
- AI value generation with OpenAI key
- Basic event system implementation

### Post-MVP
- Morph curve editing (pro-only)
- Copy/paste snaps
- Undo/redo stack
- Param color-coding
- Snapshot comparison
- DAW template exports (optional)
- Enhanced Link integration for complex morphing

---

## UX Considerations
- No scrolling in param view
- Snappy transitions via JavaScript
- Immediate MIDI feedback when editing
- Snap grid is always visible in edit mode
- Consistent panel sizes to avoid UI jumps

---

## Compatibility

### Platform
- macOS (Apple Silicon and Intel)
- Windows 10/11
- Linux (where supported by controller drivers)

### Controllers (Initial)
- Launchpad Mini Mk2
- Launchpad X
- Push 2
- APC Mini

Controllers only need to:
- Send note-on when pad is pressed
- Accept note-on for LED color updates

CC output from controller not required.

---

## Appendices

### File Format (.sb)
- JSON-based
- Tauri uses Rust serde for (de)serialization
- Compressed format optional post-MVP

### AI Prompting Strategy
- Prompt = function of:
    - Project name
    - Scene/Snap name
    - Parameter name + description
    - Pad usage description
- Goal: expressive, style-aware CC values

Example prompt:
```
Project: dark techno
Scene: intro
Pad: rumble bass - mix fader
Description: a deep rumbling bass that is sidechained to the kick
CC: 65

Return the appropriate CC value (0-127) for the intro snap in this context.
```

### Core Rust Crates
- `tokio`: Async runtime and synchronization primitives
- `midir`: MIDI I/O
- `wmidi`: MIDI message parsing
- `rust-link`: Ableton Link integration
- `serde` + `serde_json`: Serialization/deserialization
- `rig`: Robust OpenAI API integration with retry capabilities
- `tauri`: Desktop application framework
- `interpolation`: Mathematical interpolation for morph curves

### A. Future Ideas
- Preset libraries
- Param morphing automation recording
- Companion app for parameter templates
- OSC support

### B. Open Questions
- Should MIDI thru be offered?
- Should controller settings persist across sessions?

---

## Status
- ✅ UI prototype complete (HTML/JS/Tailwind)
- ✅ Architecture defined
- 🔜 Backend event bus implementation
- 🔜 Tauri integration

---

## Summary

Snap-Blaster delivers performance-grade control over MIDI CCs with a fluid workflow and clean, studio-friendly UI. It enables deep expression without the pain of drawn automation. With AI, morphing, and controller integration, it's a powerful tool for producers and performers alike.

> *Build the automation brain you wish your DAW had.*

