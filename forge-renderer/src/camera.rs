//! 2D orthographic camera for graph navigation.
//!
//! Provides zoom, pan, and screen-to-graph coordinate conversion.
//! The camera produces a `CameraUniform` that is uploaded to the GPU
//! every frame via a uniform buffer.

use crate::types::CameraUniform;
use glam::{Mat4, Vec2};

/// 2D orthographic camera with zoom and pan.
///
/// The camera defines a rectangular viewport in graph-space coordinates.
/// Zoom changes the viewport size (smaller viewport = zoomed in),
/// pan offsets the viewport center.
///
/// # Coordinate system
///
/// - Graph-space: arbitrary units, origin at (0,0), Y-up.
/// - Screen-space: pixels, origin at top-left, Y-down.
///
/// The `view_proj` matrix transforms graph-space → clip-space (-1..1).
pub struct Camera2D {
    /// Center of the viewport in graph-space.
    pub center: Vec2,
    /// Zoom level (1.0 = default, >1 = zoomed in, <1 = zoomed out).
    pub zoom: f32,
    /// Viewport width in pixels (updated on resize).
    pub viewport_width: f32,
    /// Viewport height in pixels (updated on resize).
    pub viewport_height: f32,
    /// Minimum zoom level (prevent zooming out too far).
    pub min_zoom: f32,
    /// Maximum zoom level (prevent zooming in too far).
    pub max_zoom: f32,
}

impl Camera2D {
    /// Create a camera with sensible defaults for a graph view.
    pub fn new(viewport_width: f32, viewport_height: f32) -> Self {
        Self {
            center: Vec2::ZERO,
            zoom: 1.0,
            viewport_width,
            viewport_height,
            min_zoom: 0.01,
            max_zoom: 50.0,
        }
    }

    /// Update the viewport dimensions (call on window/canvas resize).
    pub fn resize(&mut self, width: f32, height: f32) {
        self.viewport_width = width;
        self.viewport_height = height;
    }

    /// Apply a zoom delta around a focal point (in screen-space pixels).
    ///
    /// The focal point stays fixed on screen — this is the natural behavior
    /// when zooming with the mouse wheel (zoom towards cursor).
    pub fn zoom_at(&mut self, delta: f32, screen_focal: Vec2) {
        let old_graph_pos = self.screen_to_graph(screen_focal);
        self.zoom = (self.zoom * (1.0 + delta)).clamp(self.min_zoom, self.max_zoom);
        let new_graph_pos = self.screen_to_graph(screen_focal);
        // Shift center so the focal point doesn't move on screen.
        self.center += old_graph_pos - new_graph_pos;
    }

    /// Pan the camera by a screen-space delta (pixels).
    pub fn pan(&mut self, screen_delta: Vec2) {
        let scale = self.graph_units_per_pixel();
        self.center -= Vec2::new(screen_delta.x * scale, -screen_delta.y * scale);
    }

    /// Convert screen-space coordinates (pixels) to graph-space.
    pub fn screen_to_graph(&self, screen: Vec2) -> Vec2 {
        let scale = self.graph_units_per_pixel();
        let half_w = self.viewport_width * 0.5;
        let half_h = self.viewport_height * 0.5;
        Vec2::new(
            self.center.x + (screen.x - half_w) * scale,
            self.center.y - (screen.y - half_h) * scale, // Y-flip
        )
    }

    /// Convert graph-space coordinates to screen-space (pixels).
    pub fn graph_to_screen(&self, graph: Vec2) -> Vec2 {
        let scale = self.graph_units_per_pixel();
        let half_w = self.viewport_width * 0.5;
        let half_h = self.viewport_height * 0.5;
        Vec2::new(
            half_w + (graph.x - self.center.x) / scale,
            half_h - (graph.y - self.center.y) / scale, // Y-flip
        )
    }

    /// Graph-space units per pixel at the current zoom level.
    ///
    /// A "base viewport" of ~1000 graph-units wide at zoom=1.0
    /// gives a good default for typical graph layouts.
    fn graph_units_per_pixel(&self) -> f32 {
        let base_graph_width = 1000.0; // graph-units visible at zoom=1
        base_graph_width / (self.viewport_width * self.zoom)
    }

    /// Build the orthographic projection * view matrix for the GPU.
    ///
    /// This is uploaded to the camera uniform buffer every frame.
    pub fn uniform(&self) -> CameraUniform {
        let half_w = self.viewport_width * 0.5 * self.graph_units_per_pixel();
        let half_h = self.viewport_height * 0.5 * self.graph_units_per_pixel();

        // Orthographic projection: graph-space rect → clip-space [-1, 1]
        let proj = Mat4::orthographic_rh(
            self.center.x - half_w, // left
            self.center.x + half_w, // right
            self.center.y - half_h, // bottom
            self.center.y + half_h, // top
            -1.0,                   // near
            1.0,                    // far
        );

        CameraUniform {
            view_proj: proj.to_cols_array_2d(),
        }
    }

    /// Fit the camera to show all nodes with some padding (instant snap).
    ///
    /// Computes the bounding box of the given positions and adjusts
    /// center + zoom so everything is visible.
    pub fn fit_to_bounds(&mut self, min: Vec2, max: Vec2, padding: f32) {
        let (target_center, target_zoom) = self.compute_fit(min, max, padding);
        self.center = target_center;
        self.zoom = target_zoom;
    }

    /// Smoothly fit the camera toward the target bounds (lerp per frame).
    ///
    /// `lerp_factor` controls speed: 0.05 = slow glide, 0.2 = fast snap.
    /// Call every frame during simulation for a smooth camera that tracks
    /// the layout as it converges — no visual jump.
    pub fn smooth_fit_to_bounds(&mut self, min: Vec2, max: Vec2, padding: f32, lerp_factor: f32) {
        let (target_center, target_zoom) = self.compute_fit(min, max, padding);
        self.center += (target_center - self.center) * lerp_factor;
        self.zoom += (target_zoom - self.zoom) * lerp_factor;
    }

    /// Compute target center + zoom for a given bounding box (used by both
    /// instant and smooth fit).
    fn compute_fit(&self, min: Vec2, max: Vec2, padding: f32) -> (Vec2, f32) {
        let size = max - min;
        let target_center = (min + max) * 0.5;

        let padded_w = size.x + padding * 2.0;
        let padded_h = size.y + padding * 2.0;

        let aspect = self.viewport_width / self.viewport_height.max(1.0);
        let zoom_w = 1000.0 / padded_w.max(1.0);
        let zoom_h = 1000.0 / (padded_h * aspect).max(1.0);

        let target_zoom = zoom_w.min(zoom_h).clamp(self.min_zoom, self.max_zoom);
        (target_center, target_zoom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Floating-point comparison epsilon for test assertions.
    ///
    /// Sized for `f32` round-trip arithmetic at graph-space magnitudes up to
    /// a few thousand units. `f32` provides ~7 significant digits, so relative
    /// error at magnitude 1000 is ~1e-4. A smaller epsilon (e.g. 1e-6) would
    /// cause false negatives on legitimate round-trips.
    const EPSILON: f32 = 1e-4;

    /// Helper: assert two Vec2 are approximately equal within EPSILON.
    fn assert_vec2_approx(actual: Vec2, expected: Vec2, msg: &str) {
        assert!(
            (actual.x - expected.x).abs() < EPSILON && (actual.y - expected.y).abs() < EPSILON,
            "Vec2 mismatch ({}): got {:?}, expected {:?}",
            msg,
            actual,
            expected
        );
    }

    /// Helper: assert two f32 are approximately equal within EPSILON.
    fn assert_f32_approx(actual: f32, expected: f32, msg: &str) {
        assert!(
            (actual - expected).abs() < EPSILON,
            "f32 mismatch ({}): got {}, expected {}",
            msg,
            actual,
            expected
        );
    }

    // ─── Camera2D::zoom_at Tests ───

    /// Test identity zoom (delta=0.0): camera should remain unchanged.
    #[test]
    fn camera_zoom_at_delta_zero_is_identity() {
        let mut cam = Camera2D::new(800.0, 600.0);
        let initial_center = cam.center;
        let initial_zoom = cam.zoom;
        let focal_point = Vec2::new(400.0, 300.0); // center of screen

        cam.zoom_at(0.0, focal_point);

        assert_vec2_approx(cam.center, initial_center, "center after delta=0");
        assert_f32_approx(cam.zoom, initial_zoom, "zoom after delta=0");
    }

    /// Test zoom in (positive delta): zoom should increase (clamped to max_zoom).
    #[test]
    fn camera_zoom_at_positive_delta_increases_zoom() {
        let mut cam = Camera2D::new(800.0, 600.0);
        cam.zoom = 1.0;
        let focal_point = Vec2::new(400.0, 300.0);

        cam.zoom_at(0.1, focal_point); // 10% zoom increase

        assert!(cam.zoom > 1.0, "zoom should increase with positive delta");
        assert!(cam.zoom <= cam.max_zoom, "zoom should not exceed max_zoom");
    }

    /// Test zoom out (negative delta): zoom should decrease (clamped to min_zoom).
    #[test]
    fn camera_zoom_at_negative_delta_decreases_zoom() {
        let mut cam = Camera2D::new(800.0, 600.0);
        cam.zoom = 10.0;
        let focal_point = Vec2::new(400.0, 300.0);

        cam.zoom_at(-0.5, focal_point); // 50% zoom out

        assert!(cam.zoom < 10.0, "zoom should decrease with negative delta");
        assert!(
            cam.zoom >= cam.min_zoom,
            "zoom should not go below min_zoom"
        );
    }

    /// Test zoom clamping: extreme positive delta should be clamped to max_zoom.
    ///
    /// `zoom_at` computes new zoom as `zoom * (1.0 + delta)`. Starting at
    /// zoom = 10.0 and applying delta = 100.0 yields `10.0 * 101.0 = 1010.0`,
    /// which far exceeds `max_zoom = 50.0` and must be clamped.
    #[test]
    fn camera_zoom_at_clamps_to_max_zoom() {
        let mut cam = Camera2D::new(800.0, 600.0);
        cam.zoom = 10.0;
        let focal_point = Vec2::new(400.0, 300.0);

        cam.zoom_at(100.0, focal_point); // 10.0 * 101.0 = 1010, clamped to max_zoom=50.0

        assert_f32_approx(cam.zoom, cam.max_zoom, "zoom clamped to max_zoom");
    }

    /// Test zoom clamping: extreme negative delta should be clamped to min_zoom.
    #[test]
    fn camera_zoom_at_clamps_to_min_zoom() {
        let mut cam = Camera2D::new(800.0, 600.0);
        cam.zoom = 10.0;
        let focal_point = Vec2::new(400.0, 300.0);

        cam.zoom_at(-100.0, focal_point); // huge zoom out (10.0 * -99.0 < min_zoom)

        assert_f32_approx(cam.zoom, cam.min_zoom, "zoom clamped to min_zoom");
    }

    /// Test focal point preservation: screen position of the focal point should
    /// remain the same after zoom (the "anchor" does not move on screen).
    #[test]
    fn camera_zoom_at_preserves_focal_point() {
        let mut cam = Camera2D::new(800.0, 600.0);
        cam.center = Vec2::new(0.0, 0.0);
        cam.zoom = 1.0;

        let focal_screen = Vec2::new(200.0, 150.0);
        let graph_pos_before = cam.screen_to_graph(focal_screen);

        cam.zoom_at(0.5, focal_screen); // zoom in

        let graph_pos_after = cam.screen_to_graph(focal_screen);

        // The graph position that was at the focal point should still be at the focal point.
        assert_vec2_approx(
            graph_pos_after,
            graph_pos_before,
            "focal point graph position should be preserved",
        );
    }

    /// Test zoom at screen corner: focal point at top-left should remain invariant.
    #[test]
    fn camera_zoom_at_preserves_focal_point_at_corner() {
        let mut cam = Camera2D::new(1000.0, 800.0);
        cam.center = Vec2::new(500.0, 400.0);
        cam.zoom = 2.0;

        let focal_screen = Vec2::new(0.0, 0.0); // top-left corner
        let graph_pos_before = cam.screen_to_graph(focal_screen);

        cam.zoom_at(0.2, focal_screen);

        let graph_pos_after = cam.screen_to_graph(focal_screen);
        assert_vec2_approx(
            graph_pos_after,
            graph_pos_before,
            "focal point at corner preserved",
        );
    }

    // ─── Camera2D::screen_to_graph Tests ───

    /// Test screen-to-graph at top-left corner (0, 0).
    #[test]
    fn camera_screen_to_graph_top_left_corner() {
        let cam = Camera2D::new(800.0, 600.0);
        // cam.center = (0, 0), cam.zoom = 1.0
        // base_graph_width = 1000.0
        // scale = 1000.0 / (800.0 * 1.0) = 1.25 graph-units/pixel
        // half_w = 400, half_h = 300
        // screen (0, 0) -> graph (center.x + (0 - 400) * 1.25, center.y - (0 - 300) * 1.25)
        //                       = (0 - 500, 0 + 375) = (-500, 375)

        let graph = cam.screen_to_graph(Vec2::new(0.0, 0.0));

        assert_vec2_approx(graph, Vec2::new(-500.0, 375.0), "screen (0,0) to graph");
    }

    /// Test screen-to-graph at center of screen.
    #[test]
    fn camera_screen_to_graph_center() {
        let cam = Camera2D::new(800.0, 600.0);
        // cam.center = (0, 0)
        // half_w = 400, half_h = 300
        // screen (400, 300) should map to graph (0, 0)

        let graph = cam.screen_to_graph(Vec2::new(400.0, 300.0));

        assert_vec2_approx(graph, Vec2::new(0.0, 0.0), "screen center to graph origin");
    }

    /// Test screen-to-graph at bottom-right corner.
    #[test]
    fn camera_screen_to_graph_bottom_right_corner() {
        let cam = Camera2D::new(800.0, 600.0);
        // screen (800, 600) -> graph (center.x + (800 - 400) * 1.25, center.y - (600 - 300) * 1.25)
        //                            = (0 + 500, 0 - 375) = (500, -375)

        let graph = cam.screen_to_graph(Vec2::new(800.0, 600.0));

        assert_vec2_approx(
            graph,
            Vec2::new(500.0, -375.0),
            "screen bottom-right to graph",
        );
    }

    /// Test screen-to-graph with non-zero camera center.
    #[test]
    fn camera_screen_to_graph_with_offset_center() {
        let mut cam = Camera2D::new(800.0, 600.0);
        cam.center = Vec2::new(100.0, 200.0);
        // scale = 1.25, half_w = 400, half_h = 300
        // screen center (400, 300) should map to camera center (100, 200)

        let graph = cam.screen_to_graph(Vec2::new(400.0, 300.0));

        assert_vec2_approx(
            graph,
            Vec2::new(100.0, 200.0),
            "screen center with offset camera center",
        );
    }

    /// Test screen-to-graph with increased zoom.
    #[test]
    fn camera_screen_to_graph_with_zoom() {
        let mut cam = Camera2D::new(800.0, 600.0);
        cam.zoom = 2.0;
        // scale = 1000.0 / (800.0 * 2.0) = 0.625 graph-units/pixel
        // screen center (400, 300) should map to camera center (0, 0)

        let graph = cam.screen_to_graph(Vec2::new(400.0, 300.0));

        assert_vec2_approx(graph, Vec2::new(0.0, 0.0), "screen center with zoom=2");
    }

    // ─── Camera2D::fit_to_bounds Tests ───

    /// Test fit_to_bounds with valid, centered bounds.
    #[test]
    fn camera_fit_to_bounds_valid_bounds() {
        let mut cam = Camera2D::new(800.0, 600.0);
        let min = Vec2::new(-100.0, -80.0);
        let max = Vec2::new(100.0, 80.0);
        let padding = 10.0;

        cam.fit_to_bounds(min, max, padding);

        let expected_center = (min + max) * 0.5; // (0, 0)
        assert_vec2_approx(
            cam.center,
            expected_center,
            "fit_to_bounds center should be midpoint",
        );
        assert!(
            cam.zoom > 0.0 && cam.zoom <= cam.max_zoom,
            "fit_to_bounds zoom should be valid"
        );
    }

    /// Test fit_to_bounds with zero-width bounds (min.x == max.x).
    /// Should not panic, camera zoom should be clamped.
    #[test]
    fn camera_fit_to_bounds_zero_width() {
        let mut cam = Camera2D::new(800.0, 600.0);
        let min = Vec2::new(0.0, -50.0);
        let max = Vec2::new(0.0, 50.0);
        let padding = 0.0;

        // This should not panic.
        cam.fit_to_bounds(min, max, padding);

        assert!(
            cam.zoom > 0.0 && cam.zoom <= cam.max_zoom,
            "fit_to_bounds zero-width should clamp zoom"
        );
        assert_vec2_approx(
            cam.center,
            Vec2::new(0.0, 0.0),
            "fit_to_bounds zero-width center",
        );
    }

    /// Test fit_to_bounds with zero-height bounds (min.y == max.y).
    /// Should not panic, camera zoom should be clamped.
    #[test]
    fn camera_fit_to_bounds_zero_height() {
        let mut cam = Camera2D::new(800.0, 600.0);
        let min = Vec2::new(-100.0, 0.0);
        let max = Vec2::new(100.0, 0.0);
        let padding = 0.0;

        // This should not panic.
        cam.fit_to_bounds(min, max, padding);

        assert!(
            cam.zoom > 0.0 && cam.zoom <= cam.max_zoom,
            "fit_to_bounds zero-height should clamp zoom"
        );
        assert_vec2_approx(
            cam.center,
            Vec2::new(0.0, 0.0),
            "fit_to_bounds zero-height center",
        );
    }

    /// Test fit_to_bounds with inverted bounds (min > max).
    /// compute_fit uses `size = max - min`, which will be negative.
    /// The `.max(1.0)` guards should prevent panics.
    #[test]
    fn camera_fit_to_bounds_inverted_bounds() {
        let mut cam = Camera2D::new(800.0, 600.0);
        let min = Vec2::new(100.0, 50.0);
        let max = Vec2::new(-100.0, -50.0); // inverted
        let padding = 0.0;

        // This should not panic.
        cam.fit_to_bounds(min, max, padding);

        assert!(
            cam.zoom > 0.0 && cam.zoom <= cam.max_zoom,
            "fit_to_bounds inverted bounds should clamp zoom"
        );
        // center is computed as (min + max) * 0.5 = (0, 0) even if bounds are inverted
        let expected_center = (min + max) * 0.5;
        assert_vec2_approx(
            cam.center,
            expected_center,
            "fit_to_bounds inverted bounds center",
        );
    }

    /// Test fit_to_bounds with large padding.
    #[test]
    fn camera_fit_to_bounds_with_large_padding() {
        let mut cam = Camera2D::new(800.0, 600.0);
        let min = Vec2::new(-10.0, -10.0);
        let max = Vec2::new(10.0, 10.0);
        let padding = 1000.0;

        cam.fit_to_bounds(min, max, padding);

        // With large padding, zoom should be reduced (more of graph-space visible).
        let cam_no_pad = {
            let mut c = Camera2D::new(800.0, 600.0);
            c.fit_to_bounds(min, max, 0.0);
            c
        };

        assert!(
            cam.zoom < cam_no_pad.zoom,
            "fit_to_bounds with large padding should reduce zoom"
        );
    }

    /// Test fit_to_bounds preserves aspect ratio constraints.
    #[test]
    fn camera_fit_to_bounds_respects_viewport_aspect() {
        let mut cam = Camera2D::new(800.0, 600.0);
        let min = Vec2::new(-100.0, -100.0);
        let max = Vec2::new(100.0, 100.0);
        let padding = 0.0;

        cam.fit_to_bounds(min, max, padding);

        // Zoom should be clamped between min_zoom and max_zoom.
        assert!(
            cam.zoom >= cam.min_zoom && cam.zoom <= cam.max_zoom,
            "fit_to_bounds respects zoom bounds"
        );
    }

    // ─── Round-trip Tests ───

    /// Test round-trip: graph -> screen -> graph should recover original.
    #[test]
    fn camera_round_trip_graph_to_screen_to_graph() {
        let cam = Camera2D::new(1024.0, 768.0);
        let original = Vec2::new(42.5, -73.2);

        let screen = cam.graph_to_screen(original);
        let recovered = cam.screen_to_graph(screen);

        assert_vec2_approx(recovered, original, "round-trip graph->screen->graph");
    }

    /// Test round-trip with camera offset and zoom.
    #[test]
    fn camera_round_trip_with_offset_and_zoom() {
        let mut cam = Camera2D::new(1024.0, 768.0);
        cam.center = Vec2::new(500.0, -200.0);
        cam.zoom = 3.5;

        let original = Vec2::new(123.4, 456.7);

        let screen = cam.graph_to_screen(original);
        let recovered = cam.screen_to_graph(screen);

        assert_vec2_approx(recovered, original, "round-trip with offset and zoom");
    }
}
