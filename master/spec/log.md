# Project Log

## [4797102] Analyzed logging/instrumentation and prepared implementation plan

- **Found:** The application has no formal logging framework; uses println!
- **Found:** Command execution for agents uses execvp, which terminates cast prematurely
- **Found:** No persistent log files exist for debugging container crashes or resource limit issues
- **Found:** Missing structured tracing/instrumentation across critical paths (Docker, Config, Nix)
- **Decided:** Switch from execvp to wait-based process execution for agents to allow post-run analysis
- **Decided:** Integrate tracing and tracing-appender for dual console/file logging
- **Decided:** Implement structured spans for critical functions to capture context during failures
- **Open:** Finalize log file location (~/.local/state/cast vs .cast/)
- **Open:** Determine if --quiet flag is needed alongside --verbose

