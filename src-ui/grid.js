// grid.js - Grid visualization
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
                pad.classList.add('modifier');

                // Add visual indicator for current bank
                if (col === appState.currentBank) {
                    pad.classList.add('active-bank');
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

                // Check if a snap exists at this index
                const hasSnap = snapIndex < appState.project.banks[appState.currentBank].snaps.length;

                if (hasSnap && snapIndex === appState.currentSnap) {
                    pad.classList.add('active');
                } else if (hasSnap) {
                    pad.classList.add('has-snap');
                } else {
                    pad.classList.add('empty');
                }

                // Add click handler for all non-modifier pads
                pad.addEventListener('click', async () => {
                    console.log("Pad clicked with snap index:", snapIndex);

                    if (hasSnap) {
                        // Select existing snap
                        console.log("Selecting existing snap");
                        await selectSnap(snapIndex);
                    } else {
                        // Create a new snap when clicking on an empty space
                        console.log("Creating new snap");
                        await createNewSnap(snapIndex);
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
                background-color: #3b82f6;
                border: 2px solid #2563eb;
            }
            
            .grid-pad.has-snap {
                background-color: #4b5563;
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
    pads.forEach(pad => pad.classList.remove('active'));

    // Find the pad for this snap and add active class
    // Remember to add 8 because first row is for modifiers
    const padIndex = snapIndex + 8;
    if (pads[padIndex]) {
        pads[padIndex].classList.add('active');
    }
}