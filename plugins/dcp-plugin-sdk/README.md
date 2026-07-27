# dcp-plugin-sdk

Rust SDK for writing DCP plugins.

## Example

```rust
use dcp_plugin_sdk::*;

struct MyPlugin;
impl Plugin for MyPlugin {
    fn provide_context(&self) -> ContextData {
        ContextData::new("custom", json!({"key": "value"}))
    }
}

run_plugin!(MyPlugin);
```
