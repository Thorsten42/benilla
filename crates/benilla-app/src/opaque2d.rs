//! Bevy's 2D opaque pass, skipped when both its phases are empty, which on our cameras they always
//! are (UI quads are transparent, Bevy UI and egui draw in their own passes). The first pass to
//! touch the view target clears it, so the transparent pass after it takes the clear.
//! It takes Bevy's seat in `Core2d`: ordered before `main_opaque_pass_2d`, which is then removed,
//! so the edges through Bevy's system carry over to this one.

use bevy::core_pipeline::core_2d::{main_opaque_pass_2d, AlphaMask2d, Opaque2d};
use bevy::core_pipeline::{Core2d, Core2dSystems};
use bevy::ecs::schedule::ScheduleCleanupPolicy;
use bevy::prelude::*;
use bevy::render::camera::ExtractedCamera;
use bevy::render::diagnostic::RecordDiagnostics;
use bevy::render::render_phase::ViewBinnedRenderPhases;
use bevy::render::render_resource::{RenderPassDescriptor, StoreOp};
use bevy::render::renderer::{RenderContext, ViewQuery};
use bevy::render::view::{ExtractedView, ViewDepthStencilTexture, ViewTarget};
use bevy::render::RenderApp;

fn skip_empty_opaque_pass_2d(
    world: &World,
    view: ViewQuery<(
        &ExtractedCamera,
        &ExtractedView,
        &ViewTarget,
        &ViewDepthStencilTexture,
    )>,
    mut render_context: RenderContext,
) {
    let view_entity = view.entity();
    let (camera, extracted_view, target, depth) = view.into_inner();
    let (Some(opaque_phases), Some(alpha_mask_phases)) = (
        world.get_resource::<ViewBinnedRenderPhases<Opaque2d>>(),
        world.get_resource::<ViewBinnedRenderPhases<AlphaMask2d>>(),
    ) else {
        return;
    };
    let (Some(opaque_phase), Some(alpha_mask_phase)) = (
        opaque_phases.get(&extracted_view.retained_view_entity),
        alpha_mask_phases.get(&extracted_view.retained_view_entity),
    ) else {
        return;
    };
    // The one line Bevy's pass lacks: the transparent pass that follows opens unconditionally
    // and takes the target's first-call clear.
    if opaque_phase.is_empty() && alpha_mask_phase.is_empty() {
        return;
    }

    let diagnostics = render_context.diagnostic_recorder();
    let diagnostics = diagnostics.as_deref();
    let color_attachments = [Some(target.get_color_attachment())];
    let depth_stencil_attachment = Some(depth.get_attachment(StoreOp::Store));
    let mut render_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
        label: Some("main_opaque_pass_2d"),
        color_attachments: &color_attachments,
        depth_stencil_attachment,
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    });
    let pass_span = diagnostics.pass_span(&mut render_pass, "main_opaque_pass_2d");
    if let Some(viewport) = camera.viewport.as_ref() {
        render_pass.set_camera_viewport(viewport);
    }
    if !opaque_phase.is_empty() {
        if let Err(err) = opaque_phase.render(&mut render_pass, world, view_entity) {
            error!("Error encountered while rendering the 2d opaque phase {err:?}");
        }
    }
    if !alpha_mask_phase.is_empty() {
        if let Err(err) = alpha_mask_phase.render(&mut render_pass, world, view_entity) {
            error!("Error encountered while rendering the 2d alpha mask phase {err:?}");
        }
    }
    pass_span.end(&mut render_pass);
}

/// Replaces Bevy's 2D opaque pass; must be added after `DefaultPlugins` registers the original
/// (by [`crate::ui_pass::PlayerUiPlugin`]).
pub(crate) struct SkipEmptyOpaque2dPlugin;

impl Plugin for SkipEmptyOpaque2dPlugin {
    fn build(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render_app.add_systems(
            Core2d,
            skip_empty_opaque_pass_2d
                .in_set(Core2dSystems::MainPass)
                .before(main_opaque_pass_2d),
        );
        render_app
            .remove_systems_in_set(
                Core2d,
                main_opaque_pass_2d,
                ScheduleCleanupPolicy::RemoveSystemsOnly,
            )
            .expect("Bevy's main_opaque_pass_2d is in Core2d, built before this plugin");
    }
}
