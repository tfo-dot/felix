# Felix Desktop Pet 🐾

**Felix** is an interactive, animated desktop companion designed for Linux systems, optimized for the **Hyprland** Wayland compositor. Written in Rust and built using **GTK4** and **gtk4-layer-shell**, Felix lives on your screen, reacts to your system activity (like typing or active window changes), helps you stay productive with an integrated Pomodoro timer and todo checklist, and connects to your local smart home.

---

## Key Features

- **🐾 Animated & Responsive State Machine**: Felix animations react dynamically to your actions. Supported states include:
  - *Idle / Sitting*
  - *Walking* (left and right across the screen or window edges)
  - *Climbing* (up and down the borders of active windows)
  - *Typing* (animated typing when you use your keyboard)
  - *Sleeping* (when you are inactive)
  - *Working Hard* (when CPU utilization is high)
  - *Dancing* (when you complete a Pomodoro session or checklist item)
  - *Petting* (reacts to clicks with hearts and purring)
  - *Portal In / Portal Out* (spawning and despawning transitions)
- **🪟 Hyprland Window Interaction**: When running under Hyprland, Felix tracks active windows, detects their boundaries, and can walk on top of, sit on, or climb the sides of your open windows.
- **💬 Application-Aware Conversations**: Felix comments on your activity! It has custom interactive responses when it detects specific apps or games (e.g., playing *Wuthering Waves*, *Reverse: 1999*, or coding in *Kitty* terminal / *Sublime Text*).
- **⏱️ Integrated Pomodoro Timer**: A productivity timer with custom work/break intervals, visible as part of the overlay and manageable via the system tray. Felix will celebrate when you finish a focus block!
- **📋 Persistent Todo Checklist**: A simple todo checklist card pinned to the desktop widget, saving tasks to `~/.config/pet-app/tasks.json`.
- **🏠 Home Assistant Integration**: Connects to your Home Assistant instance via a Long-Lived Access Token to retrieve current weather states and adapt.
- **🎵 Media Player Tracking**: Polls media players via `playerctl` to display current playing song changes right in Felix's speech bubbles.
- **⚙️ Hot-Reloadable TOML Configuration**: Automatically watches and applies configuration updates on the fly from `~/.config/pet-app/config.toml`.
- **🌐 DBus System Tray**: Provides a system tray menu (via KSni status notifier protocol) to toggle the Pomodoro timer, toggle checklist visibility, reload configuration, hide/show the pet, and quit.

---

## Dependencies & Prerequisites

### System Packages
Ensure the following libraries are installed on your Linux system:
* **GTK4** (e.g., `libgtk-4-dev` on Debian/Ubuntu, `gtk4-devel` on Fedora/openSUSE, `gtk4` on Arch Linux)
* **gtk4-layer-shell** (e.g., `libgtk4-layer-shell-dev` on Debian/Ubuntu, `gtk4-layer-shell` on Arch Linux)
* **playerctl** (Optional, for media/song detection)

### Permissions for Keyboard Tracking (`evdev`)
Felix monitors raw keyboard events to trigger typing animations when you type in any application. To read keyboard events from `/dev/input/event*` without root permissions, your user must be a member of the `input` group:

```bash
sudo usermod -aG input $USER
```
*Note: You must log out and log back in (or restart your session) for this change to take effect.*

---

## Installation & Setup

1. Clone the repository and navigate into the folder:
   ```bash
   git clone https://github.com/yourusername/felix.git
   cd felix
   ```
2. Build in release mode:
   ```bash
   cargo build --release
   ```
3. Run the compiled binary:
   ```bash
   ./target/release/felix
   ```

---

## Configuration

On startup, Felix automatically creates a default configuration file at:
`~/.config/pet-app/config.toml`

The configuration file is watched automatically; changes are loaded instantly without needing to restart the app.

### Example `config.toml`

```toml
# Screen positioning and offsets
[anchor]
edge_bottom = true
edge_right = true
edge_top = false
edge_left = false
margin_bottom = 20
margin_right = 20
margin_top = 0
margin_left = 0

# Pet characteristics
[pet]
scale = 1.0                     # Scaling factor of the pet
size = 128                      # Base dimensions (in pixels)
cursor_speed_threshold = 30.0   # Speed above which the cursor is tracked
window_interaction = true       # Enable climbing/sitting on active windows (Hyprland only)

# Pomodoro productivity timer settings
[pomodoro]
work_duration_mins = 25
short_break_mins = 5
long_break_mins = 15
start_paused = true             # If true, the timer starts in paused state

# Desktop checklist visibility
checklist_visible = true

# Smart home integration (Home Assistant)
ha_address = "http://homeassistant.local:8123/api/states/weather.forecast"
ha_key = "YOUR_LONG_LIVED_ACCESS_TOKEN_HERE"
```

---

## Troubleshooting

### Felix fails to start or loops checking display
Felix checks if your X11/Wayland display server is running on startup. If it is launched too early in your desktop startup script (before the display compositor is fully initialized), it will retry for up to 10 seconds. Adjust your startup configuration to launch it after your compositor starts.

### Hyprland features are missing
* Ensure you are running under a Hyprland session and the environment variable `HYPRLAND_INSTANCE_SIGNATURE` is set.
* Ensure you have compiled the app with the `hyprland` cargo feature enabled (it is enabled by default).

### Typing animations do not play
Check if your user has read permissions to the input devices. Run `ls -l /dev/input/event*` to verify they are owned by the `input` group, and ensure your user is in that group.

---

## License

This project is licensed under the MIT License - see the LICENSE file for details.