// events.js - Event listeners setup
import { appState, selectSnap, updateParameterValue, updateSnapDescription } from './state.js';
import { switchView } from './views.js';
import { addParameter, setConfigPage } from './config.js';

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

    // Create mock project for testing (temporary)
    createMockProject();
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
    alert('Snap copied');
}

// Paste to the current snap or create a new one
function pasteToCurrentSnap() {
    if (!appState.project || !copiedSnap) {
        alert('No snap has been copied');
        return;
    }

    const bank = appState.project.banks[appState.currentBank];

    // Create a new snap with the copied values if we're at the end
    if (appState.currentSnap >= bank.snaps.length) {
        bank.snaps.push({
            name: copiedSnap.name,
            description: copiedSnap.description,
            values: [...copiedSnap.values]
        });

        // Select the new snap
        selectSnap(bank.snaps.length - 1);
    } else {
        // Otherwise overwrite the current snap
        bank.snaps[appState.currentSnap].name = copiedSnap.name;
        bank.snaps[appState.currentSnap].description = copiedSnap.description;
        bank.snaps[appState.currentSnap].values = [...copiedSnap.values];

        // Refresh the UI
        selectSnap(appState.currentSnap);
    }

    console.log('Pasted snap values');
}

// Create a new project
function createNewProject() {
    console.log('Creating new project...');

    // In a real app, this would call the backend
    // For now, just create a basic project
    appState.project = {
        project_name: "New Project",
        controller: "Launchpad X",
        openai_api_key: null,
        banks: [{
            name: "Default Bank",
            snaps: [{
                name: "Initial Snap",
                description: "A starting point",
                values: Array(64).fill(64)
            }]
        }],
        parameters: []
    };

    // Switch to editor view
    switchView('editor');
}

// Load a project from file
function loadProject() {
    console.log('Load project clicked');

    // In a real app, this would open a file dialog
    // For now, just use the mock project
    createMockProject();

    // Switch to editor view
    switchView('editor');
}

// Save the current project
function saveProject() {
    console.log('Save project clicked');

    // In a real app, this would open a save dialog
    console.log('Current project state:', appState.project);

    alert('Project saved (mock)');
}

// Generate AI values
function generateAIValues() {
    console.log('Generate AI values clicked');

    // In a real app, this would call the OpenAI API
    alert('AI value generation is not implemented in this demo');
}

// Create a mock project for testing
function createMockProject() {
    appState.project = {
        project_name: "Demo Project",
        controller: "Launchpad X",
        openai_api_key: null,
        banks: [{
            name: "Default Bank",
            snaps: [
                {
                    name: "Intro",
                    description: "Ambient intro section",
                    values: [64, 32, 96, 48, 60, 75, 88, 64]
                },
                {
                    name: "Drop",
                    description: "Main drop section",
                    values: [100, 64, 120, 80, 90, 110, 64, 50]
                },
                {
                    name: "Breakdown",
                    description: "Atmospheric breakdown",
                    values: [32, 110, 64, 20, 40, 85, 60, 30]
                }
            ]
        }],
        parameters: [
            { name: "Bass Level", description: "Main bass volume", cc: 1 },
            { name: "Reverb Mix", description: "Wet/dry mix for reverb", cc: 2 },
            { name: "Delay Time", description: "Delay time in ms", cc: 3 },
            { name: "Filter Cutoff", description: "Main filter cutoff frequency", cc: 4 },
            { name: "LFO Rate", description: "Modulation rate", cc: 5 },
            { name: "LFO Depth", description: "Modulation amount", cc: 6 },
            { name: "Distortion", description: "Distortion amount", cc: 7 },
            { name: "Master Volume", description: "Overall volume", cc: 8 }
        ]
    };
}