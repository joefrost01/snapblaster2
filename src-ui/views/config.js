const { invoke } = window.__TAURI__.tauri;

// Configuration View Component for parameter setup
const ConfigView = {
    // Create the view element
    create(initialState) {
        const view = document.createElement('section');
        view.id = 'config-view';
        view.className = 'view flex flex-1 flex-col overflow-y-auto p-6 pt-4 text-xs';

        // Header section with buttons
        const header = document.createElement('div');
        header.className = 'flex justify-between items-center mb-4';

        const title = document.createElement('h1');
        title.className = 'text-lg font-semibold';
        title.textContent = 'Parameter Configuration';
        header.appendChild(title);

        const addButton = document.createElement('button');
        addButton.className = 'bg-orange-500 hover:bg-orange-600 text-white px-3 py-1 rounded text-xs';
        addButton.textContent = '+ Add Parameter';
        addButton.addEventListener('click', () => this.addParameter());
        header.appendChild(addButton);

        view.appendChild(header);

        // Table of parameters
        const container = document.createElement('div');
        container.className = 'space-y-2';

        // Table header
        const tableHeader = document.createElement('div');
        tableHeader.className = 'grid grid-cols-[3fr_4fr_1fr_40px] gap-4 text-zinc-400 px-2 pb-1 border-b border-zinc-700 uppercase tracking-wider text-[0.65rem]';

        const nameHeader = document.createElement('span');
        nameHeader.textContent = 'Name';
        tableHeader.appendChild(nameHeader);

        const descHeader = document.createElement('span');
        descHeader.textContent = 'Description';
        tableHeader.appendChild(descHeader);

        const ccHeader = document.createElement('span');
        ccHeader.textContent = 'CC Number';
        tableHeader.appendChild(ccHeader);

        const actionsHeader = document.createElement('span');
        tableHeader.appendChild(actionsHeader);

        container.appendChild(tableHeader);

        // Parameters container
        const paramsContainer = document.createElement('div');
        paramsContainer.id = 'params-container';
        container.appendChild(paramsContainer);

        view.appendChild(container);

        return view;
    },

    // Update the view with current state
    update(state) {
        if (!state.project) return;

        const container = document.getElementById('params-container');
        container.innerHTML = '';

        // Create rows for each parameter
        state.project.parameters.forEach((param, index) => {
            const row = this.createParameterRow(param, index);
            container.appendChild(row);
        });
    },

    // Create a row for a parameter
    createParameterRow(param, index) {
        const row = document.createElement('div');
        row.className = `param-row grid grid-cols-[3fr_4fr_1fr_40px] gap-4 items-center px-1 py-1 ${index % 2 === 1 ? 'bg-zinc-900' : ''}`;
        row.dataset.paramId = index;

        // Name field
        const nameInput = document.createElement('input');
        nameInput.type = 'text';
        nameInput.className = 'bg-zinc-800 border border-zinc-700 p-1 text-white text-xs w-full';
        nameInput.placeholder = 'Parameter Name';
        nameInput.value = param.name;
        nameInput.addEventListener('change', () => this.updateParameter(index));
        row.appendChild(nameInput);

        // Description field
        const descInput = document.createElement('input');
        descInput.type = 'text';
        descInput.className = 'bg-zinc-800 border border-zinc-700 p-1 text-white text-xs w-full';
        descInput.placeholder = 'Description';
        descInput.value = param.description;
        descInput.addEventListener('change', () => this.updateParameter(index));
        row.appendChild(descInput);

        // CC Number field
        const ccInput = document.createElement('input');
        ccInput.type = 'number';
        ccInput.min = 0;
        ccInput.max = 127;
        ccInput.className = 'bg-zinc-800 border border-zinc-700 p-1 text-white text-xs w-full text-center';
        ccInput.value = param.cc;
        ccInput.addEventListener('change', () => this.updateParameter(index));
        row.appendChild(ccInput);

        // Wiggle button
        const wiggleButton = document.createElement('button');
        wiggleButton.title = 'Send MIDI Wiggle';
        wiggleButton.className = 'text-zinc-500 hover:text-amber-400 text-sm';
        wiggleButton.textContent = '🎚️';
        wiggleButton.addEventListener('click', () => this.wiggleParameter(param));
        row.appendChild(wiggleButton);

        return row;
    },

    // Add a new parameter
    async addParameter() {
        try {
            // Get next available CC number (avoid duplicates)
            const params = document.querySelectorAll('.param-row');
            const usedCCs = Array.from(params).map(row => {
                const ccInput = row.querySelector('input[type="number"]');
                return parseInt(ccInput.value);
            });

            // Find first available CC number
            let nextCC = 0;
            while (usedCCs.includes(nextCC) && nextCC < 127) {
                nextCC++;
            }

            await invoke('add_parameter', {
                name: `Parameter ${params.length + 1}`,
                description: '',
                cc: nextCC
            });

            // Refresh the view
            setTimeout(() => {
                window.dispatchEvent(new CustomEvent('refresh-view'));
            }, 100);
        } catch (error) {
            console.error('Error adding parameter', error);
        }
    },

    // Update a parameter
    async updateParameter(paramId) {
        const row = document.querySelector(`.param-row[data-param-id="${paramId}"]`);
        if (!row) return;

        const nameInput = row.querySelector('input[type="text"]:nth-of-type(1)');
        const descInput = row.querySelector('input[type="text"]:nth-of-type(2)');
        const ccInput = row.querySelector('input[type="number"]');

        try {
            await invoke('update_parameter', {
                paramId,
                name: nameInput.value,
                description: descInput.value,
                cc: parseInt(ccInput.value)
            });
        } catch (error) {
            console.error('Error updating parameter', error);
        }
    },

    // Send wiggle signal for MIDI Learn
    async wiggleParameter(param) {
        try {
            // Send a rapid series of changing CC values to help with MIDI learn
            await invoke('send_wiggle', {
                cc: param.cc,
                values: [64, 100, 30, 64]
            });
        } catch (error) {
            console.error('Error wiggling parameter', error);
        }
    }
};

export default ConfigView;