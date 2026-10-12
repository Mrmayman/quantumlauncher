#import <Cocoa/Cocoa.h>
#include <jni.h>
#include <stdio.h>

JNIEXPORT void JNICALL
Java_PolicyJNI_setActivationPolicy(JNIEnv *env, jclass clazz) {
    // If we're already past main-thread init (rare in premain), just log.
    if (![NSThread isMainThread]) {
        fprintf(stderr, "[policy] WARN: not on main thread\n");
    }

    [NSApplication sharedApplication];
    [NSApp setActivationPolicy:NSApplicationActivationPolicyRegular];
    [NSApp activateIgnoringOtherApps:YES];

    fprintf(stderr, "[policy] activationPolicy=%ld\n",
            (long)[NSApp activationPolicy]);
}
