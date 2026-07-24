use glam::{Vec2, Vec3};
use sdl2::pixels::Color;
use sdl2::rect::{Point, Rect};
use sdl2::render::Canvas;
use sdl2::sys;
use sdl2::video::Window;
use sdl2::{EventPump, Sdl};
use std::mem::MaybeUninit;

use crate::backend::{project_to_screen, RenderFrame, RenderObject, RenderOverlayRect, RenderSprite, RenderTarget};
use crate::components::{Material, PrimitiveShape, Shape};
use crate::resources::RenderAssets;
use vetrace_core::{Engine, InputState};

mod background;
#[cfg(feature = "render_2d")]
mod canvas_2d;
mod draw_types;
mod input;
mod object_rasterizer;
mod overlays;
mod raster;
mod target;

use background::*;
#[cfg(feature = "render_2d")]
use canvas_2d::*;
use draw_types::*;
use input::*;
use object_rasterizer::*;
use overlays::*;
use raster::*;
pub use target::SdlRenderTarget;
