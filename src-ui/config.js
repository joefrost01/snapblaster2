// config.js - Parameter configuration UI
import { appState } from './state.js';

// Render the parameter list in config view
export function renderParameterList() {
    const elements = window.snapElements;
    elements.configParamsContainer.innerHTML = '';

    if (!appState.project) return;

    // Create rows for each parameter
    appState.project.parameters.forEach((param, index) => {
        const row = createParameterRow(param, index);
        elements.configParamsContainer.appendChild(row);
    });
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

    // Update UI
    renderParameterList();

    console.log('Added new parameter:', newParam);
}