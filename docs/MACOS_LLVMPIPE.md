Guide to run Minecraft via LLVMPIPE software graphics, in a macOS x86_64 VM using QuantumLauncher. GPU acceleration isn't required.

This is useful for testing of QuantumLauncher on macOS, for those
who don't own a mac (like me).

QuantumLauncher 0.5.0 and above are supported (including latest experimental builds).

 Note: We assume you're using macports because brew dropped support for macOS x86_64.

# Part 1: Setup

1. Install https://github.com/kholia/osx-kvm
	1. This Guide assumes macOS Ventura, adjust the curl URL in step 3 if otherwise
		1. You can find the right URL in macports download page
2. Enable SSH and log in
	1. Enable: Settings -> General -> Sharing -> Remote Login
	2. Click the circle i
	3. Select "For all users" and also enable full disk access
	4. In host, type `ssh USERNAMEINVM@localhost -p 2222` to open an SSH session
3. Run (single command):
```sh
cd Downloads && curl https://github.com/macports/macports-base/releases/download/v2.12.6/MacPorts-2.12.6-13-Ventura.pkg -L -O
```
4. Open Finder, go to Downloads and run the installer
5. Close your SSH (type `exit`), and open a new one
6. Run command `sudo port install meson ninja bison flex llvm-22`
	1. Enter `y` when necessary
	2. Leave to run in background
7. Now we need to install XCode Developer Tools while this is going on. Skip this if you have
	1. To trigger the installer window, open a second SSH and just type `git`
	2. It will tell you "No developer tools were found"
	3. Go to VM window and click Install and Agree

# Part 2: Compiling Mesa

1. Once the developer tools are installed, (the earlier port command may still be running), do:
	`cd ~`
	`git clone https://github.com/lucamignatti/mesa.git mesa-llvmpipe`
	`cd mesa-llvmpipe`

Note: This clones a fork of mesa, not upstream. This has some patches that make macOS Mesa stuff easier.

2. Once both developer tools installation and port command are completed, in the same SSH window where you did `cd mesa-llvmpipe`, do
	`sudo port select --set python python314`
	`sudo port select --set python3 python314`

3. Do this (copy paste all at once):
```sh
cat > native.ini <<EOF
[binaries]
bison = '/opt/local/bin/bison'
llvm-config = '/opt/local/bin/llvm-config-mp-22'
EOF
```

4. Create a python venv (each line separate command):
```sh
python3 --version
# Ensure the above is like 3.14 or later
# If it's 3.9 then close and restart your SSH
cd ~/mesa-llvmpipe
python3 -m venv .venv
source .venv/bin/activate
pip install mako pyyaml packaging
```

5. Run this (single command):
```sh
meson setup build --native-file native.ini \
  -Dprefix=$HOME/mesa-llvmpipe-install \
  -Dbuildtype=release \
  -Dplatforms=macos \
  -Degl-native-platform=surfaceless \
  -Degl=enabled \
  -Dgallium-drivers=llvmpipe \
  -Dgles1=enabled \
  -Dgles2=enabled \
  -Dglx=disabled \
  -Dgbm=disabled \
  -Dllvm=enabled
```
6. Run `ninja -C build` and `ninja -C build install`
7. Run this (each line separate command):
```sh
mkdir -p ~/mesa-llvmpipe-install/lib/dri
cd ~/mesa-llvmpipe-install/lib/dri

GALLIUM=$(ls ../libgallium-*.dylib | xargs basename)
ln -sf "../$GALLIUM" swrast_dri.so
ln -sf "../$GALLIUM" llvmpipe_dri.so

ls ~/mesa-llvmpipe-install/lib/libEGL.dylib
ls ~/mesa-llvmpipe-install/lib/dri/swrast_dri.so
ls ~/mesa-llvmpipe-install/lib/dri/llvmpipe_dri.so
```
It should output 3 different paths. If it's less than 3, or no paths are printed, or error comes, you messed up :D

8. Run this (copy whole thing into terminal):
```sh
cat > /tmp/egltest.c <<'EOF'
#include <EGL/egl.h>
#include <stdio.h>
int main(void) {
    EGLDisplay d = eglGetDisplay(EGL_DEFAULT_DISPLAY);
    EGLint maj, min;
    if (d == EGL_NO_DISPLAY) { printf("no display\n"); return 1; }
    if (!eglInitialize(d, &maj, &min)) { printf("init failed 0x%x\n", eglGetError()); return 1; }
    printf("EGL %d.%d vendor=%s\n", maj, min, eglQueryString(d, EGL_VENDOR));
    EGLint n = 0;
    eglGetConfigs(d, NULL, 0, &n);
    printf("configs: %d\n", n);
    return 0;
}
EOF

clang -o /tmp/egltest /tmp/egltest.c \
  -I$HOME/mesa-llvmpipe-install/include \
  -L$HOME/mesa-llvmpipe-install/lib -lEGL

DYLD_LIBRARY_PATH=$HOME/mesa-llvmpipe-install/lib \
EGL_PLATFORM=surfaceless \
  /tmp/egltest
```
Should output
```
EGL 1.5 vendor=Mesa Project  
configs: SOMENUMBERHERE
```
Where SOMENUMBERHERE depends on your configuration. If that works without any error, you're good

# Part 3: Hooking into a program

1. Make a folder in home called `hook` by doing `mkdir ~/hook`
2. Save this file as `~/hook/hook_llvmpipe.m` (Don't use Apple's TextEdit, that messes up your formatting. I used `vim` editor)

```objc
// hook_llvmpipe.m
//
// Redirects NSOpenGLPixelFormat/NSOpenGLContext to Mesa EGL surfaceless.
// Since surfaceless has no window surfaces, we create a pbuffer of the
// view's backing size, render into it, and blit it to the view's CALayer
// via a CGImage in -flushBuffer.

#import <Cocoa/Cocoa.h>
#import <QuartzCore/QuartzCore.h>
#import <objc/runtime.h>
#import <objc/message.h>
#include <dlfcn.h>
#include <stdio.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>
#include <EGL/egl.h>

#define DYLD_INTERPOSE(_replacement, _replacee)                                   \
  __attribute__((used)) static struct { const void *replacement; const void *replacee; } \
  _interpose_##_replacee __attribute__((section("__DATA,__interpose"))) = {       \
    (const void *)(unsigned long)&_replacement, (const void *)(unsigned long)&_replacee };

static int g_debug = 0;
static int g_enabled = 1;
#define DBG(...) do { if (g_debug) { fprintf(stderr, "[hook] " __VA_ARGS__); fputc('\n', stderr); } } while (0)

static void on_main(void (^b)(void)) {
  if ([NSThread isMainThread]) b(); else dispatch_sync(dispatch_get_main_queue(), b);
}

static EGLDisplay dpy(void) {
  static EGLDisplay d = EGL_NO_DISPLAY;
  static dispatch_once_t once;
  dispatch_once(&once, ^{
    d = eglGetDisplay(EGL_DEFAULT_DISPLAY);
    EGLint maj = 0, min = 0;
    if (d == EGL_NO_DISPLAY || !eglInitialize(d, &maj, &min)) {
      fprintf(stderr, "[hook] eglInitialize failed (0x%x)\n", eglGetError());
      d = EGL_NO_DISPLAY;
      return;
    }
    DBG("EGL %d.%d vendor=%s", maj, min, eglQueryString(d, EGL_VENDOR));
  });
  return d;
}

typedef struct { int color, alpha, depth, stencil, samples, profile; } ZPF;
static char kKey;

static int attr_has_value(uint32_t a) {
  switch (a) {
    case 7: case 8: case 11: case 12: case 13: case 14:
    case 55: case 56: case 70: case 84: case 99: case 128:
      return 1;
    default: return 0;
  }
}

static ZPF *pf_of(id o) {
  NSData *d = o ? objc_getAssociatedObject(o, &kKey) : nil;
  return d ? (ZPF *)[d bytes] : NULL;
}

static void generic_dealloc(id self, SEL _cmd) {
  struct objc_super s = { self, [NSObject class] };
  ((void (*)(struct objc_super *, SEL))objc_msgSendSuper)(&s, _cmd);
}

static id pf_init(id self, SEL _cmd, const uint32_t *attrs) {
  struct objc_super s = { self, [NSObject class] };
  self = ((id (*)(struct objc_super *, SEL))objc_msgSendSuper)(&s, @selector(init));
  if (!self) return nil;
  ZPF pf = {0, 0, 0, 0, 0, 0x1000};
  for (const uint32_t *p = attrs; p && *p;) {
    uint32_t a = *p++;
    if (attr_has_value(a)) {
      uint32_t v = *p++;
      switch (a) {
        case 8: pf.color = v; break;
        case 11: pf.alpha = v; break;
        case 12: pf.depth = v; break;
        case 13: pf.stencil = v; break;
        case 56: pf.samples = v; break;
        case 99: pf.profile = v; break;
      }
    }
  }
  DBG("pixel format: color=%d alpha=%d depth=%d stencil=%d samples=%d profile=0x%x",
      pf.color, pf.alpha, pf.depth, pf.stencil, pf.samples, pf.profile);
  objc_setAssociatedObject(self, &kKey, [NSData dataWithBytes:&pf length:sizeof pf],
                           OBJC_ASSOCIATION_RETAIN);
  return self;
}

typedef struct {
  EGLContext ctx;
  EGLConfig cfg;
  EGLSurface surf;
  NSView *view;
  CALayer *layer;
  EGLint swap;
  EGLint width;
  EGLint height;
} ZCtx;

static __thread id tl_current = nil;

static ZCtx *zc(id o) {
  NSMutableData *d = o ? objc_getAssociatedObject(o, &kKey) : nil;
  return d ? (ZCtx *)[d mutableBytes] : NULL;
}

static EGLConfig pick_config(EGLDisplay d, const ZPF *pf) {
  EGLint count = 0;
  if (!eglGetConfigs(d, NULL, 0, &count) || count < 1) return NULL;
  EGLConfig *all = (EGLConfig *)calloc(count, sizeof(EGLConfig));
  if (!all) return NULL;
  eglGetConfigs(d, all, count, &count);
  EGLConfig best = NULL;
  long best_score = -1000000;
  for (int i = 0; i < count; i++) {
    EGLint rt = 0, r = 0, g = 0, b = 0, a = 0, dp = 0, sp = 0, smp = 0, id = 0;
    eglGetConfigAttrib(d, all[i], EGL_RENDERABLE_TYPE, &rt);
    eglGetConfigAttrib(d, all[i], EGL_RED_SIZE, &r);
    eglGetConfigAttrib(d, all[i], EGL_GREEN_SIZE, &g);
    eglGetConfigAttrib(d, all[i], EGL_BLUE_SIZE, &b);
    eglGetConfigAttrib(d, all[i], EGL_ALPHA_SIZE, &a);
    eglGetConfigAttrib(d, all[i], EGL_DEPTH_SIZE, &dp);
    eglGetConfigAttrib(d, all[i], EGL_STENCIL_SIZE, &sp);
    eglGetConfigAttrib(d, all[i], EGL_SAMPLES, &smp);
    eglGetConfigAttrib(d, all[i], EGL_CONFIG_ID, &id);
    if (!(rt & EGL_OPENGL_BIT)) continue;
    if (r != 8 || g != 8 || b != 8) continue;
    if (pf->alpha && a < 8) continue;
    if (dp < pf->depth || sp < pf->stencil) continue;
    long score = 1000
               - labs((long)a - (pf->alpha ? 8 : 0)) * 10
               - (dp - pf->depth) - (sp - pf->stencil)
               - (long)smp * 5;
    if (score > best_score) { best_score = score; best = all[i]; }
  }
  if (best) {
    EGLint id = 0;
    eglGetConfigAttrib(d, best, EGL_CONFIG_ID, &id);
    DBG("picked EGL config id 0x%x", id);
  }
  free(all);
  return best;
}

static void sync_layer(ZCtx *z) {
  NSView *v = z->view;
  CALayer *l = z->layer;
  if (!v || !l) return;
  on_main(^{
    NSSize b = v.bounds.size;
    NSSize px = [v convertSizeToBacking:b];
    CGFloat sc = b.width > 0 ? px.width / b.width : 1.0;
    [CATransaction begin];
    [CATransaction setDisableActions:YES];
    l.frame = v.bounds;
    l.contentsScale = sc;
    [CATransaction commit];
  });
}

static void attach_layer(ZCtx *z, NSView *v) {
  on_main(^{
    CALayer *l = v.layer;
    if (![l isKindOfClass:[CAMetalLayer class]]) {
      CAMetalLayer *m = [CAMetalLayer layer];
      m.opaque = YES;
      m.framebufferOnly = NO;
      [v setLayer:m];
      [v setWantsLayer:YES];
      l = m;
    }
    z->layer = l;
  });
  sync_layer(z);
}

static id ctx_init(id self, SEL _cmd, id fmt, id share) {
  struct objc_super s = { self, [NSObject class] };
  self = ((id (*)(struct objc_super *, SEL))objc_msgSendSuper)(&s, @selector(init));
  if (!self) return nil;
  EGLDisplay d = dpy();
  if (d == EGL_NO_DISPLAY) return nil;

  ZPF pf = {0, 0, 24, 0, 0, 0x1000};
  ZPF *pp = pf_of(fmt);
  if (pp) pf = *pp;
  if (pf.samples > 0) DBG("multisampling requested (%d) - ignored", pf.samples);

  eglBindAPI(EGL_OPENGL_API);

  EGLConfig cfg = pick_config(d, &pf);
  if (!cfg) {
    fprintf(stderr, "[hook] no suitable EGL config\n");
    return nil;
  }

  EGLint ca[16];
  int i = 0;
  if (pf.profile == 0x3200) {
    ca[i++] = EGL_CONTEXT_MAJOR_VERSION; ca[i++] = 3;
    ca[i++] = EGL_CONTEXT_MINOR_VERSION; ca[i++] = 2;
    ca[i++] = EGL_CONTEXT_OPENGL_PROFILE_MASK; ca[i++] = EGL_CONTEXT_OPENGL_CORE_PROFILE_BIT;
  } else if (pf.profile == 0x4100) {
    ca[i++] = EGL_CONTEXT_MAJOR_VERSION; ca[i++] = 4;
    ca[i++] = EGL_CONTEXT_MINOR_VERSION; ca[i++] = 1;
    ca[i++] = EGL_CONTEXT_OPENGL_PROFILE_MASK; ca[i++] = EGL_CONTEXT_OPENGL_CORE_PROFILE_BIT;
  }
  ca[i++] = EGL_NONE;

  ZCtx *sz = share ? zc(share) : NULL;
  EGLContext c = eglCreateContext(d, cfg, sz ? sz->ctx : EGL_NO_CONTEXT, ca);
  if (c == EGL_NO_CONTEXT) {
    fprintf(stderr, "[hook] eglCreateContext failed (0x%x)\n", eglGetError());
    return nil;
  }
  NSMutableData *data = [NSMutableData dataWithLength:sizeof(ZCtx)];
  ZCtx *z = (ZCtx *)[data mutableBytes];
  z->ctx = c; z->cfg = cfg; z->surf = EGL_NO_SURFACE;
  z->width = 640; z->height = 480;
  objc_setAssociatedObject(self, &kKey, data, OBJC_ASSOCIATION_RETAIN);
  DBG("context created (profile 0x%x)", pf.profile);
  return self;
}

static void ctx_dealloc(id self, SEL _cmd) {
  ZCtx *z = zc(self);
  if (z) {
    EGLDisplay d = dpy();
    if (tl_current == self) { eglMakeCurrent(d, EGL_NO_SURFACE, EGL_NO_SURFACE, EGL_NO_CONTEXT); tl_current = nil; }
    if (z->surf != EGL_NO_SURFACE) eglDestroySurface(d, z->surf);
    if (z->ctx != EGL_NO_CONTEXT) eglDestroyContext(d, z->ctx);
    [z->view release];
  }
  generic_dealloc(self, _cmd);
}

static void ctx_setView(id self, SEL _cmd, NSView *view) {
  ZCtx *z = zc(self);
  if (!z) return;
  EGLDisplay d = dpy();
  if (z->surf != EGL_NO_SURFACE) {
    if (tl_current == self) eglMakeCurrent(d, EGL_NO_SURFACE, EGL_NO_SURFACE, z->ctx);
    eglDestroySurface(d, z->surf);
    z->surf = EGL_NO_SURFACE;
  }
  [z->view release];
  z->view = nil;
  z->layer = nil;
  if (!view) return;
  z->view = [view retain];
  attach_layer(z, view);

  NSSize px = [view convertSizeToBacking:view.bounds.size];
  z->width = (EGLint)px.width;
  z->height = (EGLint)px.height;
  EGLint pb_attribs[] = {
    EGL_WIDTH, z->width,
    EGL_HEIGHT, z->height,
    EGL_NONE
  };
  z->surf = eglCreatePbufferSurface(d, z->cfg, pb_attribs);
  if (z->surf == EGL_NO_SURFACE)
    fprintf(stderr, "[hook] eglCreatePbufferSurface failed (0x%x)\n", eglGetError());
  else
    DBG("pbuffer surface created (%dx%d)", z->width, z->height);
  if (tl_current == self) eglMakeCurrent(d, z->surf, z->surf, z->ctx);
}

static id ctx_view(id self, SEL _cmd) { ZCtx *z = zc(self); return z ? z->view : nil; }

static void ctx_make(id self, SEL _cmd) {
  ZCtx *z = zc(self);
  if (!z) return;
  EGLDisplay d = dpy();
  eglBindAPI(EGL_OPENGL_API);
  if (!eglMakeCurrent(d, z->surf, z->surf, z->ctx)) {
    fprintf(stderr, "[hook] eglMakeCurrent failed (0x%x)\n", eglGetError());
    return;
  }
  tl_current = self;
  if (z->surf != EGL_NO_SURFACE) eglSwapInterval(d, z->swap);
}

static void ctx_clear_current(id self, SEL _cmd) {
  eglMakeCurrent(dpy(), EGL_NO_SURFACE, EGL_NO_SURFACE, EGL_NO_CONTEXT);
  tl_current = nil;
}

static id ctx_current(id self, SEL _cmd) { return tl_current; }

static void release_pixels(void *info, const void *data, size_t size) {
  (void)info; (void)size;
  free((void *)data);
}

static void ctx_flush(id self, SEL _cmd) {
  ZCtx *z = zc(self);
  if (!z || z->surf == EGL_NO_SURFACE || !z->layer) return;
  EGLDisplay d = dpy();

  {
    NSView *v = z->view;
    NSSize px = [v convertSizeToBacking:v.bounds.size];
    EGLint newW = (EGLint)px.width;
    EGLint newH = (EGLint)px.height;
    if (newW > 0 && newH > 0 && (newW != z->width || newH != z->height)) {
      DBG("resize %dx%d -> %dx%d", z->width, z->height, newW, newH);
      if (tl_current == self)
        eglMakeCurrent(d, EGL_NO_SURFACE, EGL_NO_SURFACE, z->ctx);
      if (z->surf != EGL_NO_SURFACE) eglDestroySurface(d, z->surf);
      z->width = newW;
      z->height = newH;
      EGLint pb_attribs[] = {
        EGL_WIDTH, z->width,
        EGL_HEIGHT, z->height,
        EGL_NONE
      };
      z->surf = eglCreatePbufferSurface(d, z->cfg, pb_attribs);
      if (z->surf == EGL_NO_SURFACE) {
        fprintf(stderr, "[hook] resize: eglCreatePbufferSurface failed (0x%x)\n", eglGetError());
      }
      if (tl_current == self)
        eglMakeCurrent(d, z->surf, z->surf, z->ctx);
      sync_layer(z);
    }
  }

  if (tl_current != self) {
    eglMakeCurrent(d, z->surf, z->surf, z->ctx);
    tl_current = self;
  }

  typedef void (*PFNGLREADPIXELS)(int, int, int, int, unsigned int, unsigned int, void *);
  static PFNGLREADPIXELS p_glReadPixels = NULL;
  if (!p_glReadPixels) {
    p_glReadPixels = (PFNGLREADPIXELS)eglGetProcAddress("glReadPixels");
    if (!p_glReadPixels) {
      fprintf(stderr, "[hook] glReadPixels not available\n");
      return;
    }
  }

  size_t row_bytes = (size_t)z->width * 4;
  size_t buf_size = row_bytes * (size_t)z->height;
  void *pixels = malloc(buf_size);
  if (!pixels) return;
  p_glReadPixels(0, 0, z->width, z->height,
                 0x1908 /* GL_RGBA */, 0x1401 /* GL_UNSIGNED_BYTE */, pixels);

  {
    unsigned char *bytes = (unsigned char *)pixels;
    unsigned char *tmp = (unsigned char *)malloc(row_bytes);
    if (tmp) {
      size_t half = (size_t)z->height / 2;
      for (size_t y = 0; y < half; y++) {
        unsigned char *top = bytes + y * row_bytes;
        unsigned char *bot = bytes + (size_t)(z->height - 1 - (EGLint)y) * row_bytes;
        memcpy(tmp, top, row_bytes);
        memcpy(top, bot, row_bytes);
        memcpy(bot, tmp, row_bytes);
      }
      free(tmp);
    }
  }

  CGDataProviderRef provider = CGDataProviderCreateWithData(NULL, pixels, buf_size, release_pixels);
  CGColorSpaceRef cs = CGColorSpaceCreateDeviceRGB();
  CGImageRef img = CGImageCreate(z->width, z->height, 8, 32, row_bytes,
                                 cs, kCGImageAlphaPremultipliedLast | kCGBitmapByteOrder32Big,
                                 provider, NULL, false, kCGRenderingIntentDefault);
  if (img) {
    on_main(^{
      z->layer.contents = (id)img;
      [z->layer setNeedsDisplay];
    });
    CGImageRelease(img);
  }
  CGColorSpaceRelease(cs);
  CGDataProviderRelease(provider);
}

static void ctx_update(id self, SEL _cmd) {
  ZCtx *z = zc(self);
  if (z) sync_layer(z);
}

static void ctx_clear_drawable(id self, SEL _cmd) { ctx_setView(self, @selector(setView:), nil); }

static void ctx_set_values(id self, SEL _cmd, const int *vals, long param) {
  ZCtx *z = zc(self);
  if (!z || !vals) return;
  if (param == 222) {
    z->swap = vals[0];
    if (tl_current == self && z->surf != EGL_NO_SURFACE) eglSwapInterval(dpy(), z->swap);
  }
}

static void ctx_get_values(id self, SEL _cmd, int *vals, long param) {
  ZCtx *z = zc(self);
  if (!z || !vals) return;
  vals[0] = (param == 222) ? z->swap : 0;
}

static void *ctx_cgl(id self, SEL _cmd) { return NULL; }

static __thread int in_hook = 0;

static int is_gl_name(const char *n) {
  return n && n[0] == 'g' && n[1] == 'l' && n[2] >= 'A' && n[2] <= 'Z';
}

static void *egl_lookup(const char *n) {
  if (!g_enabled || in_hook || !is_gl_name(n)) return NULL;
  in_hook = 1;
  void *p = (void *)eglGetProcAddress(n);
  in_hook = 0;
  return p;
}

static void *hook_dlsym(void *h, const char *n) {
  void *p = egl_lookup(n);
  if (p) return p;
  return dlsym(h, n);
}
DYLD_INTERPOSE(hook_dlsym, dlsym)

static void *hook_CFBundleGetFunctionPointerForName(CFBundleRef b, CFStringRef name) {
  char buf[256];
  if (name && CFStringGetCString(name, buf, sizeof buf, kCFStringEncodingUTF8)) {
    void *p = egl_lookup(buf);
    if (p) return p;
  }
  return CFBundleGetFunctionPointerForName(b, name);
}
DYLD_INTERPOSE(hook_CFBundleGetFunctionPointerForName, CFBundleGetFunctionPointerForName)

static void rep(Class c, SEL s, IMP imp, BOOL meta) {
  Method m = meta ? class_getClassMethod(c, s) : class_getInstanceMethod(c, s);
  if (!m) { fprintf(stderr, "[hook] method not found: %s\n", sel_getName(s)); return; }
  class_replaceMethod(meta ? object_getClass(c) : c, s, imp, method_getTypeEncoding(m));
}

__attribute__((constructor)) static void hook_init(void) {
  g_debug = getenv("HOOK_DEBUG") != NULL;
  if (getenv("HOOK_DISABLE")) { g_enabled = 0; return; }
  setenv("EGL_PLATFORM", "surfaceless", 0);
  setenv("MESA_LOADER_DRIVER_OVERRIDE", "llvmpipe", 0);
  setenv("GALLIUM_DRIVER", "llvmpipe", 0);

  Class cc = NSClassFromString(@"NSOpenGLContext");
  Class pc = NSClassFromString(@"NSOpenGLPixelFormat");
  if (!cc || !pc) { fprintf(stderr, "[hook] AppKit classes missing\n"); g_enabled = 0; return; }

  rep(pc, @selector(initWithAttributes:), (IMP)pf_init, NO);
  rep(pc, sel_registerName("dealloc"), (IMP)generic_dealloc, NO);

  rep(cc, @selector(initWithFormat:shareContext:), (IMP)ctx_init, NO);
  rep(cc, sel_registerName("dealloc"), (IMP)ctx_dealloc, NO);
  rep(cc, @selector(setView:), (IMP)ctx_setView, NO);
  rep(cc, @selector(view), (IMP)ctx_view, NO);
  rep(cc, @selector(makeCurrentContext), (IMP)ctx_make, NO);
  rep(cc, @selector(flushBuffer), (IMP)ctx_flush, NO);
  rep(cc, @selector(update), (IMP)ctx_update, NO);
  rep(cc, @selector(clearDrawable), (IMP)ctx_clear_drawable, NO);
  rep(cc, @selector(setValues:forParameter:), (IMP)ctx_set_values, NO);
  rep(cc, @selector(getValues:forParameter:), (IMP)ctx_get_values, NO);
  rep(cc, @selector(CGLContextObj), (IMP)ctx_cgl, NO);
  rep(cc, @selector(currentContext), (IMP)ctx_current, YES);
  rep(cc, @selector(clearCurrentContext), (IMP)ctx_clear_current, YES);
  DBG("installed");
}
```

3. Compile it (copy paste whole thing into terminal):

```sh
cd ~/hook
clang -dynamiclib -fno-objc-arc -arch x86_64 \
  -I$HOME/mesa-llvmpipe-install/include \
  -o libhook_llvmpipe.dylib \
  hook_llvmpipe.m \
  -framework Cocoa -framework QuartzCore \
  -L$HOME/mesa-llvmpipe-install/lib -lEGL
```

4. Save this file as `~/hook/run_mc.sh`

```sh
#!/bin/bash
MESA="$HOME/mesa-llvmpipe-install"  
HOOK="$HOME/hook"

export DYLD_INSERT_LIBRARIES="$HOOK/libhook_llvmpipe.dylib"  
  
# ----- Mesa llvmpipe configuration -----  
export DYLD_LIBRARY_PATH="$MESA/lib:$DYLD_LIBRARY_PATH"  
export LIBGL_DRIVERS_PATH="$MESA/lib/dri"  
export EGL_PLATFORM=surfaceless  
export GALLIUM_DRIVER=llvmpipe  
export MESA_LOADER_DRIVER_OVERRIDE=llvmpipe  
export LIBGL_ALWAYS_SOFTWARE=1  
  
# ----- Clear any leftover Vulkan/Zink vars (safe) -----  
unset VK_DRIVER_FILES VK_ICD_FILENAMES VK_INSTANCE_LAYERS VK_LAYER_PATH  
  
# ----- Optional debug output from the hook -----  
# export HOOK_DEBUG=1  
  
exec "$@"
```

5. Make it runnable: `chmod +x ~/hook/run_mc.sh`
6. In QuantumLauncher, go to Settings -> Game -> Global Pre-Launch Prefix, then add `/Users/YOURMACUSERNAME/hook/run_mc.sh`
7. Enjoy!
