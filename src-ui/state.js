// state.js - Application state management
import { api, eventBus } from './tauri-api.js';

// Global application state
export const appState = {
    currentView: 'welcome', // welcome, editor, config
    currentBank: 0,
    currentSnap: 0,
    currentTab: 0,
    project: null,
    isLoading: false
};

// Initialize state from backend
export async function initializeState() {
    try {
        appState.isLoading = true;

        // Initialize the Tauri API
        await api.initialize();

        // Get project data from backend
        const project = await api.getProject();

        // Only consider it a valid starting project if it has parameters AND snaps
        // This prevents the default empty project from bypassing the welcome screen
        if (project &&
            project.banks &&
            project.banks.length > 0 &&
            project.banks[0].snaps.length > 0 &&
            project.parameters &&
            project.parameters.length > 0) {

            console.log("Loaded existing project with parameters:", project);
            appState.project = project;
            appState.hasInitialProject = true;
        } else {
            console.log("No valid project with parameters found, starting with welcome screen");
            appState.project = project; // Still keep the project data if it exists
            appState.hasInitialProject = false;
        }

        // Set up event listeners
        setupEventListeners();

        appState.isLoading = false;
    } catch (error) {
        console.error('Error initializing state:', error);
        appState.isLoading = false;
        appState.hasInitialProject = false;
    }
}

// Set up event listeners for backend events
function setupEventListeners() {
    // Listen for snap selection events
    eventBus.on('snap-selected', ({ bank, snapId }) => {
        appState.currentBank = bank;
        appState.currentSnap = snapId;

        // Update the UI
        import('./grid.js').then(module => {
            module.createGrid();
        });

        import('./parameters.js').then(module => {
            module.updateParameters();
        });
    });

    // Listen for parameter edit events
    eventBus.on('parameter-edited', ({ paramId, value }) => {
        if (!appState.project) return;

        // Update local state
        const snap = appState.project.banks[appState.currentBank].snaps[appState.currentSnap];
        snap.values[paramId] = value;

        // Update the parameter display
        const valueElement = document.getElementById(`value-${paramId}`);
        if (valueElement) {
            valueElement.textContent = value;
        }

        // Update slider if this event wasn't triggered by the slider
        const slider = document.querySelector(`input[data-param-id="${paramId}"]`);
        if (slider && slider.value != value) {
            slider.value = value;
        }
    });

    // Listen for project loaded events
    eventBus.on('project-loaded', async () => {
        // Get the updated project from backend
        const project = await api.getProject();
        if (project) {
            appState.project = project;
            appState.currentBank = 0;
            appState.currentSnap = 0;

            // Update UI
            import('./views.js').then(module => {
                module.switchView('editor');
            });
        }
    });

    // Listen for AI generation completion
    eventBus.on('ai-generation-completed', ({ bankId, snapId, values }) => {
        if (!appState.project) return;

        // Update the snap values
        const snap = appState.project.banks[bankId].snaps[snapId];
        snap.values = values;

        // If this is the current snap, update the UI
        if (bankId === appState.currentBank && snapId === appState.currentSnap) {
            import('./parameters.js').then(module => {
                module.updateParameters();
            });
        }

        // Show notification
        showNotification('AI values generated successfully');
    });

    // Listen for AI generation failure
    eventBus.on('ai-generation-failed', ({ error }) => {
        showNotification(`AI generation failed: ${error}`, 'error');
    });

    // Listen for morph progress
    eventBus.on('morph-progressed', ({ progress }) => {
        // Update morph progress indicator if we add one
        console.log(`Morph progress: ${Math.round(progress * 100)}%`);
    });
}

// Helper functions for state manipulation
export async function selectSnap(snapIndex) {
    try {
        console.log("Selecting snap at index:", snapIndex);

        // Update local state first to make UI more responsive
        appState.currentSnap = snapIndex;

        // Update UI immediately before backend call completes
        updateSnapDetailsInUI(snapIndex);
        highlightCurrentSnap();

        // Then make the backend call
        await api.selectSnap(appState.currentBank, snapIndex);

        // Update parameters display with the correct values
        import('./parameters.js').then(module => {
            module.updateParameters();
        });

    } catch (error) {
        console.error('Error selecting snap:', error);
        // Revert to previous state on error
        const currentProject = await api.getProject();
        if (currentProject) {
            appState.project = currentProject;
        }
    }
}

// Create a new snap at the specified index
export async function createNewSnap(snapIndex) {
    if (!appState.project) return;

    try {
        console.log("Creating new snap at pad position:", snapIndex);

        // Disable UI during operation to prevent multiple clicks
        document.body.classList.add('processing');

        // Create the new snap with pad information
        await api.addSnap(
            appState.currentBank,
            snapIndex,
            `Snap at Pad ${snapIndex + 1}`,
            "New snap"
        );

        // Refresh the project data to get the new snap
        const updatedProject = await api.getProject();
        if (updatedProject) {
            appState.project = updatedProject;

            console.log("New snap created, selecting it at pad index:", snapIndex);

            // Select the new snap
            appState.currentSnap = snapIndex;
            await selectSnap(snapIndex);

            // Refresh the grid to show the new snap
            import('./grid.js').then(module => {
                module.createGrid();
            });

            // Refresh the parameters display
            import('./parameters.js').then(module => {
                module.updateParameters();
            });

            // Show a notification
            showSuccessNotification(`Created new snap at pad position ${snapIndex + 1}`);
        }
    } catch (error) {
        console.error('Error creating new snap:', error);
        showErrorNotification('Error creating new snap');
    } finally {
        // Re-enable UI
        document.body.classList.remove('processing');
    }
}

export async function updateParameterValue(paramId, value) {
    if (!appState.project) return;

    try {
        await api.editParameter(paramId, value);
        // The event listener will update the UI when the backend confirms
    } catch (error) {
        console.error('Error updating parameter value:', error);
    }
}

export async function updateSnapDescription(description) {
    if (!appState.project) return;

    try {
        await api.updateSnapDescription(
            appState.currentBank,
            appState.currentSnap,
            description
        );

        // Update local state
        const snap = appState.project.banks[appState.currentBank].snaps[appState.currentSnap];
        snap.description = description;
    } catch (error) {
        console.error('Error updating snap description:', error);
    }
}

// Simple notification function
function showNotification(message, type = 'info') {
    const notification = document.createElement('div');
    notification.className = `notification ${type}`;
    notification.textContent = message;

    // Add to DOM
    document.body.appendChild(notification);

    // Auto-remove after 3 seconds
    setTimeout(() => {
        notification.classList.add('fadeout');
        setTimeout(() => {
            document.body.removeChild(notification);
        }, 500);
    }, 3000);
}

function updateSnapDetailsInUI(snapIndex) {
    const elements = window.snapElements;
    if (!elements) return;

    if (appState.project &&
        appState.currentBank < appState.project.banks.length &&
        snapIndex < appState.project.banks[appState.currentBank].snaps.length) {

        const snap = appState.project.banks[appState.currentBank].snaps[snapIndex];

        // Update snap name and description if elements exist
        if (elements.snapName) {
            elements.snapName.textContent = snap.name;
        }

        if (elements.snapDescription) {
            elements.snapDescription.value = snap.description;
        }
    }
}

function highlightCurrentSnap() {
    import('./grid.js').then(module => {
        if (module.highlightSnap) {
            module.highlightSnap(appState.currentSnap);
        }
    });
}

function showSuccessNotification(message) {
    const notification = document.createElement('div');
    notification.className = 'notification success';
    notification.textContent = message;
    document.body.appendChild(notification);

    setTimeout(() => {
        notification.classList.add('fadeout');
        setTimeout(() => {
            if (notification.parentNode) {
                document.body.removeChild(notification);
            }
        }, 500);
    }, 3000);
}


function showErrorNotification(message) {
    const notification = document.createElement('div');
    notification.className = 'notification error';
    notification.textContent = message;
    document.body.appendChild(notification);

    setTimeout(() => {
        notification.classList.add('fadeout');
        setTimeout(() => {
            if (notification.parentNode) {
                document.body.removeChild(notification);
            }
        }, 500);
    }, 3000);
}
