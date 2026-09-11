#[cfg(target_os = "macos")]
mod macos {
    use std::{
        cell::RefCell,
        collections::{HashMap, HashSet},
        mem, ptr,
        sync::Once,
    };

    use gpui::Window;
    use objc2::{
        ffi, msg_send,
        rc::Retained,
        runtime::{AnyObject, AnyProtocol, Imp, Sel},
        sel,
    };
    use objc2_foundation::NSString;
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};

    static INSTALL_TEXT_CONTENT: Once = Once::new();

    thread_local! {
        static CONTENT_TYPES: RefCell<HashMap<usize, Retained<NSString>>> =
            RefCell::new(HashMap::new());

        /// Windows whose raw `window_handle()` has no view, cached so the
        /// panicking probe below runs once per window, not once per frame.
        static HANDLE_UNAVAILABLE: RefCell<HashSet<gpui::WindowId>> = RefCell::new(HashSet::new());
    }

    pub fn set_text_content_type(window: &Window, content_type: Option<&str>) {
        let Some(view) = ns_view(window) else {
            return;
        };

        INSTALL_TEXT_CONTENT.call_once(|| install_text_content(view));
        if view
            .class()
            .instance_method(sel!(setContentType:))
            .is_none()
        {
            return;
        }

        let ns_content_type = content_type.map(NSString::from_str);
        let ns_content_type = ns_content_type.as_ref().map_or(ptr::null_mut(), |value| {
            Retained::as_ptr(value).cast_mut().cast::<AnyObject>()
        });

        unsafe {
            let _: () = msg_send![view, setContentType: ns_content_type];
        }
    }

    fn ns_view(window: &Window) -> Option<&AnyObject> {
        // A test window's raw `window_handle()` panics instead of returning
        // `Err`. Probe under `catch_unwind` once per window, caching failures;
        // this runs every focused frame. `Window::window_handle` is a separate,
        // non-panicking call, safe as the cache key.
        let window_id = window.window_handle().window_id();
        if HANDLE_UNAVAILABLE.with(|seen| seen.borrow().contains(&window_id)) {
            return None;
        }
        let handle = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            HasWindowHandle::window_handle(window)
        })) {
            Ok(Ok(handle)) => handle,
            // A panic is expected on a test window; on a real one it is a
            // genuine failure, so warn once instead of swallowing it.
            other => {
                let first = HANDLE_UNAVAILABLE.with(|seen| seen.borrow_mut().insert(window_id));
                if first && matches!(other, Err(_)) {
                    tracing::warn!(
                        ?window_id,
                        "window handle probe panicked; skipping native text-content wiring"
                    );
                }
                return None;
            }
        };
        let RawWindowHandle::AppKit(handle) = handle.as_raw() else {
            return None;
        };

        Some(unsafe { &*(handle.ns_view.as_ptr() as *const AnyObject) })
    }

    fn install_text_content(view: &AnyObject) {
        let class = view.class();
        let class = class as *const _ as *mut _;

        unsafe {
            let protocol = AnyProtocol::get(c"NSTextContent");
            if let Some(protocol) = protocol {
                ffi::class_addProtocol(class, protocol);
            }

            let content_type_imp: Imp = mem::transmute(
                content_type as unsafe extern "C-unwind" fn(&AnyObject, Sel) -> *mut AnyObject,
            );
            let set_content_type_imp: Imp = mem::transmute(
                set_content_type as unsafe extern "C-unwind" fn(&AnyObject, Sel, *mut AnyObject),
            );

            ffi::class_addMethod(class, sel!(contentType), content_type_imp, c"@@:".as_ptr());
            ffi::class_addMethod(
                class,
                sel!(setContentType:),
                set_content_type_imp,
                c"v@:@".as_ptr(),
            );
        }
    }

    unsafe extern "C-unwind" fn content_type(this: &AnyObject, _: Sel) -> *mut AnyObject {
        let key = this as *const _ as usize;
        CONTENT_TYPES.with(|content_types| {
            content_types
                .borrow()
                .get(&key)
                .map_or(ptr::null_mut(), |value| {
                    Retained::as_ptr(value).cast_mut().cast::<AnyObject>()
                })
        })
    }

    unsafe extern "C-unwind" fn set_content_type(this: &AnyObject, _: Sel, value: *mut AnyObject) {
        let key = this as *const _ as usize;
        CONTENT_TYPES.with(|content_types| {
            let mut content_types = content_types.borrow_mut();
            if value.is_null() {
                content_types.remove(&key);
            } else if let Some(value) = unsafe { Retained::retain(value.cast::<NSString>()) } {
                content_types.insert(key, value);
            }
        });
    }
}

#[cfg(target_os = "macos")]
pub use macos::set_text_content_type;

use gpui::SharedString;

/// Presentation-independent context-menu model produced by the editor.
#[derive(Default)]
pub struct NativeMenu {
    pub items: Vec<NativeMenuItem>,
}

pub enum NativeMenuItem {
    Separator,
    Action {
        label: SharedString,
        disabled: bool,
        action: Box<dyn gpui::Action>,
    },
}

impl NativeMenu {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn menu(self, label: impl Into<SharedString>, action: Box<dyn gpui::Action>) -> Self {
        self.menu_with_disabled(label, false, action)
    }

    pub fn menu_with_disabled(
        mut self,
        label: impl Into<SharedString>,
        disabled: bool,
        action: Box<dyn gpui::Action>,
    ) -> Self {
        self.items.push(NativeMenuItem::Action {
            label: label.into(),
            disabled,
            action,
        });
        self
    }

    pub fn separator(mut self) -> Self {
        self.items.push(NativeMenuItem::Separator);
        self
    }
}
