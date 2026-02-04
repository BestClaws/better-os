# Window Manager System Improvements

This document outlines planned improvements to the window manager architecture.

## 1. Input Handling System

**Status**: Not Started  
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

### Implementation Plan
- Add input event queue to AppShell
- Route events to focused app first
- Fall back to system handlers if not handled
- Support multi-touch gestures

---

## 2. Inter-App Communication

**Status**: Not Started  
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

### Implementation Plan
- Add message queue per app
- Implement message routing in AppShell
- Add broadcast capability for system events
- Consider message size limits for embedded environment

---

## 3. Focus Management

**Status**: Not Started  
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

### Implementation Plan
- Add FocusManager to AppShell
- Emit focus events when compositor switches windows
- Track focus history for "back" functionality
- Only route input to focused app

---

## 4. Window Property Requests

**Status**: Not Started  
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

### Implementation Plan
- Add request queue in Surface
- Process requests in AppShell::update_apps()
- Allow apps to show/hide themselves
- Support geometry changes (fullscreen, windowed)

---

## 5. Dirty Region Tracking

**Status**: Not Started  
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

**Status**: Partially Complete  
**Priority**: High

### Current State
- Have `suspend()` and `resume()` methods
- Not called based on window visibility changes

### Improvements Needed
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

### Implementation Plan
- Call suspend/resume when window visibility changes
- Add lifecycle event enum
- Track app state transitions
- Document state machine clearly

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

**Status**: Not Started  
**Priority**: Medium

### Overview
Apps and system services need control over window switching and transitions.

### Design
```rust
pub enum CompositorCommand {
    SwitchToApp { app_id: AppId, transition: Transition },
    SwitchToWindow { window_id: WindowId, transition: Transition },
    SetTransition { transition: Transition },
    ShowAll,  // Split-screen or grid view
}

impl AppShell {
    pub fn send_compositor_command(&mut self, cmd: CompositorCommand);
}
```

### Implementation Plan
- Add command queue between AppShell and Compositor
- Process commands in compositor service
- Allow apps to trigger window switches
- Add permission system (which apps can control compositor)

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

**Status**: Not Started  
**Priority**: Low

### Overview
Track CPU time, memory usage, and frame times per app for debugging and optimization.

### Design
```rust
pub struct AppMetrics {
    pub cpu_time_us: u64,
    pub service_time_us: u64,
    pub ui_time_us: u64,
    pub frame_count: u32,
    pub avg_frame_time_us: u32,
}

impl AppShell {
    pub fn get_metrics(&self, app_id: AppId) -> Option<&AppMetrics>;
    pub fn reset_metrics(&mut self, app_id: AppId);
}
```

### Implementation Plan
- Add AppMetrics to AppInstance
- Time each update() and service_update() call
- Track memory allocations (if possible)
- Expose metrics via system app or logging
- Add warning thresholds for slow apps

---

## Implementation Priority

### Phase 1 (Core Functionality)
1. Input Handling System
2. Focus Management
3. Enhanced Lifecycle Events

### Phase 2 (App Features)
4. Inter-App Communication
5. Compositor Control API
6. Window Property Requests

### Phase 3 (Optimizations)
7. Dirty Region Tracking
8. Performance Monitoring
9. Shared Resource Manager

### Phase 4 (Advanced)
10. Async Event System

---

## Notes

- Memory constraints (190KB heap) must be considered for all features
- Prefer zero-copy and in-place operations
- Keep allocations predictable for embedded environment
- All features should be optional/configurable
- Document memory usage for each feature
