// events.js - Event listeners setup
import { appState, selectSnap, updateParameterValue, updateSnapDescription } from './state.js';
import { switchView } from './views.js';
import { addParameter, setConfigPage } from './config.js';
import { api, fileDialogs, eventBus } from './tauri-api.js';

// Global state for copy/paste
let copiedSnap = null;

// Setup all event listeners
export function setupEventListeners() {
    const elements = window.snapElements;

    // Header navigation
    document.getElementById('snap-btn').addEventListener('click', () => switchView('editor'));
    document.getElementById('conf-btn').addEventListener('click', () => switchView('config'));
    document.getElementById('save-btn').addEventListener('click', saveProject);
    document.getElementById('load-btn').addEventListener('click', loadProject);
    document.getElementById('ai-btn').addEventListener('click', generateAIValues);

    // Welcome view buttons
    document.getElementById('new-project-btn').addEventListener('click', createNewProject);
    document.getElementById('load-project-btn').addEventListener('click', loadProject);

    // Tab buttons for editor view
    elements.tabButtons.forEach((btn, index) => {
        btn.addEventListener('click', () => {
            elements.tabButtons.forEach(b => b.classList.remove('active'));
            btn.classList.add('active');
            appState.currentTab = index;
            import('./parameters.js').then(module => {
                module.updateParameters();
            });
        });
    });

    // Tab buttons for config view
    const configTabButtons = [
        document.getElementById('tab-1-16-conf'),
        document.getElementById('tab-17-32-conf'),
        document.getElementById('tab-33-48-conf'),
        document.getElementById('tab-49-64-conf')
    ];

    configTabButtons.forEach((btn, index) => {
        if (btn) {
            btn.addEventListener('click', () => {
                // Only allow navigating to pages that have parameters
                const paramsPerPage = 16;
                const totalParams = appState.project ? appState.project.parameters.length : 0;

                // Check if this page would have any parameters
                if (index * paramsPerPage < totalParams || index === 0) {
                    import('./config.js').then(module => {
                        module.setConfigPage(index);
                    });
                }
            });
        }
    });

    // Add parameter button
    document.getElementById('add-param-btn').addEventListener('click', addParameter);

    // Snap description
    elements.snapDescription.addEventListener('change', (e) => {
        updateSnapDescription(e.target.value);
    });

    // Copy/Paste buttons
    document.getElementById('copy').addEventListener('click', copyCurrentSnap);
    document.getElementById('paste').addEventListener('click', pasteToCurrentSnap);

    // Controller dropdown
    const controllerSelect = document.getElementById('controller-select');
    if (controllerSelect) {
        controllerSelect.addEventListener('change', async (e) => {
            try {
                await api.setController(e.target.value);
                console.log(`Controller set to ${e.target.value}`);
            } catch (error) {
                console.error('Error setting controller:', error);
            }
        });
    }

    // Initialize controller dropdown with actual device
    initializeControllerDropdown();
}

// Initialize controller dropdown with the current controller
async function initializeControllerDropdown() {
    const controllerSelect = document.getElementById('controller-select');
    if (!controllerSelect) return;

    try {
        // Get available MIDI inputs
        const inputs = await api.getMidiInputs();

        // Clear existing options
        controllerSelect.innerHTML = '';

        // Add options for each available controller
        inputs.forEach(input => {
            const option = document.createElement('option');
            option.value = input;
            option.textContent = input;
            controllerSelect.appendChild(option);
        });

        // Add generic option
        const genericOption = document.createElement('option');
        genericOption.value = 'Generic';
        genericOption.textContent = 'Generic (No Hardware)';
        controllerSelect.appendChild(genericOption);

        // Set the current selection based on project
        if (appState.project && appState.project.controller) {
            controllerSelect.value = appState.project.controller;
        }
    } catch (error) {
        console.error('Error initializing controller dropdown:', error);
    }
}

// Copy the current snap
function copyCurrentSnap() {
    if (!appState.project) return;

    const bank = appState.project.banks[appState.currentBank];
    const snap = bank.snaps[appState.currentSnap];

    // Make a deep copy of the snap
    copiedSnap = {
        name: snap.name + " (Copy)",
        description: snap.description,
        values: [...snap.values] // Create a new array with the same values
    };

    console.log('Copied snap:', copiedSnap);

    // Add a visual feedback that snap was copied
    const activeSnap = document.querySelector('.grid-pad.active');
    if (activeSnap) {
        activeSnap.classList.add('snap-copied');
        setTimeout(() => {
            activeSnap.classList.remove('snap-copied');
        }, 1000);
    }
}

// Paste to the current snap or create a new one
async function pasteToCurrentSnap() {
    if (!appState.project || !copiedSnap) {
        showNotification('No snap has been copied', 'warning');
        return;
    }

    // Get the latest bank information
    const bank = appState.project.banks[appState.currentBank];

    // Check if we need to create a new snap
    const isNewSnap = appState.currentSnap >= bank.snaps.length;

    try {
        if (isNewSnap) {
            console.log("Creating new snap for paste target");

            // Create a new snap
            await api.addSnap(
                appState.currentBank,
                copiedSnap.name,
                copiedSnap.description
            );

            // Get the updated project data to get the new snap index
            const updatedProject = await api.getProject();
            if (updatedProject) {
                appState.project = updatedProject;
                const newSnapIndex = appState.project.banks[appState.currentBank].snaps.length - 1;

                // Select the newly created snap
                console.log("Selecting new snap at index:", newSnapIndex);
                await selectSnap(newSnapIndex);

                // Update the parameter values
                console.log("Updating parameter values for new snap");
                for (let i = 0; i < copiedSnap.values.length; i++) {
                    if (i < appState.project.parameters.length) {
                        await api.editParameter(i, copiedSnap.values[i]);
                    }
                }

                // Refresh the grid to show the new snap
                import('./grid.js').then(module => {
                    module.createGrid();
                });

                // Show success message
                showNotification('Created new snap from copied values', 'success');
            }
        } else {
            // Update the existing snap
            console.log("Updating existing snap with copied values");

            // Update the description
            await api.updateSnapDescription(
                appState.currentBank,
                appState.currentSnap,
                copiedSnap.description
            );

            // Update all parameter values
            for (let i = 0; i < copiedSnap.values.length; i++) {
                if (i < appState.project.parameters.length) {
                    await api.editParameter(i, copiedSnap.values[i]);
                }
            }

            // Get the updated project data
            const updatedProject = await api.getProject();
            if (updatedProject) {
                appState.project = updatedProject;
            }

            // Refresh the parameters display
            import('./parameters.js').then(module => {
                module.updateParameters();
            });

            // Show success message
            showNotification('Updated snap with copied values', 'success');
        }
    } catch (error) {
        console.error('Error during paste operation:', error);
        showNotification('Error during paste operation', 'error');
    }
}

// Create a new project
async function createNewProject() {
    console.log('Creating new project...');

    try {
        await api.newProject();

        // Force refresh the state
        const project = await api.getProject();
        if (project) {
            appState.project = project;

            // Switch to editor view and refresh UI
            import('./views.js').then(module => {
                module.switchView('editor');
            });

            // Show a notification
            showNotification("New project created", 'success');
        } else {
            console.error("Failed to get project after creation");
            showNotification("Failed to create new project", 'error');
        }
    } catch (error) {
        console.error('Error creating new project:', error);
        showNotification("Error creating new project", 'error');
    }
}

// Load a project from file
async function loadProject() {
    console.log('Loading project...');

    try {
        const filePath = await fileDialogs.loadProjectDialog();
        if (filePath) {
            console.log("Project loaded from:", filePath);

            // Force refresh the state from backend
            const project = await api.getProject();
            if (project) {
                appState.project = project;

                // Switch to editor view and refresh UI
                import('./views.js').then(module => {
                    module.switchView('editor');
                });

                // Show a notification
                showNotification(`Project loaded: ${filePath.split('/').pop()}`, 'success');
            } else {
                console.error("Failed to get project after loading");
                showNotification("Failed to load project", 'error');
            }
        }
    } catch (error) {
        console.error('Error loading project:', error);
        showNotification("Error loading project", 'error');
    }
}

// Helper function to show notifications
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
            if (notification.parentNode) {
                document.body.removeChild(notification);
            }
        }, 500);
    }, 3000);
}

// Save the current project
async function saveProject() {
    console.log('Saving project...');

    if (!appState.project) {
        showNotification('No project to save', 'warning');
        return;
    }

    try {
        const path = await fileDialogs.saveProjectDialog();
        if (path) {
            console.log(`Project saved to: ${path}`);

            // Show success notification
            showNotification(`Project saved: ${path.split('/').pop()}`, 'success');

            // Also update the project name in the UI if it's based on the file
            const fileName = path.split('/').pop().replace('.sb', '');

            // Only update if using default name
            if (appState.project.project_name === "New Project") {
                // Update the project name
                appState.project.project_name = fileName;

                // Update the UI
                const projectNameElement = document.getElementById('project-name');
                if (projectNameElement) {
                    projectNameElement.textContent = fileName;
                }
            }
        }
    } catch (error) {
        console.error('Error saving project:', error);
        showNotification('Error saving project', 'error');
    }
}

// Generate AI values
async function generateAIValues() {
    if (!appState.project) return;

    // Check if we have an OpenAI API key
    if (!appState.project.openai_api_key) {
        const apiKey = prompt('Please enter your OpenAI API key:');
        if (apiKey) {
            try {
                await api.setOpenAIApiKey(apiKey);
            } catch (error) {
                console.error('Error setting OpenAI API key:', error);
                return;
            }
        } else {
            return; // User canceled
        }
    }

    try {
        await api.generateAIValues(appState.currentBank, appState.currentSnap);
        console.log('AI value generation requested');
    } catch (error) {
        console.error('Error generating AI values:', error);
    }
}