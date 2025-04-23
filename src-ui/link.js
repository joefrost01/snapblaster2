import { api, eventBus } from './tauri-api.js';

// Link state
let linkState = {
    connected: false,
    peers: 0,
    tempo: 120.0,
    playing: false,
    enabled: true
};

// Initialize Link UI component
export async function initializeLinkUI() {
    // Get DOM elements
    const linkStatusText = document.getElementById('link-status-text');
    const linkStatusIndicator = document.getElementById('link-status-indicator');
    const bpmInput = document.querySelector('.bpm');

    // Add event listeners
    if (bpmInput) {
        // Update tempo when input changes
        bpmInput.addEventListener('change', async (e) => {
            const tempo = parseFloat(e.target.value);
            if (!isNaN(tempo) && tempo >= 20 && tempo <= 999) {
                try {
                    await api.setLinkTempo(tempo);
                    console.log(`Tempo set to ${tempo} BPM`);
                } catch (error) {
                    console.error('Error setting tempo:', error);
                }
            }
        });

        // Also update on Enter key
        bpmInput.addEventListener('keydown', async (e) => {
            if (e.key === 'Enter') {
                const tempo = parseFloat(e.target.value);
                if (!isNaN(tempo) && tempo >= 20 && tempo <= 999) {
                    try {
                        await api.setLinkTempo(tempo);
                        console.log(`Tempo set to ${tempo} BPM`);
                    } catch (error) {
                        console.error('Error setting tempo:', error);
                    }
                }
            }
        });
    }

    // Add click to toggle Link enabled
    if (linkStatusText) {
        linkStatusText.addEventListener('click', async () => {
            try {
                await api.setLinkEnabled(!linkState.enabled);
                console.log(`Link ${linkState.enabled ? 'disabled' : 'enabled'}`);
            } catch (error) {
                console.error('Error toggling Link:', error);
            }
        });
    }

    // Initialize event listeners
    setupEventListeners();

    // Get initial Link status
    try {
        const status = await api.getLinkStatus();
        updateLinkState(status);
        updateUI();
    } catch (error) {
        console.error('Error getting Link status:', error);
    }
}

// Set up event listeners for Link
function setupEventListeners() {
    // Listen for Link status changes
    eventBus.on('link-status-changed', (data) => {
        updateLinkState({
            connected: data.connected,
            peers: data.peers
        });
        updateUI();
    });

    // Listen for tempo changes
    eventBus.on('link-tempo-changed', (data) => {
        updateLinkState({
            tempo: data.tempo
        });
        updateUI();
    });

    // Listen for transport changes
    eventBus.on('link-transport-changed', (data) => {
        updateLinkState({
            playing: data.playing
        });
        updateUI();
    });
}

// Update the Link state
function updateLinkState(newState) {
    linkState = {
        ...linkState,
        ...newState
    };
}

// Update the UI based on current Link state
function updateUI() {
    const linkStatusText = document.getElementById('link-status-text');
    const linkStatusIndicator = document.getElementById('link-status-indicator');
    const bpmInput = document.querySelector('.bpm');

    if (linkStatusText) {
        if (linkState.connected) {
            linkStatusText.textContent = `Connected (${linkState.peers} peer${linkState.peers !== 1 ? 's' : ''})`;
            linkStatusText.classList.add('connected');
            linkStatusText.classList.remove('disconnected');
        } else {
            linkStatusText.textContent = 'Disconnected';
            linkStatusText.classList.add('disconnected');
            linkStatusText.classList.remove('connected');
        }

        // Add tooltip with more info
        linkStatusText.title = `Link ${linkState.enabled ? 'enabled' : 'disabled'}, ${linkState.connected ? 'connected' : 'disconnected'} with ${linkState.peers} peer${linkState.peers !== 1 ? 's' : ''}. Click to ${linkState.enabled ? 'disable' : 'enable'}.`;
    }

    if (linkStatusIndicator) {
        linkStatusIndicator.classList.toggle('active', linkState.connected);
        linkStatusIndicator.classList.toggle('inactive', !linkState.connected);

        // Add pulsing effect if playing
        linkStatusIndicator.classList.toggle('playing', linkState.playing);
    }

    if (bpmInput) {
        // Only update if the value is different to avoid cursor jumping
        if (Math.abs(parseFloat(bpmInput.value) - linkState.tempo) > 0.1) {
            bpmInput.value = Math.round(linkState.tempo);
        }
    }
}

// Start quantized morphing between snaps
export async function startQuantizedMorph(fromSnap, toSnap, durationBars, curveType) {
    try {
        // First ensure Link is enabled
        await api.setLinkEnabled(true);

        // Get current Link status
        const status = await api.getLinkStatus();

        // If connected to peers, use quantization
        const shouldQuantize = status.connected && status.peers > 0;

        // Start the morph with quantization if we have peers
        return api.startMorph(fromSnap, toSnap, durationBars, curveType, shouldQuantize);
    } catch (error) {
        console.error('Error starting quantized morph:', error);
        throw error;
    }
}

// Check if a snap change should be quantized
export function shouldQuantizeSnapChange() {
    // This could be based on a user preference setting
    // For now, let's assume all changes are quantized if Link is connected
    return linkState.connected;
}

// Export the Link state
export { linkState };