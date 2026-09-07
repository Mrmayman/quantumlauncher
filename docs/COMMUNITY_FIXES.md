# Community Fixes

A collection of workarounds to issues, found by the community.
Each fix includes a description, workaround, and credits.

**First of all, make sure your launcher version is up to date
before resorting to any workarounds. Usually that may solve the issue on its own.**

---

## 1. Out of Stack Error (Vulkan Mods) (by [Apical Shark](https://github.com/apicalshark/))

### Symptoms

The game crashes with errors such as:

- `out of stack space`
- `OutOfMemoryError: unable to create new native thread`

<details>
<summary><b>Why does this happen?</b></summary>

Some Vulkan renderer mods (such as VulkanMod)
create a large number of native threads.
On systems using **NVIDIA proprietary drivers**,
each thread reserves too much space
(approximately **1 MB** of stack memory) by default.
Creating too many threads can quickly
exhaust the available native stack space.

</details>

### Fix

Add the following to your Java arguments:

```java
-Dorg.lwjgl.system.stackSize=256
```

---

## 2) DirectX Problems (Launcher not opening or LLVM error)

### Symptoms

- Double-clicking the launcher on Windows does nothing.
- The log stops at `Starting up the launcher...`
- Sometimes you receive the error: `error: Invalid record (Producer: 'LLVM3.8.0' Reader: 'LLVM 3.8.0')`

<details>
<summary><b>Why does this happen?</b></summary>

This happens when wgpu attempts to compile or load cached
DirectX/Vulkan shaders, but the system's LLVM/DXIL parser fails.
Switching to the OpenGL backend avoids the issue.

</details>

### Fix

Create qldir.txt in `QuantumLauncher/` folder.
(see [FAQ](https://mrmayman.github.io/quantumlauncher/faq)), and input this in it:

```txt
.
i_opengl
```

Or instead of this, for a temporary workaround you can
set the `WGPU_BACKEND` environment variable to `gl`.

### Credits

- Discovered by Spicy Bee (balos_sandor) at discord
- Workaround by [Aurlt](https://github.com/Aurlt)  

---

## 3) GLFW error 65548: Wayland window icon not supported (Minecraft 1.18.2 on Wayland) (by [Sreehari425](https://github.com/Sreehari425))

### Symptoms

When launching Minecraft 1.18.2 (and similar versions)
on Linux Wayland desktop environments (hyprland, niri ,...), the game crashes with:

```txt
GLFW error 65548: Wayland: The platform does not support setting the window icon
```

<details>
<summary><b>Why does this happen?</b></summary>

This occurs because GLFW, which Minecraft uses for window management, does not support setting window icons on Wayland. The limitation is at the Wayland protocol level, not in the launcher or game.

</details>

### Fix

Add the following environment variable to your instance configuration:

```txt
XDG_SESSION_TYPE=x11
```

This tells GLFW to use the X11 backend instead of Wayland, which works reliably through XWayland (pre-installed on most Linux systems with Wayland support).
