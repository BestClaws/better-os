# Window Manager System Improvements

This document outlines planned improvements to the window manager architecture.

## 1. Input Handling System

**Status**: ✅ COMPLETED  
**Priority**: High

### Overview
Apps currently have no way to receive user input (touch, buttons, keyboard).

### Design
```rust
pub enum InputEvent {
    Touch { x: u16, y: u16, pressed: bool },
    Button { id: u8, pressed: bool },
    Keyboard { key: char },
}

trait App {
    fn on_input(&mut self, event: InputEvent) -> bool; // Returns true if handled
}
```

### Implementation Status
- ✅ Added InputEvent enum with Touch/Button/Keyboard variants
- ✅ Input event queue added to AppShell
- ✅ Events routed to focused app automatically
- ✅ All demo apps implement on_input() handler

---

## 2. Inter-App Communication

**Status**: ✅ COMPLETED  
**Priority**: Medium

### Overview
Apps need a way to communicate with each other for data sharing and coordination.

### Design
```rust
pub struct Message {
    from: AppId,
    to: AppId,
    data: Vec<u8>,
}

trait App {
    fn on_message(&mut self, from: AppId, message: &[u8]);
}

impl AppShell {
    pub fn send_message(&mut self, from: AppId, to: AppId, data: &[u8]);
    pub fn broadcast_message(&mut self, from: AppId, data: &[u8]);
}
```

### Implementation Status
- ✅ Message struct with from/to AppId and data Vec
- ✅ Message queue and routing in AppShell
- ✅ send_message() and broadcast_message() implemented
- ✅ process_messages() called every frame
- ✅ All demo apps implement on_message() handler
- ✅ Tested with "Hello from Shapes!" message

---

## 3. Focus Management

**Status**: ✅ COMPLETED  
**Priority**: High

### Overview
Track which app is currently focused/active for input routing and lifecycle management.

### Design
```rust
pub struct FocusManager {
    focused_app: Option<AppId>,
    focus_history: Vec<AppId>,
}

pub enum FocusEvent {
    Gained,
    Lost,
}

trait App {
    fn on_focus(&mut self, event: FocusEvent);
}
```

### Implementation Status
- ✅ FocusManager struct with focus_history
- ✅ FocusEvent enum (Gained/Lost)
- ✅ Focus events emitted on window switches
- ✅ Input routing to focused app only
- ✅ focus_previous() method for "back" functionality
- ✅ All demo apps implement on_focus() handler

---

## 4. Window Property Requests

**Status**: ✅ COMPLETED  
**Priority**: Medium

### Overview
Apps should be able to request changes to their window properties dynamically.

### Design
```rust
pub struct WindowRequest {
    pub visible: Option<bool>,
    pub z_index: Option<i32>,
    pub geometry: Option<WindowGeometry>,
    pub title: Option<String>,
}

impl Surface {
    pub fn request_window_change(&mut self, request: WindowRequest);
}
```

### Implementation Status
- ✅ WindowRequest enum with SetVisible/Geometry/Title/BringToFront
- ✅ show() and hide() convenience methods on Surface
- ✅ Request queue processing in update_apps()
- ✅ process_window_request() method handles all types
- ✅ Lifecycle events triggered on visibility changes

---

## 5. Dirty Region Tracking

**Status**: ✅ COMPLETED  
**Priority**: Low (optimization)

### Overview
Currently redrawing entire windows. Track changed regions to optimize compositing.

### Design
```rust
pub struct DirtyRegion {
    x: u16,
    y: u16,
    width: u16,
    height: u16,
}

impl Surface {
    pub fn mark_dirty_region(&mut self, x: u16, y: u16, width: u16, height: u16);
}
```

### Implementation Status
- ✅ DirtyRegion struct with overlap and merge logic
- ✅ Window.dirty_regions Vec with MAX_REGIONS limit
- ✅ mark_dirty_region() with automatic merging
- ✅ Compositor blit_window() optimized to render only dirty regions
- ✅ Falls back to full window if no dirty regions marked

impl Compositor {
    fn blit_dirty_regions(&mut self, window: &Window);
}
```

### Implementation Plan
- Add dirty region list to Window
- Track regions in Surface during drawing
- Compositor only blits changed regions
- Union overlapping regions
- Fall back to full redraw if too many regions

---

## 6. Enhanced Lifecycle Events

**Status**: ✅ COMPLETED  
**Priority**: High

### Overview
Apps need lifecycle events to manage resources and respond to visibility changes.

### Design
```rust
pub enum LifecycleEvent {
    Created,
    Visible,      // Window shown
    Hidden,       // Window hidden but still active
    Suspended,    // App suspended (not updating)
    Resumed,      // App resumed
    Destroyed,
}

trait App {
    fn on_lifecycle(&mut self, event: LifecycleEvent);
}
```

### Implementation Status
- ✅ LifecycleEvent enum with all 6 states
- ✅ on_lifecycle() handler in App trait
- ✅ Lifecycle events triggered on visibility changes via WindowRequest::SetVisible
- ✅ All demo apps implement on_lifecycle() handler
- ✅ Events logged for debugging

---

## 7. Async Event System

**Status**: Not Started  
**Priority**: Medium

### Overview
Apps could await events instead of polling, better for Embassy async runtime.

### Design
```rust
pub enum AppEvent {
    Input(InputEvent),
    Message(Message),
    Focus(FocusEvent),
    Lifecycle(LifecycleEvent),
    Timer(TimerId),
}

trait AsyncApp {
    async fn wait_for_event(&mut self) -> AppEvent;
    async fn run(&mut self, surface: Surface);
}
```

### Implementation Plan
- Create AsyncApp trait alongside App
- Use Embassy channels for event queues
- Apps can select! over multiple event sources
- Maintain compatibility with sync App trait

---

## 8. Compositor Control API

**Status**: ✅ COMPLETED  
**Priority**: Medium

### Overview
Apps and system services need control over window switching and transitions.

### Design
```rust
pub enum CompositorCommand {
    SwitchToApp { app_name: String, transition, duration_ms, easing },
    SwitchToWindow { window_id: WindowId, transition, duration_ms, easing },
    SwitchNext { transition, duration_ms, easing },
    SwitchPrevious { transition, duration_ms, easing },
}

impl AppShell {
    pub fn queue_compositor_command(&mut self, cmd: CompositorCommand);
}
```

### Implementation Status
- ✅ CompositorCommand enum with 4 command types
- ✅ Command queue in Compositor
- ✅ queue_command() and take_commands() API
- ✅ Command processing in compositor_srv
- ✅ SwitchToWindow with focus update
- ✅ SwitchToApp by name lookup
- ✅ SwitchNext with wrap-around
- ✅ SwitchPrevious with wrap-around
- ✅ All commands update focus automatically

---

## 9. Shared Resource Manager

**Status**: Not Started  
**Priority**: Low

### Overview
Share resources like textures, fonts, color palettes between apps to save memory.

### Design
```rust
pub struct ResourceManager {
    textures: HashMap<ResourceId, Box<[u8]>>,
    fonts: HashMap<ResourceId, FontData>,
    shared_data: HashMap<ResourceId, Arc<[u8]>>,
}

impl AppShell {
    pub fn resource_manager(&self) -> &ResourceManager;
    pub fn register_resource(&mut self, id: ResourceId, data: Arc<[u8]>);
    pub fn get_resource(&self, id: ResourceId) -> Option<Arc<[u8]>>;
}
```

### Implementation Plan
- Add ResourceManager to AppShell
- Use Arc for zero-copy sharing
- Register system fonts/resources at startup
- Apps can register their own shared resources

---

## 10. Performance Monitoring

**Status**: ✅ COMPLETED  
**Priority**: Low

### Overview
Track CPU time, memory usage, and frame times per app for debugging and optimization.

### Design
```rust
pub struct AppMetrics {
    pub service_time_us: u64,
    pub ui_time_us: u64,
    pub frame_count: u32,
    pub avg_frame_time_us: u32,
    pub last_frame_time_us: u32,
}

impl AppShell {
    pub fn get_metrics(&self, app_id: AppId) -> Option<&AppMetrics>;
    pub fn all_metrics(&self) -> Vec<(AppId, &AppMetrics)>;
    pub fn reset_metrics(&mut self, app_id: AppId);
}
```

### Implementation Status
- ✅ AppMetrics struct tracking service and UI times
- ✅ Embassy Instant timing for precise measurements
- ✅ Per-app metrics in AppInstance
- ✅ Instrumented update_apps() with timing
- ✅ get_metrics(), all_metrics(), reset_metrics() API
- ✅ Metrics logged every 60 frames in compositor_srv
- ✅ Shows per-app performance breakdown (service: 0-10μs, UI: 5-12ms)

---

## Implementation Status Summary

### Phase 1 (Core Functionality) - ✅ COMPLETED
1. ✅ Input Handling System - Touch/Button/Keyboard events with routing
2. ✅ Focus Management - FocusManager with history and automatic routing
3. ✅ Enhanced Lifecycle Events - 6 event types with automatic triggering

### Phase 2 (App Features) - ✅ COMPLETED
4. ✅ Inter-App Communication - Message passing and broadcasting
5. ✅ Compositor Control API - 4 command types for window switching
6. ✅ Window Property Requests - SetVisible/Geometry/Title/BringToFront

### Phase 3 (Optimizations) - ✅ COMPLETED
7. ✅ Dirty Region Tracking - Region-based rendering with automatic merging
8. ✅ Performance Monitoring - Per-app timing with detailed breakdown

### Phase 4 (Future Enhancements) - NOT STARTED
9. ⏳ Shared Resource Manager - Memory optimization via shared resources
10. ⏳ Async Event System - Embassy-based event awaiting

---

## Testing Results

### System Performance (LUMA4 format, 205x251 display)
- Total frame time: ~75ms
- Rendering: 32ms
- Display flush: 19ms
- Per-app UI update: 5-12ms average
- Per-app service: 0-10μs (background tasks)
- 3 concurrent apps running smoothly

### Features Validated
- ✅ Input events injected and logged by apps
- ✅ Focus switching between apps with history
- ✅ Messages sent between apps and broadcasts
- ✅ Compositor commands switching windows
- ✅ Window visibility requests processed
- ✅ Lifecycle events triggered on visibility changes
- ✅ Metrics showing detailed per-app breakdown
- ✅ All apps responding to all event types

### Memory Usage
- 3 fullscreen windows (LUMA4): ~77KB total
- Individual window: ~26KB each
- System overhead: <5KB for managers

---

## Next Steps (Optional Future Work)

### Async Event System
Would allow apps to use Embassy's async/await for cleaner event handling:
```rust
async fn app_main(mut events: EventReceiver, surface: Surface) {
    loop {
        match events.recv().await {
            AppEvent::Input(e) => handle_input(e),
            AppEvent::Message(m) => handle_message(m),
            // ...
        }
    }
}
```

### Shared Resource Manager
Could save significant memory by sharing fonts, textures, and other resources:
- System fonts registered once at startup
- Apps reference shared resources by ID
- Zero-copy via Arc<[u8]>
- Estimated savings: 10-30KB depending on resource reuse

Both features are nice-to-have but not critical for current functionality.

### Phase 4 (Advanced)
10. Async Event System

---

## Notes

- Memory constraints (190KB heap) must be considered for all features
- Prefer zero-copy and in-place operations
- Keep allocations predictable for embedded environment
- All features should be optional/configurable
- Document memory usage for each feature
