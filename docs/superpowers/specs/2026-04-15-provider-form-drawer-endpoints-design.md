# Provider Form Drawer & Dynamic Endpoints Design

## Overview

Convert provider create/edit forms from Modal to Drawer, and replace fixed endpoint fields (OpenAI/Anthropic) with dynamic key-value pairs using a dropdown preset system.

## UI/UX Specification

### Layout

**Drawer Configuration:**
- Width: 640px
- Slide-in from right
- Header with title and close button
- Scrollable content area
- Footer with action buttons (Cancel, Save)

**Form Fields:**
1. Name (required, text input)
2. Base URL (optional, text input, placeholder: "https://api.openai.com/v1")
3. Endpoints (dynamic key-value list)
4. Enabled toggle (checkbox)

### Endpoints Component

**Structure:**
- Inline table/grid with rows
- Each row: Protocol dropdown | URL input | Remove button
- "Add Endpoint" button below the list

**Protocol Dropdown Options:**
- OpenAI
- Anthropic
- Azure
- Google
- Custom (allows freeform text input for key)

**Behavior:**
- Add: Click "Add Endpoint" → adds new row with default protocol
- Remove: Click trash icon → removes that row
- Edit: Click field to edit inline

**Data Format (stored as JSON):**
```json
{
  "openai": "https://api.openai.com/v1",
  "anthropic": "https://api.anthropic.com",
  "custom": "https://custom.endpoint.com"
}
```

### Visual Design

**Color Palette:** Follow existing app theme (DaisyUI)

**Typography:** Same as existing form components

**Spacing:** Consistent with existing form layouts (gap-4 between form controls)

**States:**
- Empty state: Show "No endpoints configured" message with "Add Endpoint" button
- Hover: Subtle highlight on rows
- Focus: Standard input focus states

## Functionality Specification

### Create Form (Drawer)
1. User clicks "Add Provider" button
2. Drawer opens with empty form
3. User fills name (required), optional base URL
4. User adds endpoints via "Add Endpoint" button
5. User selects protocol from dropdown, enters URL
6. User clicks "Create" → saves to backend
7. Drawer closes, list refreshes

### Edit Form (Drawer)
1. User clicks edit icon on provider row
2. Drawer opens with existing data pre-filled
3. Existing endpoints parsed from JSON and displayed as rows
4. User can add/remove/edit endpoints
5. User clicks "Save Changes" → updates backend
6. Drawer closes, list refreshes

### Validation
- Name: required, non-empty
- Endpoint URL: optional, but if protocol selected, URL should be non-empty
- At least one endpoint not required (base_url can serve as fallback)

### Edge Cases
- Empty endpoints: Store as `null` in database
- Duplicate protocol keys: Prevent or last-one-wins
- Very long URL: Truncate display, full value stored

## Components to Modify

1. `web/src/pages/Providers.tsx` - Main changes
2. `web/src/components/ui/Drawer.tsx` - Already exists, reuse
3. `web/src/types/index.ts` - No changes needed (endpoints is already string)

## Acceptance Criteria

1. ✓ Provider create form opens in Drawer (not Modal)
2. ✓ Provider edit form opens in Drawer (not Modal)
3. ✓ Endpoints displayed as dynamic rows, not fixed fields
4. ✓ Can add new endpoint row via button
5. ✓ Can remove endpoint row via delete button
6. ✓ Can edit protocol and URL inline in each row
7. ✓ Protocol dropdown has presets: OpenAI, Anthropic, Azure, Google, Custom
8. ✓ "Custom" option allows freeform key entry
9. ✓ Form data correctly serializes to JSON for backend
10. ✓ Edit form correctly parses existing JSON to populate rows