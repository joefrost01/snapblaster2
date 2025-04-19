// main.js - Core application setup
import { switchView, initializeViews } from './views.js';
import { setupEventListeners } from './events.js';
import { appState } from './state.js';

// Initialize the application when DOM is loaded
document.addEventListener('DOMContentLoaded', () => {
    console.log('Snap-Blaster initializing...');

    // Initialize views and state
    initializeViews();

    // Set up event listeners
    setupEventListeners();

    // Show welcome view by default
    switchView('welcome');
});