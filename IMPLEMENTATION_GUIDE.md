# Snap-Blaster Implementation Guide

## Project Structure

The Snap-Blaster project is implemented as a Tauri application with a Rust backend and HTML/JavaScript frontend. The project follows a clean, modular architecture with small, focused files.

### Backend (Rust)

- **Event System**: Core messaging system that distributes events between components
- **Model**: Data structures and state management
- **MIDI Controller Abstraction**: Trait-based system for different grid controllers
- **MIDI Service**: Handles MIDI I/O and state synchronization
- **Storage**: Project file management
- **AI Integration**: OpenAI API interface for parameter generation
- **Morph Engine**: Handles parameter interpolation between snaps
- **Link Integration**: Tempo synchronization (placeholder for Ableton Link)

### Frontend (HTML/JS)

- **Main App**: Core UI structure and navigation
- **Snap View**: Editing snapshots and parameters
- **Config View**: Setting up project parameters

## Architecture Details

### Event-Driven Design

The application uses a centralized event bus to decouple components:

1. Components publish events to the bus
2. Interested components subscribe to relevant events
3. No direct dependencies between subsystems

This approach makes the system more maintainable and testable.

### MIDI Controller Abstraction

We've created a trait-based system for MIDI controllers:

```rust
pub trait MidiGridController {
    fn handle_note_input(&mut self, note: u8, velocity: u8);
    fn set_led(&mut self, pad: u8, color: Rgb);
    fn clear_leds(&mut self);
    fn refresh_state(&mut self);
    fn get_name(&self) -> &str;
    fn send_cc(&mut self, channel: u8, cc: u8, value: u8) -> Result<(), Box<dyn Error>>;
}
```

Each supported controller implements this trait, allowing for:
- Easy addition of new controllers
- Consistent interface regardless of hardware
- Simplified testing with mock controllers

### State Management

The application uses a shared state approach:

1. `ProjectState` contains the current project and runtime information
2. Wrapped in `Arc<RwLock<>>` for thread-safe access
3. Components can read/write state as needed
4. Changes trigger events to notify other components

### Pro Features

Pro features are gated behind the `pro` feature flag:

```rust
#[cfg(feature = "pro")]
// Pro-only code here
```

This allows for easy compilation of free vs. pro versions.

## Implementation Notes

### MIDI Implementation

- Uses `midir` for MIDI I/O
- Uses `wmidi` for MIDI message parsing
- Abstracting controllers behind the `MidiGridController` trait allows for different LED control schemes for each hardware

### Morph Engine

- Linear interpolation is available in free version
- Exponential, logarithmic, and S-curve options require pro license
- Implemented using feature flags for conditional compilation

### AI Integration

- Uses OpenAI API for parameter value generation
- Context-aware prompts that include project/snap/parameter information
- Values are generated independently per parameter for more nuanced results

### Frontend Design

The frontend is designed with simplicity in mind:

- No framework dependencies (vanilla JS)
- Simple view system with create/update pattern
- CSS animations for smoother transitions
- Clean Tailwind-based styling

## Building and Testing

### Development Setup

1. Install Rust and Tauri CLI
2. Run `cargo tauri dev` for development mode
3. Run `cargo tauri build` for production build

### Testing

1. Manual tests should focus on MIDI I/O and controller interaction
2. Event system should be unit tested for reliability
3. File saving/loading requires thorough testing for compatibility

## Future Improvements

1. Enhanced Link integration with full Ableton Link support
2. More controller templates
3. Expand morph options and visualization
4. Preset libraries and sharing
5. OSC support for non-MIDI environments

## Compatibility Notes

Tested and verified on:
- macOS (Apple Silicon and Intel)
- Windows 10/11
- Linux (Ubuntu 20.04+)

Supported controllers:
- Launchpad Mini Mk2
- Launchpad X
- Push 2
- APC Mini