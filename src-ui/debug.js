// debug.js - Debug utilities for Snap-Blaster
import { eventBus } from './tauri-api.js';

// Enable debug mode for additional logging
const DEBUG = true;

// Function to initialize debugging
export function initializeDebug() {
    if (!DEBUG) return;

    // Add debug event listeners
    setupEventDebuggers();

    // Add debug UI if needed
    addDebugUI();

    console.log("Debug mode initialized");
}

// Set up event debuggers
function setupEventDebuggers() {
    // Log all events from the backend
    eventBus.on('snap-selected', (data) => {
        console.log("Event: snap-selected", data);
    });

    eventBus.on('pad-pressed', (data) => {
        console.log("Event: pad-pressed", data);
    });

    eventBus.on('parameter-edited', (data) => {
        console.log("Event: parameter-edited", data);
    });

    eventBus.on('project-loaded', () => {
        console.log("Event: project-loaded");
    });

    eventBus.on('project-saved', () => {
        console.log("Event: project-saved");
    });

    eventBus.on('ai-generation-completed', (data) => {
        console.log("Event: ai-generation-completed", data);
    });

    eventBus.on('ai-generation-failed', (data) => {
        console.log("Event: ai-generation-failed", data);
    });

    eventBus.on('morph-progressed', (data) => {
        console.log("Event: morph-progressed", data.progress);
    });

    eventBus.on('morph-completed', () => {
        console.log("Event: morph-completed");
    });
}

// Add debug UI elements
function addDebugUI() {
    // Add a debug panel to the UI if DEBUG is true
    if (!DEBUG) return;

    const debugStyle = document.createElement('style');
    debugStyle.textContent = `
        .debug-panel {
            position: fixed;
            bottom: 0;
            left: 0;
            width: 100%;
            max-height: 200px;
            overflow-y: auto;
            background-color: rgba(0, 0, 0, 0.8);
            color: #00ff00;
            font-family: monospace;
            font-size: 12px;
            padding: 8px;
            z-index: 9999;
            border-top: 1px solid #333;
            display: none;
        }
        
        .debug-toggle {
            position: fixed;
            bottom: 10px;
            right: 10px;
            background-color: rgba(0, 0, 0, 0.7);
            color: #00ff00;
            border: 1px solid #00ff00;
            font-family: monospace;
            font-size: 12px;
            padding: 4px 8px;
            cursor: pointer;
            z-index: 10000;
        }
    `;
    document.head.appendChild(debugStyle);

    // Add debug panel
    const debugPanel = document.createElement('div');
    debugPanel.className = 'debug-panel';
    debugPanel.id = 'debug-panel';
    document.body.appendChild(debugPanel);

    // Add toggle button
    const debugToggle = document.createElement('button');
    debugToggle.className = 'debug-toggle';
    debugToggle.textContent = 'Debug';
    debugToggle.addEventListener('click', () => {
        const panel = document.getElementById('debug-panel');
        if (panel) {
            panel.style.display = panel.style.display === 'none' ? 'block' : 'none';
        }
    });
    document.body.appendChild(debugToggle);

    // Override console.log to also output to debug panel
    const originalConsoleLog = console.log;
    console.log = function(...args) {
        originalConsoleLog.apply(console, args);

        const panel = document.getElementById('debug-panel');
        if (panel) {
            const logEntry = document.createElement('div');
            logEntry.textContent = args.map(arg => {
                if (typeof arg === 'object') {
                    try {
                        return JSON.stringify(arg);
                    } catch (e) {
                        return arg.toString();
                    }
                }
                return arg;
            }).join(' ');
            panel.appendChild(logEntry);
            panel.scrollTop = panel.scrollHeight;

            // Limit number of entries
            while (panel.children.length > 100) {
                panel.removeChild(panel.firstChild);
            }
        }
    };
}

// Utility to log the current application state
export function logAppState(appState) {
    if (!DEBUG) return;

    console.log("==== APP STATE ====");
    console.log("Current View:", appState.currentView);
    console.log("Current Bank:", appState.currentBank);
    console.log("Current Snap:", appState.currentSnap);
    console.log("Current Tab:", appState.currentTab);
    console.log("Project:", appState.project ? {
        name: appState.project.project_name,
        banks: appState.project.banks.length,
        parameters: appState.project.parameters.length
    } : "No project");
}

// Export debug utilities
export default {
    initializeDebug,
    logAppState
};