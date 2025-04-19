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
        if (project && project.banks && project.banks.length > 0) {
            // Only consider it a valid project if it has at least one bank
            console.log("Loaded existing project:", project);
            appState.project = project;
            appState.hasInitialProject = true;
        } else {
            console.log("No valid project found, starting with welcome screen");
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
        await api.selectSnap(appState.currentBank, snapIndex);
        // The event listener will update the UI when the backend confirms
    } catch (error) {
        console.error('Error selecting snap:', error);
    }
}

// Create a new snap at the specified index
export async function createNewSnap(snapIndex) {
    if (!appState.project) return;

    try {
        console.log("Creating new snap at index:", snapIndex);

        // Create the new snap with a named based on its position
        await api.addSnap(
            appState.currentBank,
            `Snap ${snapIndex + 1}`,
            "New snap"
        );

        // Refresh the project data to get the new snap
        const updatedProject = await api.getProject();
        if (updatedProject) {
            appState.project = updatedProject;

            // Calculate the actual index of the new snap
            // It might be at the end rather than exactly at snapIndex
            const actualIndex = appState.project.banks[appState.currentBank].snaps.length - 1;

            console.log("New snap created, selecting it at index:", actualIndex);

            // Select the new snap
            await selectSnap(actualIndex);

            // Refresh the grid to show the new snap
            import('./grid.js').then(module => {
                module.createGrid();
            });

            // Show a notification
            const notification = document.createElement('div');
            notification.className = 'notification success';
            notification.textContent = `Created new snap at position ${actualIndex + 1}`;
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
    } catch (error) {
        console.error('Error creating new snap:', error);

        // Show error notification
        const notification = document.createElement('div');
        notification.className = 'notification error';
        notification.textContent = 'Error creating new snap';
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