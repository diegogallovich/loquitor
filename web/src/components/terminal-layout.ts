/**
 * Shared geometry for terminal-styled box-drawing components.
 * All terminal boxes (InfoCard, Divider) use the same width so they
 * align visually when stacked.
 */
export const BOX_WIDTH = 42;
export const BOX_INNER_WIDTH = BOX_WIDTH - 4; // 2 corner chars + 2 inner padding spaces
