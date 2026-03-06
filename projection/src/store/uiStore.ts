/**
 * UI Store — Zustand state management for UI interactions
 *
 * Manages panel visibility, selection, and extraction mode.
 */

import { create } from 'zustand';

interface UIState {
  // ── Selection & Hover ──────────────────────────────────────────
  selectedComponentId: string | null;
  hoveredComponentId: string | null;

  // ── Panel Visibility ───────────────────────────────────────────
  editorPanelOpen: boolean;
  infoPanelOpen: boolean;

  // ── Mode ───────────────────────────────────────────────────────
  extractionMode: 'manual' | 'ai';

  // ── Actions ────────────────────────────────────────────────────
  selectComponent: (id: string | null) => void;
  hoverComponent: (id: string | null) => void;
  toggleEditorPanel: () => void;
  toggleInfoPanel: () => void;
  setExtractionMode: (mode: 'manual' | 'ai') => void;
  resetUI: () => void;
}

const initialState = {
  selectedComponentId: null,
  hoveredComponentId: null,
  editorPanelOpen: true,
  infoPanelOpen: false,
  extractionMode: 'manual' as const,
};

export const useUIStore = create<UIState>((set) => ({
  ...initialState,

  selectComponent: (id) =>
    set({ selectedComponentId: id, infoPanelOpen: id !== null }),

  hoverComponent: (id) =>
    set({ hoveredComponentId: id }),

  toggleEditorPanel: () =>
    set((s) => ({ editorPanelOpen: !s.editorPanelOpen })),

  toggleInfoPanel: () =>
    set((s) => ({ infoPanelOpen: !s.infoPanelOpen })),

  setExtractionMode: (mode) =>
    set({ extractionMode: mode }),

  resetUI: () =>
    set(initialState),
}));
