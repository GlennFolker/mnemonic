use std::io::{Error as IoError, ErrorKind as IoErrorKind};

use bevy::{
    asset::{
        io::Reader, AssetLoader, AsyncReadExt, LoadContext, LoadDirectError, LoadedAsset, ParseAssetPathError,
        ReadAssetBytesError,
    },
    prelude::*,
    utils::{hashbrown::hash_map::EntryRef, Entry, HashMap},
};
use nom::{
    error::{convert_error, VerboseError},
    Needed,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::def::{Obj, ObjCollection};
use crate::obj::parser::{parse_mtl, parse_obj, MtlDirective, ObjDirective};

#[derive(Error, Debug)]
pub enum ObjError {
    #[error("Vertex attribute index out of range: {index} >= {max}.")]
    OutOfRangeIndex { index: usize, max: usize },
    #[error("Duplicated object '{0}'.")]
    DuplicateObj(String),
    #[error("Duplicated material '{0}'.")]
    DuplicateMtl(String),
    #[error("Material with name '{0}' not found.")]
    UnknownMtl(String),
    #[error("Missing `{0}`.")]
    Missing(&'static str),
    #[error("Multiple `{0}` is not supported.")]
    Multiple(&'static str),
    #[error("Invalid preprocessor '{0}'.")]
    InvalidPreprocessor(String),
    #[error("Syntax error:\n{0}")]
    Syntax(String),
    #[error(transparent)]
    InvalidPath(#[from] ParseAssetPathError),
    #[error(transparent)]
    InvalidMtllib(#[from] ReadAssetBytesError),
    #[error(transparent)]
    InvalidImage(#[from] LoadDirectError),
    #[error(transparent)]
    Io(#[from] IoError),
}

#[derive(Copy, Clone, Serialize, Deserialize)]
pub struct ObjSettings {
    pub scale: f32,
    pub flip_v: bool,
}

impl Default for ObjSettings {
    #[inline]
    fn default() -> Self {
        Self {
            scale: 2.0,
            flip_v: true,
        }
    }
}

pub struct ObjLoader;
impl AssetLoader for ObjLoader {
    type Asset = ObjCollection;
    type Settings = ObjSettings;
    type Error = ObjError;

    async fn load<'a>(
        &'a self,
        reader: &'a mut Reader<'_>,
        settings: &'a Self::Settings,
        load_context: &'a mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        #[inline]
        fn parse_error(e: nom::Err<VerboseError<&str>>, data: &str) -> ObjError {
            ObjError::Syntax(match e {
                nom::Err::Error(e) | nom::Err::Failure(e) => convert_error(data, e),
                nom::Err::Incomplete(Needed::Unknown) => "Unexpected EoF.".into(),
                nom::Err::Incomplete(Needed::Size(size)) => format!("Unexpected EoF: Needed {size} more characters."),
            })
        }

        let &ObjSettings { scale, flip_v } = settings;

        let mut file = String::new();
        reader.read_to_string(&mut file).await?;

        let path = load_context.asset_path().clone();
        let mut objects = HashMap::<
            String,
            (
                Obj,
                Option<&str>,
                (Vec<Vec3>, Vec<Vec2>, Vec<Vec3>, HashMap<[usize; 3], usize>),
            ),
        >::new();
        let mut materials = None::<HashMap<String, (String, LoadedAsset<StandardMaterial>)>>;

        let mut cull = false;
        let mut current_obj = None;

        for dir in parse_obj::<VerboseError<&str>>(&file).map_err(|e| parse_error(e, &file))?.1 {
            match dir {
                ObjDirective::Comment(..) => continue,
                ObjDirective::Preprocess(pre) => {
                    for pre in pre {
                        match pre {
                            "check_cull" => cull = true,
                            invalid => return Err(ObjError::InvalidPreprocessor(invalid.into())),
                        }
                    }
                }
                ObjDirective::Mtllib(mtllib) => {
                    if materials.is_some() {
                        return Err(ObjError::Multiple("mtllib"))
                    }

                    let mtllib_path = path.resolve_embed(mtllib)?;
                    let file = String::from_utf8(load_context.read_asset_bytes(&mtllib_path).await?).map_err(|e| {
                        ReadAssetBytesError::Io {
                            path: mtllib_path.into(),
                            source: IoError::new(IoErrorKind::InvalidData, format!("{e}")),
                        }
                    })?;

                    let mut mtls = HashMap::<String, (StandardMaterial, LoadContext)>::new();
                    let mut current_mtl = None;

                    for dir in parse_mtl(&file).map_err(|e| parse_error(e, &file))?.1 {
                        match dir {
                            MtlDirective::Comment(..) => continue,
                            MtlDirective::Newmtl(newmtl) => {
                                current_mtl = match mtls.entry_ref(newmtl) {
                                    EntryRef::Occupied(..) => return Err(ObjError::DuplicateMtl(newmtl.into())),
                                    EntryRef::Vacant(e) => Some(e.insert((
                                        StandardMaterial {
                                            reflectance: 0.0,
                                            ..default()
                                        },
                                        load_context.begin_labeled_asset(),
                                    ))),
                                };
                            }
                            MtlDirective::MapKd(map_kd) => {
                                let (current_mtl, mtl_loader) = current_mtl.as_mut().ok_or(ObjError::Missing("mtllib"))?;
                                if current_mtl.base_color_texture.is_some() {
                                    return Err(ObjError::Multiple("map_Kd"))
                                }

                                let image = mtl_loader
                                    .loader()
                                    .direct()
                                    .load::<Image>(path.resolve_embed(map_kd)?)
                                    .await?;

                                current_mtl.base_color_texture = Some(mtl_loader.add_loaded_labeled_asset("map_Kd", image));
                            }
                        }
                    }

                    materials = Some({
                        let mut materials = HashMap::with_capacity(mtls.len());
                        for (id, (mtl, mtl_loader)) in mtls {
                            let mtl = mtl_loader.finish(mtl, None);
                            let label = format!("mtl:{id}");
                            materials.insert_unique_unchecked(id, (label, mtl));
                        }

                        materials
                    });
                }
                ObjDirective::O(o) => {
                    current_obj = match objects.entry_ref(o) {
                        EntryRef::Occupied(..) => return Err(ObjError::DuplicateObj(o.into())),
                        EntryRef::Vacant(e) => {
                            Some(e.insert((Obj::default(), None, (Vec::new(), Vec::new(), Vec::new(), HashMap::new()))))
                        }
                    };
                }
                ObjDirective::V(x, y, z) => {
                    let (.., vertices) = current_obj.as_mut().ok_or(ObjError::Missing("o"))?;
                    vertices.0.push(Vec3::new(x, y, z) * scale)
                }
                ObjDirective::Vt(u, v) => {
                    let (.., vertices) = current_obj.as_mut().ok_or(ObjError::Missing("o"))?;
                    vertices.1.push(Vec2::new(u, if flip_v { 1.0 - v } else { v }))
                }
                ObjDirective::Vn(x, y, z) => {
                    let (.., vertices) = current_obj.as_mut().ok_or(ObjError::Missing("o"))?;
                    vertices.2.push(Vec3::new(x, y, z))
                }
                ObjDirective::Usemtl(usemtl) => {
                    let (.., current_mtl, _) = current_obj.as_mut().ok_or(ObjError::Missing("o"))?;
                    if current_mtl.is_some() {
                        return Err(ObjError::Multiple("usemtl"))
                    }

                    *current_mtl = Some(usemtl);
                }
                ObjDirective::F(f) => {
                    #[inline]
                    fn vertex(
                        [position, uv, normal]: [usize; 3],
                        (positions, uvs, normals, vertices): &mut (
                            Vec<Vec3>,
                            Vec<Vec2>,
                            Vec<Vec3>,
                            HashMap<[usize; 3], usize>,
                        ),
                        obj_vertices: &mut Vec<(Vec3, Vec2, Vec3)>,
                    ) -> Result<usize, ObjError> {
                        match vertices.entry([position, uv, normal]) {
                            Entry::Occupied(vertex) => Ok::<usize, ObjError>(*vertex.get()),
                            Entry::Vacant(e) => {
                                let (position, uv, normal) = (
                                    positions.get(position).copied().ok_or(ObjError::OutOfRangeIndex {
                                        index: position,
                                        max: positions.len(),
                                    }),
                                    uvs.get(uv).copied().ok_or(ObjError::OutOfRangeIndex {
                                        index: uv,
                                        max: uvs.len(),
                                    }),
                                    normals.get(normal).copied().ok_or(ObjError::OutOfRangeIndex {
                                        index: normal,
                                        max: normals.len(),
                                    }),
                                );

                                let len = obj_vertices.len();
                                obj_vertices.push((position?, uv?, normal?));

                                Ok(*e.insert(len))
                            }
                        }
                    }

                    let (current_obj, .., builder) = current_obj.as_mut().ok_or(ObjError::Missing("o"))?;

                    let mut vertices = f.as_slice();
                    let &[a, mut b, mut c, ref rest @ ..] = vertices else {
                        unreachable!("`f` must have at least 3 vertices!")
                    };

                    vertices = rest;
                    let a = vertex(a, builder, &mut current_obj.vertices)?;

                    loop {
                        current_obj.faces.push([
                            vertex(b, builder, &mut current_obj.vertices)?,
                            vertex(c, builder, &mut current_obj.vertices)?,
                            a,
                        ]);

                        if let &[d, ref rest @ ..] = vertices {
                            b = c;
                            c = d;
                            vertices = rest;
                        } else {
                            break
                        }
                    }
                }
            }
        }

        let materials = {
            let from = materials.ok_or(ObjError::Missing("mtllib"))?;
            let mut materials = HashMap::with_capacity(from.len());
            for (id, (label, mtl)) in from {
                materials.insert_unique_unchecked(id, load_context.add_loaded_labeled_asset(label, mtl));
            }

            materials
        };

        let objects = objects
            .into_iter()
            .try_fold(Vec::new(), |mut objects, (id, (mut obj, mtl, ..))| {
                let mtl = mtl.ok_or(ObjError::Missing("usemtl"))?;
                obj.material = materials.get(mtl).ok_or_else(|| ObjError::UnknownMtl(mtl.into()))?.clone();

                if cull {
                    obj.calculate_culls();
                }

                objects.push(load_context.labeled_asset_scope(format!("obj:{id}"), |_| obj));
                Ok::<_, ObjError>(objects)
            })?;

        let materials = materials.into_values().collect();
        Ok(ObjCollection { objects, materials })
    }

    #[inline]
    fn extensions(&self) -> &[&str] {
        &["obj"]
    }
}
