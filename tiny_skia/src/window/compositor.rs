use crate::core::{Color, Rectangle, Size};
use crate::graphics::compositor::{self, Information};
use crate::graphics::damage;
use crate::graphics::{Error, Viewport};
use crate::{Backend, Primitive, Renderer, Settings};

use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};
use std::marker::PhantomData;
use std::num::NonZeroU32;

pub struct Compositor<Theme> {
    settings: Settings,
    _theme: PhantomData<Theme>,
}

pub struct Surface {
    window: softbuffer::Surface,
    buffer: Vec<u32>,
    clip_mask: tiny_skia::Mask,
    primitives: Option<Vec<Primitive>>,
    background_color: Color,
}

impl<Theme> crate::graphics::Compositor for Compositor<Theme> {
    type Settings = Settings;
    type Renderer = Renderer<Theme>;
    type Surface = Surface;

    fn new<W: HasRawWindowHandle + HasRawDisplayHandle>(
        settings: Self::Settings,
        _compatible_window: Option<&W>,
    ) -> Result<Self, Error> {
        Ok(new(settings))
    }

    fn create_renderer(&self) -> Self::Renderer {
        Renderer::new(
            Backend::new(),
            self.settings.default_font,
            self.settings.default_text_size,
        )
    }

    fn create_surface<W: HasRawWindowHandle + HasRawDisplayHandle>(
        &mut self,
        window: &W,
        width: u32,
        height: u32,
    ) -> Surface {
        #[allow(unsafe_code)]
        let window = unsafe {
            softbuffer::Surface::new(
                &softbuffer::Context::new(window)
                    .expect("Failed to create softbuffer context"),
                window,
            )
        }
        .expect("Create softbuffer for window");

        Surface {
            window,
            buffer: vec![0; width as usize * height as usize],
            clip_mask: tiny_skia::Mask::new(width, height)
                .expect("Create clip mask"),
            primitives: None,
            background_color: Color::BLACK,
        }
    }

    fn configure_surface(
        &mut self,
        surface: &mut Surface,
        width: u32,
        height: u32,
    ) {
        surface.buffer.resize((width * height) as usize, 0);
        surface.clip_mask =
            tiny_skia::Mask::new(width, height).expect("Create clip mask");
        surface.primitives = None;
    }

    fn fetch_information(&self) -> Information {
        Information {
            adapter: String::from("CPU"),
            backend: String::from("tiny-skia"),
        }
    }

    fn present<T: AsRef<str>>(
        &mut self,
        renderer: &mut Self::Renderer,
        surface: &mut Self::Surface,
        viewport: &Viewport,
        background_color: Color,
        overlay: &[T],
    ) -> Result<(), compositor::SurfaceError> {
        renderer.with_primitives(|backend, primitives| {
            present(
                backend,
                surface,
                primitives,
                viewport,
                background_color,
                overlay,
            )
        })
    }

    fn screenshot<T: AsRef<str>>(
        &mut self,
        renderer: &mut Self::Renderer,
        surface: &mut Self::Surface,
        viewport: &Viewport,
        background_color: Color,
        overlay: &[T],
    ) -> Vec<u8> {
        renderer.with_primitives(|backend, primitives| {
            screenshot(
                surface,
                backend,
                primitives,
                viewport,
                background_color,
                overlay,
            )
        })
    }
}

pub fn new<Theme>(settings: Settings) -> Compositor<Theme> {
    Compositor {
        settings,
        _theme: PhantomData,
    }
}

pub fn present<T: AsRef<str>>(
    backend: &mut Backend,
    surface: &mut Surface,
    primitives: &[Primitive],
    viewport: &Viewport,
    background_color: Color,
    overlay: &[T],
) -> Result<(), compositor::SurfaceError> {
    let physical_size = viewport.physical_size();
    let scale_factor = viewport.scale_factor() as f32;

    let mut pixels = tiny_skia::PixmapMut::from_bytes(
        bytemuck::cast_slice_mut(&mut surface.buffer),
        physical_size.width,
        physical_size.height,
    )
    .expect("Create pixel map");

    let damage = surface
        .primitives
        .as_deref()
        .and_then(|last_primitives| {
            (surface.background_color == background_color)
                .then(|| damage::list(last_primitives, primitives))
        })
        .unwrap_or_else(|| vec![Rectangle::with_size(viewport.logical_size())]);

    if damage.is_empty() {
        return Ok(());
    }

    surface.primitives = Some(primitives.to_vec());
    surface.background_color = background_color;

    let damage = damage::group(damage, scale_factor, physical_size);

    backend.draw(
        &mut pixels,
        &mut surface.clip_mask,
        primitives,
        viewport,
        &damage,
        background_color,
        overlay,
    );

    surface
        .window
        .resize(
            NonZeroU32::new(physical_size.width)
                .ok_or(compositor::SurfaceError::InvalidDimensions)?,
            NonZeroU32::new(physical_size.height)
                .ok_or(compositor::SurfaceError::InvalidDimensions)?,
        )
        .map_err(|_| compositor::SurfaceError::Resize)?;
    if let Ok(mut b) = surface.window.buffer_mut() {
        let damage = damage
            .iter()
            .filter_map(|r| {
                Some(softbuffer::Rect {
                    x: r.x as u32,
                    y: r.y as u32,
                    width: NonZeroU32::new(r.width as u32)?,
                    height: NonZeroU32::new(r.height as u32)?,
                })
            })
            .collect::<Vec<_>>();
        b.copy_from_slice(&surface.buffer);
        b.present_with_damage(&damage)
            .map_err(|e| compositor::SurfaceError::Present(e.to_string()))?;
    }

    Ok(())
}

pub fn screenshot<T: AsRef<str>>(
    surface: &mut Surface,
    backend: &mut Backend,
    primitives: &[Primitive],
    viewport: &Viewport,
    background_color: Color,
    overlay: &[T],
) -> Vec<u8> {
    let size = viewport.physical_size();

    let mut offscreen_buffer: Vec<u32> =
        vec![0; size.width as usize * size.height as usize];

    backend.draw(
        &mut tiny_skia::PixmapMut::from_bytes(
            bytemuck::cast_slice_mut(&mut offscreen_buffer),
            size.width,
            size.height,
        )
        .expect("Create offscreen pixel map"),
        &mut surface.clip_mask,
        primitives,
        viewport,
        &[Rectangle::with_size(Size::new(
            size.width as f32,
            size.height as f32,
        ))],
        background_color,
        overlay,
    );

    offscreen_buffer.iter().fold(
        Vec::with_capacity(offscreen_buffer.len() * 4),
        |mut acc, pixel| {
            const A_MASK: u32 = 0xFF_00_00_00;
            const R_MASK: u32 = 0x00_FF_00_00;
            const G_MASK: u32 = 0x00_00_FF_00;
            const B_MASK: u32 = 0x00_00_00_FF;

            let a = ((A_MASK & pixel) >> 24) as u8;
            let r = ((R_MASK & pixel) >> 16) as u8;
            let g = ((G_MASK & pixel) >> 8) as u8;
            let b = (B_MASK & pixel) as u8;

            acc.extend([r, g, b, a]);
            acc
        },
    )
}
