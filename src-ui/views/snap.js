import { createSnapGrid, createParamSlider } from '../main.js';
const { invoke } = window.__TAURI__.tauri;

// Snap View Component
const SnapView = {
    // Create the view element
    create(initialState) {
        const view = document.createElement('section');
        view.id = 'snap-view';
        view.className = 'view flex flex-1 overflow-hidden';

        // Create the sidebar
        const sidebar = document.createElement('aside');
        sidebar.className = 'w-100 bg-zinc-850 text-zinc-300 border-r border-zinc-700 p-4 pt-4 text-xs';

        // Snap grid placeholder
        const gridContainer = document.createElement('div');
        gridContainer.id = 'snap-grid-container';
        sidebar.appendChild(gridContainer);

        // Snap info area
        const infoContainer = document.createElement('div');
        infoContainer.className = 'grid grid-cols-[auto,1fr] gap-[2px] p-1 pt-4 pb-4 bg-zinc-900';

        // Project label
        const projectLabel = document.createElement('div');
        projectLabel.className = 'w-12 text-zinc-300';
        projectLabel.textContent = 'Project:';
        infoContainer.appendChild(projectLabel);

        // Project name
        const projectName = document.createElement('div');
        projectName.id = 'project-name';
        projectName.innerHTML = '<strong class="text-white">Loading...</strong>';
        infoContainer.appendChild(projectName);

        // Bank label
        const bankLabel = document.createElement('div');
        bankLabel.className = 'w-12 text-zinc-300';
        bankLabel.textContent = 'Bank:';
        infoContainer.appendChild(bankLabel);

        // Bank name
        const bankName = document.createElement('div');
        bankName.id = 'bank-name';
        bankName.innerHTML = '<strong class="text-white">Loading...</strong>';
        infoContainer.appendChild(bankName);

        // Snap label
        const snapLabel = document.createElement('div');
        snapLabel.className = 'w-12 text-zinc-300';
        snapLabel.textContent = 'Snap:';
        infoContainer.appendChild(snapLabel);

        // Snap name
        const snapName = document.createElement('div');
        snapName.id = 'snap-name';
        snapName.innerHTML = '<strong class="text-white">Loading...</strong>';
        infoContainer.appendChild(snapName);

        sidebar.appendChild(infoContainer);

        // Snap description
        const descriptionContainer = document.createElement('div');
        descriptionContainer.className = 'flex flex-col md:flex-row gap-6';

        const description = document.createElement('textarea');
        description.id = 'snap-description';
        description.rows = 10;
        description.className = 'w-full text-xs p-2 bg-zinc-800 border border-zinc-600';
        description.placeholder = 'Snap description...';
        description.addEventListener('change', async (e) => {
            // Update snap description
            const snapId = initialState.currentSnap;
            const bankId = initialState.currentBank;
            const newDescription = e.target.value;

            try {
                await invoke('update_snap_description', {
                    bankId,
                    snapId,
                    description: newDescription
                });
            } catch (error) {
                console.error('Error updating snap description', error);
            }
        });

        descriptionContainer.appendChild(description);
        sidebar.appendChild(descriptionContainer);

        // Main content area
        const content = document.createElement('section');
        content.className = 'flex-1 flex flex-col gap-4 overflow-auto';

        // Parameters container
        const parametersContainer = document.createElement('div');
        parametersContainer.id = 'parameters-container';
        parametersContainer.className = 'bg-zinc-850 p-4 pt-0 text-xs space-y-4';
        content.appendChild(parametersContainer);

        // Assemble the view
        view.appendChild(sidebar);
        view.appendChild(content);

        return view;
    },

    // Update the view with current state
    update(state) {
        if (!state.project) return;

        // Update project info
        document.getElementById('project-name').innerHTML =
            `<strong class="text-white">${state.project.project_name}</strong>`;

        // Update bank info
        const bank = state.project.banks[state.currentBank];
        document.getElementById('bank-name').innerHTML =
            `<strong class="text-white">${bank.name}</strong>`;

        // Update snap info
        const snap = bank.snaps[state.currentSnap];
        document.getElementById('snap-name').innerHTML =
            `<strong class="text-white">${snap.name}</strong>`;

        // Update snap description
        document.getElementById('snap-description').value = snap.description;

        // Update snap grid
        this.updateSnapGrid(state);

        // Update parameters based on current tab
        this.updateParameters(state);
    },

    // Update the snap grid
    updateSnapGrid(state) {
        const container = document.getElementById('snap-grid-container');
        container.innerHTML = '';

        // Get current bank's snaps
        const snaps = state.project.banks[state.currentBank].snaps;

        // Create the grid
        const grid = createSnapGrid(snaps, state.currentSnap, async (snapIndex) => {
            // Handle snap selection
            try {
                await invoke('select_snap', {
                    bankId: state.currentBank,
                    snapId: snapIndex
                });

                // Update local state (the event listener will handle the rest)
                state.currentSnap = snapIndex;
            } catch (error) {
                console.error('Error selecting snap', error);
            }
        });

        container.appendChild(grid);
    },

    // Update parameters based on current tab
    updateParameters(state) {
        const container = document.getElementById('parameters-container');
        container.innerHTML = '';

        // Calculate which parameters to show based on current tab
        const startIdx = state.currentTab * 16;
        const endIdx = startIdx + 16;

        // Get the parameters for this range
        const params = state.project.parameters.slice(startIdx, endIdx);

        // Get current snap's values
        const snap = state.project.banks[state.currentBank].snaps[state.currentSnap];

        // Create parameter rows - two columns
        for (let i = 0; i < params.length; i += 2) {
            const rowContainer = document.createElement('div');
            rowContainer.className = 'param-row grid grid-cols-[1fr_16px_1fr] gap-0';

            // First parameter
            const param1 = params[i];
            const value1 = snap.values[startIdx + i];

            // First cell
            const cell1 = document.createElement('div');
            cell1.className = 'param-cell border border-zinc-800 p-2';

            // Label with name and value
            const label1 = document.createElement('label');
            label1.className = 'flex items-center justify-between text-zinc-400 mb-1';

            const nameSpan1 = document.createElement('span');
            nameSpan1.textContent = param1.name;
            label1.appendChild(nameSpan1);

            const controls1 = document.createElement('span');
            controls1.className = 'flex items-center gap-2';

            const valueDisplay1 = document.createElement('span');
            valueDisplay1.className = 'text-xs text-zinc-400 group-hover:text-white';
            valueDisplay1.textContent = value1;
            controls1.appendChild(valueDisplay1);

            const wiggleButton1 = document.createElement('button');
            wiggleButton1.className = 'text-zinc-500 hover:text-amber-400 text-sm';
            wiggleButton1.title = 'Wiggle this param for MIDI Learn';
            wiggleButton1.textContent = '🎚️';
            wiggleButton1.addEventListener('click', () => this.wiggleParameter(param1));
            controls1.appendChild(wiggleButton1);

            label1.appendChild(controls1);
            cell1.appendChild(label1);

            // Slider
            const sliderContainer1 = document.createElement('div');
            sliderContainer1.className = 'relative group';

            const slider1 = document.createElement('input');
            slider1.type = 'range';
            slider1.min = '0';
            slider1.max = '127';
            slider1.value = value1;
            slider1.className = 'w-full transition-all duration-150 hover:brightness-110 focus:ring-1 focus:ring-amber-400';

            slider1.addEventListener('input', (e) => {
                valueDisplay1.textContent = e.target.value;
                this.updateParameterValue(startIdx + i, parseInt(e.target.value));
            });

            sliderContainer1.appendChild(slider1);
            cell1.appendChild(sliderContainer1);

            rowContainer.appendChild(cell1);

            // Spacer
            const spacer = document.createElement('div');
            spacer.className = 'bg-zinc-900';
            rowContainer.appendChild(spacer);

            // Check if we have a second parameter
            if (i + 1 < params.length) {
                const param2 = params[i + 1];
                const value2 = snap.values[startIdx + i + 1];

                // Second cell
                const cell2 = document.createElement('div');
                cell2.className = 'param-cell border border-zinc-800 p-2';

                // Label with name and value
                const label2 = document.createElement('label');
                label2.className = 'flex items-center justify-between text-zinc-400 mb-1';

                const nameSpan2 = document.createElement('span');
                nameSpan2.textContent = param2.name;
                label2.appendChild(nameSpan2);

                const controls2 = document.createElement('span');
                controls2.className = 'flex items-center gap-2';

                const valueDisplay2 = document.createElement('span');
                valueDisplay2.className = 'text-xs text-zinc-400 group-hover:text-white';
                valueDisplay2.textContent = value2;
                controls2.appendChild(valueDisplay2);

                const wiggleButton2 = document.createElement('button');
                wiggleButton2.className = 'text-zinc-500 hover:text-amber-400 text-sm';
                wiggleButton2.title = 'Wiggle this param for MIDI Learn';
                wiggleButton2.textContent = '🎚️';
                wiggleButton2.addEventListener('click', () => this.wiggleParameter(param2));
                controls2.appendChild(wiggleButton2);

                label2.appendChild(controls2);
                cell2.appendChild(label2);

                // Slider
                const sliderContainer2 = document.createElement('div');
                sliderContainer2.className = 'relative group';

                const slider2 = document.createElement('input');
                slider2.type = 'range';
                slider2.min = '0';
                slider2.max = '127';
                slider2.value = value2;
                slider2.className = 'w-full transition-all duration-150 hover:brightness-110 focus:ring-1 focus:ring-amber-400';

                slider2.addEventListener('input', (e) => {
                    valueDisplay2.textContent = e.target.value;
                    this.updateParameterValue(startIdx + i + 1, parseInt(e.target.value));
                });

                sliderContainer2.appendChild(slider2);
                cell2.appendChild(sliderContainer2);

                rowContainer.appendChild(cell2);
            } else {
                // Empty cell for layout
                const emptyCell = document.createElement('div');
                emptyCell.className = 'param-cell border border-zinc-800 p-2';
                rowContainer.appendChild(emptyCell);
            }

            container.appendChild(rowContainer);
        }
    },

    // Update a parameter value in the current snap
    async updateParameterValue(paramId, value) {
        try {
            await invoke('edit_parameter', { paramId, value });
        } catch (error) {
            console.error('Error updating parameter value', error);
        }
    },

    // Send wiggle signal for MIDI Learn
    async wiggleParameter(param) {
        try {
            // Send a rapid series of changing CC values to help with MIDI learn
            const paramId = param.cc;

            // Current value
            const currentValue = parseInt(document.querySelector(`[data-cc="${paramId}"]`)?.value || 64);

            // Send a few values to wiggle
            await invoke('send_wiggle', {
                cc: param.cc,
                values: [
                    currentValue,
                    Math.min(currentValue + 20, 127),
                    Math.max(currentValue - 20, 0),
                    currentValue
                ]
            });
        } catch (error) {
            console.error('Error wiggling parameter', error);
        }
    }
};

export default SnapView;