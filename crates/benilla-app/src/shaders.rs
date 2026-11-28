//! The game's four WESL shaders, embedded in the binary. Registered from the root of `src/`: the
//! macro does not normalize paths, so a call from a submodule would bake its directory into the
//! served path.

use bevy::prelude::*;

/// Register the four shaders under `embedded://benilla_app/shaders/…`; added after `DefaultPlugins`
/// creates the registry it fills.
pub(crate) fn plugin(app: &mut App) {
    bevy::asset::embedded_asset!(app, "shaders/ui_quad.wesl");
    bevy::asset::embedded_asset!(app, "shaders/ui_gamma.wesl");
    bevy::asset::embedded_asset!(app, "shaders/ui_node_gamma.wesl");
    bevy::asset::embedded_asset!(app, "shaders/ui_slice_gamma.wesl");
}

#[cfg(test)]
mod tests {
    use bevy::asset::io::AssetSourceId;
    use bevy::prelude::*;

    /// Reads every file in the directory back from its embedded path.
    #[test]
    fn every_game_shader_answers_at_its_embedded_path() {
        let mut app = App::new();
        app.add_plugins(bevy::asset::AssetPlugin::default());
        app.add_plugins(super::plugin);
        let server = app.world().resource::<AssetServer>();
        let source = server.get_source(AssetSourceId::from("embedded")).unwrap();

        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/src/shaders");
        let mut found = 0;
        for entry in std::fs::read_dir(dir).unwrap() {
            let name = entry.unwrap().file_name().to_string_lossy().into_owned();
            if !name.ends_with(".wesl") {
                continue;
            }
            found += 1;
            let path = format!("benilla_app/shaders/{name}");
            let read = bevy::tasks::block_on(async {
                source.reader().read(std::path::Path::new(&path)).await
            });
            assert!(
                read.is_ok(),
                "embedded://{path} does not resolve — {name} is on disk but not registered in \
                 `shaders::plugin`, or was registered from a file that is not directly under src/"
            );
        }
        assert_eq!(found, 4, "the game's shader set changed size");
    }

    /// Every `.wesl` file under `dir` of the embedded source, as the asset server would serve it.
    fn embedded_shaders(
        source: &bevy::asset::io::AssetSource,
        dir: &std::path::Path,
        out: &mut Vec<(String, String)>,
    ) {
        use bevy::tasks::futures_lite::StreamExt;
        let reader = source.reader();
        let entries: Vec<std::path::PathBuf> = bevy::tasks::block_on(async {
            match reader.read_directory(dir).await {
                Ok(stream) => stream.collect().await,
                Err(_) => Vec::new(),
            }
        });
        for path in entries {
            if bevy::tasks::block_on(reader.is_directory(&path)).unwrap_or(false) {
                embedded_shaders(source, &path, out);
            } else if path.extension().is_some_and(|e| e == "wesl") {
                let text = bevy::tasks::block_on(async {
                    let mut bytes = Vec::new();
                    let mut r = reader.read(&path).await.ok()?;
                    r.read_to_end(&mut bytes).await.ok()?;
                    String::from_utf8(bytes).ok()
                });
                // An asset path is `/`-separated on every platform, but the in-memory reader joins
                // a child with the platform's separator (`PathBuf::join`), `\` on Windows.
                let asset_path = path
                    .components()
                    .map(|c| c.as_os_str().to_string_lossy())
                    .collect::<Vec<_>>()
                    .join("/");
                if let Some(text) = text {
                    out.push((asset_path, text));
                }
            }
        }
    }

    /// No gate draws a frame, so every shader benilla ships is composed here as the pipeline cache
    /// composes it: bevy's `ShaderCache` over every embedded shader, its WESL compiler resolving
    /// the imports, under each variant's defs (the vertex layouts the mesh pipelines give, and each
    /// shader's own flags). The result must parse, validate and translate for the backend this
    /// platform renders with.
    #[test]
    fn every_shader_composes_validates_and_translates() {
        use bevy::shader::{
            Shader, ShaderCache, ShaderCacheError, ShaderCacheSource, ShaderDefVal,
        };
        let app = crate::game_plugins::schedule_tests::headless_client_unrun();
        let server = app.world().resource::<AssetServer>();
        let source = server.get_source(AssetSourceId::from("embedded")).unwrap();
        let mut files = Vec::new();
        embedded_shaders(source, std::path::Path::new(""), &mut files);

        // The cache is generic over its device; the test loads into naga alone.
        let mut cache: ShaderCache<naga::Module, ()> =
            ShaderCache::new((), |_, source, _| match source {
                ShaderCacheSource::Wgsl(text) => naga::front::wgsl::parse_str(&text)
                    .map_err(|e| ShaderCacheError::ProcessShaderError(e.emit_to_string(&text))),
                ShaderCacheSource::SpirV(_) => panic!("a WESL shader composes to WGSL"),
            });
        let mut ids = std::collections::HashMap::new();
        for (i, (path, text)) in files.into_iter().enumerate() {
            let id = AssetId::<Shader>::Uuid {
                uuid: bevy::asset::uuid::Uuid::from_u128(i as u128 + 1),
            };
            cache.set_shader(id, Shader::from_wesl(text, format!("embedded://{path}")));
            ids.insert(path, id);
        }

        let storage =
            bevy::render::settings::WgpuLimits::default().max_storage_buffers_per_shader_stage;
        let with = |group: u32, defs: &[&'static str]| -> Vec<ShaderDefVal> {
            let mut v: Vec<ShaderDefVal> = defs.iter().map(|&d| d.into()).collect();
            v.push(ShaderDefVal::UInt("MATERIAL_BIND_GROUP".into(), group));
            v.push(ShaderDefVal::UInt(
                "AVAILABLE_STORAGE_BUFFER_BINDINGS".into(),
                storage,
            ));
            v
        };
        // The 3D mesh pipeline's layout for a mesh with positions, normals and both UV sets, as
        // every model, terrain and liquid mesh carries, beside the two defs `MeshPipeline` always
        // sets; bevy's material group is 3.
        const MESH: [&str; 8] = [
            "MESH_PIPELINE",
            "VERTEX_OUTPUT_INSTANCE_INDEX",
            "VERTEX_POSITIONS",
            "VERTEX_NORMALS",
            "VERTEX_UVS",
            "VERTEX_UVS_A",
            "VERTEX_UVS_B",
            "MAY_DISCARD",
        ];
        let mesh = |extra: &[&'static str]| {
            let mut d = MESH.to_vec();
            d.extend_from_slice(extra);
            with(3, &d)
        };
        let plain = |defs: &[&'static str]| -> Vec<ShaderDefVal> {
            defs.iter().map(|&d| d.into()).collect()
        };
        let cases: Vec<(&str, Vec<Vec<ShaderDefVal>>)> = vec![
            (
                "benilla_app/shaders/ui_quad.wesl",
                vec![
                    with(2, &["VERTEX_POSITIONS", "VERTEX_UVS", "VERTEX_COLORS"]),
                    with(2, &["VERTEX_POSITIONS", "VERTEX_NORMALS", "VERTEX_UVS"]),
                ],
            ),
            ("benilla_app/shaders/ui_gamma.wesl", vec![plain(&[])]),
            (
                "benilla_app/shaders/ui_node_gamma.wesl",
                vec![plain(&[]), plain(&["ANTI_ALIAS"])],
            ),
            ("benilla_app/shaders/ui_slice_gamma.wesl", vec![plain(&[])]),
            (
                "benilla_assets/shaders/wow_model.wesl",
                vec![
                    mesh(&[]),
                    mesh(&["VERTEX_COLORS"]),
                    mesh(&["VERTEX_COLORS", "WOW_RIG_SKIN"]),
                    mesh(&["WOW_MERGED_FADE", "WOW_MERGED_SLOT"]),
                    mesh(&["WOW_SKY_DEPTH", "WOW_WATER_CLIP"]),
                ],
            ),
            // Terrain chunks carry the alpha-map layer indices as vertex colours.
            (
                "benilla_assets/shaders/terrain.wesl",
                vec![mesh(&["VERTEX_COLORS"])],
            ),
            (
                "benilla_assets/shaders/liquid.wesl",
                vec![mesh(&[]), mesh(&["VERTEX_COLORS"])],
            ),
            ("benilla_assets/shaders/wdl.wesl", vec![mesh(&[])]),
            (
                "benilla_world/shaders/sky_vertex.wesl",
                vec![mesh(&[]), mesh(&["VERTEX_COLORS"])],
            ),
            ("benilla_world/shaders/sky.wesl", vec![mesh(&[])]),
            ("benilla_world/shaders/star.wesl", vec![mesh(&[])]),
            (
                "benilla_world/shaders/cloud.wesl",
                vec![mesh(&[]), mesh(&["VERTEX_COLORS"])],
            ),
            ("benilla_world/shaders/celestial.wesl", vec![mesh(&[])]),
            (
                "benilla_world/shaders/ffx_glow.wesl",
                vec![plain(&[]), plain(&["GAMMA_OUT"])],
            ),
            (
                "benilla_world/shaders/static_gx.wesl",
                vec![plain(&[]), plain(&["GX_CUTOUT"])],
            ),
            (
                "benilla_world/shaders/wow_effect.wesl",
                vec![
                    plain(&[]),
                    plain(&["DECAL_WORLD_CLIP", "BLEND_ADD"]),
                    plain(&["WOW_PARTICLE_FLAT"]),
                    plain(&["BLEND_OPAQUE", "EFFECT_LIT"]),
                    plain(&["BLEND_ALPHAKEY"]),
                    plain(&["BLEND_MULTIPLY"]),
                    plain(&["BLEND_MOD2X"]),
                ],
            ),
        ];
        let mut ours: Vec<&str> = ids
            .keys()
            .map(String::as_str)
            .filter(|p| p.starts_with("benilla_"))
            .collect();
        ours.sort_unstable();
        let mut named: Vec<&str> = cases.iter().map(|(p, _)| *p).collect();
        named.sort_unstable();
        assert_eq!(ours, named, "every embedded benilla shader has a case");
        for (pipeline, (path, variants)) in cases.into_iter().enumerate() {
            let id = *ids
                .get(path)
                .unwrap_or_else(|| panic!("{path} is embedded"));
            for defs in variants {
                let module = cache
                    .get(pipeline, id, &defs)
                    .unwrap_or_else(|e| panic!("{path} {defs:?}: does not compose: {e}"));
                let info = naga::valid::Validator::new(
                    naga::valid::ValidationFlags::all(),
                    naga::valid::Capabilities::default(),
                )
                .validate(&module)
                .unwrap_or_else(|e| panic!("{path} {defs:?}: does not validate: {e:?}"));
                #[cfg(target_os = "macos")]
                naga::back::msl::write_string(
                    &module,
                    &info,
                    // wgpu's Metal backend writes at the device's MSL 2.x; 1.0, the default, lacks
                    // the instance index every mesh shader reads.
                    &naga::back::msl::Options {
                        lang_version: (2, 4),
                        ..Default::default()
                    },
                    &naga::back::msl::PipelineOptions::default(),
                )
                .unwrap_or_else(|e| panic!("{path} {defs:?}: does not translate to MSL: {e:?}"));
                #[cfg(not(target_os = "macos"))]
                naga::back::spv::write_vec(
                    &module,
                    &info,
                    &naga::back::spv::Options::default(),
                    None,
                )
                .unwrap_or_else(|e| panic!("{path} {defs:?}: does not translate to SPIR-V: {e:?}"));
            }
        }
    }
}
