# Multi-Platform Guide

## Platform Support Matrix

| Feature | Linux (X11) | Linux (Wayland) | macOS | Windows |
|---------|------------|-----------------|-------|---------|
| Active Window | ✅ xdotool | ✅ Hyprland/Sway | ✅ JXA | ✅ PowerShell |
| Window Tree | ✅ xdotool | ✅ Hyprland | ⚠️ stub | ⚠️ stub |
| Running Processes | ✅ /proc | ✅ /proc | ✅ ps | ✅ WMI |
| Clipboard Read | ✅ xclip | ✅ wl-paste | ✅ pbpaste | ✅ Get-Clipboard |
| Clipboard Set | ✅ xclip | ✅ wl-copy | ⚠️ stub | ✅ Set-Clipboard |
| Mouse Position | ✅ xdotool | ✅ Hyprland | ✅ osascript | ⚠️ stub |
| Monitors | ✅ xrandr | ✅ Hyprland | ✅ system_profiler | ⚠️ stub |
| System Resources | ✅ /proc | ✅ /proc | ✅ vm_stat | ✅ WMI |
| Network State | ✅ /sys | ✅ /sys | ✅ ifconfig | ✅ Get-NetAdapter |
| Audio Devices | ✅ pactl | ✅ pactl | ⚠️ stub | ⚠️ stub |
| Power State | ✅ /sys | ✅ /sys | ✅ pmset | ✅ WMI |
| Workspace | ✅ xprop | ✅ Hyprland | ⚠️ stub | ⚠️ stub |
| Notifications | ✅ D-Bus | ✅ D-Bus | ⚠️ stub | ⚠️ stub |
| Screen Capture | ✅ import | ✅ grim | ⚠️ stub | ⚠️ stub |
| OCR | ✅ tesseract | ✅ tesseract | ⚠️ stub | ⚠️ stub |
| Automation | ✅ xdotool | ✅ Hyprland | ⚠️ stub | ⚠️ stub |

## Platform-Specific Setup

### Linux (Wayland — Hyprland)

Hyprland has the best Wayland support via `hyprctl` IPC:

```bash
# Required packages (Arch)
sudo pacman -S hyprland wl-clipboard grim tesseract

# Optional for XWayland fallback
sudo pacman -S xdotool xclip

# Verify Hyprland environment
echo $HYPRLAND_INSTANCE_SIGNATURE
# Should output something like: 0x5a8b3c2f
```

### Linux (X11)

```bash
# Required packages
sudo pacman -S xdotool xclip xrandr imagemagick tesseract

# Verify DISPLAY is set
echo $DISPLAY
# Should output: :0
```

### macOS

No additional dependencies needed — uses built-in commands:

```bash
# Verify tools
osascript -e 'return version of application "System Events"'
pbpaste < /dev/null
```

### Windows

No additional dependencies needed — uses built-in PowerShell:

```powershell
# Verify PowerShell version
$PSVersionTable.PSVersion
# Should be 5.1 or higher

# Test clipboard access
Get-Clipboard
```

## Feature Limitations

### Linux Wayland (non-Hyprland/Sway)

Without compositor support for wlr-foreign-toplevel-management:
- Window list returns empty
- Active window falls back to XWayland
- Screen capture uses grim (full screen only)

### macOS

The following features are limited or unavailable:
- Screen capture (requires Accessibility API permission)
- Full window tree (requires Accessibility API)
- Clipboard write (macOS sandboxing restrictions)
- Audio device management
- System notification interception

### Windows

The following features are limited or in development:
- Window tree (full enumeration via Win32)
- Mouse position (P/Invoke needed)
- Audio device details
- Workspace management
- System notification interception

## Cross-Platform Development

### Adding a New Platform Feature

1. Add method to `PlatformBackend` trait in `platform/mod.rs`
2. Implement for Linux in `platform/linux.rs`
3. Implement for macOS in `platform/macos.rs`
4. Implement for Windows in `platform/windows.rs`
5. Add test case in integration tests
6. Document in API reference

### Conditional Compilation

DCP uses `#[cfg(target_os)]` for platform-specific code:

```rust
impl PlatformBackend for WindowsBackend {
    async fn clipboard(&self) -> Result<ClipboardData> {
        #[cfg(windows)] {
            // Windows-specific implementation
            Ok(ClipboardData { ... })
        }
        #[cfg(not(windows))]
        Ok(ClipboardData::default())
    }
}
```

## Testing Across Platforms

### CI Matrix

The CI pipeline builds and tests on all platforms:

```yaml
matrix:
  os: [ubuntu-latest, macos-latest, windows-latest]
  target:
    - x86_64-unknown-linux-gnu
    - aarch64-unknown-linux-gnu
    - x86_64-apple-darwin
    - aarch64-apple-darwin
    - x86_64-pc-windows-msvc
```

### Smoke Test

Run this on any platform to verify basic functionality:

```bash
dcp status
# Should return version, platform, sessions
```
