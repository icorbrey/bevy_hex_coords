//! # `bevy_hex_coords`
//!
//! Compact coordinate types and Bevy integration for working with pointy-top
//! hexagonal grids.
//!
//! ## Coordinate model
//!
//! [`HexCoord`] uses axial coordinates (`q`, `r`) rather than storing a full
//! cube coordinate (`q`, `r`, `s`). This works because valid hex coordinates
//! satisfy `q + r + s = 0`, so `s` can always be reconstructed as
//! `-(q + r)`. The result is the ergonomics of cube-style hex math without
//! paying to store redundant data per tile.
//!
//! ## Canonical edges and vertices
//!
//! Hex edges and vertices are also stored in compact canonical forms:
//!
//! - a [`HexEdge`] is `HexCoord + HexEdgeDiscriminator`
//! - a [`HexVert`] is `HexCoord + HexVertDiscriminator`
//!
//! Instead of storing every incident hex for a shared feature, this crate
//! assigns each edge or vertex to exactly one owning hex coordinate. That keeps
//! shared features stable and small to pass around, hash, and attach to ECS
//! entities.
//!
//! ## Worldspace convention
//!
//! All worldspace calculations assume pointy-top hexagons with the hex center
//! at the local origin. [`HexUnitSize`] is defined in terms of the inradius,
//! meaning a unit-sized hex has an edge-to-edge diameter of `2.0`. Translation
//! vectors and feature offsets are derived from that convention, then scaled by
//! the requested unit size.
//!
//! ```text
//!  __---A---__
//! F           B
//! |           |
//! |           |
//! E___     ___C
//!     ‾‾D‾‾
//! ```

use bevy::prelude::*;

/// Bevy plugin for optional transform auto-generation on hex grid entities.
///
/// When [`Self::auto_attach_transforms`] is enabled, the plugin listens for
/// newly added coordinate marker components and inserts a matching
/// [`Transform`]:
///
/// - [`HexCoord`] -> hex-center transform
/// - [`HexEdgeDiscriminator`] + [`HexCoord`] -> edge transform
/// - [`HexVertDiscriminator`] + [`HexCoord`] -> vertex transform
///
/// This is useful when hex tiles, edges, or vertices are modeled as separate
/// entities and their transforms should always be derived from grid state.
pub struct HexCoordsPlugin {
    pub auto_attach_transforms: bool,
}

impl Default for HexCoordsPlugin {
    fn default() -> Self {
        Self {
            auto_attach_transforms: false,
        }
    }
}

impl Plugin for HexCoordsPlugin {
    fn build(&self, app: &mut App) {
        if self.auto_attach_transforms {
            app.add_observer(attach_hex_transforms)
                .add_observer(attach_edge_transforms)
                .add_observer(attach_vert_transforms);
        }
    }
}

fn attach_hex_transforms(
    trigger: On<Add, HexCoord>,
    hex_coords: Query<
        (&HexCoord, &HexUnitSize),
        (Without<HexEdgeDiscriminator>, Without<HexVertDiscriminator>),
    >,
    mut commands: Commands,
) {
    let Ok((coord, unit_size)) = hex_coords.get(trigger.entity) else {
        return;
    };

    commands
        .entity(trigger.entity)
        .insert(coord.to_transform(**unit_size));
}

fn attach_edge_transforms(
    trigger: On<Add, HexEdgeDiscriminator>,
    hex_edges: Query<
        (&HexCoord, &HexEdgeDiscriminator, &HexUnitSize),
        Without<HexVertDiscriminator>,
    >,
    mut commands: Commands,
) {
    let Ok((coord, edge, unit_size)) = hex_edges.get(trigger.entity) else {
        return;
    };

    commands
        .entity(trigger.entity)
        .insert((*coord, *edge).to_transform(**unit_size));
}

fn attach_vert_transforms(
    trigger: On<Add, HexVertDiscriminator>,
    hex_verts: Query<
        (&HexCoord, &HexVertDiscriminator, &HexUnitSize),
        Without<HexEdgeDiscriminator>,
    >,
    mut commands: Commands,
) {
    let Ok((coord, vert, unit_size)) = hex_verts.get(trigger.entity) else {
        return;
    };

    commands
        .entity(trigger.entity)
        .insert((*coord, *vert).to_transform(**unit_size));
}

#[derive(Clone, Copy, Component, Debug, PartialEq, Deref, DerefMut)]
pub struct HexUnitSize(pub f32);

impl HexUnitSize {
    /// Constructs a size directly from the hex inradius.
    ///
    /// The inradius is the distance from the center of the hex to the midpoint
    /// of any edge. In this crate it is the fundamental unit used for all
    /// worldspace derivations.
    pub fn inradius(r: f32) -> Self {
        Self(r)
    }

    /// Constructs a size from the hex circumradius.
    ///
    /// For a regular pointy-top hexagon:
    ///
    /// `inradius = circumradius * sqrt(3) / 2`
    pub fn circumradius(c: f32) -> Self {
        let inradius = c * SQRT_3 / 2.0;
        Self(inradius)
    }
}

impl Default for HexUnitSize {
    fn default() -> Self {
        Self(1.0)
    }
}

/// Axial coordinates for referencing hex tiles on a pointy-top grid.
///
/// This stores the `q` and `r` axes and omits the redundant cube axis `s`,
/// which can always be recovered as `-(q + r)`.
#[derive(Clone, Copy, Component, Debug, Default, PartialEq, Eq, Hash)]
#[require(HexUnitSize)]
pub struct HexCoord {
    /// Axial `q` component.
    pub q: i32,

    /// Axial `r` component.
    pub r: i32,
    // The third axis of hex alignment. Equivalent to -1 * (q + r), excluded for
    // size optimization.
    // pub s: i32,
}

/// Canonical representation of a shared edge on the grid.
///
/// Some intuitive edges, such as [`HexCoord::bottom_left_edge`], are owned by a
/// neighboring hex instead of the receiver. That canonicalization guarantees
/// every physical edge has a single stable identifier.
pub type HexEdge = (HexCoord, HexEdgeDiscriminator);

/// Canonical representation of a shared vertex on the grid.
///
/// As with [`HexEdge`], some apparent "local" vertices are represented by a
/// neighboring hex so that each physical vertex has exactly one identifier.
pub type HexVert = (HexCoord, HexVertDiscriminator);

const SQRT_3: f32 = 1.732_050_8;

/// Half of a unit hexagon's edge length when size is measured by inradius.
const WORLDSPACE_EDGE_MIDLENGTH: f32 = 1.0 / (2.0 * SQRT_3);

/// Distance from the hex center to a vertex for a unit inradius hex.
const WORLDSPACE_VERT_OFFSET: f32 = 1.0 / SQRT_3;

impl HexCoord {
    pub const ZERO: HexCoord = HexCoord::new(0, 0);
    pub const ONE: HexCoord = HexCoord::new(1, 1);

    pub const LEFT: HexCoord = HexCoord::new(1, 0);
    pub const RIGHT: HexCoord = HexCoord::new(-1, 0);
    pub const UP_LEFT: HexCoord = HexCoord::new(0, 1);
    pub const UP_RIGHT: HexCoord = HexCoord::new(-1, 1);
    pub const DOWN_LEFT: HexCoord = HexCoord::new(1, -1);
    pub const DOWN_RIGHT: HexCoord = HexCoord::new(0, -1);

    #[inline(always)]
    pub const fn new(q: i32, r: i32) -> Self {
        Self { q, r }
    }

    #[inline(always)]
    const fn add(&self, rhs: Self) -> Self {
        Self::new(self.q + rhs.q, self.r + rhs.r)
    }

    #[inline(always)]
    const fn sub(&self, rhs: Self) -> Self {
        Self::new(self.q - rhs.q, self.r - rhs.r)
    }

    #[inline(always)]
    const fn mul(&self, rhs: i32) -> Self {
        Self::new(self.q * rhs, self.r * rhs)
    }

    #[inline(always)]
    const fn neg(&self) -> Self {
        self.mul(-1)
    }

    /// S axis navigation (corresponds to `r` component)
    const WORLDSPACE_S_UNIT: Vec3 = Vec3::new(
        -0.5,
        WORLDSPACE_VERT_OFFSET + WORLDSPACE_EDGE_MIDLENGTH,
        0.0,
    );

    /// Q axis navigation (corresponds to `q` component)
    const WORLDSPACE_Q_UNIT: Vec3 = Vec3::new(-1.0, 0.0, 0.0);

    /// Converts this axial coordinate into a worldspace translation.
    ///
    /// The transform basis deliberately uses the `q` and `s` directions because
    /// they map cleanly onto two independent worldspace vectors:
    ///
    /// - `Q = (-1, 0)`
    /// - `S = (-0.5, sqrt(3) / 2)`
    ///
    /// Using `r` directly as the second stored axis would also work, but the
    /// derived worldspace step couples both Cartesian components more awkwardly.
    #[inline]
    pub fn to_translation(&self, unit_size: f32) -> Vec3 {
        (self.q as f32 * Self::WORLDSPACE_Q_UNIT + self.r as f32 * Self::WORLDSPACE_S_UNIT)
            * unit_size
    }

    /// Wraps [`Self::to_translation`] in a translation-only transform.
    #[inline]
    pub fn to_transform(&self, unit_size: f32) -> Transform {
        Transform::from_translation(self.to_translation(unit_size))
    }

    #[inline]
    pub const fn top_right(&self) -> Self {
        self.add(Self::UP_RIGHT)
    }

    #[inline]
    pub const fn right(&self) -> Self {
        self.add(Self::RIGHT)
    }

    #[inline]
    pub const fn bottom_right(&self) -> Self {
        self.add(Self::DOWN_RIGHT)
    }

    #[inline]
    pub const fn bottom_left(&self) -> Self {
        self.add(Self::DOWN_LEFT)
    }

    #[inline]
    pub const fn left(&self) -> Self {
        self.add(Self::LEFT)
    }

    #[inline]
    pub const fn top_left(&self) -> Self {
        self.add(Self::UP_LEFT)
    }

    /// Returns all six adjacent hexes in clockwise order starting at top-right.
    #[inline]
    pub const fn neighbors(&self) -> [Self; 6] {
        [
            self.top_right(),
            self.right(),
            self.bottom_right(),
            self.bottom_left(),
            self.left(),
            self.top_left(),
        ]
    }

    /// Returns the canonical handle for the upper-right edge of this hex.
    #[inline]
    pub const fn top_right_edge(&self) -> HexEdge {
        (*self, HexEdgeDiscriminator::TopRight)
    }

    /// Returns the canonical handle for the right edge of this hex.
    #[inline]
    pub const fn right_edge(&self) -> HexEdge {
        (*self, HexEdgeDiscriminator::Right)
    }

    /// Returns the canonical handle for the lower-right edge of this hex.
    ///
    /// This edge is owned by the lower-right neighbor as that neighbor's
    /// [`HexEdgeDiscriminator::TopLeft`] edge.
    #[inline]
    pub const fn bottom_right_edge(&self) -> HexEdge {
        (self.bottom_right(), HexEdgeDiscriminator::TopLeft)
    }

    /// Returns the canonical handle for the lower-left edge of this hex.
    ///
    /// This edge is owned by the lower-left neighbor as that neighbor's
    /// [`HexEdgeDiscriminator::TopRight`] edge.
    #[inline]
    pub const fn bottom_left_edge(&self) -> HexEdge {
        (self.bottom_left(), HexEdgeDiscriminator::TopRight)
    }

    /// Returns the canonical handle for the left edge of this hex.
    ///
    /// This edge is owned by the left neighbor as that neighbor's
    /// [`HexEdgeDiscriminator::Right`] edge.
    #[inline]
    pub const fn left_edge(&self) -> HexEdge {
        (self.left(), HexEdgeDiscriminator::Right)
    }

    /// Returns the canonical handle for the upper-left edge of this hex.
    #[inline]
    pub const fn top_left_edge(&self) -> HexEdge {
        (*self, HexEdgeDiscriminator::TopLeft)
    }

    /// Returns all six boundary edges in clockwise order starting at top-right.
    #[inline]
    pub const fn edges(&self) -> [HexEdge; 6] {
        [
            self.top_right_edge(),
            self.right_edge(),
            self.bottom_right_edge(),
            self.bottom_left_edge(),
            self.left_edge(),
            self.top_left_edge(),
        ]
    }

    /// Returns the canonical handle for the top vertex of this hex.
    #[inline]
    pub const fn top_vert(&self) -> HexVert {
        (*self, HexVertDiscriminator::Top)
    }

    /// Returns the canonical handle for the upper-right vertex of this hex.
    #[inline]
    pub const fn top_right_vert(&self) -> HexVert {
        (*self, HexVertDiscriminator::TopRight)
    }

    /// Returns the canonical handle for the lower-right vertex of this hex.
    ///
    /// This vertex is owned by the lower-right neighbor as that hex's top
    /// vertex.
    #[inline]
    pub const fn bottom_right_vert(&self) -> HexVert {
        (self.bottom_right(), HexVertDiscriminator::Top)
    }

    /// Returns the canonical handle for the bottom vertex of this hex.
    ///
    /// This vertex is owned by the lower-left neighbor as that hex's
    /// upper-right vertex.
    #[inline]
    pub const fn bottom_vert(&self) -> HexVert {
        (self.bottom_left(), HexVertDiscriminator::TopRight)
    }

    /// Returns the canonical handle for the lower-left vertex of this hex.
    ///
    /// This vertex is owned by the lower-left neighbor as that hex's top
    /// vertex.
    #[inline]
    pub const fn bottom_left_vert(&self) -> HexVert {
        (self.bottom_left(), HexVertDiscriminator::Top)
    }

    /// Returns the canonical handle for the upper-left vertex of this hex.
    ///
    /// This vertex is owned by the left neighbor as that hex's upper-right
    /// vertex.
    #[inline]
    pub const fn top_left_vert(&self) -> HexVert {
        (self.left(), HexVertDiscriminator::TopRight)
    }

    /// Returns all six boundary vertices in clockwise order starting at top.
    #[inline]
    pub const fn vertices(&self) -> [HexVert; 6] {
        [
            self.top_vert(),
            self.top_right_vert(),
            self.bottom_right_vert(),
            self.bottom_vert(),
            self.bottom_left_vert(),
            self.top_left_vert(),
        ]
    }
}

impl std::ops::Add for HexCoord {
    type Output = Self;

    #[inline]
    fn add(self, rhs: Self) -> Self::Output {
        Self::add(&self, rhs)
    }
}

impl std::ops::Sub for HexCoord {
    type Output = Self;

    #[inline]
    fn sub(self, rhs: Self) -> Self::Output {
        Self::sub(&self, rhs)
    }
}

impl std::ops::Mul<i32> for HexCoord {
    type Output = Self;

    #[inline]
    fn mul(self, rhs: i32) -> Self::Output {
        HexCoord::mul(&self, rhs)
    }
}

impl std::ops::Mul<HexCoord> for i32 {
    type Output = HexCoord;

    #[inline]
    fn mul(self, rhs: HexCoord) -> Self::Output {
        HexCoord::mul(&rhs, self)
    }
}

impl std::ops::Neg for HexCoord {
    type Output = Self;

    #[inline]
    fn neg(self) -> Self::Output {
        HexCoord::neg(&self)
    }
}

#[derive(Clone, Copy, Component, Debug, PartialEq, Eq, Hash)]
#[require(HexCoord)]
pub enum HexEdgeDiscriminator {
    /// In a pointy-top configuration, the upper-left edge.
    TopLeft,

    /// In a pointy-top configuration, the upper-right edge.
    TopRight,

    /// In a pointy-top configuration, the rightmost edge.
    Right,
}

/// Operations implemented for canonical edge handles.
///
/// This trait is implemented for [`HexEdge`] rather than a dedicated struct so
/// edges remain a compact tuple in storage while still exposing semantic
/// helpers.
pub trait HexEdgeImpl {
    fn origin(&self) -> HexCoord;
    fn edge(&self) -> HexEdgeDiscriminator;

    fn to_transform(&self, unit_size: f32) -> Transform;

    fn neighbors(&self) -> [HexCoord; 2];
    fn neighboring_edges(&self) -> [HexEdge; 4];
    fn neighboring_verts(&self) -> [HexVert; 2];
}

const TOP_LEFT_EDGE_OFFSET: Vec3 = Vec3::new(
    -0.25,
    (WORLDSPACE_VERT_OFFSET + WORLDSPACE_EDGE_MIDLENGTH) / 2.0,
    0.0,
);
const TOP_RIGHT_EDGE_OFFSET: Vec3 = Vec3::new(
    0.25,
    (WORLDSPACE_VERT_OFFSET + WORLDSPACE_EDGE_MIDLENGTH) / 2.0,
    0.0,
);
const RIGHT_EDGE_OFFSET: Vec3 = Vec3::new(0.5, 0.0, 0.0);

impl HexEdgeImpl for HexEdge {
    #[inline]
    fn origin(&self) -> HexCoord {
        self.0
    }

    #[inline]
    fn edge(&self) -> HexEdgeDiscriminator {
        self.1
    }

    #[inline]
    fn to_transform(&self, unit_size: f32) -> Transform {
        let (edge_offset, degs) = match self.edge() {
            HexEdgeDiscriminator::TopLeft => (TOP_LEFT_EDGE_OFFSET, 30.0),
            HexEdgeDiscriminator::TopRight => (TOP_RIGHT_EDGE_OFFSET, 150.0),
            HexEdgeDiscriminator::Right => (RIGHT_EDGE_OFFSET, 90.0),
        };

        Transform {
            translation: self.origin().to_translation(unit_size) + unit_size * edge_offset,
            rotation: Quat::from_rotation_z(f32::to_radians(degs)),
            scale: Vec3::ONE,
        }
    }

    #[inline]
    fn neighbors(&self) -> [HexCoord; 2] {
        [
            self.origin(),
            match self.edge() {
                HexEdgeDiscriminator::TopLeft => self.origin().top_left(),
                HexEdgeDiscriminator::TopRight => self.origin().top_right(),
                HexEdgeDiscriminator::Right => self.origin().right(),
            },
        ]
    }

    #[inline]
    fn neighboring_edges(&self) -> [HexEdge; 4] {
        match self.edge() {
            HexEdgeDiscriminator::TopLeft => {
                let right = self.origin().right();

                [
                    right.top_right_edge(),
                    right.right_edge(),
                    self.origin().top_left().right_edge(),
                    self.origin().top_right_edge(),
                ]
            }
            HexEdgeDiscriminator::TopRight => [
                self.origin().top_left().right_edge(),
                self.origin().top_left_edge(),
                self.origin().right().top_left_edge(),
                self.origin().right_edge(),
            ],
            HexEdgeDiscriminator::Right => {
                let bottom_right = self.origin().bottom_right();

                [
                    self.origin().top_right_edge(),
                    self.origin().right().top_left_edge(),
                    bottom_right.top_left_edge(),
                    bottom_right.top_right_edge(),
                ]
            }
        }
    }

    #[inline]
    fn neighboring_verts(&self) -> [HexVert; 2] {
        match self.edge() {
            HexEdgeDiscriminator::TopLeft => {
                [self.origin().top_left_vert(), self.origin().top_vert()]
            }
            HexEdgeDiscriminator::TopRight => {
                [self.origin().top_vert(), self.origin().top_right_vert()]
            }
            HexEdgeDiscriminator::Right => [
                self.origin().top_right_vert(),
                self.origin().bottom_right_vert(),
            ],
        }
    }
}

#[derive(Clone, Copy, Component, Debug, PartialEq, Eq, Hash)]
#[require(HexCoord)]
pub enum HexVertDiscriminator {
    /// In a pointy-top configuration, the top vertex.
    Top,

    /// In a pointy-top configuration, the upper-right vertex.
    TopRight,
}

/// Operations implemented for canonical vertex handles.
///
/// As with [`HexEdgeImpl`], this preserves the compact tuple representation
/// while still exposing grid-topology helpers.
pub trait HexVertImpl {
    fn origin(&self) -> HexCoord;
    fn vert(&self) -> HexVertDiscriminator;

    fn to_translation(&self, unit_size: f32) -> Vec3;
    fn to_transform(&self, unit_size: f32) -> Transform;

    fn neighbors(&self) -> [HexCoord; 3];
    fn neighbor_edges(&self) -> [HexEdge; 3];
    fn neighbor_verts(&self) -> [HexVert; 3];
}

const TOP_VERT_OFFSET: Vec3 = Vec3::new(0.0, WORLDSPACE_VERT_OFFSET, 0.0);
const TOP_RIGHT_VERT_OFFSET: Vec3 = Vec3::new(0.5, WORLDSPACE_EDGE_MIDLENGTH, 0.0);

impl HexVertImpl for HexVert {
    #[inline]
    fn origin(&self) -> HexCoord {
        self.0
    }

    #[inline]
    fn vert(&self) -> HexVertDiscriminator {
        self.1
    }

    #[inline]
    fn to_translation(&self, unit_size: f32) -> Vec3 {
        self.origin().to_translation(unit_size)
            + unit_size
                * match self.vert() {
                    HexVertDiscriminator::Top => TOP_VERT_OFFSET,
                    HexVertDiscriminator::TopRight => TOP_RIGHT_VERT_OFFSET,
                }
    }

    #[inline]
    fn to_transform(&self, unit_size: f32) -> Transform {
        Transform::from_translation(self.to_translation(unit_size))
    }

    #[inline]
    fn neighbors(&self) -> [HexCoord; 3] {
        match self.vert() {
            HexVertDiscriminator::Top => [
                self.origin().top_left(),
                self.origin().top_right(),
                self.origin(),
            ],
            HexVertDiscriminator::TopRight => [
                self.origin().top_right(),
                self.origin(),
                self.origin().right(),
            ],
        }
    }

    #[inline]
    fn neighbor_edges(&self) -> [HexEdge; 3] {
        match self.vert() {
            HexVertDiscriminator::Top => [
                self.origin().top_left().right_edge(),
                self.origin().top_left_edge(),
                self.origin().top_right_edge(),
            ],
            HexVertDiscriminator::TopRight => [
                self.origin().right().top_left_edge(),
                self.origin().right_edge(),
                self.origin().top_right_edge(),
            ],
        }
    }

    #[inline]
    fn neighbor_verts(&self) -> [HexVert; 3] {
        match self.vert() {
            HexVertDiscriminator::Top => [
                self.origin().top_left().top_right_vert(),
                self.origin().top_right_vert(),
                self.origin().top_left_vert(),
            ],
            HexVertDiscriminator::TopRight => [
                self.origin().top_vert(),
                self.origin().right().top_vert(),
                self.origin().bottom_right_vert(),
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identities() {
        // Directional balance
        assert_eq!(HexCoord::ZERO, HexCoord::LEFT + HexCoord::RIGHT);
        assert_eq!(HexCoord::ZERO, HexCoord::UP_LEFT + HexCoord::DOWN_RIGHT);
        assert_eq!(HexCoord::ZERO, HexCoord::DOWN_LEFT + HexCoord::UP_RIGHT);

        // Negation/subtraction
        assert_eq!(HexCoord::ZERO, HexCoord::LEFT - HexCoord::LEFT);
        assert_eq!(HexCoord::ZERO, HexCoord::UP_LEFT - HexCoord::UP_LEFT);
        assert_eq!(HexCoord::ZERO, HexCoord::DOWN_LEFT - HexCoord::DOWN_LEFT);
        assert_eq!(HexCoord::ZERO, HexCoord::RIGHT - HexCoord::RIGHT);
        assert_eq!(HexCoord::ZERO, HexCoord::UP_RIGHT - HexCoord::UP_RIGHT);
        assert_eq!(HexCoord::ZERO, HexCoord::DOWN_RIGHT - HexCoord::DOWN_RIGHT);

        // Multiplication
        assert_eq!(HexCoord::ONE + HexCoord::ONE, HexCoord::ONE * 2);
        assert_eq!(HexCoord::ONE + HexCoord::ONE, 2 * HexCoord::ONE);
    }

    #[test]
    fn neighbors_match_directions() {
        let origin = HexCoord::ZERO;
        assert_eq!(
            origin.neighbors(),
            [
                HexCoord::UP_RIGHT,
                HexCoord::RIGHT,
                HexCoord::DOWN_RIGHT,
                HexCoord::DOWN_LEFT,
                HexCoord::LEFT,
                HexCoord::UP_LEFT,
            ]
        );

        let shifted = HexCoord::new(2, -3);
        assert_eq!(
            shifted.neighbors(),
            [
                shifted.top_right(),
                shifted.right(),
                shifted.bottom_right(),
                shifted.bottom_left(),
                shifted.left(),
                shifted.top_left(),
            ]
        );
    }

    #[test]
    fn edges_are_neighboring_hexes() {
        let origin = HexCoord::ZERO;

        assert_eq!(
            origin.top_left_edge().neighbors(),
            [origin, origin.top_left()]
        );
        assert_eq!(
            origin.top_right_edge().neighbors(),
            [origin, origin.top_right()]
        );
        assert_eq!(origin.right_edge().neighbors(), [origin, origin.right()]);
        assert_eq!(
            origin.bottom_left_edge().neighbors(),
            [origin.bottom_left(), origin]
        );
        assert_eq!(
            origin.bottom_right_edge().neighbors(),
            [origin.bottom_right(), origin]
        );
    }

    #[test]
    fn edge_neighboring_verts_match_vertices() {
        let origin = HexCoord::ZERO;

        assert_eq!(
            origin.top_left_edge().neighboring_verts(),
            [origin.top_left_vert(), origin.top_vert()]
        );
        assert_eq!(
            origin.top_right_edge().neighboring_verts(),
            [origin.top_vert(), origin.top_right_vert()]
        );
        assert_eq!(
            origin.right_edge().neighboring_verts(),
            [origin.top_right_vert(), origin.bottom_right_vert()]
        );
        assert_eq!(
            origin.bottom_left_edge().neighboring_verts(),
            [origin.bottom_left_vert(), origin.bottom_vert()]
        );
        assert_eq!(
            origin.bottom_right_edge().neighboring_verts(),
            [origin.bottom_vert(), origin.bottom_right_vert()]
        );
    }

    #[test]
    fn vertex_neighbors_are_adjacent_hexes() {
        let origin = HexCoord::ZERO;

        assert_eq!(
            origin.top_vert().neighbors(),
            [origin.top_left(), origin.top_right(), origin]
        );
        assert_eq!(
            origin.top_right_vert().neighbors(),
            [origin.top_right(), origin, origin.right()]
        );
        assert_eq!(
            origin.bottom_vert().neighbors(),
            [origin, origin.bottom_left(), origin.bottom_right()]
        );
    }

    #[test]
    fn vertex_edges_round_trip() {
        let origin = HexCoord::ZERO;

        assert_eq!(
            origin.top_vert().neighbor_edges(),
            [
                origin.top_left().right_edge(),
                origin.top_left_edge(),
                origin.top_right_edge(),
            ]
        );
        assert_eq!(
            origin.top_right_vert().neighbor_edges(),
            [
                origin.right().top_left_edge(),
                origin.right_edge(),
                origin.top_right_edge(),
            ]
        );
        assert_eq!(
            origin.bottom_vert().neighbor_edges(),
            [
                origin.bottom_right().top_left_edge(),
                origin.bottom_left().right_edge(),
                origin.bottom_left().top_right_edge(),
            ]
        );
    }
}
