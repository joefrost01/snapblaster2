// parameters.js - Parameter editing UI
import { appState, updateParameterValue } from './state.js';
import { api } from './tauri-api.js';

// Update parameters based on current tab
export function updateParameters() {
    const elements = window.snapElements;
    elements.parametersContainer.innerHTML = '';

    if (!appState.project) return;

    // Get current snap values
    const snap = appState.project.banks[appState.currentBank].snaps[appState.currentSnap];

    // Calculate parameter range for current tab
    const startIdx = appState.currentTab * 16;
    const endIdx = Math.min(startIdx + 16, appState.project.parameters.length);

    // Create parameter rows (two per row)
    for (let i = startIdx; i < endIdx; i += 2) {
        // Create a row container
        const row = document.createElement('div');
        row.className = 'param-row';

        // First parameter
        if (i < appState.project.parameters.length) {
            const param = appState.project.parameters[i];
            const value = snap.values[i] || 0;

            const cell = createParameterCell(param, value, i);
            row.appendChild(cell);
        } else {
            // Empty cell
            const emptyCell = document.createElement('div');
            emptyCell.className = 'param-cell empty';
            row.appendChild(emptyCell);
        }

        // Spacer
        const spacer = document.createElement('div');
        row.appendChild(spacer);

        // Second parameter
        if (i + 1 < endIdx && i + 1 < appState.project.parameters.length) {
            const param = appState.project.parameters[i + 1];
            const value = snap.values[i + 1] || 0;

            const cell = createParameterCell(param, value, i + 1);
            row.appendChild(cell);
        } else {
            // Empty cell
            const emptyCell = document.createElement('div');
            emptyCell.className = 'param-cell empty';
            row.appendChild(emptyCell);
        }

        elements.parametersContainer.appendChild(row);
    }
}

// Create a parameter cell for the editor view
function createParameterCell(param, value, index) {
    const cell = document.createElement('div');
    cell.className = 'param-cell';
    cell.dataset.paramId = index;

    // Parameter header with name and value
    const header = document.createElement('div');
    header.className = 'param-header';

    const nameSpan = document.createElement('div');
    nameSpan.className = 'param-name';
    nameSpan.textContent = param.name;
    header.appendChild(nameSpan);

    const valueDisplay = document.createElement('div');
    valueDisplay.className = 'param-value-display';

    const valueSpan = document.createElement('div');
    valueSpan.className = 'param-value';
    valueSpan.id = `value-${index}`;
    valueSpan.textContent = value;
    valueDisplay.appendChild(valueSpan);

    const wiggleBtn = document.createElement('button');
    wiggleBtn.className = 'wiggle-btn';
    wiggleBtn.title = 'Wiggle for MIDI Learn';
    wiggleBtn.textContent = '🎚️';
    wiggleBtn.addEventListener('click', (e) => {
        e.stopPropagation(); // Prevent the event from bubbling up
        wiggleParameter(param.cc);

        // Add visual effect
        wiggleBtn.classList.add('wiggling');
        setTimeout(() => {
            wiggleBtn.classList.remove('wiggling');
        }, 800);
    });
    valueDisplay.appendChild(wiggleBtn);

    header.appendChild(valueDisplay);
    cell.appendChild(header);

    // Slider
    const slider = document.createElement('input');
    slider.type = 'range';
    slider.min = 0;
    slider.max = 127;
    slider.value = value;
    slider.dataset.paramId = index;
    slider.addEventListener('input', (e) => {
        const value = parseInt(e.target.value);
        document.getElementById(`value-${index}`).textContent = value;
        updateParameterValue(index, value);
    });

    cell.appendChild(slider);

    return cell;
}

// Send wiggle values for MIDI learn
async function wiggleParameter(cc) {
    console.log(`Wiggling parameter CC ${cc}`);

    try {
        // Create a more distinctive pattern for MIDI learn
        // Start at 0, go to full, middle, then a rhythm pattern
        const values = [0, 127, 64, 127, 0, 127, 0, 64];

        // Show wiggle in progress indicator
        const notification = document.createElement('div');
        notification.className = 'notification info mini-notification';
        notification.textContent = `Wiggling CC ${cc}...`;
        document.body.appendChild(notification);

        await api.sendWiggle(cc, values);

        // Update notification when complete
        notification.textContent = `CC ${cc} wiggle complete`;
        notification.classList.add('fadeout');
        setTimeout(() => {
            if (notification.parentNode) {
                document.body.removeChild(notification);
            }
        }, 1000);
    } catch (error) {
        console.error('Error sending wiggle values:', error);

        // Show error notification
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