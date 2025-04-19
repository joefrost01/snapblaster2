// grid.js - Grid visualization
import { appState, selectSnap, createNewSnap } from './state.js';

// Create and update the launchpad grid
export function createGrid() {
    const elements = window.snapElements;
    elements.gridContainer.innerHTML = '';

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
            }
            // Others are for snaps (indices 8-63, map to snap 0-55)
            else {
                const snapIndex = padIndex - 8;

                // Check if a snap exists at this index
                const hasSnap = snapIndex < appState.project.banks[appState.currentBank].snaps.length;

                if (hasSnap && snapIndex === appState.currentSnap) {
                    pad.classList.add('active');
                } else if (!hasSnap) {
                    pad.classList.add('empty');
                }

                // Add click handler for all non-modifier pads
                pad.addEventListener('click', () => {
                    if (hasSnap) {
                        // Select existing snap
                        selectSnap(snapIndex);
                    } else {
                        // Create a new snap when clicking on an empty space
                        createNewSnap(snapIndex);
                    }
                });
            }

            elements.gridContainer.appendChild(pad);
        }
    }
}