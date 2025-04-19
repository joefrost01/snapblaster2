// grid.js - Grid visualization
import { appState, selectSnap } from './state.js';

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
                }

                // Add click handler for valid snaps
                if (row > 0) {
                    pad.addEventListener('click', () => {
                        if (hasSnap) {
                            selectSnap(snapIndex);
                        }
                    });
                }
            }

            elements.gridContainer.appendChild(pad);
        }
    }
}