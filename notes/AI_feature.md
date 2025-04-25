# SnapBlaster AI Feature Documentation

## Overview

SnapBlaster's AI feature provides intelligent parameter value generation based on musical context. This document outlines the design, implementation, and roadmap for this feature.

## Core Concept

The system uses structured metadata about parameters and musical context to generate appropriate MIDI CC values for different situations. Instead of using general-purpose LLMs via API calls, we will implement genre-specific, lightweight ML models for fast, offline inference with no recurring costs.

## Parameter Metadata Model

Each parameter includes the following metadata:

- **Name**: Clear, descriptive label
- **Instrument**: What the parameter controls (Lead Vocal, Bass, etc.)
- **Control Type**: The specific function (High-Pass Filter, Reverb Send, etc.)
- **Polarity**: Whether it's Unipolar (0 to 127) or Bipolar (-64 to +63)
- **Default Value**: The neutral/starting value
- **CC**: The MIDI CC number

## Snap Context Metadata

Each snap includes contextual information:

- **Section Type**: Part of the song (Verse, Chorus, Drop, etc.)
- **Energy Level**: Intensity (0-100%)
- **Genre**: Musical style (will influence which ML model is used)

## Genre-Specific ML Models

We will create specialized models for different musical genres, starting with:

1. Rock
2. Pop
3. Techno/House
4. Hip-Hop

Future releases will expand to include additional genres like Lo-Fi, Ambient, Jazz, etc.

## Technical Implementation

### Model Architecture

- Small feedforward neural network
- Input layer (~20 features for encoding metadata)
- 1-2 hidden layers (32-64 neurons)
- Output layer (single value 0-127)

The models will be optimized for CPU inference, with a target of <10ms per parameter prediction.

### Training Process

1. **Data Generation**:
    - Use rule-based system to create synthetic training examples
    - Supplement with expert-created parameter settings for specific genres
    - Collect anonymized user data as the application is used

2. **Training**:
    - Prototype models using PyTorch
    - Train separate models for each genre
    - Export trained models to ONNX format

3. **Deployment**:
    - Implement efficient inference in Rust using Tract or ONNX Runtime
    - Optimize with quantization for fast CPU inference
    - Package models with application releases

### Feature Engineering

Parameters and contexts will be encoded as follows:

- Categorical variables (Instrument, Control Type, Section) using one-hot encoding
- Numerical values (Energy, Default Value) normalized to 0-1 range
- Genre-specific features (e.g., tempo ranges, specific instrument presence)

## UI Integration

The interface will provide:

1. **Metadata Input**:
    - Dropdown selectors for parameter metadata during setup
    - Section type and energy level controls for snaps

2. **Value Generation**:
    - "Generate Intelligent Values" button
    - Genre selection option
    - Visual feedback during generation

3. **Fine-Tuning**:
    - Manual adjustment of generated values
    - Option to save favorite parameter combinations as presets

## Advantages Over LLM-Based Approach

1. **Performance**:
    - Faster responses (<10ms per parameter vs. seconds for API calls)
    - Works offline with no internet requirement

2. **Cost Efficiency**:
    - No ongoing API token costs
    - No usage limits

3. **Specialization**:
    - Models tailored to specific musical genres
    - Higher quality results for domain-specific tasks

4. **Product Evolution**:
    - New models can be featured in product updates
    - Continuous improvement through learning from user data

## Implementation Roadmap

### Phase 1: Foundation
- Implement parameter and snap metadata model
- Develop UI for metadata input
- Create synthetic training data generation system

### Phase 2: Initial Models
- Train and deploy models for 4 core genres
- Implement inference engine in Rust
- Add genre selection to UI

### Phase 3: Expansion
- Add more genre-specific models
- Implement anonymized user data collection
- Refine models with real-world usage data

### Phase 4: Advanced Features
- Allow model personalization based on user preferences
- Implement parameter relationship awareness
- Add style-transfer capability between genres

## Technical Notes

### Model Size & Distribution
- Each model will be approximately 50-100KB
- Total package size increase: ~500KB for all genres
- Models will be included in application binary

### Performance Targets
- <10ms inference time per parameter on modern CPUs
- <1 second for complete 64-parameter snap generation
- <50MB RAM usage during inference

## Marketing Considerations

The feature can be promoted as:
- "AI-powered intelligent parameter generation"
- "Genre-aware musical assistant"
- "Specialized musical AI that understands your sound"
- "Offline AI that respects your privacy"

Each new release can highlight:
- New genre models ("New Lo-Fi model for chilled beats")
- Improved accuracy ("30% more accurate House music parameters")
- New capabilities ("Cross-genre style transfer")