// config.js - Parameter configuration UI
import { appState } from './state.js';
import { api } from './tauri-api.js';

// Current page for parameter configuration (0-3)
let currentConfigPage = 0;

// Render the parameter list in config view
export function renderParameterList() {
    const elements = window.snapElements;
    elements.configParamsContainer.innerHTML = '';

    if (!appState.project) return;

    console.log("Rendering parameter list, found",
        appState.project.parameters ? appState.project.parameters.length : 0,
        "parameters");

    // Calculate parameter range for current page
    const startIdx = currentConfigPage * 16;
    const endIdx = Math.min(startIdx + 16, appState.project.parameters.length);

    // Handle case where there are no parameters
    if (appState.project.parameters.length === 0) {
        const emptyMessage = document.createElement('div');
        emptyMessage.className = 'empty-parameters-message';
        emptyMessage.innerHTML = 'No parameters defined yet. Click "Add Parameter" to create one.';
        elements.configParamsContainer.appendChild(emptyMessage);
        return;
    }

    // Create rows for each parameter in the current page
    for (let i = startIdx; i < endIdx; i++) {
        const param = appState.project.parameters[i];
        const row = createParameterRow(param, i);
        elements.configParamsContainer.appendChild(row);
    }

    // Update active tab button
    updateConfigTabButtons();
}

// Update config tab buttons to show which is active
function updateConfigTabButtons() {
    const tabButtons = [
        document.getElementById('tab-1-16-conf'),
        document.getElementById('tab-17-32-conf'),
        document.getElementById('tab-33-48-conf'),
        document.getElementById('tab-49-64-conf')
    ];

    tabButtons.forEach((btn, idx) => {
        if (!btn) return;
        const shouldBeActive = idx === currentConfigPage;
        const isActive      = btn.classList.contains('active');
        if (shouldBeActive && !isActive) {
            btn.classList.add('active');
        } else if (!shouldBeActive && isActive) {
            btn.classList.remove('active');
        }
    });
}

// Create a row for parameter configuration
// Update the createParameterRow function in config.js
function createParameterRow(param, index) {
    const row = document.createElement('div');
    row.className = 'config-param-row';
    row.dataset.paramId = index;

    // Name input
    const nameInput = document.createElement('input');
    nameInput.type = 'text';
    nameInput.value = param.name;
    nameInput.placeholder = 'Parameter Name';
    nameInput.addEventListener('change', () => updateParameter(index));
    row.appendChild(nameInput);

    // Description input
    const descInput = document.createElement('input');
    descInput.type = 'text';
    descInput.value = param.description;
    descInput.placeholder = 'Description';
    descInput.addEventListener('change', () => updateParameter(index));
    row.appendChild(descInput);

    // CC input
    const ccInput = document.createElement('input');
    ccInput.type = 'number';
    ccInput.min = 0;
    ccInput.max = 127;
    ccInput.value = param.cc;
    ccInput.addEventListener('change', () => updateParameter(index));
    row.appendChild(ccInput);

    // Wiggle button with enhanced feedback
    const wiggleBtn = document.createElement('button');
    wiggleBtn.className = 'wiggle-btn';
    wiggleBtn.title = 'Send MIDI Wiggle (for DAW MIDI Learn)';
    wiggleBtn.textContent = '🎚️';
    wiggleBtn.addEventListener('click', (e) => {
        e.stopPropagation();
        wiggleParameter(param.cc);

        // Add visual wiggle effect
        wiggleBtn.classList.add('wiggling');
        setTimeout(() => {
            wiggleBtn.classList.remove('wiggling');
        }, 800);
    });
    row.appendChild(wiggleBtn);

    return row;
}

// Add this to the wiggleParameter function in config.js
async function wiggleParameter(cc) {
    console.log(`Wiggling parameter CC ${cc}`);

    try {
        // Create a distinctive pattern for MIDI learn
        const values = [0, 127, 64, 127, 0, 127, 0, 64];

        // Visual feedback
        const notification = document.createElement('div');
        notification.className = 'notification info mini-notification';
        notification.textContent = `Wiggling CC ${cc}...`;
        document.body.appendChild(notification);

        await api.sendWiggle(cc, values);

        notification.textContent = `CC ${cc} wiggle complete`;
        notification.classList.add('fadeout');
        setTimeout(() => {
            if (notification.parentNode) {
                document.body.removeChild(notification);
            }
        }, 1000);
    } catch (error) {
        console.error('Error sending wiggle values:', error);

        const errorNotification = document.createElement('div');
        errorNotification.className = 'notification error';
        errorNotification.textContent = `Error wiggling CC ${cc}`;
        document.body.appendChild(errorNotification);

        setTimeout(() => {
            errorNotification.classList.add('fadeout');
            setTimeout(() => {
                if (errorNotification.parentNode) {
                    document.body.removeChild(errorNotification);
                }
            }, 500);
        }, 3000);
    }
}

// Update a parameter in the state
async function updateParameter(index) {
    if (!appState.project) return;

    const row = document.querySelector(`.config-param-row[data-param-id="${index}"]`);
    if (!row) return;

    const nameInput = row.querySelector('input[type="text"]:nth-of-type(1)');
    const descInput = row.querySelector('input[type="text"]:nth-of-type(2)');
    const ccInput = row.querySelector('input[type="number"]');

    try {
        // Show indication that update is in progress
        row.classList.add('updating');

        await api.updateParameter(
            index,
            nameInput.value,
            descInput.value,
            parseInt(ccInput.value)
        );

        // Update local state
        if (index < appState.project.parameters.length) {
            const param = appState.project.parameters[index];
            param.name = nameInput.value;
            param.description = descInput.value;
            param.cc = parseInt(ccInput.value);
        }

        console.log(`Updated parameter ${index}:`, nameInput.value);

        // Visual feedback for success
        row.classList.add('update-success');
        setTimeout(() => {
            row.classList.remove('update-success');
        }, 1000);
    } catch (error) {
        console.error('Error updating parameter:', error);

        // Visual feedback for error
        row.classList.add('update-error');
        setTimeout(() => {
            row.classList.remove('update-error');
        }, 1000);
    } finally {
        row.classList.remove('updating');
    }
}

// Add a new parameter
export async function addParameter() {
    if (!appState.project) return;

    // Enforce the 64 parameter limit
    if (appState.project.parameters.length >= 64) {
        alert('Maximum of 64 parameters reached');
        return;
    }

    // Find next available CC
    const usedCCs = appState.project.parameters.map(p => p.cc);
    let nextCC = 0;
    while (usedCCs.includes(nextCC) && nextCC < 127) {
        nextCC++;
    }

    try {
        const addBtn = document.getElementById('add-param-btn');

        // Call backend to add parameter
        await api.addParameter(
            `Parameter ${appState.project.parameters.length + 1}`,
            '',
            nextCC
        );

        // Get updated project from backend
        const project = await api.getProject();
        if (project) {
            appState.project = project;
            console.log("Project updated, now has", project.parameters.length, "parameters");
        }

        // Determine index and page of the new param
        const newIndex = appState.project.parameters.length - 1;
        const newPage  = Math.floor(newIndex / 16);

        if (newPage === currentConfigPage) {
            // Still on the same page → just append one row
            const newParam = appState.project.parameters[newIndex];
            const row      = createParameterRow(newParam, newIndex);
            const container = document.getElementById('config-params-container');
            const placeholder = container.querySelector('.empty-parameters-message');
            if (placeholder) placeholder.remove();
            container.appendChild(row);
        } else {
            // Page boundary crossed → go to that page and re-render
            setConfigPage(newPage);
        }

        console.log('Added new parameter with CC:', nextCC);

        appState.isDirty = true;
    } catch (error) {
        console.error('Error adding parameter:', error);
    } finally {


    }
}

// Set the current config page
export function setConfigPage(pageIndex) {
    currentConfigPage = pageIndex;
    renderParameterList();
}

const style = document.createElement('style');
style.textContent = `
.config-param-row.updating {
    opacity: 0.7;
}
.config-param-row.update-success {
    background-color: rgba(16, 185, 129, 0.2);
    transition: background-color 1s;
}
.config-param-row.update-error {
    background-color: rgba(239, 68, 68, 0.2);
    transition: background-color 1s;
}
.empty-parameters-message {
    padding: 20px;
    text-align: center;
    color: #a1a1aa;
}
.processing {
    cursor: wait;
    pointer-events: none;
}
`;
document.head.appendChild(style);