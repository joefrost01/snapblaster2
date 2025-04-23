import { appState, selectSnap, createNewSnap } from './state.js';

// Create and update the launchpad grid
export function createGrid() {
    const elements = window.snapElements;
    elements.gridContainer.innerHTML = '';

    if (!appState.project) return;

    console.log("Creating grid UI with current snap:", appState.currentSnap);

    // Create 8x8 grid (64 pads)
    for (let row = 0; row < 8; row++) {
        for (let col = 0; col < 8; col++) {
            const pad = document.createElement('div');
            pad.className = 'grid-pad';

            // Calculate pad index (0-63)
            const padIndex = row * 8 + col;

            // First row is for modifiers
            if (row === 0) {
                pad.classList.add('modifier'); // RED for modifiers (top row)

                // Add visual indicator for current bank
                if (col === appState.currentBank) {
                    pad.classList.add('active-bank'); // RED with border for active bank
                }

                // Add click handler for bank selection
                pad.addEventListener('click', () => {
                    // In a real implementation, this would switch banks
                    // But for now, just highlight the bank
                    if (col < appState.project.banks.length) {
                        // Remove active class from all modifiers
                        const modifiers = elements.gridContainer.querySelectorAll('.grid-pad.modifier');
                        modifiers.forEach(m => m.classList.remove('active-bank'));

                        // Add active class to selected bank
                        pad.classList.add('active-bank');

                        // Update state (in a real implementation, this would trigger a bank change)
                        appState.currentBank = col;
                    }
                });
            }
            // Others are for snaps (indices 8-63, map to snap 0-55)
            else {
                const snapIndex = padIndex - 8;

                // Store the snap index in the element's data attribute
                pad.dataset.snapIndex = snapIndex;

                // Access the snap if it exists
                const bank = appState.project.banks[appState.currentBank];

                // Check if a snap exists at this position
                const hasSnap = snapIndex < bank.snaps.length &&
                    bank.snaps[snapIndex] &&
                    bank.snaps[snapIndex].name !== '';

                if (hasSnap && snapIndex === appState.currentSnap) {
                    pad.classList.add('active');  // GREEN for selected snap
                } else if (hasSnap) {
                    pad.classList.add('has-snap'); // YELLOW for available snaps
                } else {
                    pad.classList.add('empty');  // OFF/dim for empty slots
                }

                // Add click handler for all non-modifier pads
                pad.addEventListener('click', async (event) => {
                    // Prevent rapid multiple clicks
                    event.preventDefault();
                    event.stopPropagation();

                    console.log("Pad clicked with snap index:", snapIndex);

                    // Disable the pad temporarily to prevent double-clicks
                    pad.style.pointerEvents = 'none';

                    try {
                        if (hasSnap) {
                            // Select existing snap
                            console.log("Selecting existing snap");
                            await selectSnap(snapIndex);
                        } else {
                            // Create a new snap when clicking on an empty space
                            console.log("Creating new snap");
                            await createNewSnap(snapIndex);
                        }
                    } catch (error) {
                        console.error("Error handling pad click:", error);
                    } finally {
                        // Re-enable the pad after a short delay
                        setTimeout(() => {
                            pad.style.pointerEvents = 'auto';
                        }, 500);
                    }
                });
            }

            elements.gridContainer.appendChild(pad);
        }
    }

    // Add CSS for grid styling
    const style = document.createElement('style');
    if (!document.querySelector('#grid-styles')) {
        style.id = 'grid-styles';
        style.textContent = `
            .grid-pad.active-bank {
                background-color: #ef4444; /* RED for modifiers (consistent with hardware) */
                border: 2px solid #b91c1c;
            }
            
            .grid-pad.has-snap {
                background-color: #eab308; /* YELLOW for available snaps */
            }
            
            .grid-pad.active {
                background-color: #22c55e !important; /* GREEN for selected snap */
                border: 2px solid #16a34a;
            }
            
            .grid-pad.empty:hover {
                border-color: #f97316;
            }
        `;
        document.head.appendChild(style);
    }

    console.log("Grid created with", elements.gridContainer.querySelectorAll('.grid-pad').length, "pads");
}

// Highlight a specific snap in the grid
export function highlightSnap(snapIndex) {
    const elements = window.snapElements;

    // Remove active class from all pads
    const pads = elements.gridContainer.querySelectorAll('.grid-pad');
    pads.forEach(pad => {
        pad.classList.remove('active');
        pad.classList.remove('midi-sending');
    });

    // Find the pad for this snap and add active class
    // Remember to add 8 because first row is for modifiers
    const padIndex = snapIndex + 8;
    if (pads[padIndex]) {
        pads[padIndex].classList.add('active');

        // Add a brief MIDI sending effect
        pads[padIndex].classList.add('midi-sending');
        setTimeout(() => {
            pads[padIndex].classList.remove('midi-sending');
        }, 200);
    }
}