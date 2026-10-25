/** Runs a refresh on demand and on an interval, never two at once. */
export class Refresher {
  private inFlight: Promise<void> | undefined;
  private timer: ReturnType<typeof setInterval> | undefined;

  constructor(private readonly refresh: () => Promise<void>) {}

  /** Starts a refresh, or returns the one already running. */
  run(): Promise<void> {
    this.inFlight ??= this.refresh().finally(() => {
      this.inFlight = undefined;
    });
    return this.inFlight;
  }

  start(intervalMs: number): void {
    this.stop();
    this.timer = setInterval(() => void this.run(), intervalMs);
  }

  stop(): void {
    if (this.timer !== undefined) clearInterval(this.timer);
    this.timer = undefined;
  }
}
