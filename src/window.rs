//! Windows window built directly on the Win32 API, without external libraries

use std::cell::RefCell;
use std::ffi::c_void;
use std::ptr::null_mut;

type Handle = *mut c_void;
type WParam = usize;
type LParam = isize;
type LResult = isize;
type WndProc = unsafe extern "system" fn(Handle, u32, WParam, LParam) -> LResult;

#[repr(C)]
struct WndClassW {
    style: u32,
    wnd_proc: Option<WndProc>,
    cls_extra: i32,
    wnd_extra: i32,
    instance: Handle,
    icon: Handle,
    cursor: Handle,
    background: Handle,
    menu_name: *const u16,
    class_name: *const u16,
}

#[repr(C)]
#[derive(Default)]
struct Point {
    x: i32,
    y: i32,
}

#[repr(C)]
struct Msg {
    hwnd: Handle,
    message: u32,
    wparam: WParam,
    lparam: LParam,
    time: u32,
    point: Point,
}

#[repr(C)]
#[derive(Default)]
struct Rect {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}

#[repr(C)]
struct BitmapInfoHeader {
    size: u32,
    width: i32,
    height: i32,
    planes: u16,
    bit_count: u16,
    compression: u32,
    size_image: u32,
    x_pels_per_meter: i32,
    y_pels_per_meter: i32,
    clr_used: u32,
    clr_important: u32,
}

#[link(name = "user32")]
unsafe extern "system" {
    fn RegisterClassW(class: *const WndClassW) -> u16;
    fn CreateWindowExW(
        ex_style: u32,
        class_name: *const u16,
        window_name: *const u16,
        style: u32,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        parent: Handle,
        menu: Handle,
        instance: Handle,
        param: *mut c_void,
    ) -> Handle;
    fn DefWindowProcW(hwnd: Handle, msg: u32, wparam: WParam, lparam: LParam) -> LResult;
    fn PeekMessageW(msg: *mut Msg, hwnd: Handle, min: u32, max: u32, remove: u32) -> i32;
    fn TranslateMessage(msg: *const Msg) -> i32;
    fn DispatchMessageW(msg: *const Msg) -> LResult;
    fn DestroyWindow(hwnd: Handle) -> i32;
    fn SetWindowTextW(hwnd: Handle, text: *const u16) -> i32;
    fn AdjustWindowRect(rect: *mut Rect, style: u32, menu: i32) -> i32;
    fn GetClientRect(hwnd: Handle, rect: *mut Rect) -> i32;
    fn GetDC(hwnd: Handle) -> Handle;
    fn ReleaseDC(hwnd: Handle, dc: Handle) -> i32;
    fn LoadCursorW(instance: Handle, name: *const u16) -> Handle;
    fn SetCapture(hwnd: Handle) -> Handle;
    fn ReleaseCapture() -> i32;
}

#[link(name = "gdi32")]
unsafe extern "system" {
    fn StretchDIBits(
        dc: Handle,
        x_dest: i32,
        y_dest: i32,
        dest_width: i32,
        dest_height: i32,
        x_src: i32,
        y_src: i32,
        src_width: i32,
        src_height: i32,
        bits: *const c_void,
        info: *const BitmapInfoHeader,
        usage: u32,
        rop: u32,
    ) -> i32;
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetModuleHandleW(name: *const u16) -> Handle;
}

const WS_OVERLAPPEDWINDOW: u32 = 0x00CF_0000;
const WS_VISIBLE: u32 = 0x1000_0000;
const CW_USEDEFAULT: i32 = 0x8000_0000_u32 as i32;
const PM_REMOVE: u32 = 1;
const IDC_ARROW: usize = 32512;
const SRCCOPY: u32 = 0x00CC_0020;
const DIB_RGB_COLORS: u32 = 0;
const BI_RGB: u32 = 0;

const WM_DESTROY: u32 = 0x0002;
const WM_CLOSE: u32 = 0x0010;
const WM_KILLFOCUS: u32 = 0x0008;
const WM_KEYDOWN: u32 = 0x0100;
const WM_KEYUP: u32 = 0x0101;
const WM_MOUSEMOVE: u32 = 0x0200;
const WM_LBUTTONDOWN: u32 = 0x0201;
const WM_LBUTTONUP: u32 = 0x0202;
const WM_MOUSEWHEEL: u32 = 0x020A;
const WHEEL_DELTA: f32 = 120.0;

/// Keys used by the program, with their Windows virtual key codes
#[derive(Clone, Copy)]
pub enum Key {
    Left = 0x25,
    Up = 0x26,
    Right = 0x27,
    Down = 0x28,
    W = 0x57,
    S = 0x53,
    N = 0x4E,
    P = 0x50,
    R = 0x52,
    T = 0x54,
    Escape = 0x1B,
}

/// What Windows sends through its messages
struct Input {
    keys: [bool; 256],
    mouse: Option<(f32, f32)>,
    mouse_down: bool,
    scroll: f32,
    closed: bool,
}

thread_local! {
    static INPUT: RefCell<Input> = const {
        RefCell::new(Input {
            keys: [false; 256],
            mouse: None,
            mouse_down: false,
            scroll: 0.0,
            closed: false,
        })
    };
}

unsafe extern "system" fn window_proc(
    hwnd: Handle,
    msg: u32,
    wparam: WParam,
    lparam: LParam,
) -> LResult {
    // Mouse coordinates come in the two signed halves of lparam
    let low = (lparam & 0xFFFF) as i16 as f32;
    let high = ((lparam >> 16) & 0xFFFF) as i16 as f32;

    let handled = INPUT.with_borrow_mut(|input| {
        match msg {
            WM_KEYDOWN | WM_KEYUP => input.keys[wparam & 0xFF] = msg == WM_KEYDOWN,
            // When the window loses focus no "key up" arrives: release every key
            WM_KILLFOCUS => {
                input.keys = [false; 256];
                input.mouse_down = false;
            }
            WM_MOUSEMOVE => input.mouse = Some((low, high)),
            WM_LBUTTONDOWN => input.mouse_down = true,
            WM_LBUTTONUP => input.mouse_down = false,
            WM_MOUSEWHEEL => input.scroll += ((wparam >> 16) & 0xFFFF) as i16 as f32 / WHEEL_DELTA,
            WM_CLOSE | WM_DESTROY => input.closed = true,
            _ => return false,
        }
        true
    });

    unsafe {
        match msg {
            // While dragging, the mouse keeps reporting even outside the window
            WM_LBUTTONDOWN => {
                SetCapture(hwnd);
            }
            WM_LBUTTONUP => {
                ReleaseCapture();
            }
            WM_CLOSE => {
                DestroyWindow(hwnd);
            }
            _ => {}
        }

        if handled { 0 } else { DefWindowProcW(hwnd, msg, wparam, lparam) }
    }
}

/// Null-terminated UTF-16 text, which is what Win32 expects
fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(Some(0)).collect()
}

pub struct Window {
    hwnd: Handle,
    /// Mouse wheel accumulated since the last `update_with_buffer`
    scroll: f32,
}

impl Window {
    /// Opens a window whose drawing area is `width` x `height` pixels
    pub fn new(title: &str, width: usize, height: usize) -> Result<Window, String> {
        let class_name = wide("RaytracerWindow");
        let title = wide(title);

        unsafe {
            let instance = GetModuleHandleW(null_mut());
            let class = WndClassW {
                style: 0,
                wnd_proc: Some(window_proc),
                cls_extra: 0,
                wnd_extra: 0,
                instance,
                icon: null_mut(),
                cursor: LoadCursorW(null_mut(), IDC_ARROW as *const u16),
                background: null_mut(),
                menu_name: null_mut(),
                class_name: class_name.as_ptr(),
            };
            RegisterClassW(&class);

            // The size passed to Windows includes borders and the title bar
            let style = WS_OVERLAPPEDWINDOW | WS_VISIBLE;
            let mut rect = Rect {
                right: width as i32,
                bottom: height as i32,
                ..Rect::default()
            };
            AdjustWindowRect(&mut rect, style, 0);

            let hwnd = CreateWindowExW(
                0,
                class_name.as_ptr(),
                title.as_ptr(),
                style,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                rect.right - rect.left,
                rect.bottom - rect.top,
                null_mut(),
                null_mut(),
                instance,
                null_mut(),
            );
            if hwnd.is_null() {
                return Err("CreateWindowExW fallo".into());
            }

            Ok(Window { hwnd, scroll: 0.0 })
        }
    }

    pub fn is_open(&self) -> bool {
        !INPUT.with_borrow(|input| input.closed)
    }

    pub fn is_key_down(&self, key: Key) -> bool {
        INPUT.with_borrow(|input| input.keys[key as usize])
    }

    /// Changes the title bar text
    pub fn set_title(&self, title: &str) {
        let title = wide(title);
        unsafe {
            SetWindowTextW(self.hwnd, title.as_ptr());
        }
    }

    /// Mouse position inside the window, in pixels
    pub fn mouse_pos(&self) -> Option<(f32, f32)> {
        INPUT.with_borrow(|input| input.mouse)
    }

    pub fn is_mouse_down(&self) -> bool {
        INPUT.with_borrow(|input| input.mouse_down)
    }

    /// How much the wheel turned since the previous frame: positive = forward
    pub fn scroll_wheel(&self) -> f32 {
        self.scroll
    }

    /// Shows the buffer stretched to the window and processes Windows messages
    pub fn update_with_buffer(&mut self, buffer: &[u32], width: usize, height: usize) {
        assert_eq!(buffer.len(), width * height, "el buffer no mide width x height");

        unsafe {
            let mut msg: Msg = std::mem::zeroed();
            while PeekMessageW(&mut msg, null_mut(), 0, 0, PM_REMOVE) != 0 {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }

        self.scroll = INPUT.with_borrow_mut(|input| std::mem::take(&mut input.scroll));
        if !self.is_open() {
            return;
        }

        // 0xRRGGBB in memory is B, G, R, 0: exactly what Windows expects
        let info = BitmapInfoHeader {
            size: std::mem::size_of::<BitmapInfoHeader>() as u32,
            width: width as i32,
            height: -(height as i32),
            planes: 1,
            bit_count: 32,
            compression: BI_RGB,
            size_image: 0,
            x_pels_per_meter: 0,
            y_pels_per_meter: 0,
            clr_used: 0,
            clr_important: 0,
        };

        unsafe {
            let mut client = Rect::default();
            GetClientRect(self.hwnd, &mut client);

            let dc = GetDC(self.hwnd);
            StretchDIBits(
                dc,
                0,
                0,
                client.right,
                client.bottom,
                0,
                0,
                width as i32,
                height as i32,
                buffer.as_ptr() as *const c_void,
                &info,
                DIB_RGB_COLORS,
                SRCCOPY,
            );
            ReleaseDC(self.hwnd, dc);
        }
    }
}
