import { ProcessTerminal, Text, TuiAltScreen } from "@earendil-works/pi-tui";
import { isQuitKey } from "./keys.js";

export function main(): void {
  const tui = new TuiAltScreen(new ProcessTerminal());
  tui.addChild(new Text("cast-tui: no instances view yet. Press q to quit."));
  tui.addInputListener((data) => {
    if (isQuitKey(data)) {
      tui.stop();
      process.exit(0);
    }
    return undefined;
  });
  tui.start();
}
