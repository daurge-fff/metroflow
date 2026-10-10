#![cfg(target_os = "android")]

use miniquad::native::android::{attach_jni_env, ACTIVITY};
use ndk_sys::{jclass, jmethodID, jobject, JNIEnv};
use std::ffi::CStr;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

unsafe fn clear(env: *mut JNIEnv) -> bool {
    if ((**env).ExceptionCheck.unwrap())(env) != 0 {
        ((**env).ExceptionClear.unwrap())(env);
        true
    } else {
        false
    }
}

unsafe fn obj_class(env: *mut JNIEnv, obj: jobject) -> jclass {
    ((**env).GetObjectClass.unwrap())(env, obj)
}

unsafe fn method(env: *mut JNIEnv, class: jclass, name: &[u8], sig: &[u8]) -> jmethodID {
    ((**env).GetMethodID.unwrap())(
        env,
        class,
        name.as_ptr() as *const _,
        sig.as_ptr() as *const _,
    )
}

unsafe fn static_method(env: *mut JNIEnv, class: jclass, name: &[u8], sig: &[u8]) -> jmethodID {
    ((**env).GetStaticMethodID.unwrap())(
        env,
        class,
        name.as_ptr() as *const _,
        sig.as_ptr() as *const _,
    )
}

unsafe fn call_obj(env: *mut JNIEnv, obj: jobject, mid: jmethodID) -> jobject {
    ((**env).CallObjectMethod.unwrap())(env, obj, mid)
}

unsafe fn call_void(env: *mut JNIEnv, obj: jobject, mid: jmethodID) {
    ((**env).CallVoidMethod.unwrap())(env, obj, mid);
}

unsafe fn call_void_i(env: *mut JNIEnv, obj: jobject, mid: jmethodID, arg: i32) {
    ((**env).CallVoidMethod.unwrap())(env, obj, mid, arg);
}

unsafe fn call_bool(env: *mut JNIEnv, obj: jobject, mid: jmethodID) -> bool {
    ((**env).CallBooleanMethod.unwrap())(env, obj, mid) != 0
}

unsafe fn call_static_int(env: *mut JNIEnv, class: jclass, mid: jmethodID) -> i32 {
    ((**env).CallStaticIntMethod.unwrap())(env, class, mid)
}

unsafe fn jstring(env: *mut JNIEnv, s: jobject) -> Option<String> {
    if s.is_null() {
        return None;
    }
    let chars = ((**env).GetStringUTFChars.unwrap())(env, s, std::ptr::null_mut());
    if chars.is_null() {
        clear(env);
        return None;
    }
    let out = CStr::from_ptr(chars).to_string_lossy().into_owned();
    ((**env).ReleaseStringUTFChars.unwrap())(env, s, chars);
    Some(out)
}

/// Каталог приватных файлов приложения (`getFilesDir()`), доступен для записи.
pub fn files_dir() -> Option<String> {
    unsafe {
        let env = attach_jni_env();
        if env.is_null() || ACTIVITY.is_null() {
            return None;
        }
        let cls = obj_class(env, ACTIVITY);
        let mid = method(env, cls, b"getFilesDir\0", b"()Ljava/io/File;\0");
        if mid.is_null() || clear(env) {
            return None;
        }
        let file = call_obj(env, ACTIVITY, mid);
        if file.is_null() || clear(env) {
            return None;
        }
        let fcls = obj_class(env, file);
        let pmid = method(env, fcls, b"getAbsolutePath\0", b"()Ljava/lang/String;\0");
        if pmid.is_null() || clear(env) {
            return None;
        }
        let path = call_obj(env, file, pmid);
        if clear(env) {
            return None;
        }
        jstring(env, path)
    }
}

/// Скрывает системные панели (статус и навигацию) — полноценный immersive.
pub fn hide_system_bars() {
    unsafe {
        let env = attach_jni_env();
        if env.is_null() || ACTIVITY.is_null() {
            return;
        }
        let acls = obj_class(env, ACTIVITY);
        let get_window = method(env, acls, b"getWindow\0", b"()Landroid/view/Window;\0");
        if get_window.is_null() || clear(env) {
            return;
        }
        let window = call_obj(env, ACTIVITY, get_window);
        if window.is_null() || clear(env) {
            return;
        }
        let wcls = obj_class(env, window);
        let get_controller = method(
            env,
            wcls,
            b"getInsetsController\0",
            b"()Landroid/view/WindowInsetsController;\0",
        );
        if get_controller.is_null() || clear(env) {
            return;
        }
        let controller = call_obj(env, window, get_controller);
        if controller.is_null() || clear(env) {
            return;
        }

        let type_cls =
            ((**env).FindClass.unwrap())(env, c"android/view/WindowInsets$Type".as_ptr());
        if type_cls.is_null() || clear(env) {
            return;
        }
        let system_bars = static_method(env, type_cls, b"systemBars\0", b"()I\0");
        if system_bars.is_null() || clear(env) {
            return;
        }
        let bars = call_static_int(env, type_cls, system_bars);
        clear(env);

        let ccls = obj_class(env, controller);
        let behavior = method(env, ccls, b"setSystemBarsBehavior\0", b"(I)V\0");
        let hide = method(env, ccls, b"hide\0", b"(I)V\0");
        clear(env);
        if !hide.is_null() {
            call_void_i(env, controller, behavior, 2);
            clear(env);
            call_void_i(env, controller, hide, bars);
            clear(env);
        }
    }
}

/// Re-applies edge-to-edge layout and hidden system bars. Running it after the
/// app regains focus makes Android re-layout the window (fixes the stale-size
/// dark stripe).
pub fn apply_immersive() {
    unsafe {
        let env = attach_jni_env();
        if env.is_null() || ACTIVITY.is_null() {
            return;
        }
        let cls = obj_class(env, ACTIVITY);
        let mid = method(env, cls, b"setFullScreen\0", b"(Z)V\0");
        if !mid.is_null() && !clear(env) {
            call_void_i(env, ACTIVITY, mid, 1);
            clear(env);
        }
    }
    hide_system_bars();
}

fn window_focus() -> Option<bool> {
    unsafe {
        let env = attach_jni_env();
        if env.is_null() || ACTIVITY.is_null() {
            return None;
        }
        let cls = obj_class(env, ACTIVITY);
        let mid = method(env, cls, b"hasWindowFocus\0", b"()Z\0");
        if mid.is_null() || clear(env) {
            return None;
        }
        let focused = call_bool(env, ACTIVITY, mid);
        clear(env);
        Some(focused)
    }
}

/// Detects focus transitions (resume) and periodically re-asserts immersive mode.
pub fn maintain_immersive() {
    static COUNTDOWN: AtomicU32 = AtomicU32::new(6);
    static FOCUS: AtomicBool = AtomicBool::new(false);

    let focused = window_focus().unwrap_or(true);
    let was_focused = FOCUS.swap(focused, Ordering::Relaxed);
    let just_resumed = focused && !was_focused;

    let remaining = COUNTDOWN.load(Ordering::Relaxed);
    if focused && (just_resumed || remaining == 0) {
        apply_immersive();
        COUNTDOWN.store(150, Ordering::Relaxed);
    } else if remaining > 0 {
        COUNTDOWN.store(remaining - 1, Ordering::Relaxed);
    }
}

/// Finishes the Activity (returns to the launcher) and then hard-exits the
/// process, so the next launch is always a clean cold start.
pub fn quit_app() {
    unsafe {
        let env = attach_jni_env();
        if env.is_null() || ACTIVITY.is_null() {
            return;
        }
        let cls = obj_class(env, ACTIVITY);
        let mid = method(env, cls, b"finishAndRemoveTask\0", b"()V\0");
        if mid.is_null() || clear(env) {
            return;
        }
        call_void(env, ACTIVITY, mid);
        clear(env);
    }
    // Give the system time to show the launcher, then skip miniquad's teardown.
    std::thread::sleep(std::time::Duration::from_millis(350));
    unsafe { libc::_exit(0) }
}

/// Opens a URL in the system browser via `Intent.ACTION_VIEW`.
pub fn open_url(url: &str) {
    let Ok(curl) = std::ffi::CString::new(url) else {
        return;
    };
    unsafe {
        let env = attach_jni_env();
        if env.is_null() || ACTIVITY.is_null() {
            return;
        }
        let uri_cls = ((**env).FindClass.unwrap())(env, c"android/net/Uri".as_ptr());
        if uri_cls.is_null() || clear(env) {
            return;
        }
        let parse = static_method(
            env,
            uri_cls,
            b"parse\0",
            b"(Ljava/lang/String;)Landroid/net/Uri;\0",
        );
        if parse.is_null() || clear(env) {
            return;
        }
        let jurl = ((**env).NewStringUTF.unwrap())(env, curl.as_ptr());
        let uri = ((**env).CallStaticObjectMethod.unwrap())(env, uri_cls, parse, jurl);
        if uri.is_null() || clear(env) {
            return;
        }

        let intent_cls = ((**env).FindClass.unwrap())(env, c"android/content/Intent".as_ptr());
        if intent_cls.is_null() || clear(env) {
            return;
        }
        let ctor = method(
            env,
            intent_cls,
            b"<init>\0",
            b"(Ljava/lang/String;Landroid/net/Uri;)V\0",
        );
        if ctor.is_null() || clear(env) {
            return;
        }
        let action = ((**env).NewStringUTF.unwrap())(env, c"android.intent.action.VIEW".as_ptr());
        let args = [ndk_sys::jvalue { l: action }, ndk_sys::jvalue { l: uri }];
        let intent = ((**env).NewObjectA.unwrap())(env, intent_cls, ctor, args.as_ptr());
        if intent.is_null() || clear(env) {
            return;
        }

        let acls = obj_class(env, ACTIVITY);
        let start = method(
            env,
            acls,
            b"startActivity\0",
            b"(Landroid/content/Intent;)V\0",
        );
        if start.is_null() || clear(env) {
            return;
        }
        ((**env).CallVoidMethod.unwrap())(env, ACTIVITY, start, intent);
        clear(env);
    }
}
