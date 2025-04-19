// views.js - UI views management
import { appState } from './state.js';
import { createGrid } from './grid.js';
import { updateParameters } from './parameters.js';

// Store DOM elements
window.snapElements = {};

// Initialize and cache DOM elements
export function initializeViews() {
    const elements = window.snapElements;

    // Views
    elements.welcomeView = document.getElementById('welcome-view');
    elements.editorView = document.getElementById('editor-view');
    elements.configView = document.getElementById('config-view');

    // Navigation buttons
    elements.snapBtn = document.getElementById('snap-btn');
    elements.confBtn = document.getElementById('conf-btn');

    // Project info
    elements.projectName = document.getElementById('project-name');
    elements.bankName = document.getElementById('bank-name');
    elements.snapName = document.getElementById('snap-name');
    elements.snapDescription = document.getElementById('snap-description');

    // Grid and parameters
    elements.gridContainer = document.getElementById('grid-container');
    elements.parametersContainer = document.getElementById('parameters-container');

    // Tab buttons
    elements.tabButtons = [
        document.getElementById('tab-1-16'),
        document.getElementById('tab-17-32'),
        document.getElementById('tab-33-48'),
        document.getElementById('tab-49-64')
    ];

    // Config view
    elements.configParamsContainer = document.getElementById('config-params-container');
}

// Switch between different views
export function switchView(viewName) {
    console.log(`Switching to ${viewName} view`);

    const elements = window.snapElements;

    // Update state
    appState.currentView = viewName;

    // Hide all views
    elements.welcomeView.classList.add('hidden');
    elements.editorView.classList.add('hidden');
    elements.configView.classList.add('hidden');

    // Show selected view
    switch (viewName) {
        case 'welcome':
            elements.welcomeView.classList.remove('hidden');
            break;
        case 'editor':
            elements.editorView.classList.remove('hidden');
            updateEditorView();
            break;
        case 'config':
            elements.configView.classList.remove('hidden');
            updateConfigView();
            break;
    }

    // Update header buttons
    elements.snapBtn.classList.remove('active');
    elements.confBtn.classList.remove('active');

    if (viewName === 'editor') {
        elements.snapBtn.classList.add('active');
    } else if (viewName === 'config') {
        elements.confBtn.classList.add('active');
    }
}

// Update the editor view with current project data
function updateEditorView() {
    if (!appState.project) return;

    const elements = window.snapElements;

    // Update project info
    elements.projectName.textContent = appState.project.project_name;
    elements.bankName.textContent = appState.project.banks[appState.currentBank].name;
    elements.snapName.textContent = appState.project.banks[appState.currentBank].snaps[appState.currentSnap].name;
    elements.snapDescription.value = appState.project.banks[appState.currentBank].snaps[appState.currentSnap].description;

    // Generate grid
    createGrid();

    // Update parameters
    updateParameters();
}

// Update the config view
function updateConfigView() {
    const elements = window.snapElements;
    elements.configParamsContainer.innerHTML = '';

    if (!appState.project) return;

    // Import and use the config module
    import('./config.js').then(module => {
        module.renderParameterList();
    });
}