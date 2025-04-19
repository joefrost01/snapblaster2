// state.js - Application state management

// Global application state
export const appState = {
    currentView: 'welcome', // welcome, editor, config
    currentBank: 0,
    currentSnap: 0,
    currentTab: 0,
    project: null
};

// Helper functions for state manipulation
export function selectSnap(snapIndex) {
    appState.currentSnap = snapIndex;

    // Update UI to reflect the selected snap
    const elements = window.snapElements;

    // Update snap info
    if (appState.project) {
        const snap = appState.project.banks[appState.currentBank].snaps[snapIndex];

        if (snap) {
            elements.snapName.textContent = snap.name;
            elements.snapDescription.value = snap.description;

            // Highlight active pad in grid
            const pads = elements.gridContainer.querySelectorAll('.grid-pad');
            pads.forEach(pad => pad.classList.remove('active'));

            // +8 because first row (8 pads) are modifiers
            if (pads[snapIndex + 8]) {
                pads[snapIndex + 8].classList.add('active');
            }

            // Update parameters for this snap
            import('./parameters.js').then(module => {
                module.updateParameters();
            });
        }
    }
}

// Create a new snap at the specified index
export function createNewSnap(snapIndex) {
    if (!appState.project) return;

    const bank = appState.project.banks[appState.currentBank];

    // Create snaps to fill any gaps
    while (bank.snaps.length < snapIndex) {
        bank.snaps.push({
            name: `Snap ${bank.snaps.length + 1}`,
            description: "Empty snap",
            values: Array(appState.project.parameters.length).fill(64) // Default middle value
        });
    }

    // Create the new snap
    bank.snaps[snapIndex] = {
        name: `Snap ${snapIndex + 1}`,
        description: "New snap",
        values: Array(appState.project.parameters.length).fill(64) // Default middle value
    };

    // Select the new snap
    selectSnap(snapIndex);

    // Recreate the grid to reflect the changes
    import('./grid.js').then(module => {
        module.createGrid();
    });

    console.log(`Created new snap at index ${snapIndex}`);
}

export function updateParameterValue(paramId, value) {
    if (!appState.project) return;

    const snap = appState.project.banks[appState.currentBank].snaps[appState.currentSnap];

    // Update the value in the state
    snap.values[paramId] = value;

    // In a real app, we'd send this to the backend
    console.log(`Updated parameter ${paramId} to ${value}`);
}

export function updateSnapDescription(description) {
    if (!appState.project) return;

    const snap = appState.project.banks[appState.currentBank].snaps[appState.currentSnap];
    snap.description = description;

    console.log(`Updated snap description: ${description}`);
}