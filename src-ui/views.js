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

    if (appState.currentBank < appState.project.banks.length) {
        elements.bankName.textContent = appState.project.banks[appState.currentBank].name;

        const snaps = appState.project.banks[appState.currentBank].snaps;
        if (appState.currentSnap < snaps.length) {
            elements.snapName.textContent = snaps[appState.currentSnap].name;
            elements.snapDescription.value = snaps[appState.currentSnap].description;
        } else if (snaps.length > 0) {
            // If current snap is out of bounds, select the first snap
            appState.currentSnap = 0;
            elements.snapName.textContent = snaps[0].name;
            elements.snapDescription.value = snaps[0].description;
        } else {
            // No snaps in this bank
            elements.snapName.textContent = "No snaps";
            elements.snapDescription.value = "";
        }
    } else if (appState.project.banks.length > 0) {
        // If current bank is out of bounds, select the first bank
        appState.currentBank = 0;
        elements.bankName.textContent = appState.project.banks[0].name;

        if (appState.project.banks[0].snaps.length > 0) {
            appState.currentSnap = 0;
            elements.snapName.textContent = appState.project.banks[0].snaps[0].name;
            elements.snapDescription.value = appState.project.banks[0].snaps[0].description;
        } else {
            // No snaps in the first bank
            elements.snapName.textContent = "No snaps";
            elements.snapDescription.value = "";
        }
    } else {
        // No banks
        elements.bankName.textContent = "No banks";
        elements.snapName.textContent = "No snaps";
        elements.snapDescription.value = "";
    }

    // Generate grid
    createGrid();

    // Update parameters
    updateParameters();

    // Update tab visibility based on number of parameters
    updateTabVisibility();
}

// Update tab visibility based on the number of parameters
function updateTabVisibility() {
    if (!appState.project) return;

    const paramCount = appState.project.parameters.length;
    const tabButtons = window.snapElements.tabButtons;

    for (let i = 0; i < tabButtons.length; i++) {
        const minParamIndex = i * 16;
        if (minParamIndex < paramCount) {
            tabButtons[i].classList.remove('disabled');
        } else {
            tabButtons[i].classList.add('disabled');
        }
    }
}

// Update the config view
function updateConfigView() {
    const elements = window.snapElements;
    elements.configParamsContainer.innerHTML = '';

    if (!appState.project) return;

    // Force a refresh of the project data before rendering
    import('./tauri-api.js').then(module => {
        module.api.getProject().then(updatedProject => {
            if (updatedProject) {
                // Update the appState with fresh data
                appState.project = updatedProject;

                // Now import and use the config module with updated data
                import('./config.js').then(module => {
                    module.renderParameterList();
                });

                console.log("Config view updated with refreshed project data:",
                    updatedProject.parameters ? updatedProject.parameters.length : 0,
                    "parameters found");
            }
        }).catch(error => {
            console.error("Error refreshing project data:", error);
        });
    });
}