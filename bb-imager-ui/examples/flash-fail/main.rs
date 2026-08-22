use bb_imager_ui::{Message, flash_fail};

struct State(flash_fail::State);

impl State {
    fn new() -> (Self, iced::Task<Message<()>>) {
        let res = State(flash_fail::State {
            has_customization: true,
            reason: "Fail Reason for Testing".into(),
            logs: iced::widget::text_editor::Content::with_text(LOGS.trim()),
        });

        (res, iced::Task::none())
    }
}

fn main() {
    let app = iced::application(
        State::new,
        |s: &mut State, msg| {
            match msg {
                Message::EditorEvent(evt) => match evt {
                    iced::widget::text_editor::Action::Edit(_) => {}
                    _ => s.0.logs.perform(evt),
                },
                Message::CopyToClipboard => return iced::clipboard::write(s.0.logs.text()),
                _ => {}
            }
            iced::Task::none()
        },
        view,
    );
    bb_imager_ui::application(app).run().unwrap()
}

fn view(s: &State) -> iced::Element<'_, Message<()>> {
    flash_fail::view(&s.0)
}

const LOGS: &str = r#"
2026-08-01T06:22:17.844654Z  INFO bb_imager_gui: Resolved GUI keymap: "us"
2026-08-01T06:22:17.858806Z  INFO bb_imager_gui::db: DB Path: /tmp/.tmpETKiuI
2026-08-01T06:22:17.860426Z  INFO iced_winit: System theme: Dark
2026-08-01T06:22:17.860737Z  INFO iced_winit: Window attributes for id `Id(
    1,
)`: WindowAttributes {
    inner_size: Some(
        Logical(
            LogicalSize {
                width: 680.0,
                height: 450.0,
            },
        ),
    ),
    min_inner_size: Some(
        Logical(
            LogicalSize {
                width: 680.0,
                height: 450.0,
            },
        ),
    ),
    max_inner_size: None,
    position: None,
    resizable: true,
    enabled_buttons: WindowButtons(
        CLOSE | MINIMIZE | MAXIMIZE,
    ),
    title: "BeagleBoard Imager v1.0.13",
    maximized: false,
    visible: false,
    transparent: false,
    blur: false,
    decorations: true,
    window_icon: None,
    preferred_theme: None,
    resize_increments: None,
    content_protected: false,
    window_level: Normal,
    active: true,
    cursor: Icon(
        Default,
    ),
    parent_window: None,
    fullscreen: None,
    platform_specific: PlatformSpecificWindowAttributes {
        name: Some(
            ApplicationName {
                general: "",
                instance: "",
            },
        ),
        activation_token: None,
        x11: X11WindowAttributes {
            visual_id: None,
            screen_id: None,
            base_size: None,
            override_redirect: false,
            x11_window_types: [
                Normal,
            ],
            embed_window: None,
        },
    },
}
2026-08-01T06:22:17.880542Z  WARN wgpu_hal::vulkan::instance: Unable to find extension: VK_EXT_physical_device_drm
2026-08-01T06:22:17.888823Z  INFO wgpu_hal::gles::egl: Using Wayland platform
2026-08-01T06:22:17.895840Z  INFO iced_wgpu::window::compositor: Settings {
    present_mode: AutoVsync,
    backends: Backends(
        NOOP | VULKAN | GL | METAL | DX12 | BROWSER_WEBGPU,
    ),
    default_font: Font {
        family: Name(
            "Nunito",
        ),
        weight: Normal,
        stretch: Normal,
        style: Normal,
    },
    default_text_size: Pixels(
        16.0,
    ),
    antialiasing: Some(
        MSAAx4,
    ),
}
2026-08-01T06:22:17.902983Z  INFO iced_wgpu::window::compositor: Available adapters: [
    AdapterInfo {
        name: "AMD Radeon RX 9060 XT (RADV GFX1200)",
        vendor: 4098,
        device: 30096,
        device_type: DiscreteGpu,
        driver: "radv",
        driver_info: "Mesa 26.1.5 (git-6a02618ccf)",
        backend: Vulkan,
    },
    AdapterInfo {
        name: "AMD Radeon RX 9060 XT (RADV GFX1200)",
        vendor: 4098,
        device: 30096,
        device_type: DiscreteGpu,
        driver: "radv",
        driver_info: "Mesa 26.1.5 (git-6a02618ccf)",
        backend: Vulkan,
    },
    AdapterInfo {
        name: "AMD Ryzen 7 9700X 8-Core Processor (RADV RAPHAEL_MENDOCINO)",
        vendor: 4098,
        device: 5056,
        device_type: IntegratedGpu,
        driver: "radv",
        driver_info: "Mesa 26.1.5 (git-6a02618ccf)",
        backend: Vulkan,
    },
    AdapterInfo {
        name: "AMD Ryzen 7 9700X 8-Core Processor (RADV RAPHAEL_MENDOCINO)",
        vendor: 4098,
        device: 5056,
        device_type: IntegratedGpu,
        driver: "radv",
        driver_info: "Mesa 26.1.5 (git-6a02618ccf)",
        backend: Vulkan,
    },
    AdapterInfo {
        name: "llvmpipe (LLVM 21.1.8, 256 bits)",
        vendor: 65541,
        device: 0,
        device_type: Cpu,
        driver: "llvmpipe",
        driver_info: "Mesa 26.1.5 (git-6a02618ccf) (LLVM 21.1.8)",
        backend: Vulkan,
    },
    AdapterInfo {
        name: "llvmpipe (LLVM 21.1.8, 256 bits)",
        vendor: 65541,
        device: 0,
        device_type: Cpu,
        driver: "llvmpipe",
        driver_info: "Mesa 26.1.5 (git-6a02618ccf) (LLVM 21.1.8)",
        backend: Vulkan,
    },
    AdapterInfo {
        name: "AMD Radeon RX 9060 XT (radeonsi, gfx1200, ACO, DRM 3.64, 7.1.5-201.fc44.x86_64)",
        vendor: 4098,
        device: 0,
        device_type: Other,
        driver: "",
        driver_info: "4.6 (Core Profile) Mesa 26.1.5 (git-6a02618ccf)",
        backend: Gl,
    },
]
2026-08-01T06:22:17.903029Z  WARN wgpu_hal::gles::egl: Re-initializing Gles context due to Wayland window
2026-08-01T06:22:17.910141Z  INFO iced_wgpu::window::compositor: Selected: AdapterInfo {
    name: "AMD Ryzen 7 9700X 8-Core Processor (RADV RAPHAEL_MENDOCINO)",
    vendor: 4098,
    device: 5056,
    device_type: IntegratedGpu,
    driver: "radv",
    driver_info: "Mesa 26.1.5 (git-6a02618ccf)",
    backend: Vulkan,
}
2026-08-01T06:22:17.910474Z  INFO iced_wgpu::window::compositor: Available formats: Copied {
    it: Iter(
        [
            Rgba8UnormSrgb,
            Bgra8UnormSrgb,
            Rgb10a2Unorm,
            Rgba8Unorm,
            Bgra8Unorm,
            Rgba16Unorm,
            Rgba16Float,
        ],
    ),
}
2026-08-01T06:22:17.910485Z  INFO iced_wgpu::window::compositor: Available alpha modes: [
    Opaque,
    PreMultiplied,
]
2
"#;
