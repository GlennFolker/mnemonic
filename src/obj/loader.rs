use std::{
    io::{Error as IoError, ErrorKind as IoErrorKind},
    num::{ParseFloatError, ParseIntError},
    str::{self, FromStr},
};

use bevy::{
    asset::{io::Reader, AssetLoader, AsyncReadExt, LoadContext, ParseAssetPathError, ReadAssetBytesError},
    prelude::*,
    utils::{hashbrown::hash_map::EntryRef, Entry, HashMap},
};
use thiserror::Error;

use super::def::{Mtl, Obj, ObjCollection};

#[derive(Error, Debug)]
pub enum ObjError {
    #[error("Expected identifier.")]
    ExpectedId,
    #[error("Expected floating-point number.")]
    ExpectedNum,
    #[error("Expected unsigned integer index.")]
    ExpectedIndex,
    #[error("Expected {{v}}/{{vt}}/{{vn}}")]
    ExpectedFace,
    #[error("Expected `usemtl` for object '{0}'.")]
    NoMtl(String),
    #[error(transparent)]
    InvalidNum(#[from] ParseFloatError),
    #[error(transparent)]
    InvalidIndex(#[from] ParseIntError),
    #[error("Vertex index may not be 0.")]
    ZeroIndex,
    #[error("Vertex attribute index out of range: {index} >= {max}.")]
    OutOfRangeIndex { index: usize, max: usize },
    #[error("Unexpected or unsupported token '{0}'.")]
    Unexpected(String),
    #[error("Multiple `{0}` is not supported.")]
    Multiple(&'static str),
    #[error("Missing `{0}`.")]
    Missing(&'static str),
    #[error("Duplicated object '{0}'.")]
    DuplicateObj(String),
    #[error("Duplicated material '{0}'.")]
    DuplicateMtl(String),
    #[error("Material with name '{0}' not found.")]
    UnknownMtl(String),
    #[error(transparent)]
    InvalidPath(#[from] ParseAssetPathError),
    #[error(transparent)]
    InvalidMtllib(#[from] ReadAssetBytesError),
    #[error(transparent)]
    Io(#[from] IoError),
}

pub struct ObjLoader;
impl AssetLoader for ObjLoader {
    type Asset = ObjCollection;
    type Settings = ();
    type Error = ObjError;

    async fn load<'a>(
        &'a self,
        reader: &'a mut Reader<'_>,
        _: &'a Self::Settings,
        load_context: &'a mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
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
        let mut materials = None::<HashMap<String, Handle<Mtl>>>;

        let mut cull = false;
        let mut current_obj = None;

        for line in file.lines() {
            let mut tokens = line
                .as_bytes()
                .split_inclusive(|&b| b == b'#' || b.is_ascii_whitespace())
                .filter_map(|s| {
                    let s = s.trim_ascii();
                    (!s.is_empty()).then_some(s)
                })
                .map(|s| unsafe { str::from_utf8_unchecked(s) });

            let first = tokens.next();
            let mut tokens = tokens.take_while(|&s| s != "#");

            match first {
                None => continue,
                Some("#") => match tokens.next() {
                    Some(">>>") => {
                        while let Some(directive) = tokens.next() {
                            match directive {
                                "check_cull" => cull = true,
                                _ => {}
                            }
                        }
                    }
                    _ => continue,
                },
                Some(command) => match command {
                    "mtllib" => {
                        if materials.is_some() {
                            return Err(ObjError::Multiple("mtllib"))
                        }

                        let mtllib_path = path.resolve_embed(tokens.next().ok_or(ObjError::ExpectedId)?)?;
                        let file = String::from_utf8(load_context.read_asset_bytes(&mtllib_path).await?).map_err(|e| {
                            ReadAssetBytesError::Io {
                                path: mtllib_path.into(),
                                source: IoError::new(IoErrorKind::InvalidData, format!("{e}")),
                            }
                        })?;

                        let mut mtls = HashMap::<String, Mtl>::new();
                        let mut current_mtl = None;

                        for line in file.lines() {
                            let mut tokens = line
                                .as_bytes()
                                .split_inclusive(|&b| b == b'#' || b.is_ascii_whitespace())
                                .filter_map(|s| {
                                    let s = s.trim_ascii();
                                    (!s.is_empty()).then_some(s)
                                })
                                .take_while(|&s| s != b"#")
                                .map(|s| unsafe { str::from_utf8_unchecked(s) });

                            match tokens.next() {
                                None => continue,
                                Some("newmtl") => {
                                    let id = tokens.next().ok_or(ObjError::ExpectedId)?;
                                    current_mtl = match mtls.entry_ref(id) {
                                        EntryRef::Occupied(..) => return Err(ObjError::DuplicateMtl(id.into())),
                                        EntryRef::Vacant(e) => Some(e.insert(Mtl::default())),
                                    };
                                }
                                Some("map_Kd") => {
                                    let current_mtl = current_mtl.as_mut().ok_or(ObjError::Missing("mtllib"))?;
                                    if current_mtl.diffuse_texture.is_some() {
                                        return Err(ObjError::Multiple("map_Kd"))
                                    }

                                    current_mtl.diffuse_texture = Some(
                                        load_context.load(path.resolve_embed(tokens.next().ok_or(ObjError::ExpectedId)?)?),
                                    );
                                }
                                Some(invalid) => return Err(ObjError::Unexpected(invalid.into())),
                            }
                        }

                        materials = Some({
                            let mut materials = HashMap::with_capacity(mtls.len());
                            for (id, mtl) in mtls {
                                let label = format!("mtl-{id}");
                                materials.insert_unique_unchecked(id, load_context.labeled_asset_scope(label, |_| mtl));
                            }

                            materials
                        });
                    }
                    "o" => {
                        let id = tokens.next().ok_or(ObjError::ExpectedId)?;
                        current_obj = match objects.entry_ref(id) {
                            EntryRef::Occupied(..) => return Err(ObjError::DuplicateObj(id.into())),
                            EntryRef::Vacant(e) => {
                                Some(e.insert((Obj::default(), None, (Vec::new(), Vec::new(), Vec::new(), HashMap::new()))))
                            }
                        };
                    }
                    "v" => {
                        let (.., vertices) = current_obj.as_mut().ok_or(ObjError::Missing("o"))?;
                        let [x, y, z] =
                            [tokens.next(), tokens.next(), tokens.next()].map(|v| v.ok_or(ObjError::ExpectedNum));
                        let [x, y, z] = [x?, y?, z?].map(f32::from_str);

                        vertices.0.push(Vec3::new(x?, y?, z?));
                        if let Some(invalid) = tokens.next() {
                            return Err(ObjError::Unexpected(invalid.into()))
                        }
                    }
                    "vt" => {
                        let (.., vertices) = current_obj.as_mut().ok_or(ObjError::Missing("o"))?;
                        let [u, v] = [tokens.next(), tokens.next()].map(|v| v.ok_or(ObjError::ExpectedNum));
                        let [u, v] = [u?, v?].map(f32::from_str);

                        vertices.1.push(Vec2::new(u?, 1.0 - v?));
                        if let Some(invalid) = tokens.next() {
                            return Err(ObjError::Unexpected(invalid.into()))
                        }
                    }
                    "vn" => {
                        let (.., vertices) = current_obj.as_mut().ok_or(ObjError::Missing("o"))?;
                        let [x, y, z] =
                            [tokens.next(), tokens.next(), tokens.next()].map(|v| v.ok_or(ObjError::ExpectedNum));
                        let [x, y, z] = [x?, y?, z?].map(f32::from_str);

                        vertices.2.push(Vec3::new(x?, y?, z?));
                        if let Some(invalid) = tokens.next() {
                            return Err(ObjError::Unexpected(invalid.into()))
                        }
                    }
                    "usemtl" => {
                        let (.., current_mtl, _) = current_obj.as_mut().ok_or(ObjError::Missing("o"))?;
                        if current_mtl.is_some() {
                            return Err(ObjError::Multiple("usemtl"))
                        }

                        *current_mtl = Some(tokens.next().ok_or(ObjError::ExpectedId)?);
                    }
                    "f" => {
                        #[inline]
                        fn face_attribs(f: &str) -> Result<[usize; 3], ObjError> {
                            let mut attribs = f.split('/');
                            let [a, b, c] =
                                [attribs.next(), attribs.next(), attribs.next()].map(|a| a.ok_or(ObjError::ExpectedIndex));

                            if let Some(a) = attribs.next() {
                                return Err(ObjError::Unexpected(a.into()))
                            }

                            let [a, b, c] = [a?, b?, c?].map(|a| usize::from_str(a));
                            let [a, b, c] = [a?, b?, c?].map(|a| a.checked_sub(1).ok_or(ObjError::ZeroIndex));
                            Ok([a?, b?, c?])
                        }

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
                        let [a, b, c] =
                            [tokens.next(), tokens.next(), tokens.next()].map(|f| f.ok_or(ObjError::ExpectedFace));
                        let [a, b, c] = [a?, b?, c?].map(face_attribs);
                        let [a, b, c] = [a?, b?, c?].map(|a| vertex(a, builder, &mut current_obj.vertices));

                        let [a, mut b, mut c] = [a?, b?, c?];
                        loop {
                            current_obj.faces.push([b, c, a]);
                            if let Some(f) = tokens.next() {
                                b = c;
                                c = vertex(face_attribs(f)?, builder, &mut current_obj.vertices)?;
                            } else {
                                break
                            }
                        }
                    }
                    invalid => return Err(ObjError::Unexpected(invalid.into())),
                },
            }
        }

        let materials = materials.ok_or(ObjError::Missing("mtllib"))?;
        let objects = objects
            .into_iter()
            .try_fold(Vec::new(), |mut objects, (id, (mut obj, mtl, ..))| {
                let mtl = mtl.ok_or(ObjError::Missing("usemtl"))?;
                obj.material = materials.get(mtl).ok_or_else(|| ObjError::UnknownMtl(mtl.into()))?.clone();

                objects.push(load_context.labeled_asset_scope(format!("obj-{id}"), |_| obj));
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
