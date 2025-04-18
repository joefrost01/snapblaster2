// Import the Tauri API
const { invoke } = window.__TAURI__.tauri;
const { open, save } = window.__TAURI__.dialog;

// App state
let currentView = "welcome"; // welcome, snap-editor, param-config

// Wait for DOM to be loaded before initializing app
document.addEventListener("DOMContentLoaded", () => {
    console.log("Snap-Blaster UI loaded");
    initializeButtons();
    setupEventListeners();
});

// Initialize button event listeners
function initializeButtons() {
    // New Project button
    const newProjectBtn = document.getElementById("new-project-btn");
    if (newProjectBtn) {
        newProjectBtn.addEventListener("click", async () => {
            try {
                console.log("Creating new project...");
                await invoke("new_project");
                console.log("New project created");

                // Switch to the snap editor view
                switchView("snap-editor");
            } catch (error) {
                console.error("Error creating new project:", error);
                alert(`Error creating new project: ${error}`);
            }
        });
    }

    // Load Project button
    const loadProjectBtn = document.getElementById("load-project-btn");
    if (loadProjectBtn) {
        loadProjectBtn.addEventListener("click", handleLoadProject);
    }

    // Header Snap button
    const snapBtn = document.getElementById("snap-btn");
    if (snapBtn) {
        snapBtn.addEventListener("click", () => {
            console.log("Snap button clicked");
            switchView("snap-editor");
        });
    }

    // Header Config button
    const confBtn = document.getElementById("conf-btn");
    if (confBtn) {
        confBtn.addEventListener("click", () => {
            console.log("Config button clicked");
            switchView("param-config");
        });
    }

    // Header Save button
    const saveBtn = document.getElementById("save-btn");
    if (saveBtn) {
        saveBtn.addEventListener("click", handleSaveProject);
    }

    // Header Load button
    const headerLoadBtn = document.getElementById("load-btn");
    if (headerLoadBtn) {
        headerLoadBtn.addEventListener("click", handleLoadProject);
    }
}

// Handle load project action
async function handleLoadProject() {
    try {
        console.log("Opening project file dialog...");
        const selected = await open({
            multiple: false,
            filters: [{
                name: "Snap-Blaster Project",
                extensions: ["sb"]
            }]
        });

        if (selected) {
            console.log(`Loading project: ${selected}`);
            await invoke("load_project", { path: selected });
            console.log("Project loaded");

            // Switch to the snap editor view
            switchView("snap-editor");
        }
    } catch (error) {
        console.error("Error loading project:", error);
        alert(`Error loading project: ${error}`);
    }
}

// Handle save project action
async function handleSaveProject() {
    try {
        console.log("Opening save dialog...");
        const savePath = await save({
            filters: [{
                name: "Snap-Blaster Project",
                extensions: ["sb"]
            }]
        });

        if (savePath) {
            console.log(`Saving project to: ${savePath}`);
            await invoke("save_project", { path: savePath });
            console.log("Project saved");
            alert(`Project saved to: ${savePath}`);
        }
    } catch (error) {
        console.error("Error saving project:", error);
        alert(`Error saving project: ${error}`);
    }
}

// Set up event listeners for Tauri events
function setupEventListeners() {
    // Listen for events from the Rust backend
    window.__TAURI__.event.listen("snap-event", (event) => {
        console.log("Received event from backend:", event);
        // Handle different event types
        const data = JSON.parse(event.payload);
        handleBackendEvent(data);
    });
}

// Handle events from the backend
function handleBackendEvent(event) {
    switch(event.type) {
        case "ProjectLoaded":
            refreshProjectData();
            break;
        case "ProjectSaved":
            console.log("Project was saved successfully");
            break;
        case "SnapSelected":
            // Update UI to show selected snap
            break;
        // Handle other event types
    }
}

// Switch between different views in the app
function switchView(viewName) {
    // Make sure each view container exists
    ensureViewContainers();

    // Hide all views
    document.getElementById("welcome-view").style.display = "none";
    document.getElementById("snap-editor-view").style.display = "none";
    document.getElementById("param-config-view").style.display = "none";

    // Show the requested view
    document.getElementById(`${viewName}-view`).style.display = "flex";

    // Update current view
    currentView = viewName;

    // Update button states
    updateHeaderButtonStates();

    console.log(`Switched to view: ${viewName}`);
}

// Make sure all view containers exist in the DOM
function ensureViewContainers() {
    const contentContainer = document.querySelector(".content");

    // Check for welcome view
    if (!document.getElementById("welcome-view")) {
        const welcomeView = document.querySelector(".welcome");
        if (welcomeView) {
            welcomeView.id = "welcome-view";
        }
    }

    // Check for snap editor view
    if (!document.getElementById("snap-editor-view")) {
        const snapEditorView = document.createElement("div");
        snapEditorView.id = "snap-editor-view";
        snapEditorView.className = "view-container";
        snapEditorView.style.display = "none";
        snapEditorView.innerHTML = `
      <div class="editor-layout">
        <div class="sidebar">
          <div class="launchpad-grid"></div>
          <div class="snap-info">
            <textarea placeholder="Snap description"></textarea>
          </div>
        </div>
        <div class="parameter-panel">
          <div id="parameter-sliders">
            <!-- Parameter sliders will be generated here -->
            <p>Snap editor view - parameters will appear here</p>
          </div>
        </div>
      </div>
    `;
        contentContainer.appendChild(snapEditorView);
    }

    // Check for parameter config view
    if (!document.getElementById("param-config-view")) {
        const paramConfigView = document.createElement("div");
        paramConfigView.id = "param-config-view";
        paramConfigView.className = "view-container";
        paramConfigView.style.display = "none";
        paramConfigView.innerHTML = `
      <div class="param-config-layout">
        <div id="parameter-list">
          <!-- Parameter configuration will be generated here -->
          <p>Parameter configuration view - list will appear here</p>
        </div>
      </div>
    `;
        contentContainer.appendChild(paramConfigView);
    }

    // Add CSS for the views
    if (!document.getElementById("view-styles")) {
        const styleElement = document.createElement("style");
        styleElement.id = "view-styles";
        styleElement.textContent = `
      .view-container {
        width: 100%;
        height: 100%;
        flex-direction: column;
      }
      .editor-layout {
        display: flex;
        width: 100%;
        height: 100%;
      }
      .sidebar {
        width: 240px;
        background-color: #27272a;
        padding: 12px;
        display: flex;
        flex-direction: column;
        gap: 12px;
      }
      .launchpad-grid {
        display: grid;
        grid-template-columns: repeat(8, 1fr);
        gap: 2px;
        background-color: #18181b;
        padding: 4px;
        border: 1px solid #3f3f46;
      }
      .parameter-panel {
        flex: 1;
        padding: 12px;
        overflow-y: auto;
      }
      .snap-info textarea {
        width: 100%;
        height: 100px;
        background-color: #3f3f46;
        border: 1px solid #52525b;
        color: white;
        padding: 8px;
        resize: none;
      }
      .param-config-layout {
        padding: 12px;
        width: 100%;
      }
    `;
        document.head.appendChild(styleElement);
    }
}

// Update header button states based on current view
function updateHeaderButtonStates() {
    const snapBtn = document.getElementById("snap-btn");
    const confBtn = document.getElementById("conf-btn");

    if (snapBtn && confBtn) {
        // Reset both buttons
        snapBtn.style.backgroundColor = "#3f3f46";
        confBtn.style.backgroundColor = "#3f3f46";

        // Highlight the active button
        if (currentView === "snap-editor") {
            snapBtn.style.backgroundColor = "#52525b";
        } else if (currentView === "param-config") {
            confBtn.style.backgroundColor = "#52525b";
        }
    }
}

// Refresh project data from the backend
async function refreshProjectData() {
    try {
        const projectData = await invoke("get_project");
        const project = JSON.parse(projectData);

        // Update UI with project data
        console.log("Project data:", project);

        // Update UI elements with project data
        updateProjectUI(project);

    } catch (error) {
        console.error("Error fetching project data:", error);
    }
}

// Update UI elements with project data
function updateProjectUI(project) {
    // This function will populate the UI with project data
    // For now just logging the project info
    console.log(`Project: ${project.project_name}`);
    console.log(`Banks: ${project.banks.length}`);
    console.log(`Parameters: ${project.parameters.length}`);
}