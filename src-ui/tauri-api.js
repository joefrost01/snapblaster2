// tauri-api.js - Bridge between UI and Rust backend

// Wait for Tauri to be ready before accessing its APIs
let tauriReady = false;
let tauriReadyCallbacks = [];

// Import Tauri API - safely with error handling
let invoke, listen, save, open;

// Function to initialize Tauri API
function initTauriApi() {
    try {
        if (window.__TAURI__) {
            invoke = window.__TAURI__.tauri.invoke;
            listen = window.__TAURI__.event.listen;

            if (window.__TAURI__.dialog) {
                save = window.__TAURI__.dialog.save;
                open = window.__TAURI__.dialog.open;
            } else {
                console.warn("Tauri dialog API not available");

                // Fallback implementations
                save = async () => null;
                open = async () => null;
            }

            tauriReady = true;

            // Call any queued callbacks
            tauriReadyCallbacks.forEach(callback => callback());
            tauriReadyCallbacks = [];

            return true;
        }
    } catch (err) {
        console.error("Error initializing Tauri API:", err);
    }

    return false;
}

// Try to initialize immediately
if (!initTauriApi()) {
    // If not ready, set up to try again when document is loaded
    document.addEventListener('DOMContentLoaded', initTauriApi);

    // Set default implementations that will show errors
    invoke = (...args) => {
        console.error("Tauri API not initialized yet", args);
        return Promise.reject("Tauri API not initialized");
    };

    listen = (...args) => {
        console.error("Tauri API not initialized yet", args);
        return Promise.reject("Tauri API not initialized");
    };

    save = async () => null;
    open = async () => null;
}

// Function to run code when Tauri is ready
function whenTauriReady(callback) {
    if (tauriReady) {
        callback();
    } else {
        tauriReadyCallbacks.push(callback);
    }
}

// A central event system for frontend components
const eventBus = {
    listeners: {},

    // Add event listener
    on(event, callback) {
        if (!this.listeners[event]) {
            this.listeners[event] = [];
        }
        this.listeners[event].push(callback);
    },

    // Remove event listener
    off(event, callback) {
        if (!this.listeners[event]) return;
        this.listeners[event] = this.listeners[event].filter(cb => cb !== callback);
    },

    // Emit event
    emit(event, data) {
        if (!this.listeners[event]) return;
        this.listeners[event].forEach(callback => callback(data));
    }
};

// Initialize Tauri event listeners
let initialized = false;

async function initializeListeners() {
    if (initialized) return;

    if (!tauriReady) {
        console.warn("Waiting for Tauri to be ready before initializing listeners");
        return new Promise((resolve) => {
            whenTauriReady(() => {
                initializeListeners().then(resolve);
            });
        });
    }

    try {
        // Listen for events from the Rust backend
        await listen('snap-event', (event) => {
            try {
                // Parse the event data
                const eventData = JSON.parse(event.payload);

                // Determine event type
                switch (eventData.type) {
                    case 'PadPressed':
                        eventBus.emit('pad-pressed', {
                            pad: eventData.pad,
                            velocity: eventData.velocity
                        });
                        break;

                    case 'CCValueChanged':
                        eventBus.emit('cc-value-changed', {
                            paramId: eventData.param_id,
                            value: eventData.value
                        });
                        break;

                    case 'SnapSelected':
                        eventBus.emit('snap-selected', {
                            bank: eventData.bank,
                            snapId: eventData.snap_id
                        });
                        break;

                    case 'ParameterEdited':
                        eventBus.emit('parameter-edited', {
                            paramId: eventData.param_id,
                            value: eventData.value
                        });
                        break;

                    case 'AIGenerationCompleted':
                        eventBus.emit('ai-generation-completed', {
                            bankId: eventData.bank_id,
                            snapId: eventData.snap_id,
                            values: eventData.values
                        });
                        break;

                    case 'AIGenerationFailed':
                        eventBus.emit('ai-generation-failed', {
                            bankId: eventData.bank_id,
                            snapId: eventData.snap_id,
                            error: eventData.error
                        });
                        break;

                    case 'MorphProgressed':
                        eventBus.emit('morph-progressed', {
                            progress: eventData.progress,
                            currentValues: eventData.current_values
                        });
                        break;

                    case 'MorphCompleted':
                        eventBus.emit('morph-completed');
                        break;

                    case 'ProjectLoaded':
                        eventBus.emit('project-loaded');
                        break;

                    case 'ProjectSaved':
                        eventBus.emit('project-saved');
                        break;
                }
            } catch (err) {
                console.error('Error parsing event:', err);
            }
        });

        await listen('link-event', (event) => {
            try {
                const eventData = JSON.parse(event.payload);

                switch (eventData.type) {
                    case 'link_status':
                        eventBus.emit('link-status-changed', {
                            connected: eventData.connected,
                            peers: eventData.peers
                        });
                        break;

                    case 'link_tempo':
                        eventBus.emit('link-tempo-changed', {
                            tempo: eventData.tempo
                        });
                        break;

                    case 'link_transport':
                        eventBus.emit('link-transport-changed', {
                            playing: eventData.playing
                        });
                        break;
                }
            } catch (err) {
                console.error('Error parsing Link event:', err);
            }
        });

        initialized = true;
        console.log("Tauri event listeners initialized");
    } catch (err) {
        console.error("Failed to initialize Tauri event listeners:", err);
    }
}

// API functions to communicate with the Rust backend
const api = {
    // Initialize the API
    async initialize() {
        if (!tauriReady) {
            return new Promise((resolve) => {
                whenTauriReady(async () => {
                    await initializeListeners();
                    resolve();
                });
            });
        }

        return initializeListeners();
    },

    // Get available MIDI input ports
    async getMidiInputs() {
        if (!tauriReady) {
            return new Promise((resolve) => {
                whenTauriReady(async () => {
                    resolve(await this.getMidiInputs());
                });
            });
        }

        try {
            const portsJson = await invoke('list_midi_inputs');
            return JSON.parse(portsJson);
        } catch (err) {
            console.error('Error getting MIDI inputs:', err);
            return [];
        }
    },

    // Get available MIDI output ports
    async getMidiOutputs() {
        if (!tauriReady) {
            return new Promise((resolve) => {
                whenTauriReady(async () => {
                    resolve(await this.getMidiOutputs());
                });
            });
        }

        try {
            const portsJson = await invoke('list_midi_outputs');
            return JSON.parse(portsJson);
        } catch (err) {
            console.error('Error getting MIDI outputs:', err);
            return [];
        }
    },

    // Set current MIDI controller
    async setController(name) {
        if (!tauriReady) {
            return new Promise((resolve, reject) => {
                whenTauriReady(async () => {
                    try {
                        await this.setController(name);
                        resolve();
                    } catch (err) {
                        reject(err);
                    }
                });
            });
        }

        try {
            await invoke('set_controller', { name });
        } catch (err) {
            console.error('Error setting controller:', err);
            throw err;
        }
    },

    // Get project data
    async getProject() {
        if (!tauriReady) {
            return new Promise((resolve) => {
                whenTauriReady(async () => {
                    resolve(await this.getProject());
                });
            });
        }

        try {
            const projectJson = await invoke('get_project');
            return JSON.parse(projectJson);
        } catch (err) {
            console.error('Error getting project:', err);
            return null;
        }
    },

    // Save project to file
    async saveProject(path) {
        if (!tauriReady) {
            return new Promise((resolve, reject) => {
                whenTauriReady(async () => {
                    try {
                        await this.saveProject(path);
                        resolve();
                    } catch (err) {
                        reject(err);
                    }
                });
            });
        }

        try {
            await invoke('save_project', { path });
        } catch (err) {
            console.error('Error saving project:', err);
            throw err;
        }
    },

    // Load project from file
    async loadProject(path) {
        if (!tauriReady) {
            return new Promise((resolve, reject) => {
                whenTauriReady(async () => {
                    try {
                        await this.loadProject(path);
                        resolve();
                    } catch (err) {
                        reject(err);
                    }
                });
            });
        }

        try {
            await invoke('load_project', { path });
        } catch (err) {
            console.error('Error loading project:', err);
            throw err;
        }
    },

    // Create new project
    async newProject() {
        if (!tauriReady) {
            return new Promise((resolve, reject) => {
                whenTauriReady(async () => {
                    try {
                        await this.newProject();
                        resolve();
                    } catch (err) {
                        reject(err);
                    }
                });
            });
        }

        try {
            await invoke('new_project');
        } catch (err) {
            console.error('Error creating new project:', err);
            throw err;
        }
    },

    // Select a snap
    async selectSnap(bankId, snapId) {
        if (!tauriReady) {
            return new Promise((resolve, reject) => {
                whenTauriReady(async () => {
                    try {
                        await this.selectSnap(bankId, snapId);
                        resolve();
                    } catch (err) {
                        reject(err);
                    }
                });
            });
        }

        try {
            await invoke('select_snap', { bankId, snapId });
        } catch (err) {
            console.error('Error selecting snap:', err);
            throw err;
        }
    },

    // Edit parameter value
    async editParameter(paramId, value) {
        if (!tauriReady) {
            return new Promise((resolve, reject) => {
                whenTauriReady(async () => {
                    try {
                        await this.editParameter(paramId, value);
                        resolve();
                    } catch (err) {
                        reject(err);
                    }
                });
            });
        }

        try {
            await invoke('edit_parameter', { paramId, value });
        } catch (err) {
            console.error('Error editing parameter:', err);
            throw err;
        }
    },

    // Generate AI values
    async generateAIValues(bankId, snapId) {
        if (!tauriReady) {
            return new Promise((resolve, reject) => {
                whenTauriReady(async () => {
                    try {
                        await this.generateAIValues(bankId, snapId);
                        resolve();
                    } catch (err) {
                        reject(err);
                    }
                });
            });
        }

        try {
            await invoke('generate_ai_values', { bankId, snapId });
        } catch (err) {
            console.error('Error generating AI values:', err);
            throw err;
        }
    },

    // Start morph between snaps
    async startMorph(fromSnap, toSnap, durationBars, curveType, quantize = false) {
        if (!tauriReady) {
            return new Promise((resolve, reject) => {
                whenTauriReady(async () => {
                    try {
                        await this.startMorph(fromSnap, toSnap, durationBars, curveType, quantize);
                        resolve();
                    } catch (err) {
                        reject(err);
                    }
                });
            });
        }

        try {
            await invoke('start_morph', { fromSnap, toSnap, durationBars, curveType, quantize });
        } catch (err) {
            console.error('Error starting morph:', err);
            throw err;
        }
    },

    // Set OpenAI API key
    async setOpenAIApiKey(apiKey) {
        if (!tauriReady) {
            return new Promise((resolve, reject) => {
                whenTauriReady(async () => {
                    try {
                        await this.setOpenAIApiKey(apiKey);
                        resolve();
                    } catch (err) {
                        reject(err);
                    }
                });
            });
        }

        try {
            await invoke('set_openai_api_key', { apiKey });
        } catch (err) {
            console.error('Error setting OpenAI API key:', err);
            throw err;
        }
    },

    // Add parameter
    async addParameter(name, description, cc) {
        if (!tauriReady) {
            return new Promise((resolve, reject) => {
                whenTauriReady(async () => {
                    try {
                        await this.addParameter(name, description, cc);
                        resolve();
                    } catch (err) {
                        reject(err);
                    }
                });
            });
        }

        try {
            await invoke('add_parameter', { name, description, cc });
        } catch (err) {
            console.error('Error adding parameter:', err);
            throw err;
        }
    },

    // Update parameter
    async updateParameter(paramId, name, description, cc) {
        if (!tauriReady) {
            return new Promise((resolve, reject) => {
                whenTauriReady(async () => {
                    try {
                        await this.updateParameter(paramId, name, description, cc);
                        resolve();
                    } catch (err) {
                        reject(err);
                    }
                });
            });
        }

        try {
            await invoke('update_parameter', { paramId, name, description, cc });
        } catch (err) {
            console.error('Error updating parameter:', err);
            throw err;
        }
    },

    // Add snap
    async addSnap(bankId, padIndex, name, description) {
        if (!tauriReady) {
            return new Promise((resolve, reject) => {
                whenTauriReady(async () => {
                    try {
                        await this.addSnap(bankId, padIndex, name, description);
                        resolve();
                    } catch (err) {
                        reject(err);
                    }
                });
            });
        }

        try {
            await invoke('add_snap', { bankId, padIndex, name, description });
        } catch (err) {
            console.error('Error adding snap:', err);
            throw err;
        }
    },

    // Update snap description
    async updateSnapDescription(bankId, snapId, description) {
        if (!tauriReady) {
            return new Promise((resolve, reject) => {
                whenTauriReady(async () => {
                    try {
                        await this.updateSnapDescription(bankId, snapId, description);
                        resolve();
                    } catch (err) {
                        reject(err);
                    }
                });
            });
        }

        try {
            await invoke('update_snap_description', { bankId, snapId, description });
        } catch (err) {
            console.error('Error updating snap description:', err);
            throw err;
        }
    },

    // Send wiggle values for MIDI learn
    async sendWiggle(cc, values) {
        if (!tauriReady) {
            return new Promise((resolve, reject) => {
                whenTauriReady(async () => {
                    try {
                        await this.sendWiggle(cc, values);
                        resolve();
                    } catch (err) {
                        reject(err);
                    }
                });
            });
        }

        try {
            await invoke('send_wiggle', { cc, values });
        } catch (err) {
            console.error('Error sending wiggle:', err);
            throw err;
        }
    },

    async getLinkStatus() {
        if (!tauriReady) {
            return new Promise((resolve) => {
                whenTauriReady(async () => {
                    resolve(await this.getLinkStatus());
                });
            });
        }

        try {
            const statusJson = await invoke('get_link_status');
            return JSON.parse(statusJson);
        } catch (err) {
            console.error('Error getting Link status:', err);
            return {
                connected: false,
                peers: 0,
                playing: false,
                tempo: 120.0
            };
        }
    },

    // Set Link tempo
    async setLinkTempo(tempo) {
        if (!tauriReady) {
            return new Promise((resolve, reject) => {
                whenTauriReady(async () => {
                    try {
                        await this.setLinkTempo(tempo);
                        resolve();
                    } catch (err) {
                        reject(err);
                    }
                });
            });
        }

        try {
            // Clamp tempo to reasonable range
            const clampedTempo = Math.max(20, Math.min(999, tempo));
            await invoke('set_link_tempo', { tempo: clampedTempo });
        } catch (err) {
            console.error('Error setting Link tempo:', err);
            throw err;
        }
    },

    async setLinkEnabled(enabled) {
        if (!tauriReady) {
            return new Promise((resolve, reject) => {
                whenTauriReady(async () => {
                    try {
                        await this.setLinkEnabled(enabled);
                        resolve();
                    } catch (err) {
                        reject(err);
                    }
                });
            });
        }

        try {
            await invoke('set_link_enabled', { enabled });
        } catch (err) {
            console.error('Error setting Link enabled:', err);
            throw err;
        }
    },

    async startLinkTransport() {
        if (!tauriReady) {
            return new Promise((resolve, reject) => {
                whenTauriReady(async () => {
                    try {
                        await this.startLinkTransport();
                        resolve();
                    } catch (err) {
                        reject(err);
                    }
                });
            });
        }

        try {
            await invoke('start_link_transport');
        } catch (err) {
            console.error('Error starting Link transport:', err);
            throw err;
        }
    },

    async stopLinkTransport() {
        if (!tauriReady) {
            return new Promise((resolve, reject) => {
                whenTauriReady(async () => {
                    try {
                        await this.stopLinkTransport();
                        resolve();
                    } catch (err) {
                        reject(err);
                    }
                });
            });
        }

        try {
            await invoke('stop_link_transport');
        } catch (err) {
            console.error('Error stopping Link transport:', err);
            throw err;
        }
    },

    async setLinkQuantum(beats) {
        if (!tauriReady) {
            return new Promise((resolve, reject) => {
                whenTauriReady(async () => {
                    try {
                        await this.setLinkQuantum(beats);
                        resolve();
                    } catch (err) {
                        reject(err);
                    }
                });
            });
        }

        try {
            // Ensure beats is in a reasonable range (1-16)
            const clampedBeats = Math.max(1, Math.min(16, beats));
            await invoke('set_link_quantum', { beats: clampedBeats });
        } catch (err) {
            console.error('Error setting Link quantum:', err);
            throw err;
        }
    }
};

// File dialog functions
const fileDialogs = {
    // Open save dialog
    async saveProjectDialog() {
        if (!tauriReady) {
            return new Promise((resolve, reject) => {
                whenTauriReady(async () => {
                    try {
                        const result = await this.saveProjectDialog();
                        resolve(result);
                    } catch (err) {
                        reject(err);
                    }
                });
            });
        }

        try {
            const filePath = await save({
                filters: [{
                    name: 'Snap-Blaster Project',
                    extensions: ['sb']
                }],
                defaultPath: 'project.sb'
            });

            if (filePath) {
                await api.saveProject(filePath);
                return filePath;
            }
        } catch (err) {
            console.error('Error opening save dialog:', err);
            throw err;
        }

        return null;
    },

    // Open load dialog
    async loadProjectDialog() {
        if (!tauriReady) {
            return new Promise((resolve, reject) => {
                whenTauriReady(async () => {
                    try {
                        const result = await this.loadProjectDialog();
                        resolve(result);
                    } catch (err) {
                        reject(err);
                    }
                });
            });
        }

        try {
            const selected = await open({
                filters: [{
                    name: 'Snap-Blaster Project',
                    extensions: ['sb']
                }],
                multiple: false
            });

            if (selected) {
                await api.loadProject(selected);
                return selected;
            }
        } catch (err) {
            console.error('Error opening load dialog:', err);
            throw err;
        }

        return null;
    }
};

// Export the API and event bus
export { api, eventBus, fileDialogs, whenTauriReady };