//! Render element wrapper and composed-frame output.

use smithay::backend::renderer::element::memory::MemoryRenderBufferRenderElement;
use smithay::backend::renderer::element::solid::SolidColorRenderElement;
use smithay::backend::renderer::element::surface::WaylandSurfaceRenderElement;
use smithay::backend::renderer::element::{Element, Id, Kind, RenderElement, UnderlyingStorage};
use smithay::backend::renderer::gles::{GlesError, GlesFrame, GlesRenderer};
use smithay::backend::renderer::utils::{CommitCounter, DamageSet, OpaqueRegions};
use smithay::backend::renderer::Color32F;
use smithay::utils::{Buffer as BufferCoords, Physical, Point, Rectangle, Scale, Transform};

pub const BACKGROUND: Color32F = Color32F::new(0.011_764_7, 0.098_039_2, 0.133_333_3, 1.0);

/// One composited element. `DrmCompositor` and the winit draw loop both need a
/// single concrete element type, so surface, solid and memory elements are
/// wrapped in this enum.
pub enum AetherRenderElement {
    Surface(WaylandSurfaceRenderElement<GlesRenderer>),
    Solid(SolidColorRenderElement),
    Memory(MemoryRenderBufferRenderElement<GlesRenderer>),
}

impl From<WaylandSurfaceRenderElement<GlesRenderer>> for AetherRenderElement {
    fn from(element: WaylandSurfaceRenderElement<GlesRenderer>) -> Self {
        Self::Surface(element)
    }
}

impl From<SolidColorRenderElement> for AetherRenderElement {
    fn from(element: SolidColorRenderElement) -> Self {
        Self::Solid(element)
    }
}

impl From<MemoryRenderBufferRenderElement<GlesRenderer>> for AetherRenderElement {
    fn from(element: MemoryRenderBufferRenderElement<GlesRenderer>) -> Self {
        Self::Memory(element)
    }
}

impl Element for AetherRenderElement {
    fn id(&self) -> &Id {
        match self {
            Self::Surface(element) => element.id(),
            Self::Solid(element) => element.id(),
            Self::Memory(element) => element.id(),
        }
    }

    fn current_commit(&self) -> CommitCounter {
        match self {
            Self::Surface(element) => element.current_commit(),
            Self::Solid(element) => element.current_commit(),
            Self::Memory(element) => element.current_commit(),
        }
    }

    fn location(&self, scale: Scale<f64>) -> Point<i32, Physical> {
        match self {
            Self::Surface(element) => element.location(scale),
            Self::Solid(element) => element.location(scale),
            Self::Memory(element) => element.location(scale),
        }
    }

    fn src(&self) -> Rectangle<f64, BufferCoords> {
        match self {
            Self::Surface(element) => element.src(),
            Self::Solid(element) => element.src(),
            Self::Memory(element) => element.src(),
        }
    }

    fn transform(&self) -> Transform {
        match self {
            Self::Surface(element) => element.transform(),
            Self::Solid(element) => element.transform(),
            Self::Memory(element) => element.transform(),
        }
    }

    fn geometry(&self, scale: Scale<f64>) -> Rectangle<i32, Physical> {
        match self {
            Self::Surface(element) => element.geometry(scale),
            Self::Solid(element) => element.geometry(scale),
            Self::Memory(element) => element.geometry(scale),
        }
    }

    fn damage_since(
        &self,
        scale: Scale<f64>,
        commit: Option<CommitCounter>,
    ) -> DamageSet<i32, Physical> {
        match self {
            Self::Surface(element) => Element::damage_since(element, scale, commit),
            Self::Solid(element) => Element::damage_since(element, scale, commit),
            Self::Memory(element) => Element::damage_since(element, scale, commit),
        }
    }

    fn opaque_regions(&self, scale: Scale<f64>) -> OpaqueRegions<i32, Physical> {
        match self {
            Self::Surface(element) => Element::opaque_regions(element, scale),
            Self::Solid(element) => Element::opaque_regions(element, scale),
            Self::Memory(element) => Element::opaque_regions(element, scale),
        }
    }

    fn alpha(&self) -> f32 {
        match self {
            Self::Surface(element) => element.alpha(),
            Self::Solid(element) => element.alpha(),
            Self::Memory(element) => element.alpha(),
        }
    }

    fn kind(&self) -> Kind {
        match self {
            Self::Surface(element) => element.kind(),
            Self::Solid(element) => element.kind(),
            Self::Memory(element) => element.kind(),
        }
    }
}

impl RenderElement<GlesRenderer> for AetherRenderElement {
    fn draw(
        &self,
        frame: &mut GlesFrame<'_, '_>,
        src: Rectangle<f64, BufferCoords>,
        dst: Rectangle<i32, Physical>,
        damage: &[Rectangle<i32, Physical>],
        opaque_regions: &[Rectangle<i32, Physical>],
    ) -> Result<(), GlesError> {
        match self {
            Self::Surface(element) => {
                <WaylandSurfaceRenderElement<GlesRenderer> as RenderElement<GlesRenderer>>::draw(
                    element,
                    frame,
                    src,
                    dst,
                    damage,
                    opaque_regions,
                )
            }
            Self::Solid(element) => <SolidColorRenderElement as RenderElement<GlesRenderer>>::draw(
                element,
                frame,
                src,
                dst,
                damage,
                opaque_regions,
            ),
            Self::Memory(element) => {
                <MemoryRenderBufferRenderElement<GlesRenderer> as RenderElement<GlesRenderer>>::draw(
                    element,
                    frame,
                    src,
                    dst,
                    damage,
                    opaque_regions,
                )
            }
        }
    }

    fn underlying_storage(&self, renderer: &mut GlesRenderer) -> Option<UnderlyingStorage<'_>> {
        match self {
            Self::Surface(element) => element.underlying_storage(renderer),
            Self::Solid(element) => element.underlying_storage(renderer),
            Self::Memory(element) => element.underlying_storage(renderer),
        }
    }
}

/// A composed frame in topmost-first order, matching `DrmCompositor` and
/// `draw_render_elements` expectations.
pub struct ComposedFrame {
    pub elements: Vec<AetherRenderElement>,
    pub clear: Color32F,
    /// Damage in this output's local physical coordinates.
    pub damage: Vec<Rectangle<i32, Physical>>,
}
