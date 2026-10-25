// Colours copied from pi's dark theme:
// earendil-works/pi packages/coding-agent/src/modes/interactive/theme/dark.json
// (commit d5b3de22d), MIT licensed, Copyright (c) Earendil Works.
import {
  type Color,
  getTerminalColorMode,
  parseColor,
  styleText,
  type TextAttributes,
} from "@earendil-works/pi-tui";

const palette = {
  text: "okhsl(234 3% 89%)",
  accent: "okhsl(295 50% 67%)",
  muted: "okhsl(229 6% 67%)",
  dim: "okhsl(229 8% 56%)",
  success: "okhsl(159 59% 67%)",
  error: "okhsl(20 72% 67%)",
  warning: "okhsl(83 88% 67%)",
} as const;

export type ThemeColor = keyof typeof palette;

const colors = Object.fromEntries(
  Object.entries(palette).map(([key, value]) => [key, parseColor(value)]),
) as Record<ThemeColor, Color>;

export function fg(color: ThemeColor, text: string, attrs: TextAttributes = {}): string {
  return styleText(text, { ...attrs, fg: colors[color] }, getTerminalColorMode());
}

export function bold(text: string): string {
  return styleText(text, { bold: true }, getTerminalColorMode());
}
