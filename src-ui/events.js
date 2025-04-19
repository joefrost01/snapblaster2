// events.js - Event listeners setup
import { appState, selectSnap, updateParameterValue, updateSnapDescription } from './state.js';
import { switchView } from './views.js';
import { addParameter } from './config.js';

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

    // Tab buttons
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

    // Add parameter button
    document.getElementById('add-param-btn').addEventListener('click', addParameter);

    // Snap description
    elements.snapDescription.addEventListener('change', (e) => {
        updateSnapDescription(e.target.value);
    });

    // Create mock project for testing (temporary)
    createMockProject();
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