# RMK Controller Changes (ca38784)

## Event Routing Mechanism

The `#[controller(subscribe = [...])]` macro generates boilerplate that routes events to handler methods.

When you write:
```rust
#[controller(subscribe = [ConnectionChangeEvent, BleStateChangeEvent, ...])]
pub struct StatusLedController { ... }
```

The macro generates:

1. **A wrapper enum** combining all subscribed event types
2. **Subscribers** for each event channel
3. **A `process_event()` implementation** that pattern-matches the event type and calls the corresponding `on_{event_name}_event()` method

## Handler Naming Convention

The macro expects handler methods following this pattern:
```
EventTypeName → on_{snake_case_name}_event
```

| Event Type | Expected Handler Method |
|------------|------------------------|
| `ConnectionChangeEvent` | `on_connection_change_event()` |
| `BleStateChangeEvent` | `on_ble_state_change_event()` |
| `BatteryStateEvent` | `on_battery_state_event()` |
| `BleProfileChangeEvent` | `on_ble_profile_change_event()` |
| `KeyEvent` | `on_key_event()` |

## Generated Code (Conceptual)

The macro generates code roughly equivalent to:
```rust
impl Controller for StatusLedController {
    type Event = GeneratedWrapperEnum;

    async fn process_event(&mut self, event: Self::Event) {
        match event {
            GeneratedWrapperEnum::ConnectionChangeEvent(e) => {
                self.on_connection_change_event(e).await
            }
            GeneratedWrapperEnum::BleStateChangeEvent(e) => {
                self.on_ble_state_change_event(e).await
            }
            // ... etc for each subscribed event
        }
    }
}
```

## PollingController Integration

The `polling_loop()` from `PollingController` trait alternates between:
1. Waiting for events from all subscribed channels using `select`
2. Calling `update()` at your specified interval (700ms in our case)

---

## Migration Summary (f0872fa → ca38784)

### Old API
```rust
impl Controller for StatusLedController {
    type Event = ControllerEvent;  // Single enum with all events

    async fn process_event(&mut self, event: Self::Event) {
        match event {
            ControllerEvent::ConnectionType(conn_type) => { ... }
            ControllerEvent::BleState(profile, state) => { ... }
            ControllerEvent::Battery(percentage) => { ... }
            // etc
        }
    }

    async fn next_message(&mut self) -> Self::Event {
        self.sub.next_message_pure().await
    }
}

impl PollingController for StatusLedController {
    const INTERVAL: Duration = Duration::from_millis(700);
    async fn update(&mut self) { ... }
}
```

### New API
```rust
#[controller(subscribe = [ConnectionChangeEvent, BleStateChangeEvent, ...])]
pub struct StatusLedController { ... }

impl StatusLedController {
    // Handler methods - called automatically by generated process_event()
    async fn on_connection_change_event(&mut self, event: ConnectionChangeEvent) { ... }
    async fn on_ble_state_change_event(&mut self, event: BleStateChangeEvent) { ... }
    async fn on_battery_state_event(&mut self, event: BatteryStateEvent) { ... }
    async fn on_ble_profile_change_event(&mut self, event: BleProfileChangeEvent) { ... }
    async fn on_key_event(&mut self, event: KeyEvent) { ... }
}

impl PollingController for StatusLedController {
    fn interval(&self) -> Duration { Duration::from_millis(700) }  // Now a method
    async fn update(&mut self) { ... }
}
```

### Key Changes

| Aspect | Old | New |
|--------|-----|-----|
| Event type | `ControllerEvent` enum | Individual event structs |
| Subscription | Manual `ControllerSub` field | `#[controller(subscribe = [...])]` macro |
| Event handling | Single `process_event()` with match | Separate `on_*_event()` methods |
| `next_message()` | Required method | Removed (macro handles it) |
| `INTERVAL` | `const` | `fn interval(&self)` method |
| Battery event | `ControllerEvent::Battery(u8)` | `BatteryStateEvent` enum with `Normal(u8)`, `Charging`, `Charged`, `NotAvailable` |
| User keys | `KeyCode::User0`-`User7` | `Action::User(0)`-`Action::User(7)` |
| Bootloader | `KeyCode::Bootloader` | `Action::KeyboardControl(KeyboardAction::Bootloader)` |
