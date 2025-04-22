use snapblaster::events::{Event, EventBus};
use snapblaster::midi::controller::{MidiGridController, Rgb};
use snapblaster::midi::controllers::launchpad_x::LaunchpadX;
use std::error::Error;
use std::thread;
use std::time::Duration;

fn main() -> Result<(), Box<dyn Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Create an event bus for testing
    let event_bus = EventBus::new(100, "test");

    // Subscribe to events to see what comes from the Launchpad X
    let event_bus_clone = event_bus.clone();
    let event_receiver = event_bus.subscribe();

    // Spawn a thread to handle events
    thread::spawn(move || {
        let mut rx = event_receiver;

        loop {
            match rx.blocking_recv() {
                Ok(event) => {
                    println!("Received event: {:?}", event);

                    // If a pad is pressed, light it up
                    if let Event::PadPressed { pad, velocity } = event {
                        if velocity > 0 {
                            // Get a new controller just for this event
                            if let Ok(mut controller) = LaunchpadX::new(event_bus_clone.clone()) {
                                // Set the pad to a color based on its index
                                let color = match pad % 8 {
                                    0 => Rgb::red(),
                                    1 => Rgb::green(),
                                    2 => Rgb::blue(),
                                    3 => Rgb::yellow(),
                                    4 => Rgb::purple(),
                                    5 => Rgb::cyan(),
                                    6 => Rgb::orange(),
                                    7 => Rgb::white(),
                                    _ => Rgb::gray(),
                                };

                                // Set the LED
                                controller.set_led(pad, color);

                                // Refresh the controller
                                controller.refresh_state();
                            }
                        }
                    }
                }
                Err(e) => {
                    println!("Error receiving event: {:?}", e);
                    break;
                }
            }
        }
    });

    // Create a Launchpad X controller
    println!("Creating Launchpad X...");
    let mut controller = LaunchpadX::new(event_bus)?;

    // Clear the LEDs
    println!("Clearing LEDs...");
    controller.clear_leds();
    controller.refresh_state();

    // Test patterns

    // Pattern 1: Rainbow across top row
    println!("Pattern 1: Rainbow across top row...");
    controller.set_led(0, Rgb::red());
    controller.set_led(1, Rgb::orange());
    controller.set_led(2, Rgb::yellow());
    controller.set_led(3, Rgb::green());
    controller.set_led(4, Rgb::cyan());
    controller.set_led(5, Rgb::blue());
    controller.set_led(6, Rgb::purple());
    controller.set_led(7, Rgb::white());
    controller.refresh_state();
    thread::sleep(Duration::from_secs(2));

    // Pattern 2: Checkerboard
    println!("Pattern 2: Checkerboard...");
    controller.clear_leds();
    for row in 0..8 {
        for col in 0..8 {
            let pad = row * 8 + col;
            if (row + col) % 2 == 0 {
                controller.set_led(pad, Rgb::white());
            } else {
                controller.set_led(pad, Rgb::black());
            }
        }
    }
    controller.refresh_state();
    thread::sleep(Duration::from_secs(2));

    // Pattern 3: Spiral
    println!("Pattern 3: Spiral...");
    controller.clear_leds();
    let mut pad_order = Vec::new();

    // Create a spiral pattern of pad indices
    let mut left = 0;
    let mut right = 7;
    let mut top = 0;
    let mut bottom = 7;

    while left <= right && top <= bottom {
        // Top row
        for col in left..=right {
            pad_order.push(top * 8 + col);
        }
        top += 1;

        // Right column
        for row in top..=bottom {
            pad_order.push(row * 8 + right);
        }
        right -= 1;

        // Bottom row (reversed)
        if top <= bottom {
            for col in (left..=right).rev() {
                pad_order.push(bottom * 8 + col);
            }
            bottom -= 1;
        }

        // Left column (reversed)
        if left <= right {
            for row in (top..=bottom).rev() {
                pad_order.push(row * 8 + left);
            }
            left += 1;
        }
    }

    // Light up the spiral pattern
    for (i, pad) in pad_order.iter().enumerate() {
        controller.set_led(
            *pad as u8,
            Rgb::new(
                ((i as f32 / pad_order.len() as f32) * 255.0) as u8,
                ((1.0 - i as f32 / pad_order.len() as f32) * 255.0) as u8,
                128,
            ),
        );
        controller.refresh_state();
        thread::sleep(Duration::from_millis(50));
    }

    thread::sleep(Duration::from_secs(2));

    // Pattern 4: Fade in/out
    println!("Pattern 4: Fade in/out...");
    for intensity in (0..=100).chain((0..100).rev()) {
        controller.clear_leds();
        for pad in 0..64 {
            // Calculate brightness as a percentage of 255, avoiding overflow
            let brightness = ((intensity as u32 * 255) / 100) as u8;
            controller.set_led(pad, Rgb::new(brightness, brightness, brightness));
        }
        controller.refresh_state();
        thread::sleep(Duration::from_millis(20));
    }

    // Clean up
    println!("Cleaning up...");
    controller.clear_leds();
    controller.refresh_state();

    println!("Test complete. Press Ctrl+C to exit.");

    // Keep the program running to handle events
    loop {
        thread::sleep(Duration::from_secs(1));
    }
}
