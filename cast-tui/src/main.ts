import { homedir } from "node:os";
import { matchesKey, ProcessTerminal, TuiAltScreen } from "@earendil-works/pi-tui";
import { listInstances } from "./cast.js";
import { isQuitKey } from "./keys.js";
import { Refresher } from "./refresh.js";
import { InstanceListView } from "./view.js";

const REFRESH_INTERVAL_MS = 2000;

export function main(): void {
  const tui = new TuiAltScreen(new ProcessTerminal());
  const view = new InstanceListView({ home: homedir() });
  tui.addChild(view);

  const refresher = new Refresher(async () => {
    try {
      view.setInstances(await listInstances());
    } catch (e) {
      view.setError(e instanceof Error ? e.message : String(e));
    }
    tui.requestRender();
  });

  tui.addInputListener((data) => {
    if (isQuitKey(data)) {
      refresher.stop();
      tui.stop();
      process.exit(0);
    } else if (matchesKey(data, "up")) {
      view.moveUp();
      tui.requestRender();
    } else if (matchesKey(data, "down")) {
      view.moveDown();
      tui.requestRender();
    } else if (matchesKey(data, "r")) {
      void refresher.run();
    }
    return undefined;
  });
  tui.start();
  void refresher.run();
  refresher.start(REFRESH_INTERVAL_MS);
}
