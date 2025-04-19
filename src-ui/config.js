// config.js - Parameter configuration UI
import { appState } from './state.js';

// Current page for parameter configuration (0-3)
let currentConfigPage = 0;

// Render the parameter list in config view
export function renderParameterList() {
    const elements = window.snapElements;
    elements.configParamsContainer.innerHTML = '';

    if (!appState.project) return;

    // Calculate parameter range for current page
    const startIdx = currentConfigPage * 16;
    const endIdx = Math.min(startIdx + 16, appState.project.parameters.length);

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

    // Remove active class from all buttons
    tabButtons.forEach(btn => {
        if (btn) btn.classList.remove('active');
    });

    // Add active class to current page button
    if (tabButtons[currentConfigPage]) {
        tabButtons[currentConfigPage].classList.add('active');
    }
}

// Create a row for parameter configuration
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

    // Wiggle button
    const wiggleBtn = document.createElement('button');
    wiggleBtn.className = 'wiggle-btn';
    wiggleBtn.title = 'Send MIDI Wiggle';
    wiggleBtn.textContent = '🎚️';
    wiggleBtn.addEventListener('click', () => {
        console.log(`Wiggling parameter CC ${param.cc}`);
    });
    row.appendChild(wiggleBtn);

    return row;
}

// Update a parameter in the state
function updateParameter(index) {
    if (!appState.project) return;

    const row = document.querySelector(`.config-param-row[data-param-id="${index}"]`);
    if (!row) return;

    const nameInput = row.querySelector('input[type="text"]:nth-of-type(1)');
    const descInput = row.querySelector('input[type="text"]:nth-of-type(2)');
    const ccInput = row.querySelector('input[type="number"]');

    const param = appState.project.parameters[index];
    param.name = nameInput.value;
    param.description = descInput.value;
    param.cc = parseInt(ccInput.value);

    console.log(`Updated parameter ${index}:`, param);
}

// Add a new parameter
export function addParameter() {
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

    // Create new parameter
    const newParam = {
        name: `Parameter ${appState.project.parameters.length + 1}`,
        description: '',
        cc: nextCC
    };

    // Add to state
    appState.project.parameters.push(newParam);

    // Add default value to each snap
    appState.project.banks.forEach(bank => {
        bank.snaps.forEach(snap => {
            snap.values.push(64); // Default middle value
        });
    });

    // Switch to last page if needed
    const pageCount = Math.ceil(appState.project.parameters.length / 16);
    if (pageCount > 0) {
        setConfigPage(pageCount - 1);
    } else {
        // Update UI
        renderParameterList();
    }

    console.log('Added new parameter:', newParam);
}

// Set the current config page
export function setConfigPage(pageIndex) {
    currentConfigPage = pageIndex;
    renderParameterList();
}