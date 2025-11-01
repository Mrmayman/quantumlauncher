# Community Fixes

A collection of fixes , found by the community
Each fix include a description , launch-argument(if required), and the contributors who discroverd it 


---

## Out of stack Error (Vulkan Mod)


**Description**

This issue is not related to Wayland. It occurs when a Vulkan-based mod attempts to spawn too many threads.  
Each native thread reserves a portion of stack memory, and on systems using **NVIDIA proprietary drivers**, this default stack size is quite large (around **1 MB per thread**). 

When the mod creates a large number of threads, the system quickly runs out of available native stack space, leading to crashes or errors such as 
```log
out of stack space

or 

OutOfMemoryError: unable to create new native thread
```
**Fix**

Add the following argument to your Java launch options:

```java
-Dorg.lwjgl.system.stackSize=256
```

**Credits**

Discovered and workaround by [Apical Shark](https://github.com/apicalshark/).

---

## SSLHandshakeException with third-party auth providers on Windows (unable to find valid certification path)

**Description**

When launching Minecraft through QuantumLauncher with **authlib-injector 1.2.5** on Windows 10/11 authentication against third party auth proivders fails due to an **SSL handshake error**, caused due to having an older version of java
```txt
[authlib-injector] [ERROR] Failed to fetch metadata:
javax.net.ssl.SSLHandshakeException: 
sun.security.validator.ValidatorException: PKIX path 
building failed: 
sun.security.provider.certpath.SunCertPathBuilderException: 
unable to find valid certification path to requested target
```

**Fix**

A) use a newer java version and supply the custom java excutable to the QuantumLauncher

B) Add the following arguments to java launch option

```java
--Djavax.net.ssl.trustStoreType=Windows-ROOT
```

**Credits**

Discovered by blackbananaman1 at discord

workaround by [Sreehari425](https://github.com/Sreehari425/)


## LLVM 3.8 Invalid Record (WGPU Graphics Backend)

**Description**

```txt

error: Invalid record (Producer: 'LLVM3.8.0' Reader: 'LLVM 3.8.0')

```

When wgpu tries to compile or read cached shader data using those backends, the underlying LLVM/DXIL parser fails, resulting in this error.

**Fix**

Force QuantumLauncher to use the OpenGL backend instead of DirectX 12 or Vulkan
set WGPU_BACKEND=gl

or

Create qldir.txt in the launcher directory

and in qldir.txt

```txt
.
i_opengl
```

**Credits**

Discoverd by Spicy Bee(balos_sandor) at discord

workaround by [Aurlt](https://github.com/Aurlt)  
   
    
