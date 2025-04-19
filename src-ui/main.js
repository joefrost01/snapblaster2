// main.js - Core application setup
import { switchView, initializeViews } from './views.js';
import { setupEventListeners } from './events.js';
import { appState, initializeState } from './state.js';
import { initializeDebug, logAppState } from './debug.js';

// Import CSS as string to avoid MIME type issues
const notificationsStyle = document.createElement('style');
notificationsStyle.textContent = `
.notification {
    position: fixed;
    bottom: 20px;
    right: 20px;
    padding: 12px 16px;
    background-color: #3f3f46;
    color: white;
    border-radius: 4px;
    z-index: 1000;
    box-shadow: 0 2px 10px rgba(0, 0, 0, 0.2);
    opacity: 1;
    transition: opacity 0.5s;
}

.notification.info {
    background-color: #3b82f6;
}

.notification.success {
    background-color: #10b981;
}

.notification.error {
    background-color: #ef4444;
}

.notification.warning {
    background-color: #f59e0b;
}

.notification.fadeout {
    opacity: 0;
}`;
document.head.appendChild(notificationsStyle);

// Initialize the application when DOM is loaded
document.addEventListener('DOMContentLoaded', async () => {
    console.log('Snap-Blaster initializing...');

    // Initialize debug tools first
    initializeDebug();

    // Initialize views and state
    initializeViews();

    // Set up event listeners
    setupEventListeners();

    // Initialize state from backend
    try {
        await initializeState();

        // Log the initial app state
        logAppState(appState);

        // Show welcome view by default, or editor if we have a project
        if (appState.hasInitialProject) {
            console.log("Starting with editor view (project exists)");
            switchView('editor');
        } else {
            console.log("Starting with welcome view (no project)");
            switchView('welcome');
        }
    } catch (error) {
        console.error('Error initializing application:', error);
        switchView('welcome');
    }

    // Add a global error handler
    window.onerror = function(message, source, lineno, colno, error) {
        console.error("GLOBAL ERROR:", message, "at", source, ":", lineno, ":", colno);
        console.error(error);
        return false;
    };

    console.log("Initialization complete");
});