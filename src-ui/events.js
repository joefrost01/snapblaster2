// events.js - Event listeners setup
import {appState, updateSnapDescription} from './state.js';
import {switchView} from './views.js';
import {addParameter} from './config.js';
import {api, eventBus, fileDialogs} from './tauri-api.js';

// Global state for copy/paste
let copiedSnap = null;

// Setup all event listeners
export function setupEventListeners() {
    const elements = window.snapElements;

    // Header navigation
    document.getElementById('snap-btn').addEventListener('click', () => switchView('editor'));
    document.getElementById('conf-btn').addEventListener('click', () => switchView('config'));
    document.getElementById('new-btn').addEventListener('click', handleNewClick);
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

            // Store current tab in app state
            appState.currentTab = index;

            // Update UI to show correct parameters
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

    const midiIndicator = document.getElementById('midi-status-indicator');
    if (midiIndicator) {
        // Check MIDI status on initial load
        setTimeout(() => {
            api.getMidiOutputs().then(outputs => {
                if (outputs && outputs.length > 0) {
                    midiIndicator.classList.remove('inactive');
                    midiIndicator.classList.add('active');
                    midiIndicator.title = `Connected MIDI outputs: ${outputs.join(', ')}`;
                }
            }).catch(err => console.error('Error checking MIDI status:', err));
        }, 1000);

        // Update when MIDI activity happens
        eventBus.on('cc-value-changed', () => {
            // Flash the indicator on MIDI activity
            midiIndicator.classList.add('midi-active');
            setTimeout(() => {
                midiIndicator.classList.remove('midi-active');
            }, 100);
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

    try {
        const bank = appState.project.banks[appState.currentBank];
        if (!bank || appState.currentSnap >= bank.snaps.length) {
            console.error("Invalid bank or snap index for copy operation");
            showNotification('No valid snap to copy', 'warning');
            return;
        }

        const snap = bank.snaps[appState.currentSnap];

        // Make a deep copy of the snap to prevent reference issues
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

        showNotification('Snap copied to clipboard', 'success');
    } catch (error) {
        console.error("Error during copy operation:", error);
        showNotification('Error copying snap', 'error');
    }
}

// Paste to the current snap or create a new one
async function pasteToCurrentSnap() {
    if (!appState.project || !copiedSnap) return showNotification('No snap copied', 'warning');
    document.body.classList.add('processing');
    try {
        const bank = appState.currentBank;
        const snap = appState.currentSnap;
        // 1) Update description immediately
        await api.updateSnapDescription(bank, snap, copiedSnap.description);
        window.snapElements.snapDescription.value = copiedSnap.description;
        appState.project.banks[bank].snaps[snap].description = copiedSnap.description;

        // 2) Build and fire off all CC edits in parallel
        const paramCount = appState.project.parameters.length;
        const promises = [];
        const values = copiedSnap.values.slice(0, paramCount);
        while (values.length < paramCount) values.push(64);

        values.forEach((val, i) => {
            // Optimistically update UI state
            const slider = document.querySelector(`input[data-param-id="${i}"]`);
            const display = document.getElementById(`value-${i}`);
            if (slider) slider.value = val;
            if (display) display.textContent = val;
            // Update in-memory model
            appState.project.banks[bank].snaps[snap].values[i] = val;
            // Queue the Tauri IPC call
            promises.push(api.editParameter(i, val));
        });

        // Wait for all to finish
        await Promise.all(promises);

        // Final full re-render (just in case)
        (await import('./parameters.js')).updateParameters();
        showNotification('Snap pasted!', 'success');
    } catch (err) {
        console.error(err);
        showNotification('Error pasting snap', 'error');
    } finally {
        document.body.classList.remove('processing');
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

            // Switch to config view and refresh UI
            import('./views.js').then(module => {
                module.switchView('config');
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
        // Add loading state
        document.body.classList.add('processing');

        const filePath = await fileDialogs.loadProjectDialog();
        if (filePath) {
            console.log("Project loaded from:", filePath);

            // Force refresh the state from backend with a direct call
            const project = await api.getProject();

            // Validate that we have a proper project with parameters
            if (project && project.parameters) {
                console.log("Project loaded successfully with",
                    project.parameters.length, "parameters and",
                    project.banks[0].snaps.length, "snaps in the first bank");

                appState.project = project;
                appState.currentBank = 0;
                appState.currentSnap = 0;

                // Switch to editor view
                import('./views.js').then(module => {
                    module.switchView('editor');

                    // After a small delay, update the parameters to ensure they're shown correctly
                    setTimeout(() => {
                        import('./parameters.js').then(paramModule => {
                            paramModule.updateParameters();
                        });
                    }, 200);
                });

                // Show a notification
                showNotification(`Project loaded: ${filePath.split('/').pop()}`, 'success');
            } else {
                console.error("Failed to get a valid project after loading");
                showNotification("Failed to load project - invalid format", 'error');
            }
        }
    } catch (error) {
        console.error('Error loading project:', error);
        showNotification("Error loading project", 'error');
    } finally {
        // Remove loading state
        document.body.classList.remove('processing');
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
        // Ensure all snaps have values for all parameters
        const paramCount = appState.project.parameters.length;
        appState.project.banks.forEach(bank => {
            bank.snaps.forEach(snap => {
                // Resize values array if needed
                if (snap.values.length < paramCount) {
                    // Preserve existing values
                    const currentValues = [...snap.values];
                    snap.values = Array(paramCount).fill(64);

                    // Copy over existing values
                    currentValues.forEach((val, idx) => {
                        snap.values[idx] = val;
                    });
                }
            });
        });

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
            appState.isDirty = false;
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

eventBus.on('project-loaded', async () => {
    try {
        // Get the updated project from backend with explicit call
        const project = await api.getProject();

        if (project) {
            console.log("Project loaded event received, updating state with",
                project.parameters ? project.parameters.length : 0, "parameters");

            // Update the state
            appState.project = project;
            appState.currentBank = 0;
            appState.currentSnap = 0;

            // Update UI with a small delay to ensure DOM is ready
            setTimeout(() => {
                import('./views.js').then(module => {
                    module.switchView('editor');
                });
            }, 100);
        }
    } catch (error) {
        console.error("Error handling project-loaded event:", error);
    }
});

function confirmAsync(message) {
    return new Promise(resolve => {
        resolve(window.confirm(message));
    });
}

// Guarded handler for the header “New” button
async function handleNewClick(evt) {
    // prevent any default if it’s ever inside a form
    evt?.preventDefault?.();

    if (appState.isDirty) {
        // this returns a Promise<boolean>
        const ok = await confirmAsync(
            'You have unsaved changes. Creating a new project will discard them.\n\nContinue?'
        );
        if (!ok) {
            console.log('User cancelled new‑project');
            return;
        }
    }

    // only now do we blow everything away
    await createNewProject();
    appState.isDirty = false;
}
