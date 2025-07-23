**UI**:
1. Doesn't scale with host window (most likely fix is to build window rendering logic manually)
2. (sort of) crossbeam::channel apparently isn't realtime safe so have to switch message passing to rtrb

**Agent**:
1. Checking server connection panics plugin
2. typing 'a' in plugin still opens FX panel on Reaper


